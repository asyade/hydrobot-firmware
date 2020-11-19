#include <Arduino.h>
#include <Stepper.h>
#include <Filter.h>
#include <EEPROM.h>
#include <OneWire.h> 
#include <DallasTemperature.h>


#define SETTINGS_MAGIC 0x4243
#define TDS_SAMPLE_INTERVAL 100
#define PH_SAMPLE_INTERVAL 100
#define TDS_1_PIN 11
#define PH_1_PIN 9
#define TEMP_PIN 40
#define TDS_FILTER_WEIGHT 5
#define PH_FILTER_WEIGHT  2
#define TEMP_RESOLUTION 12
#define STEP_DURATION 10

#define WATER_VALVE_STEPPER_PIN_STEP 60
#define WATER_VALVE_STEPPER_PIN_DIR 61
#define WATER_VALE_STEPPER_SW 56
#define WATER_VALE_STEPPER_STEPS 200
// Number of steps to open/close the valve
#define WATER_VALVE_DELTA 1900

#define BRASS_PUMP_IN_PIN 8
#define BRASS_PUMP_OUT_PIN 7
#define BRONCHUS_CAPACITY 50//50cl

#define BRONCHUS_FILL_DURATION              40000
#define BRONCHUS_EMPTY_DURATION             26000
#define BRONCHUS_STANDBY_FULL_DURATION      40000
#define BRONCHUS_STANDBY_SAMPLING_DURATION  120000

#define PPUMP_1_STEPPER_PIN_STEP 36
#define PPUMP_1_STEPPER_PIN_DIR 34
#define PPUMP_1_STEPPER_SW 30
#define PPUMP_1_STEPPER_STEPS 200

#define STATUS_UNKNOWN -1
#define RES_OK(msg) Serial.println("OK " msg)
#define RES_ERR(msg) Serial.println("ERR " msg)

#define COMMAND_BUFFER_SIZE 64
#define CMD_SEPARATOR " "

#define HEALT_CHECK_INTERVAL 1000

#define S_TDS_CONNECTED                   ((1 << 0))
#define S_PH_CONNECTED                    ((1 << 2))
#define S_OSMOS_SWITCH_OPENED             ((1 << 3))
#define S_OSMOS_SWITCH_OPENING            ((1 << 4))
#define S_OSMOS_SWITCH_CLOSING            ((1 << 5))
#define S_OSMOS_SWITCH_CLOSED             ((1 << 6))
#define S_PERISTALIC_PUMP_ON              ((1 << 7))
#define S_PERISTALIC_PUMP_REV             ((1 << 8))
#define S_BRONCHUS_STANDBY_FULL           ((1 << 9))
#define S_BRONCHUS_STANDBY_SAMPLING       ((1 << 10))
#define S_BRONCHUS_WAIT_FULL              ((1 << 11))
#define S_BRONCHUS_WAIT_EMPTY             ((1 << 12))

#define M_STEPPER_RUNNING ((M_OSMOS_SWITCH_BUSY | S_PERISTALIC_PUMP_ON | S_PERISTALIC_PUMP_REV ))
#define M_OSMOS_SWITCH_BUSY ((S_OSMOS_SWITCH_OPENING | S_OSMOS_SWITCH_CLOSING))

unsigned int status = S_OSMOS_SWITCH_CLOSING | S_BRONCHUS_STANDBY_SAMPLING;
unsigned long int breath_step = millis();

Stepper valve_stepper(WATER_VALE_STEPPER_STEPS, WATER_VALVE_STEPPER_PIN_STEP, WATER_VALVE_STEPPER_PIN_DIR);
Stepper ppump_1_stepper(PPUMP_1_STEPPER_STEPS, PPUMP_1_STEPPER_PIN_STEP, PPUMP_1_STEPPER_PIN_DIR);

int water_valve_current_angle = -WATER_VALVE_DELTA;

char command_buffer[COMMAND_BUFFER_SIZE];
int command_buffer_idx = 0;

ExponentialFilter<long> tds_1_filter(TDS_FILTER_WEIGHT, 1);
long tds_1_raw;

