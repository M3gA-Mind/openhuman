//! Registry records for additional first-party TinyBus modules.

use crate::modules::types::{LoadPolicy, ModuleRecord, PlatformAsset, SourceSpec};

/// The `tinybox` module, loaded on demand.
#[cfg(feature = "modules")]
pub(crate) const TINYBOX: ModuleRecord = ModuleRecord {
    id: "tinybox",
    description: "Sandbox execution and lifecycle operations through TinyBox",
    bus_name: tinybox_bus::INTERFACE,
    object_path: tinybox_bus::OBJECT_PATH,
    version: "0.2.3",
    release_url: "https://github.com/tinyhumansai/tinybox/releases/tag/v0.2.3",
    assets: &[
        PlatformAsset {
            host_key: "archlinux-arm64",
            archive: "tinybox-0.2.3-archlinux-arm64.tar.gz",
            sha256: "f5793f3163855faeff97f06ae3ee117c8ab3a6a28e166e0a4c10d663c2c98c59",
        },
        PlatformAsset {
            host_key: "archlinux-x86_64",
            archive: "tinybox-0.2.3-archlinux-x86_64.tar.gz",
            sha256: "0c275b1eb643f01781eb03bac9efcfa9516c14e387aea23f4d740ef2c4a9f3fb",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinybox-0.2.3-fedora-43-arm64.tar.gz",
            sha256: "a78cb0729a3f513fa32ecd3690781ffe76f256f9ab31c3008d72cc28fbab745a",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinybox-0.2.3-fedora-43-x86_64.tar.gz",
            sha256: "1df8a55ed71b2e39e7b6767df3f9d4c8bd6d096d97fad81aa4d24c2f4e200459",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinybox-0.2.3-fedora-44-arm64.tar.gz",
            sha256: "5f5df28fda5f94d594f8f2be60d4a0b1734d525f0db2ffe7d725e20eeb26fdd8",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinybox-0.2.3-fedora-44-x86_64.tar.gz",
            sha256: "f30ec1e1b25f6e760361f5878b0852029e010710e4dfad50bee13e08ac6bf25b",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinybox-0.2.3-macos-15-arm64.tar.gz",
            sha256: "b23fba12090d384033f604a093213f1dbad6c211719c8ac1330245862010f38e",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinybox-0.2.3-macos-15-x86_64.tar.gz",
            sha256: "e3e9178df6c1dea7c3f08eff9df06dcdf8d1e0f83ba23dfdafb70867d991af6a",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinybox-0.2.3-macos-26-arm64.tar.gz",
            sha256: "7e524cffcd7e2ad1ad21a393ca2b9ccdad2cfb4563b331270b182180d5f1c719",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinybox-0.2.3-macos-26-x86_64.tar.gz",
            sha256: "e8bd5991e261db8f8560f94c119dfa987be690dc19f8b898dd60686d29ab79e4",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinybox-0.2.3-ubuntu-22.04-arm64.tar.gz",
            sha256: "3cd95331c40559de0c36b155ee596bdc99e057a32f0e54a4e5cfc652d6c60ab1",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinybox-0.2.3-ubuntu-22.04-x86_64.tar.gz",
            sha256: "aa03d3e8f88d635ea2ab134fdf27454326e0e43a696cd195ea3e626a1ed7aaf8",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinybox-0.2.3-ubuntu-24.04-arm64.tar.gz",
            sha256: "649309f8ed2b6739ae03dbd6b944bd820788c713f99b521a4ef65c762485a2eb",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinybox-0.2.3-ubuntu-24.04-x86_64.tar.gz",
            sha256: "8c3013d933bd9dc235975db5f5046da3c982511f7425502f4f4b288081b89376",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinybox-0.2.3-windows-11-arm64.zip",
            sha256: "bad2e51d65f6b37d8cb7464e3431caee640d11fae61d2ce786b570e6c31173e1",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinybox-0.2.3-windows-2022-x86_64.zip",
            sha256: "0f495036f98e88e566376b2c2180b1d09d597f433b3f9b34a88786c14414f61b",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinybox-0.2.3-windows-2025-x86_64.zip",
            sha256: "2119e8ba6d735aed78c0afb1d0ccb69416e048abff56ad8e46e4efd742c52ff8",
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
    version: "0.1.45",
    release_url: "https://github.com/tinyhumansai/tinychannels/releases/tag/v0.1.45",
    assets: &[
        PlatformAsset {
            host_key: "archlinux-arm64",
            archive: "tinychannels-module-0.1.45-archlinux-arm64.tar.gz",
            sha256: "2adc56f7ea983f208a46d77d4bfc9bf05b5f414e3be7c1a578c2079c57e2a983",
        },
        PlatformAsset {
            host_key: "archlinux-x86_64",
            archive: "tinychannels-module-0.1.45-archlinux-x86_64.tar.gz",
            sha256: "c264de7479ea0f67f3c8fb3ed51480d0fc91e4c3ac8e96d27a9f2d65fc5c6c7a",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinychannels-module-0.1.45-fedora-43-arm64.tar.gz",
            sha256: "02795d3c0643dec350d961c08c9ba3ed463463c70f9025c0faa71fb9fcfb9c8b",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinychannels-module-0.1.45-fedora-43-x86_64.tar.gz",
            sha256: "e6897ee47572d049cd2d8956f5a9dba226f013af2510c11290b02be1cd3518e4",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinychannels-module-0.1.45-fedora-44-arm64.tar.gz",
            sha256: "cd9d6328464fa0ecb46ad51fe794ca0920101e1ac2fbce4acf19d5bb4bcacf14",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinychannels-module-0.1.45-fedora-44-x86_64.tar.gz",
            sha256: "f845f4b838c7b95ce0f4e7c6d8cd07e920d8e9874e5186cac6a607c52c952638",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinychannels-module-0.1.45-macos-15-arm64.tar.gz",
            sha256: "4b6a4cdf406c5ce95c6c8afe8ec1c3592e5aa3c7a3aee6f18359cd8ea994a55e",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinychannels-module-0.1.45-macos-15-x86_64.tar.gz",
            sha256: "c6692f4517fbc0d0630df24d68507aec61546bb97a1399732fcfba7c66bdd603",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinychannels-module-0.1.45-macos-26-arm64.tar.gz",
            sha256: "07de20fa41ba08e4cd3fd727b9c305abca197bce0594d367de50e6c5021345d9",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinychannels-module-0.1.45-macos-26-x86_64.tar.gz",
            sha256: "4e65f802b3aeb3aadc73af0a696e00de324ec4eef363e07a3d50f0cde2b1a84b",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinychannels-module-0.1.45-ubuntu-22.04-arm64.tar.gz",
            sha256: "d521277a7d963332c61233e6a083d920cabbaf38befb346def1fa864bf74adb4",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinychannels-module-0.1.45-ubuntu-22.04-x86_64.tar.gz",
            sha256: "add0a97cf1ab4b85cc5c651e4d4204ff700595c1b94d6aede4a8e2220fa73b3d",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinychannels-module-0.1.45-ubuntu-24.04-arm64.tar.gz",
            sha256: "a51a42c8ee18b26322ce8981a01f6c83e5a6d6a633c002dd3040fa12cb60e513",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinychannels-module-0.1.45-ubuntu-24.04-x86_64.tar.gz",
            sha256: "9cf0d1be60f55601c263fc29c1c91172db2bfcb30d4ea19a11bcb0e032ed7c19",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinychannels-module-0.1.45-windows-11-arm64.zip",
            sha256: "f5c36f904623a3c6e6b523df62618a6576cc60157f1457de11a7f2b11dea3cfb",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinychannels-module-0.1.45-windows-2022-x86_64.zip",
            sha256: "07545499e434b3e25d58a48f59874c49b7a9a5e609d904ce52011932e7640bc2",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinychannels-module-0.1.45-windows-2025-x86_64.zip",
            sha256: "e732561700bde58c35b896f02c395540a5bb45824e59e3c3fea0231786c217fe",
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
    version: "0.4.5",
    release_url: "https://github.com/tinyhumansai/tinyhosts/releases/tag/v0.4.5",
    assets: &[
        PlatformAsset {
            host_key: "archlinux-arm64",
            archive: "tinyhosts-0.4.5-archlinux-arm64.tar.gz",
            sha256: "3acc6a429997a067a39b91b2c414ccc8f0352245147be127173a2ed6965abd3d",
        },
        PlatformAsset {
            host_key: "archlinux-x86_64",
            archive: "tinyhosts-0.4.5-archlinux-x86_64.tar.gz",
            sha256: "1d4104f2d1da999c05c08a6226a3b324e77fc362cfe2ba7c0b5a81ac17e31c48",
        },
        PlatformAsset {
            host_key: "fedora-43-arm64",
            archive: "tinyhosts-0.4.5-fedora-43-arm64.tar.gz",
            sha256: "6bed53d9a240f28940faf48e3391d9e45192cc1e9b9ba4470343b43f765f24bb",
        },
        PlatformAsset {
            host_key: "fedora-43-x86_64",
            archive: "tinyhosts-0.4.5-fedora-43-x86_64.tar.gz",
            sha256: "d11945bb9f328c656ed0ce706b46ebeff1c7b8547f93e9232748cfeababd8f57",
        },
        PlatformAsset {
            host_key: "fedora-44-arm64",
            archive: "tinyhosts-0.4.5-fedora-44-arm64.tar.gz",
            sha256: "c2ad9b3128c32da6fd3575904aef262e219f949c08745c1d5b9a1d48b0bf095d",
        },
        PlatformAsset {
            host_key: "fedora-44-x86_64",
            archive: "tinyhosts-0.4.5-fedora-44-x86_64.tar.gz",
            sha256: "bb5704decd33d35a6054961ebb7e4ca9a4934bb642e50c1aa5a0a1d6a9b30d1d",
        },
        PlatformAsset {
            host_key: "macos-15-arm64",
            archive: "tinyhosts-0.4.5-macos-15-arm64.tar.gz",
            sha256: "7314f37b28cdb4b891d0441eeefb5fdddf39bb6760ddd1b33b55304c683263f8",
        },
        PlatformAsset {
            host_key: "macos-15-x86_64",
            archive: "tinyhosts-0.4.5-macos-15-x86_64.tar.gz",
            sha256: "f412018f1b74e26e0ab837a59d3f3c1f5c3da316f5858d5a3621e85a9366b9d3",
        },
        PlatformAsset {
            host_key: "macos-26-arm64",
            archive: "tinyhosts-0.4.5-macos-26-arm64.tar.gz",
            sha256: "1bc91723ad21eae0a0fd221af38f7b8349c33314753e842a6547586908d79669",
        },
        PlatformAsset {
            host_key: "macos-26-x86_64",
            archive: "tinyhosts-0.4.5-macos-26-x86_64.tar.gz",
            sha256: "fce287ecacbeb0f383386cf9a71a8efd70f2e70849a00786a69ce5e8aef434d2",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-arm64",
            archive: "tinyhosts-0.4.5-ubuntu-22.04-arm64.tar.gz",
            sha256: "a79976107cfd2d439d6f692d2479ce7a6f336e1550a67c9131d6f6f41ed07ff0",
        },
        PlatformAsset {
            host_key: "ubuntu-22.04-x86_64",
            archive: "tinyhosts-0.4.5-ubuntu-22.04-x86_64.tar.gz",
            sha256: "b6bd602900fd9887fd91a9dec762b94add86b49d19759a1de1af271d52dc7ad7",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-arm64",
            archive: "tinyhosts-0.4.5-ubuntu-24.04-arm64.tar.gz",
            sha256: "32a560d5a53723f0338020b9d889ec01354b463b21fdec21807d576ba5d99240",
        },
        PlatformAsset {
            host_key: "ubuntu-24.04-x86_64",
            archive: "tinyhosts-0.4.5-ubuntu-24.04-x86_64.tar.gz",
            sha256: "62e71cd7c5e0f414d7059f96d4b62bfb952e8aab6c7ae3d716f93da3b7e96791",
        },
        PlatformAsset {
            host_key: "windows-11-arm64",
            archive: "tinyhosts-0.4.5-windows-11-arm64.zip",
            sha256: "e433c2881178727693d90bba7d4ba33ec1439ee28c3ca3d9e7eb1b1c4cabb0dc",
        },
        PlatformAsset {
            host_key: "windows-2022-x86_64",
            archive: "tinyhosts-0.4.5-windows-2022-x86_64.zip",
            sha256: "515105650694e8b290e8aa53039dee760054ce6c791a9901e57b8225139e9274",
        },
        PlatformAsset {
            host_key: "windows-2025-x86_64",
            archive: "tinyhosts-0.4.5-windows-2025-x86_64.zip",
            sha256: "8b9b5ba24fb645545e0e37b433deb244d4f65d26c58a8ff5acc7c05b95d9e5df",
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
