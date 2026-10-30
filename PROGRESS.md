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

## Step 7

Completed the short macOS Action 4 hardware check: authentication, reconnect using
saved pairing identity, version/status, photo mode and shutter, video mode, actual
recording start/stop transitions, concurrent historical GPS playback, and sleep.
GPS TX interval median was 99.943ms in the verification log. No TX followed the
sleep request; a subsequent version command returned Sleeping locally.

Added README, exact mode-ACK evidence and test boundaries in
`docs/macos-validation.md`, the six-fix upstream GPS playback file, and upstream
license/fixture attribution. Named the combined photo/video status helper
`is_capturing` to match the wire semantics. Format, three focused tests, and strict
Clippy pass. iOS aarch64 and Android aarch64 core compilation checks both pass.
No mobile app execution, video metadata extraction, broadcast wake, or forced
reconnection stress testing is claimed.

## Follow-up: intermittent empty scans

On the user's awake Action 4, a fresh process discovered the camera with the
expected 0x08aa manufacturer ID and FA marker. In a separate same-process check,
the first five-second scan saw 23 cached peripherals and no target; the second
scan discovered the Action 4. This reproduces a short-window miss, without proving
its underlying radio/OS cause. No name-prefix filtering is used.

Added a Bluetooth adapter state check before scanning, candidate/result debug logs,
and a clearer fallback display name (including advertisement-name fallback).
Changed CLI's default scan duration to 10 seconds and added `scan <1..60>`.
Permission/adapter failures are reported separately instead of silently producing
an empty result. Existing focused tests and strict Clippy pass.

## Follow-up: command DATA only

Reverted d601fa9 (the full raw-frame display) at the user's request. The replacement
logs only explicit control requests and their matching pending acknowledgements,
plus one-shot GPS writes. DATA is the actual CmdSet + CmdID + payload in uppercase
hex; framing and checksums are omitted. Command matching retains the exact sequence
check and the existing narrow Action 4 mode-switch ACK compatibility rule.

Background state pushes, subscription traffic, periodic GPS and unrelated ACKs are
silent. Removed generic BLE frame debug logs as well. CLI output omits tracing
prefixes, with `RUST_LOG=warn` available to disable command DATA. The three existing
focused tests and strict Clippy pass; no additional hardware actions were taken.
