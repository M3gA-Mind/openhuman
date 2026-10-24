//! Registry records for additional first-party TinyBus modules.

use crate::modules::types::{LoadPolicy, ModuleRecord, PlatformAsset, SourceSpec};

/// The `tinybox` module, loaded on demand.
#[cfg(feature = "modules")]
pub(crate) const TINYBOX: ModuleRecord = ModuleRecord {
    id: "tinybox",
    description: "Sandbox execution and lifecycle operations through TinyBox",
    bus_name: tinybox_bus::INTERFACE,
    object_path: tinybox_bus::OBJECT_PATH,
    version: "0.2.1",
    release_url: "https://github.com/tinyhumansai/tinybox/releases/tag/v0.2.1",
    assets: &[
        PlatformAsset {
            host_key: "archlinux-arm64",
            archive: "tinybox-0.2.1-archlinux-arm64.tar.gz",
            sha256: "ff2404f24840632a00d8107d02ea4ac258cc56b77e650653b6c440e769da282a",
        },
        PlatformAsset {
            host_key: "archlinux-x86_64",
            archive: "tinybox-0.2.1-archlinux-x86_64.tar.gz",
            sha256: "3b8f14a175fb0e73578047192f5aeefe3f76014d5a1baf3fb85d63f300ee48fc",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinybox-0.2.1-fedora-43-arm64.tar.gz",
            sha256: "8b53c7f60cfddfadda3f0269aee15676069d727c1fc8c10f7aa8211baa55292a",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinybox-0.2.1-fedora-43-x86_64.tar.gz",
            sha256: "aaafcd8b33a844e44362f7566d4c384c6a541abfd5888b0e415158c8d5bc0ea4",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinybox-0.2.1-fedora-44-arm64.tar.gz",
            sha256: "5441fda12064b029e745229e0c9fd2fd64dcc8e30f3416f5cdbe877583524295",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinybox-0.2.1-fedora-44-x86_64.tar.gz",
            sha256: "1d48b6b734bba68dc3160b85f0cb9e8f7e96e36acab18afd30f71a20e002df2b",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinybox-0.2.1-macos-15-arm64.tar.gz",
            sha256: "8d59ecb80220913770a25ab32116c6ff8e55cdb6fac1db634eab3b8767e22690",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinybox-0.2.1-macos-15-x86_64.tar.gz",
            sha256: "4c1f22592913ca5c57a840cca65a638493cb76b95cee4d88b7367d762a5afa01",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinybox-0.2.1-macos-26-arm64.tar.gz",
            sha256: "daaec8cbb2a8060783c53621794ae6c03b62c9f5295753c5040fe0bbdf75b98e",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinybox-0.2.1-macos-26-x86_64.tar.gz",
            sha256: "de5c07812e54b5ae421bd28a0071566c36d124c07f87dde2eef6e0df83745350",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinybox-0.2.1-ubuntu-22.04-arm64.tar.gz",
            sha256: "488082b3619698a82dfe14d13547ffed90aabc9278f64384f39e7b2f3b8ec020",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinybox-0.2.1-ubuntu-22.04-x86_64.tar.gz",
            sha256: "d6dc0320b2944ebc3d1972d80d4a47e459c5ebb24d0c41ef0f64e5f442f90685",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinybox-0.2.1-ubuntu-24.04-arm64.tar.gz",
            sha256: "ff1f9e0b9658c463a4b8f1e6aeb36cc7f9bc8b97117e8d17fd8986245a8e5b97",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinybox-0.2.1-ubuntu-24.04-x86_64.tar.gz",
            sha256: "87f9017e0384babe52a43cc9002a23d13558cdbab2d1807caf6d48152684871a",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinybox-0.2.1-windows-11-arm64.zip",
            sha256: "86ab6d28269b9e7eb59b54f6534fdc3e9bff67ebfcf66311a0ce6fdd16eaf1d1",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinybox-0.2.1-windows-2022-x86_64.zip",
            sha256: "68b7089dd4141434d3dfc91b32fae688cae549a297d0a933797a3b2b43e92795",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinybox-0.2.1-windows-2025-x86_64.zip",
            sha256: "059e7fbc993627d866c6b57d59286fe80eefc4ebe6542d4535d0e0f7d08e22a9",
        },
    ],
    load: LoadPolicy::Lazy,
    source: SourceSpec {
        submodule: "tinybox",
        manifest: "crates/tinybox-module/Cargo.toml",
        package: "tinybox-module",
        lib_name: "tinybox",
        features: &[],
    },
};

