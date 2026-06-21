import AppKit
import AVFoundation
import Foundation
import Speech

func localeId(for code: String) -> String {
    switch code.lowercased() {
    case "ru": return "ru-RU"
    case "en": return "en-US"
    case "de": return "de-DE"
    case "fr": return "fr-FR"
    case "es": return "es-ES"
    case "it": return "it-IT"
    case "pt": return "pt-BR"
    case "uk": return "uk-UA"
    case "pl": return "pl-PL"
    case "ja": return "ja-JP"
    case "ko": return "ko-KR"
    case "zh": return "zh-CN"
    default:
        if code.contains("-") { return code }
        return "\(code)-\(code.uppercased())"
    }
}

func encode(_ dict: [String: Any]) throws -> Data {
    try JSONSerialization.data(withJSONObject: dict, options: [.sortedKeys])
}

func emit(_ dict: [String: Any]) {
    if let data = try? encode(dict) {
        FileHandle.standardOutput.write(data)
        FileHandle.standardOutput.write(Data("\n".utf8))
    }
}

func authorizationStatusString(_ status: SFSpeechRecognizerAuthorizationStatus) -> String {
    switch status {
    case .authorized: return "authorized"
    case .denied: return "denied"
    case .restricted: return "restricted"
    case .notDetermined: return "not_determined"
    @unknown default: return "unknown"
    }
}

func bootstrapAppKit() {
    let app = NSApplication.shared
    app.setActivationPolicy(.accessory)
}

final class AuthorizeDelegate: NSObject, NSApplicationDelegate {
    func applicationDidFinishLaunching(_ notification: Notification) {
        NSApplication.shared.activate(ignoringOtherApps: true)
        SFSpeechRecognizer.requestAuthorization { status in
            let payload: [String: Any] = [
                "authorization": authorizationStatusString(status),
                "ready": status == .authorized,
                "message": status == .authorized
                    ? "Speech recognition authorized"
                    : "Open System Settings → Privacy & Security → Speech Recognition and allow Live Translator.",
            ]
            emit(payload)
            NSApplication.shared.terminate(status == .authorized ? 0 : 1)
        }
    }
}

private var authorizeDelegateHolder: AuthorizeDelegate?

func authorizeSync() {
    let app = NSApplication.shared
    app.setActivationPolicy(.accessory)
    let delegate = AuthorizeDelegate()
    authorizeDelegateHolder = delegate
    app.delegate = delegate
    app.activate(ignoringOtherApps: true)
    app.run()
    authorizeDelegateHolder = nil
}

func check(lang: String) {
    let locale = Locale(identifier: localeId(for: lang))
    guard SFSpeechRecognizer(locale: locale) != nil else {
        emit([
            "available": false,
            "ready": false,
            "status": "unsupported",
            "lang": lang,
            "error": "Speech recognizer not available for \(lang)",
        ])
        return
    }

    let auth = SFSpeechRecognizer.authorizationStatus()
    let authStr = authorizationStatusString(auth)
    let ready = auth == .authorized
    let recognizer = SFSpeechRecognizer(locale: locale)!
    let onDevice = recognizer.supportsOnDeviceRecognition

    emit([
        "available": true,
        "ready": ready,
        "on_device": onDevice,
        "status": ready ? "ready" : "needs_permission",
        "authorization": authStr,
        "locale": locale.identifier,
        "lang": lang,
    ])
}

func makePCMBuffer(samples: [Float], sampleRate: Double) -> AVAudioPCMBuffer? {
    guard let format = AVAudioFormat(
        commonFormat: .pcmFormatFloat32,
        sampleRate: sampleRate,
        channels: 1,
        interleaved: false
    ) else { return nil }

    guard let buffer = AVAudioPCMBuffer(pcmFormat: format, frameCapacity: AVAudioFrameCount(samples.count)) else {
        return nil
    }
    buffer.frameLength = AVAudioFrameCount(samples.count)
    guard let channel = buffer.floatChannelData?[0] else { return nil }
    for (i, sample) in samples.enumerated() {
        channel[i] = sample
    }
    return buffer
}

