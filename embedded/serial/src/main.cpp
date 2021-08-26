#include <Arduino.h>

#define PACKET_SIZE 5

#define BTN_COUNT 4
const uint8_t btns[BTN_COUNT] = {PB6, PB5, PB4, PB3};
int8_t btn_states[BTN_COUNT];

#define POT_COUNT 4
#define POT_IDLE_TIMEOUT 100
const uint8_t pots[POT_COUNT] = {PA3, PA2, PA1, PA0};
uint16_t pot_values[POT_COUNT];
uint16_t pot_states[POT_COUNT];

void handle_pots();
void handle_btns();

void setup()
{

  Serial.begin(115200);
  Serial.write("USB intialized\n");

  //Initialize potensiometers
  for (int i = 0; i < POT_COUNT; i++)
  {
    pinMode(pots[i], GPIO_MODE_ANALOG);
    pot_values[i] = 0;
  }

  //Initialize buttons
  for (int i = 0; i < BTN_COUNT; i++)
  {
    pinMode(btns[i], INPUT_PULLDOWN);
    btn_states[i] = false;
  }

  Serial1.begin(115200);
}

void loop()
{
  delay(100);
  handle_btns();
  handle_pots();
}

void send_packet(uint8_t cmd, uint8_t data1, uint8_t data2, uint8_t data3)
{

  uint8_t checksum = cmd + data1 + data2 + data3;

  char data[5] = {cmd, data1, data2, data3, checksum};
  Serial1.write(data);
}

void uint16_to_uint8(uint16_t value, uint8_t out[])
{
  out[0] = (uint8_t)(value & 0b00001111);
  out[1] = (uint8_t)(value >> 4);
}

void handle_pots()
{
  for (int i = 0; i < POT_COUNT; i++)
  {
    int value = analogRead(pots[i]);
    uint16_t pot_state = pot_states[i];

    if (abs(pot_values[i] - value) > 10)
    {
      pot_states[i] = POT_IDLE_TIMEOUT;
      pot_values[i] = value;
    }

    if(pot_state > 1){
      uint8_t bytes[2];
      uint16_to_uint8(value, bytes);
      send_packet('P', i, bytes[0], bytes[1]);
      pot_state--;
    }
    
  }
}

void handle_btns()
{
  for (int i = 0; i < BTN_COUNT; i++)
  {
    int value = digitalRead(btns[i]);
    if (value != btn_states[i])
    {
      btn_states[i] = value;
      send_packet('B', i, value, 0);
    }
    
  }
}
