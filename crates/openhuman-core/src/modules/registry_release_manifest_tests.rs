use super::*;

#[test]
fn tinychannels_registry_matches_scoped_runtime_release() {
    let record = find("tinychannels").expect("compiled TinyChannels module");
    assert_eq!(record.version, "0.1.41");
    assert_eq!(tinychannels_bus::CONTRACT_VERSION, 22);
    assert_eq!(
        tinychannels_bus::module_relay::MODULE_OPERATION_VERSION,
        (1, 23)
    );
    let expected = [
        (
            "archlinux-arm64",
            "tinychannels-module-0.1.41-archlinux-arm64.tar.gz",
            "2003a4a9b7efc9a77aaafacee6318a369208a6879fb6e5b417359b3dd2b74077",
        ),
        (
            "archlinux-x86_64",
            "tinychannels-module-0.1.41-archlinux-x86_64.tar.gz",
            "03ffce6d6a3fc0fbeba60e15acb6858078be8e4eb6d499df685513ab8b06ddfc",
        ),
        (
            "fedora-43-arm64",
            "tinychannels-module-0.1.41-fedora-43-arm64.tar.gz",
            "41f6c6b93b34a6198e9df36a7a3e891a375c02c481c8908d3fbbd8799b205766",
        ),
        (
            "fedora-43-x86_64",
            "tinychannels-module-0.1.41-fedora-43-x86_64.tar.gz",
            "515784a8cff8ae0ad288c8c83019e9b58946c24b98f1266ca459b1c34938898d",
        ),
        (
            "fedora-44-arm64",
            "tinychannels-module-0.1.41-fedora-44-arm64.tar.gz",
            "b8d97fdeca3d69d30c3b2ac22b8bc5b91e0609915c1d5dc7333bf83cede4bb51",
        ),
        (
            "fedora-44-x86_64",
            "tinychannels-module-0.1.41-fedora-44-x86_64.tar.gz",
            "6a54828d32169aaed412ffada03e872e438d36070f7dcd02c2cfd7362c1a6466",
        ),
        (
            "macos-15-arm64",
            "tinychannels-module-0.1.41-macos-15-arm64.tar.gz",
            "a379fd8544840fc9febb60a231049925d54d97f788733f76d6828cd2eb62d6c5",
        ),
        (
            "macos-15-x86_64",
            "tinychannels-module-0.1.41-macos-15-x86_64.tar.gz",
            "3c29a22fa45b28992531f313bdbc2fb60b181e1ca857d1f3ee4f38ab0e0a8009",
        ),
        (
            "macos-26-arm64",
            "tinychannels-module-0.1.41-macos-26-arm64.tar.gz",
            "eac384aa33ae329de68574854a8826176f609b44c1da7f29e30651637c2dbb9e",
        ),
        (
            "macos-26-x86_64",
            "tinychannels-module-0.1.41-macos-26-x86_64.tar.gz",
            "63ba314520f55352d1c14f00a3d0b876c8635e3e5312c157fd3b4c8471af317d",
        ),
        (
            "ubuntu-22.04-arm64",
            "tinychannels-module-0.1.41-ubuntu-22.04-arm64.tar.gz",
            "1ed57f902982d1fb8bbee81fc3c1082b4c8697220678113cc9995cdafa7c78a6",
        ),
        (
            "ubuntu-22.04-x86_64",
            "tinychannels-module-0.1.41-ubuntu-22.04-x86_64.tar.gz",
            "e103f0578fa790ceffdbd615613dacfb2f8107dc15b67bedc858886032b0d0c9",
        ),
        (
            "ubuntu-24.04-arm64",
            "tinychannels-module-0.1.41-ubuntu-24.04-arm64.tar.gz",
            "c7146b118cad444d413fb50f7cafad45ffeaaaac0a56babe5e7cc09cd81d4e0a",
        ),
        (
            "ubuntu-24.04-x86_64",
            "tinychannels-module-0.1.41-ubuntu-24.04-x86_64.tar.gz",
            "9558fdd434c48503a32f3c0c2592273d529f2b87577d5e7bdb17ada9ccd1c6cc",
        ),
        (
            "windows-11-arm64",
            "tinychannels-module-0.1.41-windows-11-arm64.zip",
            "8abb0618631afaa396b87430a4c3131117b925f967272ec4eeb812711bdc8084",
        ),
        (
            "windows-2022-x86_64",
            "tinychannels-module-0.1.41-windows-2022-x86_64.zip",
            "0a8e76fe2350e15f5d9e8bba004c7b7d5506fad36c3b9e922d09698ab92dead5",
        ),
        (
            "windows-2025-x86_64",
            "tinychannels-module-0.1.41-windows-2025-x86_64.zip",
            "57760db1782c85faa375215c477cb509f50a10a398d32cddc90394e738970f09",
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
    assert_eq!(record.version, "0.4.0");
    assert_eq!(tinyvoice_bus::CONTRACT_VERSION, (1, 12));
    let expected = [
        (
            "archlinux-arm64",
            "tinyvoice-module-0.4.0-archlinux-arm64.tar.gz",
            "216a99dc13cb7216fd8682bb88c87e381edcc4432ac4eb865db2b473cf18656a",
        ),
        (
            "archlinux-x86_64",
            "tinyvoice-module-0.4.0-archlinux-x86_64.tar.gz",
            "1de1939573e21248ede86a568f511e8d1f4a493e6cd38f5fdcd942d5000734cc",
        ),
        (
            "fedora-43-arm64",
            "tinyvoice-module-0.4.0-fedora-43-arm64.tar.gz",
            "97164ab70529a8f38b7650c3bcbb92798c55140275f0302a811148f9a37a39e2",
        ),
        (
            "fedora-43-x86_64",
            "tinyvoice-module-0.4.0-fedora-43-x86_64.tar.gz",
            "031cad7e8a4e47a2641d7a56ff8c6ca83a05b74b6c3545063956a527acea7476",
        ),
        (
            "fedora-44-arm64",
            "tinyvoice-module-0.4.0-fedora-44-arm64.tar.gz",
            "8697a58237c102f48ac68a8ea8a5ea67675670b1fb495d6ecb00cca9051a882e",
        ),
        (
            "fedora-44-x86_64",
            "tinyvoice-module-0.4.0-fedora-44-x86_64.tar.gz",
            "2cc8feb4ee0662bf0dfc558640da63d45095efba119f11f9049b9e840e411bda",
        ),
        (
            "macos-15-arm64",
            "tinyvoice-module-0.4.0-macos-15-arm64.tar.gz",
            "de6907175ccde6cc2c25140d75220537dd44d986e5e3b5c51586bf8deead3d88",
        ),
        (
            "macos-15-x86_64",
            "tinyvoice-module-0.4.0-macos-15-x86_64.tar.gz",
            "5405bc4d4a08c62b33c6a1ae671a512bd5379304b5c969cf34a7138bc887dd08",
        ),
        (
            "macos-26-arm64",
            "tinyvoice-module-0.4.0-macos-26-arm64.tar.gz",
            "b08ffb8ab2883a7c35b245f48a38eb226b48271e7d862b95855a55cb82501685",
        ),
        (
            "macos-26-x86_64",
            "tinyvoice-module-0.4.0-macos-26-x86_64.tar.gz",
            "c0bd82d8164d603af8137c578c93e3b27f6018af8ddbffd93d103cf8878f3261",
        ),
        (
            "ubuntu-22.04-arm64",
            "tinyvoice-module-0.4.0-ubuntu-22.04-arm64.tar.gz",
            "42e2e1a10e195f062b47e593119171ebc8229c740fe3d2bb882d94aa944e6344",
        ),
        (
            "ubuntu-22.04-x86_64",
            "tinyvoice-module-0.4.0-ubuntu-22.04-x86_64.tar.gz",
            "0030adce6ffb5d2a196ab35bcb3c4edcc0ae205810c729bc2a04af58aa7a7c78",
        ),
        (
            "ubuntu-24.04-arm64",
            "tinyvoice-module-0.4.0-ubuntu-24.04-arm64.tar.gz",
            "a3ade2060ab22e16115a926e1c3fa32536777ec122565dfdc1f01a364c15979e",
        ),
        (
            "ubuntu-24.04-x86_64",
            "tinyvoice-module-0.4.0-ubuntu-24.04-x86_64.tar.gz",
            "4ddc8eebce5d644eff123488cd4422b9e227915f3d8793e0a838f141ca7a04e9",
        ),
        (
            "windows-11-arm64",
            "tinyvoice-module-0.4.0-windows-11-arm64.zip",
            "e44c9457dfde9aacbda869954201237f1e4ddba97ddcf6c79c3de92e04b7b171",
        ),
        (
            "windows-2022-x86_64",
            "tinyvoice-module-0.4.0-windows-2022-x86_64.zip",
            "82a8f02df915a532cf4115f8ed2ffa290ec0690e350d003bf30f98b8c07af02a",
        ),
        (
            "windows-2025-x86_64",
            "tinyvoice-module-0.4.0-windows-2025-x86_64.zip",
            "c220042612de4f8c728907d5f5ff44264a97bf15943ac91758434a2c0132b8d0",
        ),
    ];
    assert_eq!(record.assets.len(), expected.len());
    for (key, archive, digest) in expected {
        let asset = record.asset_for(key).unwrap();
        assert_eq!(asset.archive, archive);
        assert_eq!(asset.sha256, digest);
    }
}