@MainActor
func recognizeOnMain(lang: String, sampleRate: Int, samples: [Float]) async {
    bootstrapAppKit()
    let locale = Locale(identifier: localeId(for: lang))
    guard let recognizer = SFSpeechRecognizer(locale: locale) else {
        emit(["error": "Speech recognizer not available for \(lang)"])
        return
    }

    let auth = SFSpeechRecognizer.authorizationStatus()
    guard auth == .authorized else {
        emit([
            "error": "Speech recognition not authorized (\(authorizationStatusString(auth))). Click “Allow Speech Recognition” in Settings.",
            "authorization": authorizationStatusString(auth),
        ])
        return
    }

    guard recognizer.isAvailable else {
        emit(["error": "Speech recognizer unavailable for \(locale.identifier)"])
        return
    }

    let request = SFSpeechAudioBufferRecognitionRequest()
    request.shouldReportPartialResults = false
    request.addsPunctuation = true
    request.taskHint = .dictation
    if recognizer.supportsOnDeviceRecognition {
        request.requiresOnDeviceRecognition = true
    }

    guard let buffer = makePCMBuffer(samples: samples, sampleRate: Double(sampleRate)) else {
        emit(["error": "Failed to build audio buffer"])
        return
    }
    request.append(buffer)
    request.endAudio()

    let transcript: String? = await withCheckedContinuation { continuation in
        var finished = false
        let task = recognizer.recognitionTask(with: request) { result, error in
            if finished { return }
            if let error {
                finished = true
                emit(["error": error.localizedDescription])
                continuation.resume(returning: nil)
                return
            }
            guard let result, result.isFinal else { return }
            finished = true
            continuation.resume(returning: result.bestTranscription.formattedString)
        }
        _ = task
    }

    if let transcript {
        emit(["transcript": transcript, "no_speech_prob": 0.0])
    }
}

func recognizeSync(lang: String, sampleRate: Int, samples: [Float]) {
    bootstrapAppKit()
    let semaphore = DispatchSemaphore(value: 0)
    Task { @MainActor in
        await recognizeOnMain(lang: lang, sampleRate: sampleRate, samples: samples)
        semaphore.signal()
    }
    while semaphore.wait(timeout: .now()) == .timedOut {
        RunLoop.current.run(mode: .default, before: Date(timeIntervalSinceNow: 0.05))
    }
}

func readFloatSamples(from path: String?) -> [Float]? {
    let data: Data
    if let path {
        guard let fileData = try? Data(contentsOf: URL(fileURLWithPath: path)) else { return nil }
        data = fileData
    } else {
        data = FileHandle.standardInput.readDataToEndOfFile()
    }
    guard !data.isEmpty, data.count % MemoryLayout<Float>.size == 0 else { return nil }
    return data.withUnsafeBytes { raw in
        let buffer = raw.bindMemory(to: Float.self)
        return Array(buffer)
    }
}

@main
enum AppleSpeechMain {
    static func main() {
        let args = Array(CommandLine.arguments.dropFirst())
        guard let command = args.first else {
            fputs("usage: apple-speech check <lang>\n", stderr)
            fputs("       apple-speech authorize\n", stderr)
            fputs("       apple-speech recognize <lang> <sample_rate> [pcm_file]\n", stderr)
            exit(2)
        }

        switch command {
        case "check":
            guard args.count == 2 else {
                fputs("usage: apple-speech check <lang>\n", stderr)
                exit(2)
            }
            check(lang: args[1])
        case "authorize":
            authorizeSync()
        case "recognize":
            guard args.count >= 3 else {
                fputs("usage: apple-speech recognize <lang> <sample_rate> [pcm_file]\n", stderr)
                exit(2)
            }
            let lang = args[1]
            guard let sampleRate = Int(args[2]) else {
                emit(["error": "Invalid sample rate"])
                exit(1)
            }
            let path = args.count > 3 ? args[3] : nil
            guard let samples = readFloatSamples(from: path), !samples.isEmpty else {
                emit(["error": "No PCM float32 samples"])
                exit(1)
            }
            recognizeSync(lang: lang, sampleRate: sampleRate, samples: samples)
        default:
            fputs("unknown command: \(command)\n", stderr)
            exit(2)
        }
    }
}
