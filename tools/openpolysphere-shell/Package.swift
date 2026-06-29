// swift-tools-version: 5.10
import PackageDescription

let package = Package(
    name: "OpenPolySphereShell",
    platforms: [.macOS(.v14)],
    products: [
        .executable(name: "OpenPolySphere", targets: ["OpenPolySphereShell"]),
    ],
    targets: [
        .executableTarget(
            name: "OpenPolySphereShell",
            linkerSettings: [
                .linkedFramework("AppKit"),
                .linkedFramework("WebKit"),
            ]
        ),
    ]
)
