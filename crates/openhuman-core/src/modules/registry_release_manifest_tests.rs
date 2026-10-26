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
    assert_eq!(record.version, "0.4.1");
    assert_eq!(tinyvoice_bus::CONTRACT_VERSION, (1, 12));
    let expected = [
        (
            "archlinux-arm64",
            "tinyvoice-module-0.4.1-archlinux-arm64.tar.gz",
            "b6bb17e6097f1b089f014c2392e028a481b0c6f8543652d7fc8ae890c86777a7",
        ),
        (
            "archlinux-x86_64",
            "tinyvoice-module-0.4.1-archlinux-x86_64.tar.gz",
            "fde049b4112de018799a91215cbc84572a9b4bd598bb44cfa61d8fcd1b96c9f9",
        ),
        (
            "fedora-43-arm64",
            "tinyvoice-module-0.4.1-fedora-43-arm64.tar.gz",
            "58a9bd07922770610b7f42e6f3708940ca501f430d27cd3ebbb6115402fdb9c8",
        ),
        (
            "fedora-43-x86_64",
            "tinyvoice-module-0.4.1-fedora-43-x86_64.tar.gz",
            "e8c95d1d85f4d89a1b8f8fa1740f8e174c835dab3ad856a99184749ca846bca7",
        ),
        (
            "fedora-44-arm64",
            "tinyvoice-module-0.4.1-fedora-44-arm64.tar.gz",
            "15370fb697a4ffdafe9264728b45efbbe155a91cf00aeba55471b0daa85c4768",
        ),
        (
            "fedora-44-x86_64",
            "tinyvoice-module-0.4.1-fedora-44-x86_64.tar.gz",
            "c7ee0327548bd05079ba30af0108385bea723bb8080f846c4def31ddbab57fcd",
        ),
        (
            "macos-15-arm64",
            "tinyvoice-module-0.4.1-macos-15-arm64.tar.gz",
            "77332a814521ac6ae0a5fd36671088ab6596b981544130b852431fe1d08e9f49",
        ),
        (
            "macos-15-x86_64",
            "tinyvoice-module-0.4.1-macos-15-x86_64.tar.gz",
            "2b4556461e3a30cb06fa5b548cc7e9ee8aee63475de05d16d4809140c44d9f4c",
        ),
        (
            "macos-26-arm64",
            "tinyvoice-module-0.4.1-macos-26-arm64.tar.gz",
            "17c1a6a80c1ae7a46fb19ce129e4b0dffb3d6391747bf9d1a5df0fab6ac75c48",
        ),
        (
            "macos-26-x86_64",
            "tinyvoice-module-0.4.1-macos-26-x86_64.tar.gz",
            "1cf0fbd9f20ffc449a9df938cdc15be4d9c2d4077a839ddeba4423ed8ec2f175",
        ),
        (
            "ubuntu-22.04-arm64",
            "tinyvoice-module-0.4.1-ubuntu-22.04-arm64.tar.gz",
            "001b8b76b920dcab616a1f504efb2893c0ce87a251513dfd8e52c6414b239c7f",
        ),
        (
            "ubuntu-22.04-x86_64",
            "tinyvoice-module-0.4.1-ubuntu-22.04-x86_64.tar.gz",
            "c6f6a764747324a2789d9a6cd3764b6b5a5dc08a4a1ca305e157f65df762ae0d",
        ),
        (
            "ubuntu-24.04-arm64",
            "tinyvoice-module-0.4.1-ubuntu-24.04-arm64.tar.gz",
            "8cb7d71e60213896326b85a551dfdd8af2ff1ee8b1a6d6fab27a53ed0d457f57",
        ),
        (
            "ubuntu-24.04-x86_64",
            "tinyvoice-module-0.4.1-ubuntu-24.04-x86_64.tar.gz",
            "7755ee1652735ac081bedbdc6dd6a80f8171639cd480f3a0d044230f5da2b0f3",
        ),
        (
            "windows-11-arm64",
            "tinyvoice-module-0.4.1-windows-11-arm64.zip",
            "ea575671e6d119886ec09ea43eed5bd35a7c9445aa8d54fa4621e26100856c6c",
        ),
        (
            "windows-2022-x86_64",
            "tinyvoice-module-0.4.1-windows-2022-x86_64.zip",
            "b472b4ef877bca793b45fca87bb5edbc43f119748cff85a3a25ea6d83815e254",
        ),
        (
            "windows-2025-x86_64",
            "tinyvoice-module-0.4.1-windows-2025-x86_64.zip",
            "c8c4cd75139c09a31082747d9ad4c1b656d9f6d4f8572370ee8017779c3fc1c0",
        ),
    ];
    assert_eq!(record.assets.len(), expected.len());
    for (key, archive, digest) in expected {
        let asset = record.asset_for(key).unwrap();
        assert_eq!(asset.archive, archive);
        assert_eq!(asset.sha256, digest);
    }
}
