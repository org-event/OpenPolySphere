// swift-tools-version: 5.10
import PackageDescription

let package = Package(
    name: "apple-translate",
    platforms: [.macOS(.v14)],
    products: [
        .executable(name: "apple-translate", targets: ["AppleTranslate"]),
    ],
    targets: [
        .executableTarget(name: "AppleTranslate"),
    ]
)
