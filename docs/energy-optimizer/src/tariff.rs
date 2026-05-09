/// Electricity tariff fetcher with offline fallback.
///
/// Supports Octopus Agile-style API (half-hourly p/kWh slots).
/// If the API is unreachable the last cached JSON is used; if no cache
/// exists, compile-time fallback values from Config are used.
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TariffSlot {
    pub valid_from: DateTime<Utc>,
    pub valid_to: DateTime<Utc>,
    pub pence_per_kwh: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TariffCache {
    pub fetched_at: DateTime<Utc>,
    pub slots: Vec<TariffSlot>,
}

/// Return the pence/kWh rate for `now`, from cache or fallback.
pub fn current_rate(
    cache_path: &str,
    fallback_peak: f32,
    fallback_offpeak: f32,
    peak_hours: &[[u8; 2]],
) -> f32 {
    let now = Utc::now();
    if let Ok(rate) = rate_from_cache(cache_path, now) {
        return rate;
    }
    let local_hour = chrono::Local::now().hour() as u8;
    let is_peak = peak_hours.iter().any(|[s, e]| local_hour >= *s && local_hour <= *e);
    if is_peak { fallback_peak } else { fallback_offpeak }
}

fn rate_from_cache(cache_path: &str, now: DateTime<Utc>) -> Result<f32> {
    let raw = std::fs::read_to_string(cache_path)?;
    let cache: TariffCache = serde_json::from_str(&raw)?;
    let slot = cache
        .slots
        .iter()
        .find(|s| now >= s.valid_from && now < s.valid_to)
        .ok_or_else(|| anyhow::anyhow!("No tariff slot for now"))?;
    Ok(slot.pence_per_kwh)
}

/// Fetch half-hourly slots from Octopus Agile API and write to cache file.
/// Call once per day from a background task.
pub async fn refresh_tariff(api_url: &str, cache_path: &str) -> Result<()> {
    #[derive(Deserialize)]
    struct ApiResponse {
        results: Vec<ApiSlot>,
    }
    #[derive(Deserialize)]
    struct ApiSlot {
        valid_from: DateTime<Utc>,
        valid_to: DateTime<Utc>,
        value_inc_vat: f32,
    }

    let resp = reqwest::get(api_url).await?.json::<ApiResponse>().await?;
    let slots: Vec<TariffSlot> = resp
        .results
        .into_iter()
        .map(|s| TariffSlot {
            valid_from: s.valid_from,
            valid_to: s.valid_to,
            pence_per_kwh: s.value_inc_vat,
        })
        .collect();

    let cache = TariffCache { fetched_at: Utc::now(), slots };
    if let Some(parent) = Path::new(cache_path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(cache_path, serde_json::to_string_pretty(&cache)?)?;
    tracing::info!("Tariff cache refreshed: {} slots", cache.slots.len());
    Ok(())
}

use chrono::Timelike;
