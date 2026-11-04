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

## Follow-up: mode ACK compatibility for every model

Removed the Action 4 device-ID restriction at the user's request. Any model's
1D04 mode-switch request can be acknowledged as 1D04 or 1D03. Matching still
requires an ACK with the exact pending sequence, and nonzero response codes remain
errors. Updated the existing focused compatibility test and current documentation;
this changes the supported rule without claiming new hardware coverage.

## Follow-up: typed protocol enums

Added 25 named-value enums from the DATA protocol document in `enums.rs`, exported
from the core root, plus context-aware CaptureFormat/CaptureRate enums. Coverage
includes models, return/verification codes, key modes/states/events, recording,
subscription, all 17 camera modes, state, resolution/photo sizes, FPS indices,
stabilization, aspect ratio, user/power/temperature modes, countdown/loop presets,
and new-status tags. No ID was invented for the unsupported Osmo Nano.

Changed settings APIs and the CLI to pass enums. State decoding distinguishes
photo sizes by camera model and FPS/burst/slow-motion meanings by mode. Unknown
received values retain their codes, except invalid user modes normalize to General
as documented; unknown outgoing settings are rejected. Continuous measurements
and reserved FOV data remain integers. Preserved the user's supernight CLI alias.

Updated examples and documented API migrations (including typed model/error values,
verification mode, and countdown/loop field names). Five focused tests and strict
Clippy pass; aarch64 iOS/Android core checks and CLI help pass. No hardware control
was performed for this type/API refactor.

## Android port: JNI bridge and Compose HUD

Added `crates/android-bridge` as a JNI cdylib: it owns a Tokio runtime, wraps
scan/connect/version/record/shutter/mode and synchronous GPS updates, persists the
controller identity in the app files directory, and pushes JSON events to Kotlin through
`BridgeListener`. `btleplug::platform::init` runs inside the Java-called `nativeInit`;
failed scans always emit a terminal `devices` event so the UI cannot stay stuck, and
panicked scan tasks are converted into an error event. Host and aarch64-linux-android
checks pass. The adapter-not-ready message no longer says "terminal running this CLI",
since the same core now runs in the app.

Vendored btleplug 0.13.5's Android Java support classes as the `:btleplug-jni` Gradle
module (upstream publishes no AAR) and added the Compose app: a single-activity dark HUD
with scan/connect, pairing dialog, mode chips, record/shutter controls, status cards, and
a `LocationManager` GPS injector sending `GpsSample` JSON at a requested 100ms interval.
Permissions are requested per feature; release keeps btleplug and JNI classes through R8
rules. Gradle builds the native library with `scripts/build-android.sh` (cargo-ndk and
NDK 29) before packaging; `-PskipRust` skips it.

Verified: debug APK builds, installs and launches on an arm64 API 37 emulator; connect
screen, scanning state and the adapter-not-ready error path render; release APK builds
with R8 and still contains `libosmo_bridge.so` and the btleplug classes. No phone or
camera was used: the emulator has no usable BLE adapter, so pairing, recording, real
scanning and GPS embedding in footage are not claimed.

## Fix: Android scans were rejected before start_scan

btleplug's Android backend implements `Central::adapter_state()` as
`Ok(CentralState::Unknown)` (droidplug/adapter.rs). The macOS-era pre-scan guard treated
Unknown as "adapter not ready", so every Android scan failed before `start_scan` ran,
regardless of granted permissions. The guard is now compiled only for non-Android
targets; Android relies on `start_scan`, which reports the real failure.

Mapped Java-side scan failures to actionable messages: no adapter / Bluetooth off, and
permission denied (SecurityException or a message containing "permission"). Added a unit
test for the mapping; six focused tests, fmt and strict Clippy pass, and the Android
cdylib builds. Interactive emulator validation was blocked by host load (SystemUI ANR),
so the fix still needs confirmation on the Xiaomi 15.

## Fix: Android CCCD writes failing with GATT status 128

On a Xiaomi 15 the first connect died while enabling notifications:
"Unable to write client characteristic configuration descriptor, status: 128"
(GATT_ERROR). The GATT stack frequently rejects the first CCCD write right after
service discovery and MTU negotiation. `Link::open` now retries the subscription up
to three times with 300/600ms backoff, and `Camera::connect` retries the whole GATT
session once after a 600ms pause when the failure is a CCCD write. The BLE open
timeout grew from 15s to 20s to cover retries. Six focused tests, fmt and strict
Clippy pass; the behaviour still needs confirmation on the phone.

## Android: remaining duration in video modes

Video-like modes (everything except Photo, PanoramicPhoto and SubjectTracking) now show
the remaining recordable time (`1h 02m`) in the status card instead of the remaining shot
count; photo modes keep showing shots. Duration formatting moved to `Formatters.kt` with
JVM unit tests covering both the recording clock and the remaining-time label.

## Android: Material 3 design pass and UI file split

Moved the Compose UI onto Material 3 design tokens: a full dark color scheme built from
M3 surface-container roles, the M3 shape and type scales, and M3 components (TopAppBar,
Card, ListItem, FilterChip, Switch, Surface actions, AlertDialog, AnimatedContent screen
transition). The dark camera-HUD identity and amber accent are preserved; camera metrics
use monospace inside the type scale.

Split the ~800-line UI file into focused files: OsmoLinkApp (shell and permissions),
ConnectScreen, ControlScreen, StatusHero, ModePicker, StatusDetails, GpsCard, ControlBar,
PairingDialog, Labels, Permissions. Kotlin unit tests and the debug build pass; both
screens were checked on the emulator.

## Android: foreground service and persistent notification

Moved bridge state and the GPS tracker into a process-wide `OsmoRepository` so they no
longer follow the activity lifecycle; `ControllerViewModel` is now a thin facade.
Added `OsmoService`, a foreground service typed `connectedDevice|location`, started while
a camera is connected or location injection is on. Its ongoing notification shows the
current mode, recording duration, battery, remaining capacity/shots and GPS rate, with
"断开"/"停止定位" actions. Notification text comes from a pure `notificationContent()`
function with unit tests. POST_NOTIFICATIONS is requested after the first connection.

Verified on the emulator: the service reports `isForeground=true`, `types=0x18` and the
expected notification title/text. Real-device background survival and OEM battery
managers remain unverified.

## Android: Live Update for recording

The ongoing notification now uses a native chronometer for the recording duration, and on
Android 16+ requests a Live Update (`Notification.ProgressStyle`, `setRequestPromotedOngoing`,
`setShortCriticalText("REC")`, `POST_PROMOTED_NOTIFICATIONS`). It falls back to the compat
notification when the system is older or the user disabled promoted notifications. The title
no longer embeds the timer, so updates no longer depend on status-push cadence.

Verified on the API 37 emulator: the notification has `PROMOTED_ONGOING`, template
`android.app.Notification$ProgressStyle`, and `shortCriticalText=REC`. Vendor islands
(Xiaomi Super Island etc.) are intentionally not implemented.
