// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "parakeet-coreml",
    platforms: [
        .macOS(.v14)
    ],
    dependencies: [
        .package(url: "https://github.com/FluidInference/FluidAudio.git", from: "0.12.0"),
    ],
    targets: [
        .executableTarget(
            name: "parakeet-coreml",
            dependencies: [
                .product(name: "FluidAudio", package: "FluidAudio"),
            ],
            path: "Sources"
        ),
    ]
)
