//! Registry record for the `tinyvoice` module.

#[cfg(feature = "voice")]
use crate::modules::types::{LoadPolicy, ModuleRecord, PlatformAsset, SourceSpec};

#[cfg(feature = "voice")]
/// The `tinyvoice` module: the host-agnostic half of the voice pipeline.
///
/// STT hallucination detection used by the voice transcription pipeline.
///
/// Lazy: transcript screening loads the module only when voice transcription
/// needs it.
pub(crate) const TINYVOICE: ModuleRecord = ModuleRecord {
    id: "tinyvoice",
    description: "Voice transcript hallucination detection",
    bus_name: tinyvoice_bus::names::BUS_NAME,
    object_path: tinyvoice_bus::names::OBJECT_PATH,
    version: "0.4.0",
    release_url: "https://github.com/tinyhumansai/tinyvoice/releases/tag/v0.4.0",
    assets: &[
        PlatformAsset {
            host_key: "archlinux-arm64",
            archive: "tinyvoice-module-0.4.0-archlinux-arm64.tar.gz",
            sha256: "216a99dc13cb7216fd8682bb88c87e381edcc4432ac4eb865db2b473cf18656a",
        },
        PlatformAsset {
            host_key: "archlinux-x86_64",
            archive: "tinyvoice-module-0.4.0-archlinux-x86_64.tar.gz",
            sha256: "1de1939573e21248ede86a568f511e8d1f4a493e6cd38f5fdcd942d5000734cc",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinyvoice-module-0.4.0-fedora-43-arm64.tar.gz",
            sha256: "97164ab70529a8f38b7650c3bcbb92798c55140275f0302a811148f9a37a39e2",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinyvoice-module-0.4.0-fedora-43-x86_64.tar.gz",
            sha256: "031cad7e8a4e47a2641d7a56ff8c6ca83a05b74b6c3545063956a527acea7476",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinyvoice-module-0.4.0-fedora-44-arm64.tar.gz",
            sha256: "8697a58237c102f48ac68a8ea8a5ea67675670b1fb495d6ecb00cca9051a882e",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinyvoice-module-0.4.0-fedora-44-x86_64.tar.gz",
            sha256: "2cc8feb4ee0662bf0dfc558640da63d45095efba119f11f9049b9e840e411bda",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinyvoice-module-0.4.0-macos-15-arm64.tar.gz",
            sha256: "de6907175ccde6cc2c25140d75220537dd44d986e5e3b5c51586bf8deead3d88",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinyvoice-module-0.4.0-macos-15-x86_64.tar.gz",
            sha256: "5405bc4d4a08c62b33c6a1ae671a512bd5379304b5c969cf34a7138bc887dd08",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinyvoice-module-0.4.0-macos-26-arm64.tar.gz",
            sha256: "b08ffb8ab2883a7c35b245f48a38eb226b48271e7d862b95855a55cb82501685",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinyvoice-module-0.4.0-macos-26-x86_64.tar.gz",
            sha256: "c0bd82d8164d603af8137c578c93e3b27f6018af8ddbffd93d103cf8878f3261",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinyvoice-module-0.4.0-ubuntu-22.04-arm64.tar.gz",
            sha256: "42e2e1a10e195f062b47e593119171ebc8229c740fe3d2bb882d94aa944e6344",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinyvoice-module-0.4.0-ubuntu-22.04-x86_64.tar.gz",
            sha256: "0030adce6ffb5d2a196ab35bcb3c4edcc0ae205810c729bc2a04af58aa7a7c78",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinyvoice-module-0.4.0-ubuntu-24.04-arm64.tar.gz",
            sha256: "a3ade2060ab22e16115a926e1c3fa32536777ec122565dfdc1f01a364c15979e",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinyvoice-module-0.4.0-ubuntu-24.04-x86_64.tar.gz",
            sha256: "4ddc8eebce5d644eff123488cd4422b9e227915f3d8793e0a838f141ca7a04e9",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinyvoice-module-0.4.0-windows-11-arm64.zip",
            sha256: "e44c9457dfde9aacbda869954201237f1e4ddba97ddcf6c79c3de92e04b7b171",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinyvoice-module-0.4.0-windows-2022-x86_64.zip",
            sha256: "82a8f02df915a532cf4115f8ed2ffa290ec0690e350d003bf30f98b8c07af02a",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinyvoice-module-0.4.0-windows-2025-x86_64.zip",
            sha256: "c220042612de4f8c728907d5f5ff44264a97bf15943ac91758434a2c0132b8d0",
        },
    ],
    load: LoadPolicy::Lazy,
    source: SourceSpec {
        submodule: "tinyvoice",
        manifest: "crates/tinyvoice-module/Cargo.toml",
        package: "tinyvoice-module",
        lib_name: "tinyvoice_module",
        features: &[],
    },
};
