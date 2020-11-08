#include <Arduino.h>
#include <Stepper.h>
#include <Filter.h>
#include <EEPROM.h>

#define SETTINGS_MAGIC 0x4243
#define TDS_SAMPLE_INTERVAL 100
#define TDS_1_PIN 11
#define TDS_2_PIN 12

#define WATER_VALVE_STEPPER_PIN_STEP 60
#define WATER_VALVE_STEPPER_PIN_DIR 61
#define WATER_VALE_STEPPER_SW 56
#define WATER_VALE_STEPPER_STEPS 200
// Number of steps to open/close the valve
#define WATER_VALVE_DELTA 3000

#define PPUMP_1_STEPPER_PIN_STEP 36
#define PPUMP_1_STEPPER_PIN_DIR 34
#define PPUMP_1_STEPPER_SW 30
#define PPUMP_1_STEPPER_STEPS 200

#define STATUS_OPENED 1
#define STATUS_OPENING 2
#define STATUS_CLOSING 3
#define STATUS_CLOSED 0

#define STATUS_ON   4
#define STATUS_OFF  5
#define STATUS_REV  6

#define STATUS_UNKNOWN -1
#define RES_OK(msg) Serial.println("OK " msg)
#define RES_ERR(msg) Serial.println("ERR " msg)

#define COMMAND_BUFFER_SIZE 64
#define TDS_FILTER_WEIGHT 5

const char *cmd_separator = " ";

Stepper valve_stepper(WATER_VALE_STEPPER_STEPS, WATER_VALVE_STEPPER_PIN_STEP, WATER_VALVE_STEPPER_PIN_DIR);
Stepper ppump_1_stepper(PPUMP_1_STEPPER_STEPS, PPUMP_1_STEPPER_PIN_STEP, PPUMP_1_STEPPER_PIN_DIR);

char ppump_1_status = STATUS_OFF;
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
  int         tds_1_map[2];
  int         tds_2_map[2];
  int           magic;
}               t_settings;

t_settings settings;

void set_default() {
  settings.magic = SETTINGS_MAGIC;
  settings.tds_1_map[0]    = 500;
  settings.tds_1_map[1]    = 2000;
  settings.tds_2_map[0]    = 500;
  settings.tds_2_map[1]    = 2000;
  EEPROM.put(0, settings);
}

// Load default settings
inline void M0() {
  set_default();
  RES_OK("M0");
}

void echo_tds_cal() {
  Serial.print("TDS1 ");
  Serial.print(settings.tds_1_map[0]);
  Serial.print(" ");
  Serial.print(settings.tds_1_map[1]);
  Serial.print(" TDS2 ");
  Serial.print(settings.tds_2_map[0]);
  Serial.print(" ");
  Serial.println(settings.tds_2_map[1]);
}

// Set tds calibration value
inline void M1() {
  char *token;
  int calibration_value[2];
  while ((token = strtok(NULL, cmd_separator)) != NULL) {
    calibration_value[0] = atoi(strtok(NULL, cmd_separator));
    calibration_value[1] = atoi(strtok(NULL, cmd_separator));
    if (token == NULL) {
      RES_ERR("BAD_REQUEST");
    } else if (strncasecmp(token, "TDS1", 4) == 0) {
      settings.tds_1_map[0] = calibration_value[0];
      settings.tds_1_map[1] = calibration_value[1];
    } else if (strncasecmp(token, "TDS2", 4) == 0) {
      settings.tds_2_map[0] = calibration_value[0];
      settings.tds_2_map[1] = calibration_value[1];
    }
  }
  EEPROM.put(0, settings);
  Serial.print("OK M1 ");
  echo_tds_cal();
}

// Read tds calibration values
inline void M2() {
  Serial.print("OK M2 ");
  echo_tds_cal();
}

// Read raw sensore values
inline void G0() {
  Serial.print("OK G0 TDS1 ");
  Serial.print(tds_1_raw);
  Serial.print(" TDS2 ");
  Serial.println(tds_2_raw);
}

// Read filtred sensore values
inline void G1() {
  Serial.print("OK G1 TDS1 ");
  Serial.print(tds_1_filter.Current());
  Serial.print(" TDS2 ");
  Serial.println(tds_2_filter.Current());
}

// Controle water valve status
inline void S0() {
  char *command = strtok(NULL, cmd_separator);
  if (command != NULL) {
    if (strcasecmp(command, "OPEN") == 0) {
      switch (water_valve_status)
      {
      case STATUS_OPENED:
      case STATUS_OPENING:
      case STATUS_CLOSING:
        RES_ERR("S0 BUSY");
        break;
      case STATUS_CLOSED:
        water_valve_status = STATUS_OPENING;
        RES_OK("S0 PENDING");
        break;
      }
    } else if (strcasecmp(command, "CLOSE") == 0) {
      switch (water_valve_status)
      {
      case STATUS_CLOSED:
      case STATUS_OPENING:
      case STATUS_CLOSING:
        RES_ERR("S0 BUSY");
        break;
      case STATUS_OPENED:
        water_valve_status = STATUS_CLOSING;
        RES_OK("S0 PENDING");
        break;
      }
    } else {
        RES_ERR("S0 BAD_REQUEST");
    }
  }
}

