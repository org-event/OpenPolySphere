import Foundation

#if canImport(Translation)
import Translation

@available(macOS 15.0, *)
enum StatusPayload {
    static func encode(_ dict: [String: Any]) throws -> Data {
        try JSONSerialization.data(withJSONObject: dict, options: [.sortedKeys])
    }

    static func statusString(_ status: LanguageAvailability.Status) -> String {
        switch status {
        case .installed:
            return "installed"
        case .supported:
            return "supported"
        case .unsupported:
            return "unsupported"
        @unknown default:
            return "unsupported"
        }
    }

    static func check(from: String, to: String) async throws -> Data {
        let source = Locale.Language(identifier: from)
        let target = Locale.Language(identifier: to)
        let availability = LanguageAvailability()
        let status = await availability.status(from: source, to: target)
        let statusStr = statusString(status)
        let available = status == .installed || status == .supported
        let ready = status == .installed
        return try encode([
            "available": available,
            "ready": ready,
            "status": statusStr,
            "from": from,
            "to": to,
        ])
    }

    static func translate(from: String, to: String, text: String) async throws -> Data {
        let source = Locale.Language(identifier: from)
        let target = Locale.Language(identifier: to)
        let availability = LanguageAvailability()
        let status = await availability.status(from: source, to: target)
        guard status == .installed else {
            let hint = status == .supported
                ? "Language pack not downloaded. Open System Settings → General → Language & Region, or the Translate app, and add \(from) and \(to)."
                : "This language pair is not supported by Apple Translation."
            return try encode(["error": hint, "status": statusString(status)])
        }

        let session: TranslationSession
        if #available(macOS 26.0, *) {
            session = TranslationSession(installedSource: source, target: target)
            guard await session.isReady else {
                return try encode(["error": "Translation session not ready"])
            }
        } else {
            return try encode([
                "error": "Headless Apple Translation requires macOS 26.0 or later. Use Opus-MT or OpenRouter on this Mac.",
                "status": statusString(status),
            ])
        }

        let response = try await session.translate(text)
        return try encode(["translation": response.targetText])
    }
}

@available(macOS 15.0, *)
func run() async {
    let args = CommandLine.arguments.dropFirst()
    guard let command = args.first else {
        fputs("usage: apple-translate check <from> <to>\n", stderr)
        fputs("       apple-translate translate <from> <to> <text>\n", stderr)
        exit(2)
    }

    do {
        switch command {
        case "check":
            let rest = Array(args.dropFirst())
            guard rest.count == 2 else {
                fputs("usage: apple-translate check <from> <to>\n", stderr)
                exit(2)
            }
            let data = try await StatusPayload.check(from: rest[0], to: rest[1])
            FileHandle.standardOutput.write(data)
            FileHandle.standardOutput.write(Data("\n".utf8))
        case "translate":
            let rest = Array(args.dropFirst())
            guard rest.count >= 3 else {
                fputs("usage: apple-translate translate <from> <to> <text>\n", stderr)
                exit(2)
            }
            let from = rest[0]
            let to = rest[1]
            let text = rest.dropFirst(2).joined(separator: " ")
            let data = try await StatusPayload.translate(from: from, to: to, text: text)
            FileHandle.standardOutput.write(data)
            FileHandle.standardOutput.write(Data("\n".utf8))
        default:
            fputs("unknown command: \(command)\n", stderr)
            exit(2)
        }
    } catch {
        let payload = try? JSONSerialization.data(withJSONObject: ["error": error.localizedDescription])
        if let payload {
            FileHandle.standardOutput.write(payload)
            FileHandle.standardOutput.write(Data("\n".utf8))
        }
        exit(1)
    }
}

if #available(macOS 15.0, *) {
    await run()
} else {
    let payload = try! JSONSerialization.data(withJSONObject: [
        "available": false,
        "ready": false,
        "status": "unsupported",
        "error": "Requires macOS 15.0 or later",
    ])
    FileHandle.standardOutput.write(payload)
    FileHandle.standardOutput.write(Data("\n".utf8))
    exit(1)
}

#else

let payload = try! JSONSerialization.data(withJSONObject: [
    "available": false,
    "ready": false,
    "status": "unsupported",
    "error": "Translation framework unavailable",
])
FileHandle.standardOutput.write(payload)
FileHandle.standardOutput.write(Data("\n".utf8))
exit(1)

#endif
