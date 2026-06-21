// swift-tools-version: 5.10
import PackageDescription

let package = Package(
    name: "apple-speech",
    platforms: [.macOS(.v14)],
    products: [
        .executable(name: "apple-speech", targets: ["AppleSpeech"]),
    ],
    targets: [
        .executableTarget(
            name: "AppleSpeech"
        ),
    ]
)
