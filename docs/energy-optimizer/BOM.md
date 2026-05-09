# Bill of Materials — Sovereign Home Energy Optimizer V0.1

Three variants. All use the same software stack; only the hardware scope differs.

> Prices in **GBP, ex-VAT**, mid-2025 estimates. Alternatives listed where a primary part
> may be out of stock. Assume ±15% volatility. Always buy 10–20% spare passive components.

---

## Variant A — Baseline (no battery, no solar)

**Goal**: Monitor whole-house + 2–4 circuits; shed deferrable loads at peak; < £200 all-in.

| # | Component | Primary Model | Alt / Notes | Qty | Unit £ | Line £ |
|---|-----------|--------------|-------------|-----|--------|--------|
| 1 | **Gateway/Host** | Raspberry Pi 4 Model B 4 GB | Orange Pi 5 (faster, ~£65); Intel N100 NUC (~£150) | 1 | 55 | 55 |
| 2 | **MicroSD** (OS) | SanDisk Endurance 32 GB A1 | Any A1/A2 rated card — avoid cheap no-name | 1 | 8 | 8 |
| 3 | **Sensor Node MCU** | ESP32-S3 DevKitC-1 (N16R8) | Lolin S3 Mini (~£4, fewer pins); WEMOS D1 Mini ESP32 | 2 | 6 | 12 |
| 4 | **Whole-house CT** | YHDC SCT-013-100 (100A:50mA) | SCT-013-000 (100A:1V output, no burden R needed) | 1 | 9 | 9 |
| 5 | **Circuit energy meter** | PZEM-004T v3.0 (10A inline) | PZEM-016 for RS485 bus (4+ meters on one pair) | 3 | 8 | 24 |
| 6 | **CT burden resistor** | 33Ω ¼W (for SCT-013-100) | Included in some SCT breakout boards | 2 | 0.10 | 0.20 |
| 7 | **Relay module** (5 V, 10 A) | HiLetgo 4-ch optocoupled relay | Sainsmart 8-ch; or individual SRD-05VDC-SL-C | 1 | 7 | 7 |
| 8 | **SSR** (if controlling >10 A loads) | Fotek SSR-40DA (40A, AC load) | Celduc SO865070; use with heat sink | 2 | 8 | 16 |
| 9 | **DIN-rail enclosure** | Camdenboss CBRS01 or equiv 6-way | Phoenix Contact; Gewiss; or 3D-print a lid | 1 | 18 | 18 |
| 10 | **MCB / fuse protection** | Eaton FAZ-C6/1 6A MCB (per relay output) | Any 6A B/C-curve MCB; Hager; Schneider iC60 | 4 | 4 | 16 |
| 11 | **Terminal blocks** | Phoenix PT 2.5/6-PVH-THR (6-way strip) | Wago 221 series levers for tool-free; 2.5 mm² rated | 2 | 6 | 12 |
| 12 | **PSU (12 V, 2 A)** | Mean Well HDR-60-12 DIN rail | Any CE-marked 12V 2A; use 5V for ESP32 direct | 1 | 14 | 14 |
| 13 | **DC-DC buck (12→5 V)** | Mini360 or LM2596 module | AMS1117-5V if < 800 mA; Mini360 more efficient | 1 | 2 | 2 |
| 14 | **Hookup wire** | 0.5 mm² stranded (signal); 1.5 mm² (power) | Buy 5 m rolls of each colour; silicone insulation preferred | — | 6 | 6 |
| 15 | **Connectors & misc** | JST-PH 2-pin, Dupont headers, heatshrink | —  | — | 5 | 5 |
| | | | **Variant A Total** | | | **~£204** |

