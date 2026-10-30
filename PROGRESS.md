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
