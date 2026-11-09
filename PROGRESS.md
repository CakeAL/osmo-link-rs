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

## Fix: fall back when OEM live updates fail

A Xiaomi device crashed when recording started and again when reconnecting while the camera
was still recording, which points at the Android 16 promoted-notification path being built
whenever `capturing` turned true. Notification construction/updates are now wrapped so any
failure falls back to the standard ongoing notification, and promoted Live Updates are
disabled on Xiaomi builds until the vendor implementation is validated on device. Foreground
service types and the chronometer remain unchanged.

## Video GPS extraction crate

Added `crates/osmo_gps_extract_gpx`, a standalone MP4 -> GPX tool for Osmo Action
4/5/6 files. It finds the `stsd` sample entry `djmd` instead of assuming a fixed
track number, maps `stsc`/`stsz`/`stco` or `co64` to sample offsets, and decodes
each sample with `prost` using the public DJI `dvtm_ac203/ac204/ac206` field
numbering. GPX 1.1 output is produced by the `gpx` crate; timestamps use `time`
and the CLI supports `none`, `discard` and `linear` resampling (default 2 Hz
linear, matching pyosmogps' CLI default). Accelerometer/derivative GPX
extensions were dropped because `gpx` 0.10 does not expose extensions.

The MP4 container layer remains a small local box parser. `mp4` 0.14 rejects
DJI's 32-byte `udta/meta/hdlr` with `hdlr size too small`, and `mp4parse` is a
streaming parser that would read the whole 16 GB `mdat`; neither can seek straight
to the metadata samples.

Unit tests cover sample-table mapping (including multiple samples per chunk),
missing `djmd` detection, prost round-trip, timestamp parsing, resampling and GPX
output. Verified with `/Volumes/SD_Card/DCIM/DJI_001/DJI_20261105124234_0003_D.MP4`
(Action 6, 16 GB): detected `dvtm_ac206.proto`, `DJI OsmoAction6`, serial
`9KRVP8A00B06LC`, 71,386 raw points, first `22.7080722, 114.2485718, 52.287 m`
at `2026-11-05T02:42:52Z`, last `22.7518894, 114.4701271, 32.279 m` at
`2026-11-05T03:18:46Z`. Default 2 Hz linear output has 4,309 points. Action 4/5
files were not available for hardware verification.

`cargo fmt --all --check`, `cargo test --workspace` (18 tests) and
`cargo clippy --workspace --all-targets -- -D warnings` pass.

## Android: Material You, settings and capture controls

Reworked the scan, pairing and control screens using Material 3 surfaces and
system-bar insets. The capture control dispatches shutter or record commands
according to the camera mode. Status details collapse below the GPS row, and
temperature warnings remain visible. Android sends and displays verification
code 2333 during pairing.

Added settings for system permission screens, system/dark/light appearance,
a custom RGB theme seed and an About page. Appearance choices persist locally.
GPS starts once per new connection when precise location is available. Turning
GPS off during the session prevents an automatic restart on activity recreation.

The foreground service requests the active camera and location types. Android
16+ recordings request a Live Update when eligible. If construction or posting
fails, the service falls back to the ongoing notification with its chronometer.
The Xiaomi compatibility path uses the ongoing notification.

Rust fmt, workspace tests, strict Clippy and the iOS and Android cross-checks
pass. Android unit tests and both debug and R8 release builds pass.

An API 37 emulator run with simulated camera events verified theme settings
across activity recreation, the 2333 pairing dialog, the mode-specific capture
button, GPS auto-start and manual stop, and the transition between a promoted
recording notification and the ordinary ongoing notification. Camera pairing,
GPS data in captured video and Xiaomi notification presentation still require
hardware verification.

## Android: predictive back

Settings and About use AndroidX predictive back progress to slide the current
page away from the touched edge and reveal its parent. A cancelled gesture
restores the current page. The remote page delegates back-to-home to Android.
Activity Compose 1.9.3 supplies the fixes for ordinary back dispatch after
a predictive gesture.

On the API 37 emulator, injected edge gestures verified left and right preview,
cancellation, commit, dialog dismissal and system back-to-home. Ordinary
dispatcher back after cancellation and toolbar back also passed. Rust fmt,
workspace tests and strict Clippy, Android unit tests, and debug and R8 release
builds passed.

## Android: system cross-activity transitions

Moved Settings and About into non-exported activities in the same task as
MainActivity. Android supplies the opening animation and predictive back
preview. Appearance is shared at application scope. The foreground notification
brings the existing remote activity to the front.

On the API 37 emulator, the screen recording showed the system opening
transition. Window Manager reported `TYPE_CROSS_ACTIVITY` between pages and
`TYPE_RETURN_TO_HOME` at the remote screen. Both swipe edges, cancellation,
dialog dismissal, shared appearance, and returning to the existing activity
from the notification passed. Rust checks, Android unit tests, and debug and
R8 release builds passed.

## Android: opt-in compatible back animation

Added a saved appearance switch for application-rendered navigation. The
setting takes effect the next time Settings is opened. System mode continues
to use separate activities. Compatible mode keeps Settings and About inside
the remote activity, slides new pages in, and positions the outgoing page
using touch coordinates from AndroidX predictive back events. An ordinary
back event still plays a slide when the system sends only completion.

The API 37 emulator passed mode switching, cancelled and committed gestures,
and ordinary back dispatch. Frame comparison measured 129 px of page movement
for about 130 px of forward finger travel, 86 px when dragging back, and zero
offset after cancellation. Debug, R8 release, Android unit tests and Rust
checks passed. HyperOS gesture callbacks still need device verification.

## iOS: native remote and recording Live Activity

Added the SwiftUI iOS 16.0 app and the WidgetKit recording extension under
`ios/`. The app reuses the Android mode catalog and status projection, and
uses the Rust BLE worker through a statically linked C ABI. It keeps the
controller identity and confirmed pairing history in Application Support.
Capture actions check the latest camera state, and recording waits for a status
change after the command ACK.

System navigation and the capture control adopt Liquid Glass on iOS 26.
On iOS 16.1 and later, ActivityKit presents confirmed recordings on the Lock
Screen and on Dynamic Island devices. The system renders the timer from the
reported elapsed time. A connection interruption replaces the timer with
reconnect status. Stopping the recording or disconnecting ends the activity.
Phone GPS injection is unavailable because Core Location lacks the satellite
count required by the DJI GPS packet.

Unsigned iPhone Debug and Release builds and the arm64 Simulator Debug build
passed on Xcode 27.2. The iPhone build embeds the extension and declares iOS 16.0
for the app and iOS 16.1 for the extension. A host smoke check decoded the 17-mode
Rust startup event in Swift. Rust fmt, all 18 workspace tests, strict Clippy and
the iOS device, iOS simulator and Android bridge cross-checks passed.
Pairing, camera control, background delivery and Live Activity presentation
still need hardware verification.