ExponentialFilter<long> ph_1_filter(PH_FILTER_WEIGHT, 600);
long ph_1_raw;

float temp_1_raw;

OneWire temp_bus(TEMP_PIN);
DallasTemperature temp_sensore(&temp_bus);
DeviceAddress temp_sensore_address;

unsigned long int last_tds_update = millis();
unsigned long int last_ph_update = millis();
unsigned long int last_temp_update = millis();
unsigned long int temp_conversion_duration = 750 / (1 << (12 - TEMP_RESOLUTION));

typedef struct  s_settings
{
  int           tds_1_map[2];
  int           magic;
}               t_settings;

t_settings settings;

void set_default() {
  settings.magic = SETTINGS_MAGIC;
  settings.tds_1_map[0]    = 538;
  settings.tds_1_map[1]    = 127;
  EEPROM.put(0, settings);
}

// Load default settings
inline void M0() {
  set_default();
  RES_OK("M0");
}

void echo_tds_cal() {
  Serial.print(" TDS1 ");
  Serial.print(settings.tds_1_map[0]);
  Serial.print(" ");
  Serial.print(settings.tds_1_map[1]);
  Serial.println();
}

// Set tds calibration value
inline void M1() {
  char *token;
  int calibration_value[2];
  while ((token = strtok(NULL, CMD_SEPARATOR)) != NULL) {
    calibration_value[0] = atoi(strtok(NULL, CMD_SEPARATOR));
    calibration_value[1] = atoi(strtok(NULL, CMD_SEPARATOR));
    if (token == NULL) {
      RES_ERR("BAD_REQUEST");
    } else if (strncasecmp(token, "TDS1", 4) == 0) {
      settings.tds_1_map[0] = calibration_value[0];
      settings.tds_1_map[1] = calibration_value[1];
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
  Serial.print("OK G0");
  Serial.print(" TDS1 ");
  Serial.print(tds_1_raw);
  Serial.print(" PH1 ");
  Serial.print(ph_1_raw);
  Serial.println();
}

#define RES2 857.39 //TODO check that const match with our sensor
#define ECREF 255.86
#define VREF 5.0 

inline float read_ec(float analog, float temperature) {
  float voltage = analog * (float)VREF/ 1024.0;
  float compensationCoefficient=1.0+0.02*(temperature-25.0); //temperature compensation formula: fFinalResult(25^C) = fFinalResult(current)/(1.0+0.02*(fTP-25.0));
  float compensationVolatge=voltage/compensationCoefficient; //temperature compensation
  float tdsValue=(133.42*compensationVolatge*compensationVolatge*compensationVolatge - ECREF*compensationVolatge*compensationVolatge + RES2*compensationVolatge)*0.5; //convert voltage value to tds value
//Serial.print("voltage:");
//Serial.print(averageVoltage,2);
//Serial.print("V ");
  return tdsValue;
}

inline float read_ph(float ph, float temperature) {
  return (float)ph / 100.0;
}

// Read filtred sensore values & status
inline void G1() {
  Serial.print("OK G1");
  Serial.print(" TDS1 ");
  Serial.print(read_ec(tds_1_filter.Current(), 25));
  Serial.print(" PH1 ");
  Serial.print(read_ph(ph_1_filter.Current(), 25));
  Serial.print(" T1 ");
  Serial.print(temp_1_raw);
  Serial.print(" STATUS ");
  Serial.print(status);
  Serial.println();
}

// Controle water valve status
inline void S0() {
  char *command = strtok(NULL, CMD_SEPARATOR);
  if (command != NULL) {
    if (status & M_OSMOS_SWITCH_BUSY) {
      RES_ERR("S0 BUSY");
    } else if (strncasecmp(command, "ON", 2) == 0) {
      status |= S_OSMOS_SWITCH_OPENING;
      status &= ~(S_OSMOS_SWITCH_OPENED | S_OSMOS_SWITCH_CLOSED);
    } else if (strncasecmp(command, "OFF", 3) == 0) {
      status |= S_OSMOS_SWITCH_CLOSING;
      status &= ~(S_OSMOS_SWITCH_OPENED | S_OSMOS_SWITCH_CLOSED);
    } else {
      RES_ERR("S0 BAD_REQUEST");
    }
  }
}

// Controle peristatic pump status
inline void S1() {
  char *command = strtok(NULL, CMD_SEPARATOR);
  if (command != NULL) {
    if (strncasecmp(command, "ON", 2) == 0) {
      status |= S_PERISTALIC_PUMP_ON;
      status &= ~S_PERISTALIC_PUMP_REV;
      RES_OK("S1 ON");
    } else if (strncasecmp(command, "OFF", 3) == 0) {
      status &= ~(S_PERISTALIC_PUMP_ON | S_PERISTALIC_PUMP_REV);
      RES_OK("S1 OFF");
    } else if (strncasecmp(command, "REV", 3) == 0) {
      status |= S_PERISTALIC_PUMP_REV;
      status &= ~S_PERISTALIC_PUMP_ON;
      RES_OK("S1 REV");
    } else {
        RES_ERR("S1 BAD_REQUEST");
    }
  }
}

inline void S2() {
  char *command = strtok(NULL, CMD_SEPARATOR);
  if (command != NULL) {
    if (strncasecmp(command, "FILL", 2) == 0) {
      // status |= S_BRASS_PUMP_IN_ON;
      // digitalWrite(BRASS_PUMP_IN_PIN, HIGH);
    } else if (strncasecmp(command, "EMPTY", 3) == 0) {
      // status &= ~S_BRASS_PUMP_IN_ON;
      // digitalWrite(BRASS_PUMP_IN_PIN, LOW);
    } else {
        RES_ERR("S2 BAD_REQUEST");
    }
  }
}


void setup() {
  Serial.begin(9600);
  pinMode(TDS_1_PIN, INPUT);
  pinMode(PH_1_PIN, INPUT);
  pinMode(WATER_VALE_STEPPER_SW, OUTPUT);
  pinMode(PPUMP_1_STEPPER_SW, OUTPUT);
  pinMode(BRASS_PUMP_OUT_PIN, OUTPUT);
  pinMode(BRASS_PUMP_IN_PIN, OUTPUT);
  digitalWrite(BRASS_PUMP_IN_PIN, LOW);
  digitalWrite(BRASS_PUMP_OUT_PIN, LOW);
  digitalWrite(WATER_VALE_STEPPER_SW, HIGH);
  digitalWrite(PPUMP_1_STEPPER_SW, HIGH);
  temp_sensore.begin();
  temp_sensore.getAddress(temp_sensore_address, 0);
  temp_sensore.setWaitForConversion(false);
  temp_sensore.requestTemperaturesByAddress(temp_sensore_address);
  last_temp_update = millis();
  ppump_1_stepper.setSpeed(50);
  valve_stepper.setSpeed(1000);
  EEPROM.get(0, settings);
  if (settings.magic != SETTINGS_MAGIC) {
    RES_ERR("CAL");
    set_default();
  }
}

unsigned long int step_begin = millis();

void loop() {
  step_begin = millis();
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
    char *command = strtok(command_buffer, CMD_SEPARATOR);
    // Load default settings
    if (command == NULL)
      Serial.println("PROCESS ERROR EMPTY COMMAND");
    else if (strncasecmp(command, "M0", 2) == 0)    M0();
    else if (strncasecmp(command, "M1", 2) == 0)    M1();
    else if (strncasecmp(command, "M2", 2) == 0)    M2();
    else if (strncasecmp(command, "S0", 2) == 0)    S0();
    else if (strncasecmp(command, "S1", 2) == 0)    S1();
    else if (strncasecmp(command, "S2", 2) == 0)    S2();
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
  if (status & S_OSMOS_SWITCH_CLOSING) {
    digitalWrite(WATER_VALE_STEPPER_SW, LOW);
    if (water_valve_current_angle < WATER_VALVE_DELTA) {
      valve_stepper.step(-1);
      water_valve_current_angle += 1;
    } else {
      RES_OK("S0 OFF");
      status |= S_OSMOS_SWITCH_CLOSED;
      status &= ~(S_OSMOS_SWITCH_CLOSING);
    }
  }
  else if (status & S_OSMOS_SWITCH_OPENING) {
    digitalWrite(WATER_VALE_STEPPER_SW, LOW);
    if (water_valve_current_angle > -WATER_VALVE_DELTA) {
      valve_stepper.step(1);
      water_valve_current_angle -= 1;
    } else {
      RES_OK("S0 ON");
      status |= S_OSMOS_SWITCH_OPENED;
      status &= ~(S_OSMOS_SWITCH_OPENING);
    }
  } else {
    digitalWrite(WATER_VALE_STEPPER_SW, HIGH);
  }
  // Peristatic pump stepper update
  if (status & S_PERISTALIC_PUMP_ON) {
    digitalWrite(PPUMP_1_STEPPER_SW, LOW);
    ppump_1_stepper.step(-1);
  } else if (status & S_PERISTALIC_PUMP_REV) {
    digitalWrite(PPUMP_1_STEPPER_SW, LOW);
    ppump_1_stepper.step(1);
  } else {
    digitalWrite(PPUMP_1_STEPPER_SW, HIGH);
  }
  // Tds update
  if (millis() - last_tds_update >= TDS_SAMPLE_INTERVAL) {
    last_tds_update = millis();
    tds_1_raw = analogRead(TDS_1_PIN);
    tds_1_filter.Filter(tds_1_raw);
    if (tds_1_raw == 0) {
      status &= ~S_TDS_CONNECTED;
    } else {
      status |= S_TDS_CONNECTED;
    }
  }
  // Ph update
  if (millis() - last_ph_update >= PH_SAMPLE_INTERVAL && status & S_BRONCHUS_STANDBY_SAMPLING) {
    last_ph_update = millis();
    ph_1_raw = 1023 - analogRead(PH_1_PIN);
    ph_1_filter.Filter(map(ph_1_raw, 0, 1024, 0, 1400));
    if (ph_1_raw == 0) {
      status &= ~S_PH_CONNECTED;
    } else {
      status |= S_PH_CONNECTED;
    }
  //  Serial.println((float)ph_1_filter.Current() / 100.0);
  }
  // Temp update
  // > Avoid temperature sampling when stepper are on caus the dallas sensore are too slow
  if (!(status & M_STEPPER_RUNNING) && millis() - last_temp_update > temp_conversion_duration) {
    temp_1_raw = temp_sensore.getTempC(temp_sensore_address);
    last_temp_update = millis();
    temp_sensore.requestTemperaturesByAddress(temp_sensore_address);
  }

  // Breath update
  if (status & S_BRONCHUS_STANDBY_FULL && millis() - breath_step >= BRONCHUS_STANDBY_FULL_DURATION) { // the bronche are full empty theme
    status &= ~S_BRONCHUS_STANDBY_FULL;
    status |= S_BRONCHUS_STANDBY_SAMPLING;
    breath_step = millis();
  } else if (status & S_BRONCHUS_STANDBY_SAMPLING && millis() - breath_step >= BRONCHUS_STANDBY_SAMPLING_DURATION) { // the bronche are full empty theme
    status &= ~S_BRONCHUS_STANDBY_SAMPLING;
    status |= S_BRONCHUS_WAIT_EMPTY;
    breath_step = millis();
    digitalWrite(BRASS_PUMP_OUT_PIN, HIGH);
  } else if (status & S_BRONCHUS_WAIT_EMPTY && millis() - breath_step >= BRONCHUS_EMPTY_DURATION) {
    status &= ~S_BRONCHUS_WAIT_EMPTY;
    status |= S_BRONCHUS_WAIT_FULL;
    digitalWrite(BRASS_PUMP_OUT_PIN, LOW);
    digitalWrite(BRASS_PUMP_IN_PIN, HIGH);
    breath_step = millis();
  } else if (status & S_BRONCHUS_WAIT_FULL && millis() - breath_step >= BRONCHUS_FILL_DURATION) {
    status &= ~S_BRONCHUS_WAIT_FULL;
    status |= S_BRONCHUS_STANDBY_FULL;
    digitalWrite(BRASS_PUMP_IN_PIN, LOW);
    breath_step = millis();
  }
  unsigned long int step_duration = millis() - step_begin;
}
