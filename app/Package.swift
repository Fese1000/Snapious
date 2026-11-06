// swift-tools-version:5.9
import PackageDescription

// Snapious: native macOS-App (SwiftUI) auf dem Rust-Kern.
// Vor dem Bauen:  scripts/build-ffi.sh   (erzeugt Bindings und die statische Bibliothek)
let package = Package(
    name: "Snapious",
    platforms: [.macOS(.v14)],
    products: [
        .executable(name: "Snapious", targets: ["SnapiousApp"])
    ],
    targets: [
        // C-Header und Modulbeschreibung der Rust-Bibliothek (von UniFFI erzeugt)
        .systemLibrary(name: "snapious_ffiFFI", path: "Sources/CSnapiousFFI/include"),
        // Von UniFFI erzeugte Swift-Schnittstelle; linkt die Rust-Bibliothek statisch ein
        .target(
            name: "SnapiousFFI",
            dependencies: ["snapious_ffiFFI"],
            path: "Sources/SnapiousFFI",
            linkerSettings: [
                // Rust-Bibliothek statisch einbinden; "-w" blendet Hinweise zur Mindest-Version der
                // vorgebauten Rust-Standardbibliothek aus (harmlos)
                .unsafeFlags(["../target/release/libsnapious_ffi.a", "-Xlinker", "-w"]),
                .linkedLibrary("c++"),
            ]
        ),
        // Selbsttest ohne Fenster (nur Foundation): prüft die Verbindung Swift ↔ Rust
        .executableTarget(
            name: "SnapiousSelfTest",
            dependencies: ["SnapiousFFI"],
            path: "Sources/SnapiousSelfTest"
        ),
        // Oberfläche (SwiftUI; braucht die vollständige Xcode-Toolchain wegen der SwiftUI-Makros)
        .executableTarget(
            name: "SnapiousApp",
            dependencies: ["SnapiousFFI"],
            path: "Sources/SnapiousApp"
        ),
    ]
)
