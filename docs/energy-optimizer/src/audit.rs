/// Local tamper-evident audit chain.
///
/// Each record is: SHA-256(prev_hash || action_json || timestamp_iso)
/// additionally HMAC-SHA256 signed with a local install key so records
/// cannot be silently regenerated without the key.
///
/// Storage: sled embedded key-value store (no external DB process required).
/// Export: periodic JSON file dump to user-controlled path (e.g. USB drive).
use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sled::Db;
use std::path::Path;

type HmacSha256 = Hmac<sha2::Sha256>;

const GENESIS_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const KEY_LATEST_HASH: &[u8] = b"__latest_hash__";
const KEY_RECORD_COUNT: &[u8] = b"__record_count__";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    pub seq: u64,
    pub ts: DateTime<Utc>,
    pub prev_hash: String,
    pub action_json: String,
    pub record_hash: String,
    pub hmac: String,
}

pub struct AuditChain {
    db: Db,
    hmac_key: Vec<u8>,
    export_path: Option<String>,
    export_interval: u32,
}

impl AuditChain {
    pub fn open(
        db_path: impl AsRef<Path>,
        hmac_key_hex: &str,
        export_path: Option<String>,
        export_interval: u32,
    ) -> Result<Self> {
        let key = hex::decode(hmac_key_hex)?;
        if key.len() != 32 {
            bail!("HMAC key must be 32 bytes (64 hex chars)");
        }
        let db = sled::open(db_path)?;
        Ok(Self { db, hmac_key: key, export_path, export_interval })
    }

    /// Append an action to the chain. Returns the new record.
    pub fn append(&self, action_json: &str) -> Result<AuditRecord> {
        let seq = self.next_seq()?;
        let prev_hash = self.latest_hash()?;
        let ts = Utc::now();
        let ts_str = ts.to_rfc3339();

        // SHA-256 hash of (prev_hash || action_json || timestamp)
        let mut hasher = Sha256::new();
        hasher.update(prev_hash.as_bytes());
        hasher.update(action_json.as_bytes());
        hasher.update(ts_str.as_bytes());
        let record_hash = hex::encode(hasher.finalize());

        // HMAC over (seq || record_hash)
        let mut mac = HmacSha256::new_from_slice(&self.hmac_key)?;
        mac.update(seq.to_le_bytes().as_ref());
        mac.update(record_hash.as_bytes());
        let hmac_hex = hex::encode(mac.finalize().into_bytes());

        let record = AuditRecord {
            seq,
            ts,
            prev_hash,
            action_json: action_json.to_owned(),
            record_hash: record_hash.clone(),
            hmac: hmac_hex,
        };

        let key = format!("rec:{:020}", seq);
        self.db.insert(key.as_bytes(), serde_json::to_vec(&record)?.as_slice())?;
        self.db.insert(KEY_LATEST_HASH, record_hash.as_bytes())?;
        self.db.insert(KEY_RECORD_COUNT, &seq.to_le_bytes())?;
        self.db.flush()?;

        if seq % self.export_interval as u64 == 0 {
            if let Err(e) = self.export_json() {
                tracing::warn!("Audit export failed: {e}");
            }
        }

        Ok(record)
    }

    /// Verify the entire chain from genesis. Returns (total_records, first_broken_seq).
    pub fn verify(&self) -> Result<(u64, Option<u64>)> {
        let mut prev_hash = GENESIS_HASH.to_owned();
        let mut count = 0u64;

        for item in self.db.scan_prefix(b"rec:") {
            let (_, v) = item?;
            let rec: AuditRecord = serde_json::from_slice(&v)?;

            let mut hasher = Sha256::new();
            hasher.update(rec.prev_hash.as_bytes());
            hasher.update(rec.action_json.as_bytes());
            hasher.update(rec.ts.to_rfc3339().as_bytes());
            let expected_hash = hex::encode(hasher.finalize());

            if expected_hash != rec.record_hash || rec.prev_hash != prev_hash {
                return Ok((count, Some(rec.seq)));
            }

            let mut mac = HmacSha256::new_from_slice(&self.hmac_key)?;
            mac.update(rec.seq.to_le_bytes().as_ref());
            mac.update(rec.record_hash.as_bytes());
            let expected_hmac = hex::encode(mac.finalize().into_bytes());
            if expected_hmac != rec.hmac {
                return Ok((count, Some(rec.seq)));
            }

            prev_hash = rec.record_hash;
            count += 1;
        }
        Ok((count, None))
    }

    /// Export all records to a JSON file in export_path.
    pub fn export_json(&self) -> Result<()> {
        let dir = match &self.export_path {
            Some(p) => p.clone(),
            None => return Ok(()),
        };
        std::fs::create_dir_all(&dir)?;
        let count = self.record_count()?;
        let path = format!("{}/audit_{}.json", dir, count);

        let mut records = Vec::new();
        for item in self.db.scan_prefix(b"rec:") {
            let (_, v) = item?;
            let rec: AuditRecord = serde_json::from_slice(&v)?;
            records.push(rec);
        }
        let json = serde_json::to_string_pretty(&records)?;
        std::fs::write(&path, json)?;
        tracing::info!("Audit chain exported to {path}");
        Ok(())
    }

    fn latest_hash(&self) -> Result<String> {
        match self.db.get(KEY_LATEST_HASH)? {
            Some(v) => Ok(String::from_utf8(v.to_vec())?),
            None => Ok(GENESIS_HASH.to_owned()),
        }
    }

    fn next_seq(&self) -> Result<u64> {
        Ok(self.record_count()? + 1)
    }

    fn record_count(&self) -> Result<u64> {
        match self.db.get(KEY_RECORD_COUNT)? {
            Some(v) => Ok(u64::from_le_bytes(v.as_ref().try_into()?)),
            None => Ok(0),
        }
    }
}
