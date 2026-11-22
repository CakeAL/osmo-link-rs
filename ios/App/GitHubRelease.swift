import Foundation

struct GitHubRelease: Decodable, Identifiable {
    let tag: String
    let notes: String?
    let pageURL: URL

    var id: String { tag }

    private enum CodingKeys: String, CodingKey {
        case tag = "tag_name"
        case notes = "body"
        case pageURL = "html_url"
    }

    enum CheckResult {
        case noRelease
        case upToDate
        case available(GitHubRelease)
    }

    private static let endpoint = URL(string: "https://api.github.com/repos/CakeAL/osmo-link-rs/releases/latest")!
    private static let releasePage = "https://github.com/CakeAL/osmo-link-rs/releases/tag/"

    static func check(installedVersion: String, session: URLSession = .shared) async throws -> CheckResult {
        var request = URLRequest(url: endpoint, timeoutInterval: 10)
        request.setValue("application/vnd.github+json", forHTTPHeaderField: "Accept")
        let (data, response) = try await session.data(for: request)
        switch (response as? HTTPURLResponse)?.statusCode {
        case 404:
            return .noRelease
        case 200:
            let release = try JSONDecoder().decode(Self.self, from: data)
            guard try isNewer(release.tag, than: installedVersion) else { return .upToDate }
            guard release.pageURL.absoluteString.lowercased().hasPrefix(releasePage.lowercased()) else {
                throw URLError(.badURL)
            }
            return .available(release)
        default:
            throw URLError(.badServerResponse)
        }
    }

    static func isNewer(_ tag: String, than installedVersion: String) throws -> Bool {
        let installed = try versionParts(installedVersion)
        let latest = try versionParts(tag)
        return installed.lexicographicallyPrecedes(latest)
    }

    private static func versionParts(_ version: String) throws -> [Int32] {
        let unprefixed = version.first == "v" || version.first == "V" ? version.dropFirst() : version[...]
        let parts = unprefixed.split(separator: ".", omittingEmptySubsequences: false)
        guard (2...3).contains(parts.count) else { throw URLError(.cannotParseResponse) }
        var numbers = try parts.map { part in
            guard part.utf8.allSatisfy({ (48...57).contains($0) }), let number = Int32(part) else {
                throw URLError(.cannotParseResponse)
            }
            return number
        }
        if numbers.count == 2 { numbers.append(0) }
        return numbers
    }
}