**Procurement channels**: Raspberry Pi from [rpilocator.com](https://rpilocator.com) or The Pi Hut; PZEM from AliExpress (2–3 week lead) or Amazon UK; MCBs from TLC Electrical, CEF, or Screwfix.

---

## Variant B — Small battery buffer (3–5 kWh LFP)

**Goal**: Shift off-peak import, extend autonomy during short outages; ~£1,800–2,400 all-in.

Includes all Variant A components, plus:

| # | Component | Primary Model | Alt / Notes | Qty | Unit £ | Line £ |
|---|-----------|--------------|-------------|-----|--------|--------|
| B1 | **LFP Battery module** | 48V 100Ah (4.8 kWh) grade-A EVE/CATL cells with pre-assembled BMS | Seplos Mason 280Ah kit; DIY 16× 280Ah prismatic | 1 | 1,100 | 1,100 |
| B2 | **BMS** (if not included) | Daly Smart BMS 100A 16S 48V | JK BMS (better balancing); Seplos BMS | 1 | 55 | 55 |
| B3 | **Hybrid inverter/charger** | Growatt SPF 3000TL LVM (3kW, 48V, off-grid capable) | Victron MultiPlus-II 48/3000 (best support, ~£700); EASun 3kW | 1 | 350 | 350 |
| B4 | **DC breaker (battery side)** | 63A 2P DC MCB | ANL fuse 200A near battery; both needed | 1 | 18 | 18 |
| B5 | **Battery cable** | 25 mm² flexible (red/black), 2 m each | —  | 4 | 8 | 32 |
| B6 | **Shunt (for SoC metering)** | Victron SmartShunt 500A | Daly shunt (included with BMS if JK); Votronic | 1 | 55 | 55 |
| B7 | **Battery sensor node** | ESP32-S3 + INA226 (reads shunt) | ADS1115 + shunt; or Victron VE.Direct to USB | 1 | 10 | 10 |
| B8 | **Enclosure (battery)** | Steel IP55 600×600×200 mm | Purpose-built LFP cabinet; outdoor-rated needed if garage | 1 | 80 | 80 |
| B9 | **Installation labour** | Certified electrician (Part P / G83) | 4–8 h @ £60–90/h | — | — | ~£400 |
| | | | **Variant B extras** | | | **~£2,100** |
| | | | **Variant B Total (incl. A)** | | | **~£2,300** |

> **Critical**: LFP battery and inverter installation that connects to the DNO grid requires G98/G99 notification and a Part P certified electrician. Budget £300–500 for labour and DNO paperwork.

---

## Variant C — Solar + battery + full self-sufficiency stack

**Goal**: 3–5 kWp PV + 4.8 kWh battery; target >70% self-sufficiency in south UK; ~£4,500–6,000 all-in.

Includes all Variant B components, plus:

| # | Component | Primary Model | Alt / Notes | Qty | Unit £ | Line £ |
|---|-----------|--------------|-------------|-----|--------|--------|
| C1 | **PV panels** | 410 Wp mono PERC (e.g. JA Solar JAM54S30) | Longi HiMO5; Risen; any Tier-1 410–450 Wp | 8 | 120 | 960 |
| C2 | **MPPT charge controller** | Victron SmartSolar MPPT 100/50 | Epever Tracer 5420AN (budget); EPSolar | 1 | 180 | 180 |
| C3 | **DC isolator (PV string)** | 2-pole 1000 V DC 32A isolator | TUV-certified required; Lewden; Hager | 2 | 22 | 44 |
| C4 | **PV cable** | 6 mm² single-core solar cable (red/black), 20 m | —  | 1 | 45 | 45 |
| C5 | **MC4 connectors** | Staubli genuine MC4 pair (10-pack) | Do not use no-name MC4 on roof | 1 | 18 | 18 |
| C6 | **AC isolator / changeover** | Contactum 80A 4-pole changeover | Grid-tie auto-transfer required for grid-parallel | 1 | 35 | 35 |
| C7 | **Monitoring CT (PV output)** | PZEM-004T (second unit on PV AC output) | —  | 1 | 8 | 8 |
| C8 | **Scaffolding / roof work** | 1-day scaffold hire + electrician | MCS installer required for SEG/FIT eligibility | — | — | ~£800 |
| C9 | **MCS registration / warranty** | MCS-certified installer surcharge | Optional but needed for Smart Export Guarantee payments | — | — | ~£500 |
| | | | **Variant C extras** | | | **~£2,590** |
| | | | **Variant C Total (incl. A+B)** | | | **~£4,900** |

---

## MQTT Topic Map (reference)

```
home/sensors/{device_id}        →  {"watts":1234.5,"energy_wh":5678,"voltage":234,"current_a":5.3,"power_factor":0.97}
home/control/{device_id}        →  {"state": true}   ← published by Rust engine
home/override/{device_id}       →  {"state": false, "source": "ui"}   ← published by Tauri UI
home/audit/latest               →  AuditRecord JSON  ← published after every action
home/status                     →  {"ts": "…", "total_w": 1234.5}   ← heartbeat 1 Hz
```

---

## Audit Record Format (JSON, one per action)

```json
{
  "seq": 42,
  "ts": "2025-05-09T14:23:01.123Z",
  "prev_hash": "a3f1...b9c2",
  "action_json": "{\"device_id\":\"washing_machine\",\"kind\":\"TurnOff\",\"reason\":\"shed_priority_1\",\"total_watts_at_decision\":3712.5}",
  "record_hash": "e7d4...0f11",
  "hmac": "9c2a...3b8e"
}
```

Chain integrity: `record_hash = SHA256(prev_hash || action_json || ts_rfc3339)`  
Tamper evidence: `hmac = HMAC-SHA256(local_key, seq_le_bytes || record_hash)`

---

## 30-Day Milestone Plan

| Days | Milestone | Deliverable |
|------|-----------|-------------|
| 1–3  | Finalise BOM, place orders, set up dev environment | `config.toml` populated, RPi OS + Mosquitto running |
| 4–10 | ESP32 firmware → MQTT sensor data flowing | `esp32_pzem_node.ino` flashed; readings visible in MQTT Explorer |
| 11–17| Rust core: sensor ingestion, rule engine, relay output | `cargo run` turns off relay at threshold; audit DB populated |
| 18–23| Tauri UI: live chart, device table, manual override | UI running at `http://localhost:1420`; override works end-to-end |
| 24–27| Offline degradation, HMAC key generation, audit export | Verify: unplug internet → rules continue; audit exports to USB |
| 28–30| Integration test, BOM cost reconciliation, deploy docs | MVP signed off; electrician sign-off checklist complete |

---

## Safety Checklist (non-negotiable)

- [ ] All 230 V wiring done by a Part P certified electrician
- [ ] RCD + MCB on every relay output circuit
- [ ] SSR mounted on aluminium heat sink (min 50 cm²) with thermal paste
- [ ] Battery installation: ventilation, F-gas/fire-safe enclosure, temperature sensor
- [ ] DNO G98 notification submitted before energising battery/PV
- [ ] Firmware signed before flashing (SHA256 hash recorded in audit chain)
- [ ] HMAC key stored in encrypted vault; backed up offline
- [ ] First-run audit chain verify: `energy-optimizer --verify` shows 0 broken records
