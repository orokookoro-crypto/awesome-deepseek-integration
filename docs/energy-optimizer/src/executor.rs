/// Action executor: sends MQTT control commands and optionally toggles GPIO.
///
/// MQTT payload published to control_topic: {"state": true}  or  {"state": false}
/// GPIO: Linux sysfs GPIO (works on RPi; swap to rppal crate for RPi-specific features).
use crate::config::DeviceConfig;
use crate::decision::{Action, ActionKind};
use anyhow::Result;
use rumqttc::AsyncClient;
use serde_json::json;
use std::collections::HashMap;
use tracing::{error, info};

pub struct Executor {
    mqtt: AsyncClient,
    device_map: HashMap<String, DeviceConfig>,
}

impl Executor {
    pub fn new(mqtt: AsyncClient, devices: Vec<DeviceConfig>) -> Self {
        let device_map = devices.into_iter().map(|d| (d.id.clone(), d)).collect();
        Self { mqtt, device_map }
    }

    /// Execute an action: MQTT publish + optional GPIO toggle.
    /// Returns the action JSON string (used by the audit chain).
    pub async fn execute(&self, action: &Action) -> Result<String> {
        let dev = match self.device_map.get(&action.device_id) {
            Some(d) => d,
            None => {
                error!("Unknown device_id: {}", action.device_id);
                anyhow::bail!("Unknown device_id: {}", action.device_id);
            }
        };

        let state_on = action.kind == ActionKind::TurnOn;
        let payload = json!({"state": state_on}).to_string();

        // Publish MQTT command
        self.mqtt
            .publish(
                &dev.control_topic,
                rumqttc::QoS::AtLeastOnce,
                false,
                payload.as_bytes(),
            )
            .await?;

        info!(
            "Executed {:?} on '{}' ({}) — {}W total — reason: {}",
            action.kind, dev.label, dev.id, action.total_watts_at_decision, action.reason
        );

        // Optional GPIO toggle (e.g. direct relay on Raspberry Pi)
        if let Some(pin) = dev.gpio_pin {
            if let Err(e) = toggle_gpio(pin, state_on) {
                error!("GPIO toggle failed for pin {pin}: {e}");
                // Non-fatal: MQTT command already sent
            }
        }

        Ok(serde_json::to_string(action)?)
    }
}

fn toggle_gpio(pin: u64, state_on: bool) -> Result<()> {
    use sysfs_gpio::{Direction, Pin};
    let p = Pin::new(pin);
    p.export()?;
    p.set_direction(Direction::Out)?;
    p.set_value(if state_on { 1 } else { 0 })?;
    Ok(())
}
