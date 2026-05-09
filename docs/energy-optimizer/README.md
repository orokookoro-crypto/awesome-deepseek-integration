# Sovereign Home Energy Optimizer V0.1

Offline-first, auditable home energy management system.  
Rust decision core · ESP32-S3 sensors · Tauri UI · local tamper-evident audit chain.

## Design principles

- **Local first** — all inference and control runs on-premises; no cloud dependency for core operation
- **Auditable** — every action written to a SHA-256 + HMAC chained log; exportable to USB
- **Safe degradation** — loss of internet → rule-based fallback; loss of MQTT → sensor nodes cache and retry
- **Minimal cloud surface** — optional tariff API fetch (Octopus Agile); everything else is LAN-only
- **Signed firmware** — ESP32 OTA and Rust binary both carry SHA-256 checksums verified before install

## Repository layout

```
docs/energy-optimizer/
├── BOM.md                   ← Full bill of materials, 3 variants, GBP estimates
├── config.example.toml      ← Annotated configuration template
├── Cargo.toml               ← Rust workspace manifest
├── src/
│   ├── main.rs              ← Async main: MQTT event loop + control tick
│   ├── config.rs            ← Typed configuration structs
│   ├── sensor.rs            ← SensorReading types, payload parser, featurizer
│   ├── decision.rs          ← Rule-based decision engine (Phase 1)
│   ├── executor.rs          ← MQTT publish + GPIO toggle
│   ├── audit.rs             ← Tamper-evident local audit chain (sled + SHA-256 + HMAC)
│   └── tariff.rs            ← Tariff fetch/cache (Octopus Agile compatible)
├── firmware/
│   └── esp32_pzem_node.ino  ← Arduino sketch: ESP32-S3 + PZEM-004T → MQTT
└── tauri/
    └── src/
        └── App.tsx          ← React UI: live chart, device table, audit trail, overrides
```

## Quick start

### 1. Broker

```bash
sudo apt install mosquitto mosquitto-clients
# Add to /etc/mosquitto/conf.d/local.conf:
#   listener 1883
#   listener 9001
#   protocol websockets
#   allow_anonymous true   # replace with password_file in production
sudo systemctl restart mosquitto
```

### 2. Sensor node firmware

Open `firmware/esp32_pzem_node.ino` in Arduino IDE.  
Install libraries: `PZEM004Tv30`, `PubSubClient`, `ArduinoJson`.  
Edit `WIFI_SSID`, `WIFI_PASSWORD`, `MQTT_BROKER`, `DEVICE_ID`.  
Flash to each ESP32-S3 node.

### 3. Rust core

```bash
# Generate HMAC key
openssl rand -hex 32

cp config.example.toml config.toml
# Edit config.toml: set hmac_key_hex, MQTT host, device list

cargo build --release
sudo ./target/release/energy-optimizer config.toml
```

### 4. Tauri UI

```bash
cd tauri
npm install
npm run tauri dev
```

### 5. Verify audit chain integrity

```bash
# Built-in verify mode (planned CLI flag):
RUST_LOG=info ./target/release/energy-optimizer --verify config.toml
# Output: "Verified 1024 records — chain intact"
```

## MQTT topic map

| Topic | Direction | Payload |
|-------|-----------|---------|
| `home/sensors/{id}` | ESP32 → core | `{"watts":1234.5,"energy_wh":5678,"voltage":234,"current_a":5.3,"power_factor":0.97}` |
| `home/control/{id}` | core → relay | `{"state":true}` |
| `home/override/{id}` | UI → core | `{"state":false,"source":"ui"}` |
| `home/audit/latest` | core → UI | `AuditRecord` JSON |
| `home/status` | core → UI | `{"ts":"…","total_w":1234.5}` |

## Phase 2: ML upgrade path

`decision.rs::DecisionEngine::decide()` returns `Vec<Action>` — the interface is stable.  
To add GBDT inference:

1. Train a LightGBM model on 30+ days of sensor logs
2. Export to ONNX
3. Add `tract` crate dependency
4. Replace the rule block in `decide()` with `tract` inference
5. Retain rule-based fallback for when model confidence < threshold

No changes needed to `main.rs`, `executor.rs`, `audit.rs`, or the UI.

## Security notes

- HMAC key must be 32 random bytes; generated once at install with `openssl rand -hex 32`
- Store key in encrypted vault (e.g. `age`, `pass`, or RPi TPM-backed keystore)
- Back up key offline; loss of key means audit records cannot be re-verified
- Replace `allow_anonymous true` in Mosquitto with password file + TLS for multi-occupancy
- All 230 V wiring must be performed by a Part P certified electrician (UK)
- DNO G98 notification required before energising battery or PV export

## Licence

MIT — see project root LICENSE file.
