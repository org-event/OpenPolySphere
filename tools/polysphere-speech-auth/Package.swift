// swift-tools-version: 5.10
import PackageDescription

let package = Package(
    name: "PolySphereSpeech",
    platforms: [.macOS(.v14)],
    products: [
        .executable(name: "PolySphereSpeech", targets: ["PolySphereSpeechAuth"]),
    ],
    targets: [
        .executableTarget(name: "PolySphereSpeechAuth"),
    ]
)
