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

func emit(_ dict: [String: Any], outPath: String? = nil) {
    guard let data = try? encode(dict) else { return }
    if let outPath {
        try? data.write(to: URL(fileURLWithPath: outPath))
        return
    }
    FileHandle.standardOutput.write(data)
    FileHandle.standardOutput.write(Data("\n".utf8))
}

func parseFlags(_ args: [String]) -> (args: [String], outPath: String?, context: [String]) {
    var rest = args
    var outPath: String?
    var context: [String] = []
    while let i = rest.firstIndex(of: "--out"), i + 1 < rest.count {
        outPath = rest[i + 1]
        rest.removeSubrange(i...(i + 1))
    }
    while let i = rest.firstIndex(of: "--context"), i + 1 < rest.count {
        context = rest[i + 1]
            .split(separator: ",")
            .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { !$0.isEmpty }
        rest.removeSubrange(i...(i + 1))
    }
    return (rest, outPath, context)
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
    NSApplication.shared.setActivationPolicy(.accessory)
}

func check(lang: String, outPath: String? = nil) {
    let locale = Locale(identifier: localeId(for: lang))
    guard SFSpeechRecognizer(locale: locale) != nil else {
        emit([
            "available": false,
            "ready": false,
            "status": "unsupported",
            "lang": lang,
            "error": "Speech recognizer not available for \(lang)",
        ], outPath: outPath)
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
    ], outPath: outPath)
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
func runRecognition(
    recognizer: SFSpeechRecognizer,
    samples: [Float],
    sampleRate: Int,
    requireOnDevice: Bool,
    context: [String]
) async -> (transcript: String?, error: String?) {
    let request = SFSpeechAudioBufferRecognitionRequest()
    request.shouldReportPartialResults = false
    request.addsPunctuation = true
    request.taskHint = .dictation
    if !context.isEmpty {
        request.contextualStrings = context
    }
    if requireOnDevice, recognizer.supportsOnDeviceRecognition {
        request.requiresOnDeviceRecognition = true
    }

    guard let buffer = makePCMBuffer(samples: samples, sampleRate: Double(sampleRate)) else {
        return (nil, "Failed to build audio buffer")
    }
    request.append(buffer)
    request.endAudio()

    return await withCheckedContinuation { (continuation: CheckedContinuation<(String?, String?), Never>) in
        var finished = false
        let task = recognizer.recognitionTask(with: request) { result, error in
            if finished { return }
            if let error {
                finished = true
                continuation.resume(returning: (nil, error.localizedDescription))
                return
            }
            guard let result, result.isFinal else { return }
            finished = true
            continuation.resume(returning: (result.bestTranscription.formattedString, nil))
        }
        _ = task
    }
}

func siriDisabledHint(_ message: String) -> String {
    if message.localizedCaseInsensitiveContains("Siri and Dictation are disabled") {
        return "\(message). Enable Siri (System Settings → Apple Intelligence & Siri) and Dictation (Keyboard → Dictation), then restart."
    }
    return message
}

@MainActor
func recognizeOnMain(
    lang: String,
    sampleRate: Int,
    samples: [Float],
    outPath: String? = nil,
    context: [String] = []
) async {
    bootstrapAppKit()
    let locale = Locale(identifier: localeId(for: lang))
    guard let recognizer = SFSpeechRecognizer(locale: locale) else {
        emit(["error": "Speech recognizer not available for \(lang)"], outPath: outPath)
        return
    }

    let auth = SFSpeechRecognizer.authorizationStatus()
    guard auth == .authorized else {
        emit([
            "error": "Speech recognition not authorized (\(authorizationStatusString(auth))). Click “Allow Speech Recognition” in Settings.",
            "authorization": authorizationStatusString(auth),
        ], outPath: outPath)
        return
    }

    guard recognizer.isAvailable else {
        emit(["error": "Speech recognizer unavailable for \(locale.identifier)"], outPath: outPath)
        return
    }

    var outcome = await runRecognition(
        recognizer: recognizer,
        samples: samples,
        sampleRate: sampleRate,
        requireOnDevice: true,
        context: context
    )
    if let message = outcome.error,
       message.localizedCaseInsensitiveContains("Siri and Dictation are disabled") {
        outcome = await runRecognition(
            recognizer: recognizer,
            samples: samples,
            sampleRate: sampleRate,
            requireOnDevice: false,
            context: context
        )
    }

    if let transcript = outcome.transcript {
        emit(["transcript": transcript, "no_speech_prob": 0.0], outPath: outPath)
    } else if let message = outcome.error {
        emit(["error": siriDisabledHint(message)], outPath: outPath)
    }
}

