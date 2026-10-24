use super::*;

#[test]
fn tinycomputer_registry_matches_bus_contract_and_published_release() {
    let desktop = find("tinycomputer").expect("compiled computer module");
    assert_eq!(desktop.bus_name, tinycomputer_bus::names::INTERFACE);
    assert_eq!(desktop.object_path, tinycomputer_bus::names::OBJECT_PATH);
    assert_eq!(desktop.version, "0.15.1");
    assert_eq!(tinycomputer_bus::CONTRACT_VERSION, (2, 28));

    let mut actual = desktop
        .assets
        .iter()
        .map(|asset| (asset.host_key, asset.archive, asset.sha256))
        .collect::<Vec<_>>();
    let mut published = [
        (
            "archlinux-arm64",
            "tinycomputer-0.15.1-archlinux-arm64.tar.gz",
            "6ad069503afe943824ae566a2057d2c3494d080f3c545ebd9c43b46648f09821",
        ),
        (
            "archlinux-x86_64",
            "tinycomputer-0.15.1-archlinux-x86_64.tar.gz",
            "5056d62922f5ef240b4d98fb1ceaf32cb12872e1505e18b5b716daf71ec7cf07",
        ),
        (
            "fedora-43-arm64",
            "tinycomputer-0.15.1-fedora-43-arm64.tar.gz",
            "aa0f965bc364b746ecc9463ad6d0e9aea6ea3c2d12f3c3b9a751802cc8b76e0e",
        ),
        (
            "fedora-43-x86_64",
            "tinycomputer-0.15.1-fedora-43-x86_64.tar.gz",
            "017ce21785624ad5422883060c854af372f62b625fcdd2a3aaef386b52cb9958",
        ),
        (
            "fedora-44-arm64",
            "tinycomputer-0.15.1-fedora-44-arm64.tar.gz",
            "3d084b6f73010230ec1b3cd94b92118751094713ec1ee9ec9fe9fcf0379dc882",
        ),
        (
            "fedora-44-x86_64",
            "tinycomputer-0.15.1-fedora-44-x86_64.tar.gz",
            "a4b24930c6b1a8144554f8dd37f5746d4b8fca0f47ba1a36b740f523095f260c",
        ),
        (
            "macos-15-arm64",
            "tinycomputer-0.15.1-macos-15-arm64.tar.gz",
            "4f783bdcebc4b16af54aa85b963dda5b0083a2c7c8b20a4b8e9807e1351f9c31",
        ),
        (
            "macos-15-x86_64",
            "tinycomputer-0.15.1-macos-15-x86_64.tar.gz",
            "58725f5838a3788d6a970b86e91dca5796b84589b1e9b1fd357e9ac6465a9252",
        ),
        (
            "macos-26-arm64",
            "tinycomputer-0.15.1-macos-26-arm64.tar.gz",
            "ef10caa01120d71b9045d70cca3b559ac95c9d7e05734b8d2215d8e0eb5d0559",
        ),
        (
            "macos-26-x86_64",
            "tinycomputer-0.15.1-macos-26-x86_64.tar.gz",
            "55f93c8f9893055e092c8a3ef711757f3bd76afd38eca84ad70ca47b83c07ebd",
        ),
        (
            "ubuntu-22.04-arm64",
            "tinycomputer-0.15.1-ubuntu-22.04-arm64.tar.gz",
            "f2e37604107fcbac023abf5ca0e54acf556668deb6225bb9a325e119aac4fc13",
        ),
        (
            "ubuntu-22.04-x86_64",
            "tinycomputer-0.15.1-ubuntu-22.04-x86_64.tar.gz",
            "4e7ee56bda5a6e4755ac0610f4da22cac71c5df2d3a3321dd83dfd0bc19427df",
        ),
        (
            "ubuntu-24.04-arm64",
            "tinycomputer-0.15.1-ubuntu-24.04-arm64.tar.gz",
            "88be9bc3f75fc233b286dd18667cd88e5a1f75a40579950b0f5335b1457f83ff",
        ),
        (
            "ubuntu-24.04-x86_64",
            "tinycomputer-0.15.1-ubuntu-24.04-x86_64.tar.gz",
            "cc839ff33bec552e8e1550a203f914af70772258d2070929e45b57cd8a00e9bb",
        ),
        (
            "windows-11-arm64",
            "tinycomputer-0.15.1-windows-11-arm64.zip",
            "c312551895d5c5e2b2bd9e42d02177e94c637c88ef0f9c1c0e8ae891096d4507",
        ),
        (
            "windows-2022-x86_64",
            "tinycomputer-0.15.1-windows-2022-x86_64.zip",
            "34b11d7bb2e291b853215492f8d8263a353fba42605535e08d2718403129d5f5",
        ),
        (
            "windows-2025-x86_64",
            "tinycomputer-0.15.1-windows-2025-x86_64.zip",
            "016dfb8b620e6f1582230cf20075d5060e96b84b7bd2b19ad171afedae7e0d5c",
        ),
    ];
    assert_eq!(desktop.assets.len(), published.len());
    actual.sort_unstable();
    published.sort_unstable();
    assert_eq!(actual, published);
}
