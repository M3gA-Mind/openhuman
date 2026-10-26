use super::*;

#[test]
fn tinychannels_registry_matches_scoped_runtime_release() {
    let record = find("tinychannels").expect("compiled TinyChannels module");
    assert_eq!(record.version, "0.1.44");
    assert_eq!(tinychannels_bus::CONTRACT_VERSION, 33);
    assert_eq!(
        tinychannels_bus::module_relay::MODULE_OPERATION_VERSION,
        (1, 25)
    );
    let expected = [
        (
            "archlinux-arm64",
            "tinychannels-module-0.1.44-archlinux-arm64.tar.gz",
            "eed9a315d258374f9fdcea9b57af6077d399d4ee2b540de4d2bf5c3f657a48b3",
        ),
        (
            "archlinux-x86_64",
            "tinychannels-module-0.1.44-archlinux-x86_64.tar.gz",
            "ec605c43423cfc5e4092f214dd62849f2c3f74af88c243536fcc877af2558377",
        ),
        (
            "fedora-43-arm64",
            "tinychannels-module-0.1.44-fedora-43-arm64.tar.gz",
            "8f5796ae29ac17e32bdf4e27e6c41e3e2d091d6bfd3d70de0bdf632101813f99",
        ),
        (
            "fedora-43-x86_64",
            "tinychannels-module-0.1.44-fedora-43-x86_64.tar.gz",
            "0220e40f4837ed51c45e57a23c8a150e91c16edf7e0aeaaacea758ddceb0bae0",
        ),
        (
            "fedora-44-arm64",
            "tinychannels-module-0.1.44-fedora-44-arm64.tar.gz",
            "c2561af18e50c719d77cf3e147ba210dd2f6ba337cac8a82bbd88dd782fe93a9",
        ),
        (
            "fedora-44-x86_64",
            "tinychannels-module-0.1.44-fedora-44-x86_64.tar.gz",
            "23fd6b2e38030ed8b62dea168f694c06953e2649effcb4dc97119f96639d481c",
        ),
        (
            "macos-15-arm64",
            "tinychannels-module-0.1.44-macos-15-arm64.tar.gz",
            "7c5d790fe8ae1b40048082ab25545b6a887c875d8b3f287d916b073d9405f6cb",
        ),
        (
            "macos-15-x86_64",
            "tinychannels-module-0.1.44-macos-15-x86_64.tar.gz",
            "44d7b63ba640ff44c1abf564037d3876ea52e9b279206f90aa20277b6556d5fe",
        ),
        (
            "macos-26-arm64",
            "tinychannels-module-0.1.44-macos-26-arm64.tar.gz",
            "76593e4ffd92c503605d442e18f023944d96b727f571be001874735f8ff8da2e",
        ),
        (
            "macos-26-x86_64",
            "tinychannels-module-0.1.44-macos-26-x86_64.tar.gz",
            "388286e5439ef24ee7287dbbaa7012b227eb80d6daf07efc2b7397b6fd2e993a",
        ),
        (
            "ubuntu-22.04-arm64",
            "tinychannels-module-0.1.44-ubuntu-22.04-arm64.tar.gz",
            "28454c71b16a3ccb2af5b7bfae56a21e11b9775fb4f3cd16143b1412da384a12",
        ),
        (
            "ubuntu-22.04-x86_64",
            "tinychannels-module-0.1.44-ubuntu-22.04-x86_64.tar.gz",
            "78156c9978c51971c77cbc35bc4c7f8be642e94cc18234bccd3d6bbbe68e662e",
        ),
        (
            "ubuntu-24.04-arm64",
            "tinychannels-module-0.1.44-ubuntu-24.04-arm64.tar.gz",
            "efd80ee940ef0e473eb6972c52e34ce51c95062a19a255d4bc07b9241779ecd0",
        ),
        (
            "ubuntu-24.04-x86_64",
            "tinychannels-module-0.1.44-ubuntu-24.04-x86_64.tar.gz",
            "5bb4f2bef5b926e51381036d5a0d8131b8e4d876b14291792aaeb0f18cd52353",
        ),
        (
            "windows-11-arm64",
            "tinychannels-module-0.1.44-windows-11-arm64.zip",
            "4adb184bacba8b8384ed3a111762fdd8fabe9b3f7291b3bb63b41f6c4a9e5011",
        ),
        (
            "windows-2022-x86_64",
            "tinychannels-module-0.1.44-windows-2022-x86_64.zip",
            "2ab0a447bae6870e2732347b28610559c4067286534faff315fab6e5ae5ae28a",
        ),
        (
            "windows-2025-x86_64",
            "tinychannels-module-0.1.44-windows-2025-x86_64.zip",
            "4573911f8d444bde8cb907b01cbd74183fbcaafc36926b62a9dbc4bff33c51f4",
        ),
    ];
    assert_eq!(record.assets.len(), expected.len());
    for (key, archive, sha256) in expected {
        let asset = record.asset_for(key).expect("supported platform asset");
        assert_eq!(asset.archive, archive);
        assert_eq!(asset.sha256, sha256);
    }
}