/// The `tinychannels` module, loaded on demand.
pub(crate) const TINYCHANNELS: ModuleRecord = ModuleRecord {
    id: "tinychannels",
    description: "Channel provider lifecycle and message transport",
    bus_name: tinychannels_bus::names::BUS_NAME,
    object_path: tinychannels_bus::names::OBJECT_PATH,
    version: "0.1.40",
    release_url: "https://github.com/tinyhumansai/tinychannels/releases/tag/v0.1.40",
    assets: &[
        PlatformAsset {
            host_key: "archlinux-arm64",
            archive: "tinychannels-module-0.1.40-archlinux-arm64.tar.gz",
            sha256: "2643b5aefb51a882739640666e456022921304b1ff3cf935005803e1baeae2db",
        },
        PlatformAsset {
            host_key: "archlinux-x86_64",
            archive: "tinychannels-module-0.1.40-archlinux-x86_64.tar.gz",
            sha256: "b65e7e2e99af174166ce18acce7b3e9948b03f93644ac34958a2dd1d45b9783f",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinychannels-module-0.1.40-fedora-43-arm64.tar.gz",
            sha256: "06470acac32353419b9e2c8eed0b13244de9957bc6d7575932b8bf5989579780",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinychannels-module-0.1.40-fedora-43-x86_64.tar.gz",
            sha256: "5956996b8fdc829e530a5cf808a8f9561dba7f91aead25df4f091a6c5ead605f",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinychannels-module-0.1.40-fedora-44-arm64.tar.gz",
            sha256: "e449125869811077cb020b6dbd9f5dffc25fbf1526b8ee2402f0fac0b2f53bbc",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinychannels-module-0.1.40-fedora-44-x86_64.tar.gz",
            sha256: "f5ab13f94fe850ec90a95f1d527ee9c55ab3fdeb427f3c1c2e390c35b300d76e",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinychannels-module-0.1.40-macos-15-arm64.tar.gz",
            sha256: "98d31e633601c365200c679f288b13f9d7aef2023ebb9853bcbad690ddbbe1a0",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinychannels-module-0.1.40-macos-15-x86_64.tar.gz",
            sha256: "4ac740b410102b29ba469f37f450b3e03fac3ce31ccb9d3538c1633025a7bd4d",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinychannels-module-0.1.40-macos-26-arm64.tar.gz",
            sha256: "ed2aab10a198193e5bebb5957095acad664a5a66a02d6142563815df6c353b53",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinychannels-module-0.1.40-macos-26-x86_64.tar.gz",
            sha256: "210a7904dd942629af46d7ad36d0b0312f3a1cf0aa238b667ca6309ebf86c8e2",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinychannels-module-0.1.40-ubuntu-22.04-arm64.tar.gz",
            sha256: "48d360aea7849bb93050dbb56f9c28de6848572632908b635fb77df2a65c3bb1",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinychannels-module-0.1.40-ubuntu-22.04-x86_64.tar.gz",
            sha256: "6026081ac8ceb7b91daf0101364c92f99c10c5072a270b91eaeed6acad5d1f90",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinychannels-module-0.1.40-ubuntu-24.04-arm64.tar.gz",
            sha256: "0b4ef6b3c9c2c81f1efdbc5cbe8713478ff4e2f76682625465ee8c7120dfb798",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinychannels-module-0.1.40-ubuntu-24.04-x86_64.tar.gz",
            sha256: "bec22856f0b5f36f8c48f893c53d83015961a596d4ad56a33714ca378c2b4c98",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinychannels-module-0.1.40-windows-11-arm64.zip",
            sha256: "5f463711be6f8c856e13f77ba5d376b1b4e5b9519551411ae7bfd94a196bd633",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinychannels-module-0.1.40-windows-2022-x86_64.zip",
            sha256: "5df4ce49573c7d1e03a93b50cf092bf62dc6b92f50bcfe1471aff50a844268ce",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinychannels-module-0.1.40-windows-2025-x86_64.zip",
            sha256: "e8dd10cc102a7889d7d9ba8f50416b2ee3cec810f203555bf40cd0f157765552",
        },
    ],
    load: LoadPolicy::Lazy,
    source: SourceSpec {
        submodule: "tinychannels",
        manifest: "crates/tinychannels-module/Cargo.toml",
        package: "tinychannels-module",
        lib_name: "tinychannels_module",
        features: &[],
    },
};

