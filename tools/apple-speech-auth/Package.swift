// swift-tools-version: 5.10
import PackageDescription

let package = Package(
    name: "LiveTranslateSpeech",
    platforms: [.macOS(.v14)],
    products: [
        .executable(name: "LiveTranslateSpeech", targets: ["AppleSpeechAuth"]),
    ],
    targets: [
        .executableTarget(name: "AppleSpeechAuth"),
    ]
)
