//! Digest gate for tests that load the published TinyFlows v0.9.0 module.

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

const V0_9_0_MODULE_HASHES: &[&str] = &[
    "360cdfe76ba9144bacce3fa888429d6c3c9b79a272c85c5833499c25879361bb",
    "ac9f902d9b52d408298086e679851c6b556e8409544c36ff3db28f721c5d9420",
    "015ad10b30edb792012cb8c1c8cb3d6e1fa59c299cb735d028643596eeff1df2",
    "b5f64d24adda4b2bd21d92db2c70b6a24e5da6da9cba268f821f27f7ff8fa42e",
    "a15d343779c2464709e1c3983898b2ff162c438b315112d9c3b490fbaf6b3092",
    "10ef00ee80959e3073240c57490b43c579fe3ca8317e174443d2578f39cce115",
    "7c12488e1fd1d79e2eab507705037af71a299448543e74cf97eaedb4839f1054",
    "faefab6ca305c63904bf0eb895472a64e1432fc3cd7684d41aa7fc12ee3d55ab",
    "dfef3237000936415421ba5dfd9e6ac610bc3742686e21265d699a34d649c4ff",
    "b01ac7c86926e544ca174f0f6c18a0448a9559f217dd294320fb0b5f371a55fb",
    "b0412c8dc68bda3737b98c4a485a1685773f75a502259462a59c09e0ad8cf1ff",
];

/// Finds the staged platform artifact and confirms it matches a published v0.9.0 bundle.
pub fn staged_v0_9_0_module() -> (PathBuf, String) {
    let artifact = openhuman_core::modules::local_artifact("tinyflows")
        .expect("stage the published TinyFlows v0.9.0 module on OPENHUMAN_MODULE_PATH");
    let digest = verify_v0_9_0_module(&artifact);
    (artifact, digest)
}

/// Confirms `artifact` is one of the public platform bundles for v0.9.0.
pub fn verify_v0_9_0_module(artifact: &Path) -> String {
    let digest = hex::encode(Sha256::digest(
        std::fs::read(artifact).expect("read staged TinyFlows native artifact"),
    ));
    assert!(
        V0_9_0_MODULE_HASHES.contains(&digest.as_str()),
        "staged TinyFlows library is not a published v0.9.0 module: {} sha256={digest}",
        artifact.display()
    );
    digest
}
