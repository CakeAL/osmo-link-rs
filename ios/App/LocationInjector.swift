import CoreLocation
import Foundation
import OSLog

/// Streams Core Location fixes into the Rust bridge.
///
/// iOS does not expose how many satellites contributed to a fix. The protocol
/// field is therefore transmitted as `0` instead of estimating a count.
@MainActor
final class LocationInjector: NSObject, ObservableObject {
    @Published private(set) var isRunning = false
    @Published private(set) var authorization = CLAuthorizationStatus.notDetermined
    @Published private(set) var rate = 0.0
    @Published private(set) var accuracy: Double?
    @Published private(set) var hasFix = false
    @Published private(set) var message: String?

    private let manager = CLLocationManager()
    private let logger = Logger(subsystem: "com.cakeal.osmolink", category: "Location")
    private var wantsUpdates = false
    private var fixTimes: [Date] = []

    override init() {
        super.init()
        authorization = manager.authorizationStatus
        manager.delegate = self
        manager.desiredAccuracy = kCLLocationAccuracyBest
        manager.distanceFilter = kCLDistanceFilterNone
        manager.pausesLocationUpdatesAutomatically = false
        manager.activityType = .otherNavigation
    }

    var isAuthorized: Bool {
        authorization == .authorizedWhenInUse || authorization == .authorizedAlways
    }

    var needsSettings: Bool {
        authorization == .denied || authorization == .restricted
    }

    func start() {
        wantsUpdates = true
        switch manager.authorizationStatus {
        case .notDetermined:
            message = "正在请求定位权限…"
            manager.requestWhenInUseAuthorization()
        case .authorizedWhenInUse, .authorizedAlways:
            beginUpdates()
        case .denied, .restricted:
            isRunning = false
            message = "定位权限已关闭，请在系统设置中允许。"
        @unknown default:
            isRunning = false
            message = "当前系统无法使用定位。"
        }
    }

    func stop() {
        wantsUpdates = false
        manager.stopUpdatingLocation()
        osmo_ios_gps_stop()
        isRunning = false
        rate = 0
        accuracy = nil
        hasFix = false
        message = nil
        fixTimes.removeAll()
    }

    private func beginUpdates() {
        guard wantsUpdates, !isRunning else { return }
        manager.startUpdatingLocation()
        isRunning = true
        message = nil
    }

    private func handleAuthorization(_ status: CLAuthorizationStatus) {
        authorization = status
        guard wantsUpdates else { return }
        switch status {
        case .authorizedWhenInUse, .authorizedAlways:
            beginUpdates()
        case .denied, .restricted:
            isRunning = false
            message = "定位权限已关闭，请在系统设置中允许。"
        default:
            break
        }
    }

    private func emit(_ fix: LocationFix) {
        guard wantsUpdates else { return }
        let now = Date()
        fixTimes.append(now)
        fixTimes.removeAll { now.timeIntervalSince($0) > 5 }
        if let first = fixTimes.first, fixTimes.count > 1 {
            rate = Double(fixTimes.count - 1) / max(0.01, now.timeIntervalSince(first))
        }

        let sample = GpsSample(fix: fix)
        guard let data = try? JSONEncoder().encode(sample),
              let json = String(data: data, encoding: .utf8)
        else {
            logger.error("Could not encode location sample")
            return
        }
        if !osmo_ios_gps_update(json) {
            message = "定位暂未写入相机。"
        } else {
            message = nil
        }
        hasFix = true
        accuracy = fix.horizontalAccuracy
    }

    private func report(_ error: Error) {
        switch (error as? CLError)?.code {
        case .denied:
            message = "定位权限已关闭，请在系统设置中允许。"
        case .locationUnknown:
            // Core Location emits this while it is still acquiring a fix.
            break
        default:
            message = error.localizedDescription
        }
    }
}

extension LocationInjector: CLLocationManagerDelegate {
    nonisolated func locationManagerDidChangeAuthorization(_ manager: CLLocationManager) {
        let status = manager.authorizationStatus
        Task { @MainActor in handleAuthorization(status) }
    }

    nonisolated func locationManager(_ manager: CLLocationManager, didUpdateLocations locations: [CLLocation]) {
        guard let location = locations.last,
              location.horizontalAccuracy >= 0,
              abs(location.timestamp.timeIntervalSinceNow) < 5
        else {
            return
        }
        let fix = LocationFix(location)
        Task { @MainActor in emit(fix) }
    }

    nonisolated func locationManager(_ manager: CLLocationManager, didFailWithError error: Error) {
        Task { @MainActor in report(error) }
    }
}

/// Sendable snapshot extracted from `CLLocation` before crossing actors.
private struct LocationFix: Sendable {
    let timestampMilliseconds: Int64
    let longitude: Double
    let latitude: Double
    let altitude: Double
    let northMetersPerSecond: Double
    let eastMetersPerSecond: Double
    let verticalAccuracy: Double
    let horizontalAccuracy: Double
    let speedAccuracy: Double

    init(_ location: CLLocation) {
        timestampMilliseconds = Int64(location.timestamp.timeIntervalSince1970 * 1000)
        longitude = location.coordinate.longitude
        latitude = location.coordinate.latitude
        altitude = location.verticalAccuracy >= 0 ? location.altitude : 0
        verticalAccuracy = max(0, location.verticalAccuracy)
        horizontalAccuracy = max(0, location.horizontalAccuracy)
        speedAccuracy = max(0, location.speedAccuracy)
        let speed = max(0, location.speed)
        let course = location.course >= 0 ? location.course * .pi / 180 : 0
        northMetersPerSecond = speed * cos(course)
        eastMetersPerSecond = speed * sin(course)
    }
}

private struct GpsSample: Encodable {
    let timestamp_ms: Int64
    let longitude: Double
    let latitude: Double
    let altitude_m: Double
    let north_mps: Double
    let east_mps: Double
    let down_mps = 0.0
    let vertical_accuracy_m: Double
    let horizontal_accuracy_m: Double
    let speed_accuracy_mps: Double
    /// Core Location has no public satellite count. The Rust bridge transmits
    /// this field verbatim, so zero means "unavailable" to the camera.
    let satellites: UInt32 = 0

    init(fix: LocationFix) {
        timestamp_ms = fix.timestampMilliseconds
        longitude = fix.longitude
        latitude = fix.latitude
        altitude_m = fix.altitude
        north_mps = fix.northMetersPerSecond
        east_mps = fix.eastMetersPerSecond
        vertical_accuracy_m = fix.verticalAccuracy
        horizontal_accuracy_m = fix.horizontalAccuracy
        speed_accuracy_mps = fix.speedAccuracy
    }
}