func recognizeSync(
    lang: String,
    sampleRate: Int,
    samples: [Float],
    outPath: String? = nil,
    context: [String] = []
) {
    bootstrapAppKit()
    let semaphore = DispatchSemaphore(value: 0)
    Task { @MainActor in
        await recognizeOnMain(
            lang: lang,
            sampleRate: sampleRate,
            samples: samples,
            outPath: outPath,
            context: context
        )
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

final class AuthDelegate: NSObject, NSApplicationDelegate {
    let outPath: String?

    init(outPath: String?) {
        self.outPath = outPath
    }

    func applicationDidFinishLaunching(_ notification: Notification) {
        NSApplication.shared.activate(ignoringOtherApps: true)
        SFSpeechRecognizer.requestAuthorization { status in
            let payload: [String: Any] = [
                "authorization": authorizationStatusString(status),
                "ready": status == .authorized,
            ]
            if let outPath = self.outPath,
               let data = try? JSONSerialization.data(withJSONObject: payload, options: [.sortedKeys]) {
                try? data.write(to: URL(fileURLWithPath: outPath))
            } else {
                emit(payload)
            }
            DispatchQueue.main.async {
                NSApplication.shared.terminate(status == .authorized ? 0 : 1)
            }
        }
    }
}

private var authDelegateHolder: AuthDelegate?

func authorizeApp(outPath: String?) {
    let app = NSApplication.shared
    app.setActivationPolicy(.accessory)
    let delegate = AuthDelegate(outPath: outPath)
    authDelegateHolder = delegate
    app.delegate = delegate
    app.activate(ignoringOtherApps: true)
    app.run()
    authDelegateHolder = nil
}

@main
enum LiveTranslateSpeechMain {
    static func main() {
        let args = Array(CommandLine.arguments.dropFirst())
        guard let first = args.first else {
            authorizeApp(outPath: nil)
            return
        }

        switch first {
        case "check":
            let (parsed, outPath, _) = parseFlags(Array(args.dropFirst()))
            guard parsed.count == 1 else {
                fputs("usage: LiveTranslateSpeech check <lang> [--out path]\n", stderr)
                exit(2)
            }
            check(lang: parsed[0], outPath: outPath)
            exit(0)
        case "recognize":
            let (parsed, outPath, context) = parseFlags(Array(args.dropFirst()))
            guard parsed.count >= 2 else {
                fputs("usage: LiveTranslateSpeech recognize <lang> <sample_rate> [pcm_file] [--context words] [--out path]\n", stderr)
                exit(2)
            }
            let lang = parsed[0]
            guard let sampleRate = Int(parsed[1]) else {
                emit(["error": "Invalid sample rate"], outPath: outPath)
                exit(1)
            }
            let path = parsed.count > 2 ? parsed[2] : nil
            guard let samples = readFloatSamples(from: path), !samples.isEmpty else {
                emit(["error": "No PCM float32 samples"], outPath: outPath)
                exit(1)
            }
            recognizeSync(
                lang: lang,
                sampleRate: sampleRate,
                samples: samples,
                outPath: outPath,
                context: context
            )
            exit(0)
        default:
            // `open -W LiveTranslator.app --args /tmp/auth.json` passes the output path.
            authorizeApp(outPath: first)
        }
    }
}
