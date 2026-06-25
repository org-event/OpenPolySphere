// swift-tools-version: 5.10
import PackageDescription

let package = Package(
    name: "polysphere-speech",
    platforms: [.macOS(.v14)],
    products: [
        .executable(name: "polysphere-speech", targets: ["PolySphereSpeech"]),
    ],
    targets: [
        .executableTarget(
            name: "PolySphereSpeech"
        ),
    ]
)
