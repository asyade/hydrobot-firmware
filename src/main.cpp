#include <Arduino.h>
#include <Stepper.h>
#include <Filter.h>
#include <EEPROM.h>

#define STEPS 200

#define SETTINGS_MAGIC 0x4242
#define TDS_SAMPLE_INTERVAL 100
#define TDS_1_PIN 11
#define TDS_2_PIN 12

#define WATER_VALE_STEPPER_SW 56
#define WATER_VALVE_DELTA 3800

#define STATUS_OPENED 1
#define STATUS_OPENING 2
#define STATUS_CLOSING 3
#define STATUS_CLOSED 0

#define STATUS_UNKNOWN -1

#define RES_PROCESS_DONE "PROCESS DONE"
#define RES_PROCESS_BUSY "PROCESS BUSY"
#define RES_CAL_REQUIRED "CALIBRATION REQUIRED"
#define RES_PROCESS_ALREADY "PROCESS ALREADY"
#define RES_PROCESS_BACKGROUND "PROCESS BACKGROUND"
#define RES_OVERFLOW "PROCESS ERROR overflow"

#define COMMAND_BUFFER_SIZE 64
#define TDS_FILTER_WEIGHT 5

#define CMD_LOG(prefix, message) {Serial.print(prefix); Serial.print(" "); Serial.println(message);}

Stepper stepper(STEPS, 60, 61);
char water_valve_status = STATUS_CLOSING;
int water_valve_current_angle = -WATER_VALVE_DELTA;

char command_buffer[COMMAND_BUFFER_SIZE];
int command_buffer_idx = 0;

ExponentialFilter<long> tds_1_filter(TDS_FILTER_WEIGHT, 1);
ExponentialFilter<long> tds_2_filter(TDS_FILTER_WEIGHT, 1);
long tds_1_raw, tds_2_raw;
unsigned long int last_update = millis();

typedef struct  s_settings
{
  int           tds_1_map_from[2];
  int         tds_1_map_to[2];
  int          tds_2_map_from[2];
  int         tds_2_map_to[2];
  int           magic;
}               t_settings;

t_settings settings;

void set_default() {
   CMD_LOG("M0", "Default settings loaded")
    settings.magic = SETTINGS_MAGIC;
    settings.tds_1_map_from[0]  = 0;
    settings.tds_1_map_from[1]  = 1024;
    settings.tds_1_map_to[0]    = 0;
    settings.tds_1_map_to[1]    = 4000;
    settings.tds_2_map_from[0]  = 0;
    settings.tds_2_map_from[1]  = 1024;
    settings.tds_2_map_to[0]    = 0;
    settings.tds_2_map_to[1]    = 4000;
    EEPROM.put(0, settings);
}

void setup() {
  pinMode(TDS_1_PIN, INPUT);
  pinMode(WATER_VALE_STEPPER_SW, OUTPUT);
  digitalWrite(WATER_VALE_STEPPER_SW, HIGH);
  Serial.begin(9600);
  stepper.setSpeed(1000);
  EEPROM.get(0, settings);
  if (settings.magic != SETTINGS_MAGIC) {
    Serial.println(RES_CAL_REQUIRED);
    set_default();
  }
}

// Load default settings
inline void M0() {
  set_default();
}

// Set tds calibration value
inline void M1() {
  set_default();
}

