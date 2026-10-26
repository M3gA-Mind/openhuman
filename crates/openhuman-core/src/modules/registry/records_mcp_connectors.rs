//! Registry records for the `tinymcp` and `tinyconnectors` modules.

use crate::modules::types::{LoadPolicy, ModuleRecord, PlatformAsset, SourceSpec};

/// The `tinymcp` module: the Model Context Protocol client.
///
/// Owns both transports (Streamable HTTP and a subprocess over stdio), the
/// statically declared server set a host puts in its own configuration, the
/// dynamic registry of user-installed servers with its SQLite store, the
/// reconnect supervisor, the browser sign-in flow, and the write-audit log.
///
/// Lazy, because dialing an MCP server is something most sessions never do: a
/// host with no installed servers and no configured ones would otherwise pay a
/// download and a `dlopen` for a capability it never reaches. That differs from
/// the module's own `lazy = false` export hint, which speaks for a host whose
/// servers should be connected the moment it comes up — this host decides when
/// that moment is, and does so on the first ask.
///
/// **What stays out of the module is host policy**, and the split is the same
/// one the contract's own documentation draws: the prompt-injection scan over
/// remote tool definitions, the `mcp_clients` RPC surface, the
/// agent-facing tools, and the proxy *scoping* decision all belong to this
/// application's threat model, not to a protocol client. `tinymcp-bus` carries
/// the vocabulary; this table says which bytes may speak it.
pub(crate) const TINYMCP: ModuleRecord = ModuleRecord {
    id: "tinymcp",
    description: "Model Context Protocol client: transports, registry, and the write-audit log",
    bus_name: tinymcp_bus::names::INTERFACE,
    object_path: tinymcp_bus::names::OBJECT_PATH,
    version: "0.20.4",
    release_url: "https://github.com/tinyhumansai/tinymcp/releases/tag/v0.20.4",
    assets: &[
        PlatformAsset {
            host_key: "archlinux-rolling-arm64",
            archive: "tinymcp-0.20.4-archlinux-arm64.tar.gz",
            sha256: "d3e414f18bfe29346e8d25ac3bdca0c36ed294f19e4c2eb335472ea2824e33e2",
        },
        PlatformAsset {
            host_key: "archlinux-rolling-x86_64",
            archive: "tinymcp-0.20.4-archlinux-x86_64.tar.gz",
            sha256: "1a6d936e07665fdaa702c1b70f31386194e9288a9a91bf0dafa5e86dfa4c0c64",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinymcp-0.20.4-fedora-43-arm64.tar.gz",
            sha256: "e96a0bf7100a23ee86636ecc2e04180b3001810d9102b24a29cb8a45f4fac295",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinymcp-0.20.4-fedora-43-x86_64.tar.gz",
            sha256: "67ba0b8a70b7d124bdb19c39b32fe312cbf26d3d724f4d63f1f32d4d81d1d06c",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinymcp-0.20.4-fedora-44-arm64.tar.gz",
            sha256: "a10a30eac6b9bb0d7b0ab68fe6007e9bc3440572318d02082fc881158c441578",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinymcp-0.20.4-fedora-44-x86_64.tar.gz",
            sha256: "fd567d48f8f55b2953b4961a932da8d0357cc394c8115c24da7a4218f6206091",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinymcp-0.20.4-macos-15-arm64.tar.gz",
            sha256: "8f237c1fa91ecce4475505cbf4677c04059d1c4d16e8a63a6e60944f51afb41e",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinymcp-0.20.4-macos-15-x86_64.tar.gz",
            sha256: "0ff41bc259ed0e95b83926f2917e0d76d3766e6bbd10c723e8bd42bce562e277",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinymcp-0.20.4-macos-26-arm64.tar.gz",
            sha256: "95e1294d191b4f8629b8bf525b83c2bc8ee3c7044dc8f6b77f5a4f44b5327c8c",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinymcp-0.20.4-macos-26-x86_64.tar.gz",
            sha256: "c7d9b7291199dbeec97ed7fd97fbf5f08267040b2bbd27f37a3d03559e7bc1f0",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinymcp-0.20.4-ubuntu-22.04-arm64.tar.gz",
            sha256: "6243a3fe7b6c6a2972874380c057ec612c7477747e1a744cced6a559ed056301",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinymcp-0.20.4-ubuntu-22.04-x86_64.tar.gz",
            sha256: "f116c78d8dacceaae42935b34ff3e5fe5e537ab0c441a7750d15404547500210",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinymcp-0.20.4-ubuntu-24.04-arm64.tar.gz",
            sha256: "0ee8f3f4032a5480883c1c4beae651673858b4329f9014796a2e9f1f71c1a5ad",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinymcp-0.20.4-ubuntu-24.04-x86_64.tar.gz",
            sha256: "83ddf549809be714c82ac93c0ad393dc7c52474941c20fe7ae17f78a24f05822",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinymcp-0.20.4-windows-11-arm64.zip",
            sha256: "8773fafd0fa68360969c0d7cf58920e29f82ac4573c4371a546275fd8ba66a53",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinymcp-0.20.4-windows-2022-x86_64.zip",
            sha256: "7ecfb93c4ebb242726dba6203512745d7eec5c19671ef560e836f316e1e04aea",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinymcp-0.20.4-windows-2025-x86_64.zip",
            sha256: "4562fd55fa3e3126f455391e69b67e71736b5fbb73ac0eecf491da83d35507fc",
        },
    ],
    load: LoadPolicy::Lazy,
    source: SourceSpec {
        submodule: "tinymcp",
        manifest: "crates/tinymcp/Cargo.toml",
        package: "tinymcp",
        lib_name: "tinymcp",
        features: &[],
    },
};

