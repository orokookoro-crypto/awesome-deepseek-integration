use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub mqtt: MqttConfig,
    pub audit: AuditConfig,
    pub strategy: StrategyConfig,
    pub devices: Vec<DeviceConfig>,
    pub grid: GridConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttConfig {
    pub host: String,
    pub port: u16,
    pub client_id: String,
    /// Optional username/password for local broker auth
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    /// Path to the local sled DB for the audit chain
    pub db_path: String,
    /// Directory for periodic JSON export (USB mount point etc.)
    pub export_path: Option<String>,
    /// Export every N actions
    pub export_interval: u32,
    /// HMAC key hex string (32 bytes = 64 hex chars). Generate once at install.
    pub hmac_key_hex: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyConfig {
    /// Peak rate hours (24h, inclusive), e.g. [[7,9],[16,21]]
    pub peak_hours: Vec<[u8; 2]>,
    /// Off-peak power threshold before shedding (watts)
    pub offpeak_threshold_w: f32,
    /// Peak power threshold before shedding (watts)
    pub peak_threshold_w: f32,
    /// Minimum off duration seconds per device to avoid relay chatter
    pub min_off_seconds: u64,
    /// If true, never override a manual user action for this many seconds
    pub manual_hold_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    pub id: String,
    pub label: String,
    /// MQTT topic for current power reading (watts float)
    pub sensor_topic: String,
    /// MQTT topic to publish control command {"state": true/false}
    pub control_topic: String,
    /// GPIO pin number (None = MQTT-only control, no local GPIO)
    pub gpio_pin: Option<u64>,
    /// Device priority 1 (lowest, shed first) – 10 (highest, shed last)
    pub priority: u8,
    /// Watts drawn when on (for estimation when sensor unavailable)
    pub rated_watts: f32,
    pub sheddable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridConfig {
    /// Agile Octopus / other tariff API endpoint (offline fallback if None)
    pub tariff_api: Option<String>,
    /// Cache file path for tariff data
    pub tariff_cache_path: String,
    /// Peak p/kWh — used when API unreachable
    pub fallback_peak_pence: f32,
    /// Off-peak p/kWh
    pub fallback_offpeak_pence: f32,
}

impl Config {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let raw = fs::read_to_string(path)?;
        let cfg: Config = toml::from_str(&raw)?;
        Ok(cfg)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mqtt: MqttConfig {
                host: "localhost".into(),
                port: 1883,
                client_id: "energy-optimizer".into(),
                username: None,
                password: None,
            },
            audit: AuditConfig {
                db_path: "/var/lib/energy-optimizer/audit.db".into(),
                export_path: Some("/media/usb/energy-audit".into()),
                export_interval: 100,
                hmac_key_hex: "0".repeat(64),
            },
            strategy: StrategyConfig {
                peak_hours: vec![[7, 9], [16, 21]],
                offpeak_threshold_w: 3500.0,
                peak_threshold_w: 2000.0,
                min_off_seconds: 300,
                manual_hold_seconds: 1800,
            },
            devices: vec![],
            grid: GridConfig {
                tariff_api: None,
                tariff_cache_path: "/var/lib/energy-optimizer/tariff.json".into(),
                fallback_peak_pence: 28.5,
                fallback_offpeak_pence: 7.5,
            },
        }
    }
}
