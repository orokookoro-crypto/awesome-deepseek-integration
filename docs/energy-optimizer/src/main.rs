mod audit;
mod config;
mod decision;
mod executor;
mod sensor;
mod tariff;

use anyhow::Result;
use config::Config;
use decision::DecisionEngine;
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};
use sensor::{parse_payload, SensorState};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

/// MQTT topic hierarchy
/// home/sensors/{device_id}        → SensorReading JSON (published by ESP32 nodes)
/// home/control/{device_id}        → {"state": true/false} (published by executor)
/// home/override/{device_id}       → {"state": true/false, "source": "ui"} (from Tauri UI)
/// home/audit/latest               → latest AuditRecord JSON (published after each action)
/// home/status                     → heartbeat {"ts": "...", "total_w": 1234.5}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("energy_optimizer=info".parse()?))
        .init();

    let cfg_path = std::env::args().nth(1).unwrap_or_else(|| "config.toml".into());
    let cfg = Config::load(&cfg_path).unwrap_or_else(|e| {
        warn!("Could not load {cfg_path}: {e} — using defaults");
        Config::default()
    });

    info!("Starting Energy Optimizer v{}", env!("CARGO_PKG_VERSION"));

    // --- Audit chain ---
    let audit = Arc::new(audit::AuditChain::open(
        &cfg.audit.db_path,
        &cfg.audit.hmac_key_hex,
        cfg.audit.export_path.clone(),
        cfg.audit.export_interval,
    )?);

    // --- Sensor state (shared across tasks) ---
    let sensor_state = Arc::new(SensorState::new());

    // --- MQTT client ---
    let mut mqtt_opts = MqttOptions::new(
        &cfg.mqtt.client_id,
        &cfg.mqtt.host,
        cfg.mqtt.port,
    );
    mqtt_opts.set_keep_alive(Duration::from_secs(30));
    if let (Some(u), Some(p)) = (&cfg.mqtt.username, &cfg.mqtt.password) {
        mqtt_opts.set_credentials(u, p);
    }
    let (client, mut eventloop) = AsyncClient::new(mqtt_opts, 64);
    let client = Arc::new(client);

    // Subscribe to sensor topics and UI override topic
    for dev in &cfg.devices {
        client.subscribe(&dev.sensor_topic, QoS::AtMostOnce).await?;
        let override_topic = format!("home/override/{}", dev.id);
        client.subscribe(&override_topic, QoS::AtLeastOnce).await?;
    }

    // --- Decision engine (behind a Mutex because it holds mutable device state) ---
    let engine = Arc::new(Mutex::new(DecisionEngine::new(
        cfg.strategy.clone(),
        cfg.devices.clone(),
    )));

    // --- Executor ---
    let exec = Arc::new(executor::Executor::new(
        client.clone().as_ref().clone(),
        cfg.devices.clone(),
    ));

    // Tariff refresh task (once per day, non-blocking)
    if let Some(api) = cfg.grid.tariff_api.clone() {
        let cache = cfg.grid.tariff_cache_path.clone();
        tokio::spawn(async move {
            loop {
                if let Err(e) = tariff::refresh_tariff(&api, &cache).await {
                    warn!("Tariff refresh failed: {e}");
                }
                tokio::time::sleep(Duration::from_secs(86_400)).await;
            }
        });
    }

    // Control loop task (1 second tick)
    {
        let sensor_state = sensor_state.clone();
        let engine = engine.clone();
        let exec = exec.clone();
        let audit = audit.clone();
        let client = client.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;

                let actions = engine.lock().await.decide(&sensor_state);
                for action in actions {
                    match exec.execute(&action).await {
                        Ok(action_json) => {
                            match audit.append(&action_json) {
                                Ok(rec) => {
                                    let _ = client
                                        .publish(
                                            "home/audit/latest",
                                            QoS::AtMostOnce,
                                            false,
                                            serde_json::to_vec(&rec).unwrap_or_default(),
                                        )
                                        .await;
                                }
                                Err(e) => error!("Audit append failed: {e}"),
                            }
                        }
                        Err(e) => error!("Executor error: {e}"),
                    }
                }

                // Heartbeat
                let hb = serde_json::json!({
                    "ts": chrono::Utc::now().to_rfc3339(),
                    "total_w": sensor_state.total_watts()
                });
                let _ = client
                    .publish("home/status", QoS::AtMostOnce, false, hb.to_string().as_bytes())
                    .await;
            }
        });
    }

    // MQTT event loop — sensor ingestion + UI override handling
    loop {
        match eventloop.poll().await {
            Ok(Event::Incoming(Packet::Publish(p))) => {
                let topic = p.topic.as_str();

                // Sensor reading
                if let Some(dev_id) = sensor_topic_device_id(topic, &cfg.devices) {
                    match parse_payload(&dev_id, &p.payload) {
                        Ok(reading) => sensor_state.update(reading),
                        Err(e) => warn!("Bad sensor payload on {topic}: {e}"),
                    }
                }

                // Manual override from UI
                if let Some(dev_id) = override_topic_device_id(topic) {
                    #[derive(serde::Deserialize)]
                    struct OverrideCmd { state: bool }
                    if let Ok(cmd) = serde_json::from_slice::<OverrideCmd>(&p.payload) {
                        engine.lock().await.register_manual_override(&dev_id, cmd.state);
                        // Record the override in the audit chain
                        let ov_json = serde_json::json!({
                            "type": "manual_override",
                            "device_id": dev_id,
                            "state": cmd.state,
                            "ts": chrono::Utc::now().to_rfc3339()
                        }).to_string();
                        if let Err(e) = audit.append(&ov_json) {
                            error!("Audit override record failed: {e}");
                        }
                        info!("Manual override: {dev_id} → {}", cmd.state);
                    }
                }
            }
            Ok(_) => {}
            Err(e) => {
                error!("MQTT event loop error: {e}");
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

fn sensor_topic_device_id(topic: &str, devices: &[config::DeviceConfig]) -> Option<String> {
    devices.iter().find(|d| d.sensor_topic == topic).map(|d| d.id.clone())
}

fn override_topic_device_id(topic: &str) -> Option<String> {
    topic.strip_prefix("home/override/").map(str::to_owned)
}