pub(crate) const TINYCONNECTORS: ModuleRecord = ModuleRecord {
    id: "tinyconnectors",
    description: "OAuth connector integrations: accounts, actions, and triggers",
    bus_name: tinyconnectors_bus::names::INTERFACE,
    object_path: tinyconnectors_bus::names::OBJECT_PATH,
    version: "0.22.1",
    release_url: "https://github.com/tinyhumansai/tinyconnectors/releases/tag/v0.22.1",
    assets: &[
        PlatformAsset {
            host_key: "archlinux-arm64",
            archive: "tinyconnectors-0.22.1-archlinux-arm64.tar.gz",
            sha256: "91d5a9c342245f5c8c3bcbe3bffeb1622df0b2838646bb2c43890de35af684a3",
        },
        PlatformAsset {
            host_key: "archlinux-x86_64",
            archive: "tinyconnectors-0.22.1-archlinux-x86_64.tar.gz",
            sha256: "5215fcf05382c9c0e96d0696849a56d81d16644684aa23885a0bb98f3b17626f",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinyconnectors-0.22.1-fedora-43-arm64.tar.gz",
            sha256: "9574021fde72759ddba04adaa07acff4a67130e7e093fb3fc02d899ea3793498",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinyconnectors-0.22.1-fedora-43-x86_64.tar.gz",
            sha256: "39a1ef3420ff2084124659e344eb3f758485c314e6a53ae4d5c9cafdb9d288c8",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinyconnectors-0.22.1-fedora-44-arm64.tar.gz",
            sha256: "bbe6472272efd0bec44976ab4ed7eaed04c4bde931db23d9386f7e85ce66fef8",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinyconnectors-0.22.1-fedora-44-x86_64.tar.gz",
            sha256: "499de6644a213cd17ac73e8e83b7c5ce44e5f39dd3abfed12e5f0a67232bb09d",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinyconnectors-0.22.1-macos-15-arm64.tar.gz",
            sha256: "2c95a98e0fdd45ac31640d939a9c6a7d18e8fdd9d03fe43dcbf17ed20433a00e",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinyconnectors-0.22.1-macos-15-x86_64.tar.gz",
            sha256: "fd1cffc2480468cb3acbeee22d574035c7b671014e306c375d2a830411827d19",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinyconnectors-0.22.1-macos-26-arm64.tar.gz",
            sha256: "c1d22420d9f0986423f24271df7b5ae93221ea322123b041be6dd3cb4081d373",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinyconnectors-0.22.1-macos-26-x86_64.tar.gz",
            sha256: "7a8c5b00b84ba5f73feada42ffd3f68ad117c10fc5324208e5c4bcc705b03532",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinyconnectors-0.22.1-ubuntu-22.04-arm64.tar.gz",
            sha256: "37dad4d685ee8c055f26a93693447137c9343b1c3d3fd00c61db771c60f2097e",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinyconnectors-0.22.1-ubuntu-22.04-x86_64.tar.gz",
            sha256: "9ab94e32d65a0ca411ceb0f5567f895e5255d7c64a2502e7d7788bec0ba59c93",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinyconnectors-0.22.1-ubuntu-24.04-arm64.tar.gz",
            sha256: "c980f3659fa79c1401e1a5ecd95c058e0a11578bea5f8a258d803a5b0ee61b5e",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinyconnectors-0.22.1-ubuntu-24.04-x86_64.tar.gz",
            sha256: "3a34cdadfd783178d8db2ac2af3b89994e4f73087d321fd1c353b763262c7d9a",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinyconnectors-0.22.1-windows-11-arm64.zip",
            sha256: "ced31db483eeabf5f799789ddcb0b073209495620b52959e05952d64b1f06f33",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinyconnectors-0.22.1-windows-2022-x86_64.zip",
            sha256: "3c998167fc1d4356f8977b5e3e8aa2b0e54ff0b1e1092b577de75f591e8d08bd",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinyconnectors-0.22.1-windows-2025-x86_64.zip",
            sha256: "34f14dfe26b18462be18b805a04359b3a7635d65795c1fd0d352a91d80d186df",
        },
    ],
    // Lazy: a user with no connected accounts should not pay to load it, and
    // most sessions never touch a connector. Safe even signed out — the module
    // loads without configuration and still answers the capability members.
    load: LoadPolicy::Lazy,
    source: SourceSpec {
        submodule: "tinyconnectors",
        manifest: "crates/tinyconnectors/Cargo.toml",
        package: "tinyconnectors",
        lib_name: "tinyconnectors",
        features: &[],
    },
};
