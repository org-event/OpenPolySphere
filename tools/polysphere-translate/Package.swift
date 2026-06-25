// swift-tools-version: 5.10
import PackageDescription

let package = Package(
    name: "polysphere-translate",
    platforms: [.macOS(.v14)],
    products: [
        .executable(name: "polysphere-translate", targets: ["PolySphereTranslate"]),
    ],
    targets: [
        .executableTarget(name: "PolySphereTranslate"),
    ]
)
