import AppKit
import Darwin
import Foundation
import WebKit

private let serverURL = URL(string: "http://127.0.0.1:5050/")!

private var serverProcess: Process?
private var appDelegate: ShellAppDelegate?

// MARK: - Bundle paths

private func bundleResourcesURL() -> URL {
    Bundle.main.bundleURL.appendingPathComponent("Contents/Resources", isDirectory: true)
}

private func bundleFrameworksURL() -> URL {
    Bundle.main.bundleURL.appendingPathComponent("Contents/Frameworks", isDirectory: true)
}

private func userSupportURL() -> URL {
    let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
    return base.appendingPathComponent("OpenPolySphere", isDirectory: true)
}

private func translatorURL() -> URL {
    bundleResourcesURL().appendingPathComponent("translator")
}

private func applyBundleEnvironment() {
    let res = bundleResourcesURL()
    let support = userSupportURL()
    try? FileManager.default.createDirectory(at: support, withIntermediateDirectories: true)

    setenv("CALL_TRANSLATOR_HOME", res.path, 1)
    setenv("TRANSLATOR_DATA_DIR", support.path, 1)
    setenv(
        "ORT_DYLIB_PATH",
        bundleFrameworksURL().appendingPathComponent("libonnxruntime.dylib").path,
        1
    )
    setenv(
        "POLYSPHERE_SPEECH_AUTH_APP",
        res.appendingPathComponent("Helpers/PolySphereSpeech.app").path,
        1
    )
}

private func loadDotEnv() {
    let envFile = userSupportURL().appendingPathComponent(".env")
    guard let raw = try? String(contentsOf: envFile, encoding: .utf8) else { return }

    var groqValue: String?
    for line in raw.split(separator: "\n", omittingEmptySubsequences: false) {
        let trimmed = line.trimmingCharacters(in: .whitespaces)
        if trimmed.isEmpty || trimmed.hasPrefix("#") { continue }
        guard let eq = trimmed.firstIndex(of: "=") else { continue }
        let key = String(trimmed[..<eq])
        let value = String(trimmed[trimmed.index(after: eq)...])
            .trimmingCharacters(in: .whitespaces)
            .trimmingCharacters(in: CharacterSet(charactersIn: "\"'"))
        if key == "GROQ_API_KEY", !value.isEmpty {
            groqValue = value
        }
        if stdlib_getenv(key) == nil {
            setenv(key, value, 1)
        }
    }
    if groqValue != nil, stdlib_getenv("OPENROUTER_API_KEY") == nil {
        setenv("OPENROUTER_API_KEY", groqValue!, 1)
    }
}

private func stdlib_getenv(_ name: String) -> String? {
    guard let c = getenv(name) else { return nil }
    return String(cString: c)
}

// MARK: - CLI forwarding (setup, --help, etc.)

private func runTranslatorCLI(arguments: [String]) -> Never {
    applyBundleEnvironment()
    loadDotEnv()

    let translator = translatorURL()
    let argv = [translator.path] + arguments
    let cArgs = argv.map { strdup($0) } + [nil]
    defer { cArgs.compactMap { $0 }.forEach { free($0) } }

    execv(translator.path, cArgs)
    fputs("failed to exec translator at \(translator.path)\n", stderr)
    exit(127)
}

// MARK: - Server process

private func startServer() throws {
    applyBundleEnvironment()
    loadDotEnv()

    let process = Process()
    process.executableURL = translatorURL()
    process.arguments = ["serve"]
    process.environment = ProcessInfo.processInfo.environment
    try process.run()
    serverProcess = process
}

private func stopServer() {
    guard let process = serverProcess, process.isRunning else {
        serverProcess = nil
        return
    }
    process.terminate()
    let deadline = Date().addingTimeInterval(5)
    while process.isRunning, Date() < deadline {
        Thread.sleep(forTimeInterval: 0.1)
    }
    if process.isRunning {
        kill(process.processIdentifier, SIGKILL)
    }
    process.waitUntilExit()
    serverProcess = nil
}

private func waitForServer(timeout: TimeInterval = 30) -> Bool {
    let deadline = Date().addingTimeInterval(timeout)
    while Date() < deadline {
        if serverResponding() { return true }
        if let process = serverProcess, !process.isRunning { return false }
        Thread.sleep(forTimeInterval: 0.25)
    }
    return false
}

private func serverResponding() -> Bool {
    let sem = DispatchSemaphore(value: 0)
    var ok = false
    var request = URLRequest(url: serverURL)
    request.httpMethod = "HEAD"
    request.timeoutInterval = 2
    URLSession.shared.dataTask(with: request) { _, response, _ in
        if let http = response as? HTTPURLResponse, (200 ... 499).contains(http.statusCode) {
            ok = true
        }
        sem.signal()
    }.resume()
    _ = sem.wait(timeout: .now() + 3)
    return ok
}

// MARK: - GUI

final class ShellAppDelegate: NSObject, NSApplicationDelegate, NSWindowDelegate {
    private var window: NSWindow!
    private var webView: WKWebView!

    func applicationDidFinishLaunching(_ notification: Notification) {
        NSApp.setActivationPolicy(.regular)
        do {
            try startServer()
        } catch {
            showFatal("Could not start OpenPolySphere server:\n\(error.localizedDescription)")
            return
        }

        guard waitForServer() else {
            stopServer()
            showFatal("OpenPolySphere server did not start on http://127.0.0.1:5050")
            return
        }

        let config = WKWebViewConfiguration()
        config.preferences.setValue(true, forKey: "developerExtrasEnabled")
        webView = WKWebView(frame: .zero, configuration: config)
        webView.load(URLRequest(url: serverURL))

        window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 1180, height: 820),
            styleMask: [.titled, .closable, .miniaturizable, .resizable],
            backing: .buffered,
            defer: false
        )
        window.title = "OpenPolySphere"
        window.contentView = webView
        window.delegate = self
        window.center()
        window.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        true
    }

    func applicationWillTerminate(_ notification: Notification) {
        stopServer()
    }

    func windowWillClose(_ notification: Notification) {
        NSApp.terminate(nil)
    }

    private func showFatal(_ message: String) {
        let alert = NSAlert()
        alert.messageText = "OpenPolySphere"
        alert.informativeText = message
        alert.alertStyle = .critical
        alert.runModal()
        NSApp.terminate(nil)
    }
}

// MARK: - Entry

if CommandLine.arguments.count > 1 {
    runTranslatorCLI(arguments: Array(CommandLine.arguments.dropFirst()))
}

let app = NSApplication.shared
let delegate = ShellAppDelegate()
appDelegate = delegate
app.delegate = delegate
app.run()