// Controle water valve status
inline void S1() {
  char *command = strtok(NULL, " ");
  if (command != NULL) {
    if (strcasecmp(command, "OPEN") == 0) {
      switch (water_valve_status)
      {
      case STATUS_OPENED:
        CMD_LOG("S1", RES_PROCESS_ALREADY)
        break;
      case STATUS_OPENING:
      case STATUS_CLOSING:
        CMD_LOG("S1", RES_PROCESS_BUSY);
        break;
      case STATUS_CLOSED:
        water_valve_status = STATUS_OPENING;
        CMD_LOG("S1", RES_PROCESS_BACKGROUND);
        break;
      }
    } else if (strcasecmp(command, "CLOSE") == 0) {
      switch (water_valve_status)
      {
      case STATUS_CLOSED:
        CMD_LOG("S1", RES_PROCESS_ALREADY);
        break;
      case STATUS_OPENING:
      case STATUS_CLOSING:
        CMD_LOG("S1", RES_PROCESS_BUSY);
        break;
      case STATUS_OPENED:
        water_valve_status = STATUS_CLOSING;
        CMD_LOG("S1", RES_PROCESS_BACKGROUND);
        break;
      }
    } else {
      CMD_LOG("S1", "ERROR argument 1 must be 'OPEN' or 'CLOSE'")
    }
  }
}

void loop() {
  // Fill the command buffer
  while (Serial.available() > 0) {
    if (command_buffer_idx + 1 >= COMMAND_BUFFER_SIZE) {
      Serial.println(RES_OVERFLOW);
      command_buffer_idx = 0;
    } else {
      int rd = Serial.read();
      command_buffer[command_buffer_idx++] = rd;
      if (rd == '\n') break;
    }
  }
  // Check for command into the buffer
  if (command_buffer_idx > 0 && command_buffer[command_buffer_idx - 1] == '\n') {
    command_buffer[command_buffer_idx - 1] = '\0';
    char *tokens = command_buffer;
    char *command = strtok(command_buffer, " ");
    // Load default settings
    if (command == NULL) {
      Serial.println("PROCESS ERROR EMPTY COMMAND");
    } else if (strcasecmp(command, "M0") == 0) {
      M0();
    } else if (strcasecmp(command_buffer, "M1") == 0) {
      M1();
    } else if (strcasecmp(command_buffer, "S1") == 0) {
      S1();
    } else if (strcasecmp(command_buffer, "S1 CLOSE\n") == 0) {

    } else {
      command_buffer[command_buffer_idx] = '\0';
      Serial.print("Unknown command ");
      Serial.println(command_buffer);
    }
    command_buffer_idx = 0;
  }
  // Water valve status/stepper update
  if (water_valve_status == STATUS_CLOSING) {
    digitalWrite(WATER_VALE_STEPPER_SW, LOW);
    if (water_valve_current_angle < WATER_VALVE_DELTA) {
      stepper.step(10);
      water_valve_current_angle += 10;
    } else {
      CMD_LOG("S1", RES_PROCESS_DONE);
      water_valve_status = STATUS_CLOSED;
    }
  }
  else if (water_valve_status == STATUS_OPENING) {
    digitalWrite(WATER_VALE_STEPPER_SW, LOW);
    if (water_valve_current_angle > -WATER_VALVE_DELTA) {
      stepper.step(-10);
      water_valve_current_angle -= 10;
    } else {
      CMD_LOG("S1", RES_PROCESS_DONE);
      water_valve_status = STATUS_OPENED;
    }
  } else {
    digitalWrite(WATER_VALE_STEPPER_SW, HIGH);
  }
  // Tds update
  if (last_update - millis() >= TDS_SAMPLE_INTERVAL) {
    last_update = millis();
    tds_1_raw = analogRead(TDS_1_PIN);
    tds_2_raw = analogRead(TDS_2_PIN);
    long maped = map(tds_1_raw, settings.tds_1_map_from[0], settings.tds_1_map_from[1], settings.tds_1_map_to[0], settings.tds_1_map_to[1]);
    tds_1_filter.Filter(maped);
    maped = map(tds_2_raw, settings.tds_2_map_from[0], settings.tds_2_map_from[1], settings.tds_2_map_to[0], settings.tds_2_map_to[1]);
    tds_2_filter.Filter(maped);
    Serial.print(tds_1_filter.Current());
    Serial.print(" ");
    Serial.print(tds_2_filter.Current());
    Serial.println();
  }
}