#[cfg(feature = "voice")]
#[test]
fn voice_capture_release_pins_summary_contract_and_every_platform_digest() {
    let record = find("tinyvoice").unwrap();
    assert_eq!(record.version, "0.4.2");
    assert_eq!(tinyvoice_bus::CONTRACT_VERSION, (1, 12));
    let expected = [
        (
            "archlinux-arm64",
            "tinyvoice-module-0.4.2-archlinux-arm64.tar.gz",
            "0cef54f6248019db21282fa76db4f9663fbe830c3b18c2804ab28833d60329e1",
        ),
        (
            "archlinux-x86_64",
            "tinyvoice-module-0.4.2-archlinux-x86_64.tar.gz",
            "707838c2fb8c3e4ad861d56e37418fb71c1b21bc56826fb159fd46057f1af9f0",
        ),
        (
            "fedora-43-arm64",
            "tinyvoice-module-0.4.2-fedora-43-arm64.tar.gz",
            "51c1d67747872016127c3633fda9ea7111eaa8142782e6b6da4ef15ed9c4a171",
        ),
        (
            "fedora-43-x86_64",
            "tinyvoice-module-0.4.2-fedora-43-x86_64.tar.gz",
            "01aac694078952c8dea1090bc99b584930c4a8a043ff8defb0512110f4aa9433",
        ),
        (
            "fedora-44-arm64",
            "tinyvoice-module-0.4.2-fedora-44-arm64.tar.gz",
            "21bb4a50fe65aaa6a5d09f94d15933c3dd7878b7cf919daa06f2f0939ea421be",
        ),
        (
            "fedora-44-x86_64",
            "tinyvoice-module-0.4.2-fedora-44-x86_64.tar.gz",
            "7113c298e9abbbfacc55a45e237b4962ce9b0256f9ffe12e901ee45cddb226d0",
        ),
        (
            "macos-15-arm64",
            "tinyvoice-module-0.4.2-macos-15-arm64.tar.gz",
            "1c0cf8391e8afb35a13763b6c05fe811c14a640fdf9a79e17bae7f32952009d0",
        ),
        (
            "macos-15-x86_64",
            "tinyvoice-module-0.4.2-macos-15-x86_64.tar.gz",
            "72c6d9fb1aad0b4ce3f7cceaad45111d4450e9a1285140945dcc585a69e16bde",
        ),
        (
            "macos-26-arm64",
            "tinyvoice-module-0.4.2-macos-26-arm64.tar.gz",
            "2b37df831a1196f5d379994a1b85e9f69015d7cde47564470afbb2ff31a85b77",
        ),
        (
            "macos-26-x86_64",
            "tinyvoice-module-0.4.2-macos-26-x86_64.tar.gz",
            "1f9ffd9307505693be31df2f6eaa0e6de32f104bd46640c06dc82bffa3e03fa7",
        ),
        (
            "ubuntu-22.04-arm64",
            "tinyvoice-module-0.4.2-ubuntu-22.04-arm64.tar.gz",
            "ad63d27d2c40f8388d93a9d3556bfc8539c4b5ffe8839efaa0c1b7a1b7a61d43",
        ),
        (
            "ubuntu-22.04-x86_64",
            "tinyvoice-module-0.4.2-ubuntu-22.04-x86_64.tar.gz",
            "c2523bf5066757ce0c81e7f5c97a4b902312007ce079606d005e008ca8be66a5",
        ),
        (
            "ubuntu-24.04-arm64",
            "tinyvoice-module-0.4.2-ubuntu-24.04-arm64.tar.gz",
            "78da28da3a483731bec8d53bc561219cc6dec793d66f27eb908755d88118461e",
        ),
        (
            "ubuntu-24.04-x86_64",
            "tinyvoice-module-0.4.2-ubuntu-24.04-x86_64.tar.gz",
            "1b542b8b39223525f40f02245ef21b7d995a039eef9cb8a2586bb46cce9668f9",
        ),
        (
            "windows-11-arm64",
            "tinyvoice-module-0.4.2-windows-11-arm64.zip",
            "734db41e5c5cd5580c5aa4fd7a9c3e5281aee62ea17c31d8c7037c7356360e82",
        ),
        (
            "windows-2022-x86_64",
            "tinyvoice-module-0.4.2-windows-2022-x86_64.zip",
            "47701888511b5b8ac71a5f92affcb04334b43e479b43e2b5a0937f2134f4cc5e",
        ),
        (
            "windows-2025-x86_64",
            "tinyvoice-module-0.4.2-windows-2025-x86_64.zip",
            "46b29e886d3ec82b996ca7138a7a60577fe6236383df59cb34aedfd3c670e9b0",
        ),
    ];
    assert_eq!(record.assets.len(), expected.len());
    for (key, archive, digest) in expected {
        let asset = record.asset_for(key).unwrap();
        assert_eq!(asset.archive, archive);
        assert_eq!(asset.sha256, digest);
    }
}
