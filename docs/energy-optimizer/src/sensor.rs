use crate::config::DeviceConfig;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// One timestamped power reading from a single channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorReading {
    pub ts: DateTime<Utc>,
    pub device_id: String,
    /// Instantaneous active power in watts
    pub watts: f32,
    /// Cumulative energy in Wh (from PZEM energy register, if available)
    pub energy_wh: Option<f32>,
    /// Voltage (V) if available
    pub voltage: Option<f32>,
    /// Current (A) if available
    pub current_a: Option<f32>,
    /// Power factor if available
    pub power_factor: Option<f32>,
}

/// Thread-safe snapshot of the latest sensor readings, keyed by device_id.
#[derive(Debug, Clone, Default)]
pub struct SensorState(pub Arc<RwLock<HashMap<String, SensorReading>>>);

impl SensorState {
    pub fn new() -> Self {
        Self(Arc::new(RwLock::new(HashMap::new())))
    }

    pub fn update(&self, reading: SensorReading) {
        if let Ok(mut guard) = self.0.write() {
            guard.insert(reading.device_id.clone(), reading);
        }
    }

    pub fn snapshot(&self) -> HashMap<String, SensorReading> {
        self.0.read().map(|g| g.clone()).unwrap_or_default()
    }

    /// Total household watts across all channels.
    pub fn total_watts(&self) -> f32 {
        self.snapshot().values().map(|r| r.watts).sum()
    }

    /// Watts for a specific device_id, falling back to rated_watts if stale/missing.
    pub fn watts_for(&self, device: &DeviceConfig) -> f32 {
        self.snapshot()
            .get(&device.id)
            .map(|r| r.watts)
            .unwrap_or(device.rated_watts)
    }
}

/// Parse a raw MQTT payload into a SensorReading.
/// Accepts both PZEM-style full JSON and simple `{"watts": 1234.5}` payloads.
pub fn parse_payload(device_id: &str, payload: &[u8]) -> Result<SensorReading> {
    #[derive(Deserialize)]
    struct Raw {
        watts: f32,
        energy_wh: Option<f32>,
        voltage: Option<f32>,
        current_a: Option<f32>,
        power_factor: Option<f32>,
    }
    let raw: Raw = serde_json::from_slice(payload)?;
    Ok(SensorReading {
        ts: Utc::now(),
        device_id: device_id.to_owned(),
        watts: raw.watts,
        energy_wh: raw.energy_wh,
        voltage: raw.voltage,
        current_a: raw.current_a,
        power_factor: raw.power_factor,
    })
}

/// Featurize the current sensor state for the decision model.
/// Returns a flat Vec<f32> suitable for GBDT inference.
/// Order: [total_w, hour_of_day, is_peak, per_device_watts...]
pub fn featurize(state: &SensorState, devices: &[DeviceConfig], hour: u8, is_peak: bool) -> Vec<f32> {
    let snap = state.snapshot();
    let mut features = vec![state.total_watts(), hour as f32, is_peak as u8 as f32];
    for d in devices {
        features.push(snap.get(&d.id).map(|r| r.watts).unwrap_or(0.0));
    }
    features
}
