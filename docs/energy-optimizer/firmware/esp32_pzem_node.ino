/*
 * ESP32-S3 Energy Sensor Node
 * Hardware: ESP32-S3 + PZEM-004T v3.0 (per channel)
 *
 * PZEM-004T wiring (TTL serial):
 *   PZEM TX  → ESP32 GPIO 16 (RX1)
 *   PZEM RX  → ESP32 GPIO 17 (TX1)
 *   PZEM VCC → 5V rail
 *   PZEM GND → GND
 *
 * MQTT payload published to home/sensors/{DEVICE_ID}:
 *   {"watts":1234.5,"energy_wh":5678.9,"voltage":234.1,"current_a":5.27,"power_factor":0.97}
 *
 * Libraries required (Arduino Library Manager):
 *   - PZEM004Tv30 by Jakub Mandula
 *   - PubSubClient by Nick O'Leary
 *   - ArduinoJson by Benoit Blanchon
 */

#include <WiFi.h>
#include <PubSubClient.h>
#include <PZEM004Tv30.h>
#include <ArduinoJson.h>

// ── Configuration ──────────────────────────────────────────────────────────
#define WIFI_SSID        "YourSSID"
#define WIFI_PASSWORD    "YourPassword"
#define MQTT_BROKER      "192.168.1.100"   // RPi / NUC running Mosquitto
#define MQTT_PORT        1883
#define DEVICE_ID        "clamp_kitchen"   // Unique per node
#define PUBLISH_TOPIC    "home/sensors/clamp_kitchen"
#define PUBLISH_INTERVAL_MS 1000           // 1-second readings

// PZEM serial: use HardwareSerial1 (GPIO 16/17)
PZEM004Tv30 pzem(Serial1, 16, 17);

WiFiClient   wifiClient;
PubSubClient mqtt(wifiClient);

unsigned long lastPublish = 0;

// ──────────────────────────────────────────────────────────────────────────
void setup() {
  Serial.begin(115200);
  WiFi.begin(WIFI_SSID, WIFI_PASSWORD);

  Serial.print("Connecting WiFi");
  while (WiFi.status() != WL_CONNECTED) {
    delay(500);
    Serial.print(".");
  }
  Serial.println("\nWiFi OK: " + WiFi.localIP().toString());

  mqtt.setServer(MQTT_BROKER, MQTT_PORT);
}

void reconnect() {
  while (!mqtt.connected()) {
    Serial.print("MQTT connect...");
    if (mqtt.connect(DEVICE_ID)) {
      Serial.println("OK");
    } else {
      Serial.printf("failed rc=%d — retry in 5s\n", mqtt.state());
      delay(5000);
    }
  }
}

void loop() {
  if (!mqtt.connected()) reconnect();
  mqtt.loop();

  if (millis() - lastPublish >= PUBLISH_INTERVAL_MS) {
    lastPublish = millis();

    float voltage  = pzem.voltage();
    float current  = pzem.current();
    float power    = pzem.power();
    float energy   = pzem.energy();   // kWh from PZEM, convert to Wh
    float pf       = pzem.pf();

    if (isnan(power)) {
      Serial.println("PZEM read error — skipping");
      return;
    }

    StaticJsonDocument<256> doc;
    doc["watts"]        = power;
    doc["energy_wh"]    = energy * 1000.0f;   // kWh → Wh
    doc["voltage"]      = voltage;
    doc["current_a"]    = current;
    doc["power_factor"] = pf;

    char buf[256];
    serializeJson(doc, buf);
    mqtt.publish(PUBLISH_TOPIC, buf);

    Serial.printf("[%s] %.1f W | %.2f A | %.1f V | PF %.2f\n",
                  DEVICE_ID, power, current, voltage, pf);
  }
}
