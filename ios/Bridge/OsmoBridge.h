#ifndef OsmoBridge_h
#define OsmoBridge_h

#include <stdbool.h>
#include <stdint.h>

typedef void (*OsmoEventCallback)(const char *json);

bool osmo_ios_start(const char *state_directory, OsmoEventCallback callback);
bool osmo_ios_scan(void);
bool osmo_ios_connect(uint32_t index);
bool osmo_ios_disconnect(void);
bool osmo_ios_capture(uint8_t mode, bool capturing);
bool osmo_ios_set_mode(uint8_t code);

#endif
