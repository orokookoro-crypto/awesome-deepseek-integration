/// Rule-based decision engine (Phase 1, no ML model required).
///
/// Phase 2 can swap `decide()` internals for GBDT inference by loading
/// a LightGBM/XGBoost model exported to ONNX and running it through
/// the `tract` crate — the Action type and caller interface stay identical.
use crate::config::{DeviceConfig, StrategyConfig};
use crate::sensor::SensorState;
use chrono::{DateTime, Local, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ActionKind {
    TurnOff,
    TurnOn,
    Hold,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub ts: DateTime<Utc>,
    pub device_id: String,
    pub kind: ActionKind,
    pub total_watts_at_decision: f32,
    pub threshold_used_w: f32,
    pub is_peak: bool,
    /// "manual_hold" | "below_threshold" | "shed_priority_{N}" | "restore"
    pub reason: String,
}

/// Per-device mutable state the engine needs between ticks.
#[derive(Debug, Default)]
pub struct DeviceState {
    pub is_on: bool,
    pub last_changed: Option<DateTime<Utc>>,
    /// Set when a user manually overrides the engine's decision.
    pub manual_hold_until: Option<DateTime<Utc>>,
}

pub struct DecisionEngine {
    strategy: StrategyConfig,
    devices: Vec<DeviceConfig>,
    device_state: HashMap<String, DeviceState>,
}

impl DecisionEngine {
    pub fn new(strategy: StrategyConfig, devices: Vec<DeviceConfig>) -> Self {
        let mut device_state = HashMap::new();
        for d in &devices {
            device_state.insert(d.id.clone(), DeviceState { is_on: true, ..Default::default() });
        }
        Self { strategy, devices, device_state }
    }

    /// Called once per control tick (default: every 1 s).
    /// Returns only Actions that represent a state change (nothing if Hold everywhere).
    pub fn decide(&mut self, sensor: &SensorState) -> Vec<Action> {
        let now = Utc::now();
        let local_hour = Local::now().hour() as u8;
        let is_peak = self.is_peak_hour(local_hour);
        let threshold = if is_peak {
            self.strategy.peak_threshold_w
        } else {
            self.strategy.offpeak_threshold_w
        };
        let total_w = sensor.total_watts();
        let mut actions = Vec::new();

        if total_w > threshold {
            // Shed loop: turn off sheddable devices in ascending priority order.
            let mut candidates: Vec<&DeviceConfig> = self
                .devices
                .iter()
                .filter(|d| d.sheddable)
                .collect();
            candidates.sort_by_key(|d| d.priority);

            let mut remaining = total_w;
            for dev in candidates {
                if remaining <= threshold {
                    break;
                }
                let state = self.device_state.get_mut(&dev.id).unwrap();
                if self.is_manual_hold(state, now) {
                    continue;
                }
                if self.is_min_off_elapsed(state, now) && state.is_on {
                    remaining -= sensor.watts_for(dev);
                    state.is_on = false;
                    state.last_changed = Some(now);
                    actions.push(Action {
                        ts: now,
                        device_id: dev.id.clone(),
                        kind: ActionKind::TurnOff,
                        total_watts_at_decision: total_w,
                        threshold_used_w: threshold,
                        is_peak,
                        reason: format!("shed_priority_{}", dev.priority),
                    });
                }
            }
        } else {
            // Restore loop: turn on sheddable devices in descending priority order.
            let mut candidates: Vec<&DeviceConfig> = self
                .devices
                .iter()
                .filter(|d| d.sheddable)
                .collect();
            candidates.sort_by_key(|d| std::cmp::Reverse(d.priority));

            for dev in candidates {
                let state = self.device_state.get_mut(&dev.id).unwrap();
                if self.is_manual_hold(state, now) {
                    continue;
                }
                if !state.is_on && self.is_min_off_elapsed(state, now) {
                    state.is_on = true;
                    state.last_changed = Some(now);
                    actions.push(Action {
                        ts: now,
                        device_id: dev.id.clone(),
                        kind: ActionKind::TurnOn,
                        total_watts_at_decision: total_w,
                        threshold_used_w: threshold,
                        is_peak,
                        reason: "restore".into(),
                    });
                }
            }
        }

        actions
    }

    /// Register a manual override from the UI. Engine will not touch this device
    /// for `manual_hold_seconds`.
    pub fn register_manual_override(&mut self, device_id: &str, state_on: bool) {
        let hold_until = Utc::now()
            + chrono::Duration::seconds(self.strategy.manual_hold_seconds as i64);
        if let Some(s) = self.device_state.get_mut(device_id) {
            s.is_on = state_on;
            s.manual_hold_until = Some(hold_until);
            s.last_changed = Some(Utc::now());
        }
    }

    fn is_peak_hour(&self, hour: u8) -> bool {
        self.strategy.peak_hours.iter().any(|[start, end]| hour >= *start && hour <= *end)
    }

    fn is_manual_hold(&self, state: &DeviceState, now: DateTime<Utc>) -> bool {
        state.manual_hold_until.map(|h| now < h).unwrap_or(false)
    }

    fn is_min_off_elapsed(&self, state: &DeviceState, now: DateTime<Utc>) -> bool {
        state
            .last_changed
            .map(|lc| {
                (now - lc).num_seconds() >= self.strategy.min_off_seconds as i64
            })
            .unwrap_or(true)
    }
}

// Bring Reverse into scope
use std::cmp::Reverse;
