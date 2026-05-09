/**
 * Sovereign Home Energy Optimizer — Tauri UI (React + TypeScript)
 *
 * Architecture:
 *  - Connects to local MQTT broker via WebSocket (ws://localhost:9001)
 *  - Displays live sensor readings, audit trail, device controls
 *  - Manual override publishes to home/override/{device_id}
 *  - All state is local — no cloud calls from the UI layer
 *
 * Prerequisites:
 *   npm install mqtt recharts @tauri-apps/api
 *   Mosquitto must have listener 9001 websockets enabled.
 */

import { useState, useEffect, useCallback } from "react";
import mqtt, { MqttClient } from "mqtt";
import {
  LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer,
} from "recharts";

// ── Types ──────────────────────────────────────────────────────────────────

interface DeviceState {
  id: string;
  label: string;
  watts: number;
  isOn: boolean;
  manualHeld: boolean;
  lastUpdated: string;
}

interface AuditRecord {
  seq: number;
  ts: string;
  action_json: string;
  record_hash: string;
}

interface PowerPoint {
  ts: string;
  total_w: number;
}

// ── Constants ──────────────────────────────────────────────────────────────

const BROKER_WS_URL = "ws://localhost:9001";
const MAX_HISTORY   = 120; // 2 min of 1-second points

// ── Component ─────────────────────────────────────────────────────────────

export default function App() {
  const [client,      setClient]      = useState<MqttClient | null>(null);
  const [connected,   setConnected]   = useState(false);
  const [devices,     setDevices]     = useState<Record<string, DeviceState>>({});
  const [auditLog,    setAuditLog]    = useState<AuditRecord[]>([]);
  const [powerHist,   setPowerHist]   = useState<PowerPoint[]>([]);
  const [totalWatts,  setTotalWatts]  = useState(0);

  // ── MQTT connection ──
  useEffect(() => {
    const c = mqtt.connect(BROKER_WS_URL, { clientId: "tauri-ui" });
    c.on("connect", () => {
      setConnected(true);
      c.subscribe(["home/status", "home/audit/latest", "home/sensors/#"]);
    });
    c.on("close",   () => setConnected(false));
    c.on("message", handleMessage(c));
    setClient(c);
    return () => { c.end(); };
  }, []);

  const handleMessage = (c: MqttClient) => (topic: string, payload: Buffer) => {
    try {
      const msg = JSON.parse(payload.toString());

      if (topic === "home/status") {
        const w = msg.total_w as number;
        setTotalWatts(w);
        setPowerHist(prev => [
          ...prev.slice(-MAX_HISTORY + 1),
          { ts: new Date(msg.ts).toLocaleTimeString(), total_w: w },
        ]);
      }

      if (topic === "home/audit/latest") {
        setAuditLog(prev => [msg as AuditRecord, ...prev].slice(0, 50));
      }

      if (topic.startsWith("home/sensors/")) {
        const devId = topic.replace("home/sensors/", "");
        setDevices(prev => ({
          ...prev,
          [devId]: {
            ...prev[devId],
            id: devId,
            label: prev[devId]?.label ?? devId,
            watts: msg.watts,
            isOn: msg.watts > 5,
            manualHeld: prev[devId]?.manualHeld ?? false,
            lastUpdated: new Date().toLocaleTimeString(),
          },
        }));
      }
    } catch { /* malformed payload */ }
  };

  // ── Manual override ──
  const sendOverride = useCallback((deviceId: string, stateOn: boolean) => {
    if (!client) return;
    const payload = JSON.stringify({ state: stateOn, source: "ui" });
    client.publish(`home/override/${deviceId}`, payload, { qos: 1 });
    setDevices(prev => ({
      ...prev,
      [deviceId]: { ...prev[deviceId], isOn: stateOn, manualHeld: true },
    }));
  }, [client]);

  // ── Render ──
  return (
    <div style={{ fontFamily: "monospace", padding: "1rem", maxWidth: 960, margin: "0 auto" }}>
      <header style={{ display: "flex", alignItems: "center", gap: "1rem", marginBottom: "1rem" }}>
        <h1 style={{ margin: 0 }}>⚡ Energy Optimizer</h1>
        <span style={{ color: connected ? "#0a0" : "#a00" }}>
          {connected ? "● LIVE" : "○ OFFLINE"}
        </span>
        <span style={{ marginLeft: "auto", fontSize: "1.5rem", fontWeight: "bold" }}>
          {totalWatts.toFixed(0)} W total
        </span>
      </header>

      {/* Power history chart */}
      <section style={{ marginBottom: "1.5rem" }}>
        <h2 style={{ marginBottom: "0.25rem" }}>Live Load (W)</h2>
        <ResponsiveContainer width="100%" height={160}>
          <LineChart data={powerHist}>
            <CartesianGrid strokeDasharray="3 3" />
            <XAxis dataKey="ts" tick={{ fontSize: 10 }} interval="preserveStartEnd" />
            <YAxis />
            <Tooltip />
            <Line type="monotone" dataKey="total_w" stroke="#2563eb" dot={false} strokeWidth={2} />
          </LineChart>
        </ResponsiveContainer>
      </section>

      {/* Device controls */}
      <section style={{ marginBottom: "1.5rem" }}>
        <h2>Devices</h2>
        <table style={{ width: "100%", borderCollapse: "collapse" }}>
          <thead>
            <tr style={{ borderBottom: "1px solid #ccc", textAlign: "left" }}>
              <th>Device</th><th>Watts</th><th>State</th><th>Override</th>
            </tr>
          </thead>
          <tbody>
            {Object.values(devices).map(dev => (
              <tr key={dev.id} style={{ borderBottom: "1px solid #eee" }}>
                <td>{dev.label} {dev.manualHeld && <span title="Manual hold active">🔒</span>}</td>
                <td>{dev.watts.toFixed(0)} W</td>
                <td style={{ color: dev.isOn ? "#0a0" : "#a00" }}>{dev.isOn ? "ON" : "OFF"}</td>
                <td>
                  <button onClick={() => sendOverride(dev.id, true)}  style={{ marginRight: 4 }}>ON</button>
                  <button onClick={() => sendOverride(dev.id, false)}>OFF</button>
                </td>
              </tr>
            ))}
            {Object.keys(devices).length === 0 && (
              <tr><td colSpan={4} style={{ padding: "0.5rem", color: "#888" }}>No sensor data yet…</td></tr>
            )}
          </tbody>
        </table>
      </section>

      {/* Audit trail */}
      <section>
        <h2>Audit Trail</h2>
        <div style={{ maxHeight: 240, overflowY: "auto", fontSize: "0.78rem" }}>
          {auditLog.map(r => (
            <div key={r.seq} style={{ borderBottom: "1px solid #eee", padding: "0.2rem 0" }}>
              <span style={{ color: "#888" }}>#{r.seq} {new Date(r.ts).toLocaleTimeString()} </span>
              <span>{r.action_json.slice(0, 120)}</span>
              <span style={{ color: "#aaa", marginLeft: 4 }}>[{r.record_hash.slice(0, 8)}…]</span>
            </div>
          ))}
        </div>
      </section>
    </div>
  );
}
