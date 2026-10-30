# Implementation log

Reference: DJI Osmo-GPS-Controller-Demo, revision
`92fe23e5a749f189593f980a26a105c3bb66aa1c`.

Each completed step gets its own commit:

1. Workspace and protocol codec, checked against the official sample.
2. BLE discovery and DJI authentication.
3. Camera controls and status subscription.
4. GPS encoding and streaming.
5. Interactive macOS CLI and persistent controller identity.
6. Disconnect/reconnect handling and platform capability boundaries.
7. macOS hardware validation and usage documentation.

## Step 1

Created the two-crate workspace and bounded streaming frame decoder. CRC16 and
CRC32 use DJI's initial value 0x3aa3; fields are encoded explicitly in little endian.
The compact protocol test covers the official sample, corruption, split frames,
combined frames, and resynchronization after other BLE data.

## Step 2

Implemented BLE discovery, notification assembly, MTU-based writes, and DJI's
two-way authentication (including camera-initiated confirmation). A single task
owns notifications and writes, correlates acknowledgements by sequence and
command, and bounds command response waits. Failed authentication closes BLE.
The CLI initially exposes scanning; interactive controls follow in step 5.

## Step 3

Added version queries, explicit start/stop, mode switching, button events and
states, restart, sleep, and status subscription. Both 1D02 and 1D06 state messages
are decoded; the latest state is available through a watch channel. A successful
sleep acknowledgement blocks further writes. Command targets use the camera ID
returned by authentication, rather than the demo's inconsistent hardcoded IDs.

## Step 4

Added a platform-independent GPS input type, explicit coordinate/velocity/accuracy
unit conversion, and UTC+8 date rollover. Single fixes and latest-value 10Hz
streaming share the camera worker. Old inputs expire after 2 seconds; sleep blocks
GPS writes. A focused test checks units, midnight rollover, and invalid fixes.

## Step 5

Added the interactive CLI, piped input, camera selection, persistent controller
identity and pairing history, protocol logging, controls, and timestamp-paced
JSONL GPS playback. `wait` keeps reception and playback active in scripted checks.
Per user clarification, GPS now defaults to 1Hz, configurable via
`ConnectOptions::gps_interval` (100ms..1s); GNSS acquisition frequency is separate.

## Step 6

Added bounded reconnect (at most two attempts), subscription restoration, explicit
async disconnect, connection-state events, and cleanup on drop. Commands are never
replayed after reconnect; GPS requires a fresh input. Sleep requests pause writes
immediately, including when the response is uncertain. Wake/snapshot report the
backend's missing manufacturer-advertising capability. Android initialization is
re-exported for a later native shell.

Hardware finding: Action 4 (SDK-v1.1 DEBUG AC203-03.04.80.15) authenticates as
0x0000ff33. Its mode-switch ACK uses 1D03 despite receiving 1D04; compatibility
accepts that pair only for Action 4 and an exact pending sequence. Start/stop,
version, and pairing use their expected command IDs.

The user subsequently selected 10Hz again. Final default is 100ms; acquisition
rate remains independent of transmission rate. No interpolation or fabricated
sampling timestamps are introduced.
