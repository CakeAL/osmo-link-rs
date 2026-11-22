import CoreLocation
import Foundation

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
        stopGps()
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

        let sample = LocationSample(
            timestampMs: fix.timestampMilliseconds,
            longitude: fix.longitude,
            latitude: fix.latitude,
            altitudeM: fix.altitude,
            northMps: fix.northMetersPerSecond,
            eastMps: fix.eastMetersPerSecond,
            downMps: 0,
            verticalAccuracyM: fix.verticalAccuracy,
            horizontalAccuracyM: fix.horizontalAccuracy,
            speedAccuracyMps: fix.speedAccuracy,
            satellites: 0
        )
        if !updateGps(sample: sample) {
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