#[cfg(feature = "hosting")]
/// The `tinyhosts` module, loaded on demand.
pub(crate) const TINYHOSTS: ModuleRecord = ModuleRecord {
    id: "tinyhosts",
    description: "Hosting provider operations",
    bus_name: tinyhosts_bus::BUS_NAME,
    object_path: tinyhosts_bus::OBJECT_PATH,
    version: "0.4.3",
    release_url: "https://github.com/tinyhumansai/tinyhosts/releases/tag/v0.4.3",
    assets: &[
        PlatformAsset {
            host_key: "archlinux-arm64",
            archive: "tinyhosts-0.4.3-archlinux-arm64.tar.gz",
            sha256: "97388fd17b72b92f0384397378fece2e7d0806004676edcdfc2b1798669b67a8",
        },
        PlatformAsset {
            host_key: "archlinux-x86_64",
            archive: "tinyhosts-0.4.3-archlinux-x86_64.tar.gz",
            sha256: "93e0500e30b77aca6d05de69636c4afc014a3856809e3f781868635d626ef9c0",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinyhosts-0.4.3-fedora-43-arm64.tar.gz",
            sha256: "fef318d960bbaea005a2f1f8bd1c64ff0bce09a3e4cef4922e7a75019dadd29f",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinyhosts-0.4.3-fedora-43-x86_64.tar.gz",
            sha256: "cef9120fc178bf5b40f06e23727d47901746952aab4129eecbc3babd2f516421",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinyhosts-0.4.3-fedora-44-arm64.tar.gz",
            sha256: "be1a4c1c2da9e8745fd18a3d5c050cc99fc6655f6eba2e914e191086e7b0ec6d",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinyhosts-0.4.3-fedora-44-x86_64.tar.gz",
            sha256: "ed5a96104a1db21f3852e0d3824cb493c5091318d36f44de97122abc8f7969ef",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinyhosts-0.4.3-macos-15-arm64.tar.gz",
            sha256: "79a3fa3c643f50d7e46fec58bfbd3117590853ab76792175b62b4325846377cf",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinyhosts-0.4.3-macos-15-x86_64.tar.gz",
            sha256: "4ac3e43e3d48210f40361b199760df738ed3ab01c248f2d45495a8e4c1d50509",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinyhosts-0.4.3-macos-26-arm64.tar.gz",
            sha256: "efff57af745179807ef8eb41a1c83df6cce1cde23ae664ac8e2e67065dae06b7",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinyhosts-0.4.3-macos-26-x86_64.tar.gz",
            sha256: "6c7f70714c7d2246436487ce986c58e1962a393d1d9f3db6d1634c7ce3f75dad",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinyhosts-0.4.3-ubuntu-22.04-arm64.tar.gz",
            sha256: "d554e1a524040b0ae023ff6aec0c0129d9e01526077ca5d6ba1249ce5cd440f0",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinyhosts-0.4.3-ubuntu-22.04-x86_64.tar.gz",
            sha256: "c6d7cf38c8b3e062e1a8af7785cf8c902c6e7d2d1be51a502407537369210b50",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinyhosts-0.4.3-ubuntu-24.04-arm64.tar.gz",
            sha256: "9eec6afb328b22dad77e96ca802c39a83656f0d57faea9c2d2d2017547e03c0b",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinyhosts-0.4.3-ubuntu-24.04-x86_64.tar.gz",
            sha256: "70da542ec03e90485c84b8ef1f75867b15f09292ac941d3a457be8bec410f6bc",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinyhosts-0.4.3-windows-11-arm64.zip",
            sha256: "28e10831884064238ebf897d49eb7050f98e5550f7552061015d018fe4c778d3",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinyhosts-0.4.3-windows-2022-x86_64.zip",
            sha256: "19e525b143662e3580d90dcba003b661a97b5bcd8ed4e00872081766cdd2b59d",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinyhosts-0.4.3-windows-2025-x86_64.zip",
            sha256: "83483a4e5eaf8fd0f02c7bc6575d506f443bf0938408288e4cc4930f092fac89",
        },
    ],
    load: LoadPolicy::Lazy,
    source: SourceSpec {
        submodule: "tinyhosts",
        manifest: "Cargo.toml",
        package: "tinyhosts",
        lib_name: "tinyhosts",
        features: &[],
    },
};