// Controle peristatic pump status
inline void S1() {
  char *command = strtok(NULL, cmd_separator);
  if (command != NULL) {
    if (strncasecmp(command, "ON", 2) == 0) {
      ppump_1_status = STATUS_ON;
      RES_OK("S1 ON");
    } else if (strncasecmp(command, "OFF", 3) == 0) {
      ppump_1_status = STATUS_OFF;
      RES_OK("S1 OFF");
    } else if (strncasecmp(command, "REV", 3) == 0) {
      ppump_1_status = STATUS_REV;
      RES_OK("S1 REV");
    } else {
        RES_ERR("S1 BAD_REQUEST");
    }
  }
}

void setup() {
  Serial.begin(9600);
  pinMode(TDS_1_PIN, INPUT);
  pinMode(WATER_VALE_STEPPER_SW, OUTPUT);
  pinMode(PPUMP_1_STEPPER_SW, OUTPUT);
  digitalWrite(WATER_VALE_STEPPER_SW, HIGH);
  digitalWrite(PPUMP_1_STEPPER_SW, HIGH);
  ppump_1_stepper.setSpeed(1000);
  valve_stepper.setSpeed(1000);
  EEPROM.get(0, settings);
  if (settings.magic != SETTINGS_MAGIC) {
    RES_ERR("CAL");
    set_default();
  }
}

void loop() {
  // Fill the command buffer
  while (Serial.available() > 0) {
    if (command_buffer_idx + 1 >= COMMAND_BUFFER_SIZE) {
      RES_ERR("OVERFLOW");
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
    char *command = strtok(command_buffer, cmd_separator);
    // Load default settings
    if (command == NULL)
      Serial.println("PROCESS ERROR EMPTY COMMAND");
    else if (strncasecmp(command, "M0", 2) == 0)    M0();
    else if (strncasecmp(command, "M1", 2) == 0)    M1();
    else if (strncasecmp(command, "M2", 2) == 0)    M2();
    else if (strncasecmp(command, "S0", 2) == 0)    S0();
    else if (strncasecmp(command, "S1", 2) == 0)    S1();
    else if (strncasecmp(command, "G0", 2) == 0)    G0();
    else if (strncasecmp(command, "G1", 2) == 0)    G1();
    else {
      command_buffer[command_buffer_idx] = '\0';
      RES_ERR("UNKNOW");
      Serial.println(command_buffer);
    }
    command_buffer_idx = 0;
  }
  // Water valve status/stepper update
  if (water_valve_status == STATUS_CLOSING) {
    digitalWrite(WATER_VALE_STEPPER_SW, LOW);
    if (water_valve_current_angle < WATER_VALVE_DELTA) {
      valve_stepper.step(1);
      water_valve_current_angle += 1;
    } else {
      RES_OK("S0 DONE CLOSE");
      water_valve_status = STATUS_CLOSED;
    }
  }
  else if (water_valve_status == STATUS_OPENING) {
    digitalWrite(WATER_VALE_STEPPER_SW, LOW);
    if (water_valve_current_angle > -WATER_VALVE_DELTA) {
      valve_stepper.step(-1);
      water_valve_current_angle -= 1;
    } else {
      RES_OK("S0 DONE OPEN");
      water_valve_status = STATUS_OPENED;
    }
  } else {
    digitalWrite(WATER_VALE_STEPPER_SW, HIGH);
  }
  // Peristatic pump stepper update
  if (ppump_1_status == STATUS_OFF) {
    digitalWrite(PPUMP_1_STEPPER_SW, HIGH);
  } else {
    digitalWrite(PPUMP_1_STEPPER_SW, LOW);
    ppump_1_stepper.step(ppump_1_status == STATUS_ON ? 1 : -1);
  }
  // Tds update
  if (last_update - millis() >= TDS_SAMPLE_INTERVAL) {
    last_update = millis();
    tds_1_raw = analogRead(TDS_1_PIN);
    tds_2_raw = analogRead(TDS_2_PIN);
    tds_1_filter.Filter(map(tds_1_raw, 0, settings.tds_1_map[0], 0, settings.tds_1_map[1]));
    tds_2_filter.Filter(map(tds_2_raw, 0, settings.tds_2_map[0], 0, settings.tds_2_map[1]));
    // Serial.print(tds_1_filter.Current());
    // Serial.print(" ");
    // Serial.print(tds_2_filter.Current());
    // Serial.println();
  }

}
