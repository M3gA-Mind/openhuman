//! Registry record for the `tinyeventqueues` module.

use crate::modules::types::{LoadPolicy, ModuleRecord, PlatformAsset, SourceSpec};

/// The `tinyeventqueues` module: durable inbound event queues. It persists,
/// deduplicates, filters and paces inbound events (Composio triggers, agent
/// webhooks) on the host's storage, loaded on first ingest, and calls the
/// host back (`Deliver`, `ClassifySalience`) to run triage.
pub(crate) const TINYEVENTQUEUES: ModuleRecord = ModuleRecord {
    id: "tinyeventqueues",
    description: "Durable inbound event queues: dedupe, filtering, pacing and retry before triage",
    bus_name: tinyeventqueues_bus::INTERFACE,
    object_path: tinyeventqueues_bus::OBJECT_PATH,
    version: "0.2.2",
    release_url: "https://github.com/tinyhumansai/tinyeventqueues/releases/tag/v0.2.2",
    assets: &[
        PlatformAsset {
            host_key: "archlinux-rolling-x86_64",
            archive: "tinyeventqueues-0.2.2-archlinux-rolling-x86_64.tar.gz",
            sha256: "dccab532afeedcd9a80e58d9cca3cb110a7d195f2b9de2e68f763308535c476a",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinyeventqueues-0.2.2-fedora-43-arm64.tar.gz",
            sha256: "8cbe823e16b2a8453263e91f61f4bebef7711cb1ad4a253ce218cc39b25cea52",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinyeventqueues-0.2.2-fedora-43-x86_64.tar.gz",
            sha256: "e8ee389f658fa30cb618a292c1d58c8bf8620a6f3943458acc896fb96570a53e",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinyeventqueues-0.2.2-fedora-44-arm64.tar.gz",
            sha256: "138dcb313e6323cca84899b0515ebf867297f93f90667e602ce4578bca1346da",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinyeventqueues-0.2.2-fedora-44-x86_64.tar.gz",
            sha256: "04abdce4544608102487dfb9b6cedfc87821e8147807c5135c3570f7ab8a1155",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinyeventqueues-0.2.2-macos-15-arm64.tar.gz",
            sha256: "aee037365a59779c9c9e7a2ed5b67b79577e393b048e88d9776c011b49fafdf3",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinyeventqueues-0.2.2-macos-15-x86_64.tar.gz",
            sha256: "dec371f56d9177104d476ace983316151ebb5d828cab87d932583e60e21eb882",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinyeventqueues-0.2.2-macos-26-arm64.tar.gz",
            sha256: "b58ba50823ea0a422595e363ed6740b82888ca4cc129364370a03404c16e4722",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinyeventqueues-0.2.2-macos-26-x86_64.tar.gz",
            sha256: "963b305c7de5101db93cb39247e61d1f6fee908da016b0c5e7ab988043999179",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinyeventqueues-0.2.2-ubuntu-22.04-arm64.tar.gz",
            sha256: "c35d106e9c9e51736dd391b7dd5f68ea0401ea916acb81d7f1560785f2d8e0e1",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinyeventqueues-0.2.2-ubuntu-22.04-x86_64.tar.gz",
            sha256: "5ecc44587e6a149742f334d975ff34aa89ce5bee6f8e3bfc841f8cb973fc4cdf",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinyeventqueues-0.2.2-ubuntu-24.04-arm64.tar.gz",
            sha256: "83b36613dac7dcabcf853a97115051dc154d41ee182068ddc326b4e8beb98f1c",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinyeventqueues-0.2.2-ubuntu-24.04-x86_64.tar.gz",
            sha256: "a28454a658c699ed9d62a8b1ce7796f2a4d4dfdb4fd4343d5a49a53564cfa778",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinyeventqueues-0.2.2-windows-11-arm64.zip",
            sha256: "ff71926e11fb5667e57a1783acb5bf9c548b551de6e655da5f48392b3f590b41",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinyeventqueues-0.2.2-windows-2022-x86_64.zip",
            sha256: "f75f9eb755d973bba10d7f193fedfc483c870945d5332bd77be7bf822c36a2d5",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinyeventqueues-0.2.2-windows-2025-x86_64.zip",
            sha256: "fa63ae8a1fb9411d0c04714a704860f610e4c638a01c388d8faed4398de37998",
        },
    ],
    load: LoadPolicy::Lazy,
    source: SourceSpec {
        submodule: "tinyeventqueues",
        manifest: "crates/tinyeventqueues/Cargo.toml",
        package: "tinyeventqueues",
        lib_name: "tinyeventqueues",
        features: &[],
    },
};
