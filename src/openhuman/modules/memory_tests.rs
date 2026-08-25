//! Tests for the memory module client.
//!
//! Nothing here loads a module. What is testable without one is what decides a
//! caller's next move: that construction is genuinely I/O-free, that the static
//! capability answer is the one the module actually serves, and that a bus error
//! comes back as the right `MemoryError` variant. The round trips are covered
//! where they can be honest — `tinymemory`'s own loader E2E, against a real
//! broker and a real `dlopen`.

use std::sync::Arc;

use tinymemory_api::capabilities::{Capabilities, Capability};
use tinymemory_api::error::MemoryError;
use tinymemory_api::provider::MemoryProvider;

use super::{from_bus, ModuleMemoryProvider, MODULE_ID};
use crate::openhuman::config::Config;
use crate::openhuman::modules::registry;

fn provider() -> ModuleMemoryProvider {
    ModuleMemoryProvider::new(Arc::new(Config::default()))
}

/// A bus failure carrying `name`.
fn failure(name: &str) -> tinybus::Error {
    tinybus::Error::MethodFailed {
        name: name.to_string(),
        message: "something went wrong".to_string(),
    }
}

#[test]
fn construction_touches_no_io_and_needs_no_runtime() {
    // The load-bearing property of this type. `CoreContext::memory_binding` is
    // synchronous and roughly 4000 pre-boot tests call it with no tokio runtime,
    // so a constructor that loaded the module — or merely dialled the bus — would
    // panic across the whole suite rather than in one place.
    //
    // This test runs outside `#[tokio::test]` on purpose: that is what makes it a
    // test of the absence of a runtime requirement.
    let provider = provider();
    assert_eq!(provider.driver_id(), MODULE_ID);
}

#[test]
fn the_advertised_capabilities_match_the_pinned_artifact() {
    // Renamed from `..._cover_the_complete_memory_api`, which asserted
    // `capabilities == Capabilities::all()`. That encoded #5598 as the expected
    // behaviour: the host advertised all eighteen families the contract crate
    // declares while the then-pinned v1.0.1 artifact served thirteen, so the other
    // five answered UnknownMethod instead of reporting themselves absent.
    //
    // The part that was always true is still pinned below: the host assembles
    // the memory RPC surface and its tool families from this set before the
    // async bus starts, so a missing mandatory family is a boot-time defect.
    let capabilities = provider().capabilities();

    for mandatory in Capability::MANDATORY {
        assert!(capabilities.contains(mandatory), "{mandatory:?} is missing");
    }
    assert!(capabilities.contains(Capability::Tree));

    // A strict subset of the contract: the artifact is a released binary and the
    // contract is the crate this host compiles against, so the contract may be
    // ahead but can never be behind.
    assert!(
        Capabilities::all().contains_all(capabilities),
        "the artifact advertises a family the contract does not declare",
    );
    // Stated on the pinned branch, not on `capabilities`, so the documented
    // `OPENHUMAN_MEMORY_MODULE_ASSUME_FULL_CAPABILITIES=1` override cannot turn
    // this red. Every assertion above holds under both configurations; this one
    // is about the pin itself.
    assert_ne!(
        super::capabilities_for(false),
        Capabilities::all(),
        "advertising the whole contract is the #5598 over-claim",
    );
}

#[test]
fn the_full_capability_override_restores_the_whole_contract() {
    // The escape hatch for a locally-built module, which does serve the whole
    // contract. Asserted through `capabilities_for` rather than by setting
    // `OPENHUMAN_MEMORY_MODULE_ASSUME_FULL_CAPABILITIES` — mutating a
    // process-global env var would race every other test in this binary.
    assert_eq!(super::capabilities_for(true), Capabilities::all());
    assert_ne!(
        super::capabilities_for(true),
        super::capabilities_for(false)
    );
}

#[test]
fn the_registry_record_matches_the_interface_the_module_serves() {
    // A record whose bus name or object path disagrees with the module produces a
    // proxy that resolves to nothing, and the failure surfaces as an unhelpful
    // transport error rather than a mismatch.
    let record = registry::find(MODULE_ID).expect("the memory module is registered");
    assert_eq!(record.bus_name, "ai.tinyhumans.tinymemory.Memory");
    assert_eq!(record.object_path, "/ai/tinyhumans/tinymemory/Memory");
}

#[test]
fn the_memory_record_publishes_one_asset_per_supported_host() {
    // The release exists now, so the question this test used to ask ("are the
    // assets deliberately absent?") is settled. What is worth pinning instead is
    // that the set is complete: a record missing a host silently reports
    // `Unsupported` there rather than failing loudly, so a platform can lose the
    // driver without anything saying so.
    //
    // The digests themselves are checked structurally by `registry`'s own tests
    // (lowercase, 64 hex chars) and semantically by tinybus, which refetches the
    // release manifest and refuses on disagreement. Nothing here can verify they
    // came from the release rather than a local build — that is a review rule,
    // and it is written on the record itself.
    let record = registry::find(MODULE_ID).expect("registered");
    assert_eq!(
        record.assets.len(),
        11,
        "expected one asset per released host, got {:?}",
        record.assets.iter().map(|a| a.host_key).collect::<Vec<_>>()
    );
    for asset in record.assets {
        assert!(
            asset.archive.contains(record.version),
            "{} names version-less or mismatched archive {}",
            asset.host_key,
            asset.archive
        );
    }
}

#[test]
fn a_not_found_survives_the_round_trip_as_not_found() {
    // `get`'s contract makes a missing entry `Ok(None)` and an `Invalid` a real
    // failure, so collapsing the two would be observable to a caller.
    let error = from_bus(&failure(tinymemory_api::wire::NOT_FOUND));
    assert!(matches!(error, MemoryError::NotFound(_)), "{error:?}");
}

#[test]
fn an_invalid_input_is_reported_as_something_the_caller_can_fix() {
    let error = from_bus(&failure(tinymemory_api::wire::INVALID));
    assert!(matches!(error, MemoryError::Invalid(_)), "{error:?}");
}

#[test]
fn a_path_escape_does_not_arrive_as_a_caller_mistake() {
    // The mapping's most security-relevant case: a sandbox escape must not be
    // reclassified as a malformed argument.
    let error = from_bus(&failure(tinymemory_api::wire::PATH_ESCAPE));
    assert!(matches!(error, MemoryError::PathEscape(_)), "{error:?}");
}

#[test]
fn an_unsupported_capability_keeps_its_family_name() {
    let error = from_bus(&failure(tinymemory_api::wire::UNSUPPORTED));
    assert!(
        matches!(error, MemoryError::Unsupported { .. }),
        "{error:?}"
    );
}

#[test]
fn an_unrecognised_wire_name_is_a_backend_failure_not_an_input_error() {
    // A module newer than this build may name an error the table lacks. Telling a
    // caller its input was wrong when it was not sends it into a rewrite loop over
    // something already correct.
    let error = from_bus(&failure("ai.tinyhumans.tinymemory.Error.SomethingNewer"));
    assert!(matches!(error, MemoryError::Other(_)), "{error:?}");
}

#[test]
fn a_missing_module_is_a_backend_failure_the_caller_cannot_fix() {
    let error = from_bus(&failure("ai.tinyhumans.tinybus.Error.ModuleUnavailable"));
    assert!(matches!(error, MemoryError::Other(_)), "{error:?}");
}

#[test]
fn the_debug_form_never_renders_the_config() {
    // `Config` carries credentials and `Debug` output reaches logs.
    let rendered = format!("{:?}", provider());
    assert!(rendered.contains("ModuleMemoryProvider"), "{rendered}");
    assert!(!rendered.contains("Config"), "{rendered}");
}

#[tokio::test]
async fn a_disabled_host_reports_down_rather_than_erroring() {
    // `health` is the one method whose job is to answer "is this reachable", so an
    // unreachable module is a `Down` health rather than a failure. Status output
    // depends on that distinction.
    let mut config = Config::default();
    config.modules.enabled = false;

    let provider = ModuleMemoryProvider::new(Arc::new(config));
    let health = provider.health().await;
    assert!(
        matches!(health, tinymemory_api::health::MemoryHealth::Down { .. }),
        "a disabled module host must report Down, got {health:?}"
    );
}

#[tokio::test]
async fn a_call_against_a_disabled_host_fails_instead_of_hanging() {
    let mut config = Config::default();
    config.modules.enabled = false;

    let provider = ModuleMemoryProvider::new(Arc::new(config));
    let outcome =
        tinymemory_api::provider::mandatory::MemoryCore::get(&provider, "ns", "key").await;
    assert!(outcome.is_err(), "expected an error, got {outcome:?}");
}

#[tokio::test]
async fn shutdown_on_an_unused_driver_is_a_no_op() {
    // A shutdown must not be the thing that downloads and loads a module. Nothing
    // has been used here, so there is nothing to release.
    let mut config = Config::default();
    config.modules.enabled = false;

    let provider = ModuleMemoryProvider::new(Arc::new(config));
    assert!(provider.shutdown().await.is_ok());
}

#[test]
fn the_capability_list_matches_the_pinned_release() {
    // ARTIFACT_CAPABILITIES describes what ONE specific release of the module
    // serves. Re-pinning the registry to a newer release without re-reading that
    // list would silently re-introduce #5598 in the other direction — the host
    // would under-claim and hide families the new artifact does have.
    //
    // Tying the two together here means the pin bump is a red test, not a
    // silent drift.
    let record = crate::openhuman::modules::registry::find(super::MODULE_ID)
        .expect("the tinymemory module must be in the registry");
    assert_eq!(
        record.version,
        super::ARTIFACT_CAPABILITIES_PIN,
        "the registry pin moved to {} but ARTIFACT_CAPABILITIES is still the list read from {}. \
         Re-read Capability::ALL at the new tag, update both, and re-run.",
        record.version,
        super::ARTIFACT_CAPABILITIES_PIN,
    );
}

#[test]
fn the_advertised_set_does_not_over_claim_the_artifact() {
    // The regression guard for #5598 proper: the driver must not advertise a
    // family the pinned artifact cannot serve. Capabilities::all() is what the
    // CONTRACT declares; the artifact is older and smaller.
    use tinymemory_api::capabilities::{Capabilities, Capability};

    // `capabilities_for(false)` rather than `artifact_capabilities()`: the
    // invariant is a property of the pinned list, and reading the environment
    // here would make this test fail for anyone running with the documented
    // `OPENHUMAN_MEMORY_MODULE_ASSUME_FULL_CAPABILITIES=1` override.
    let advertised = super::capabilities_for(false);

    // Four of the five families v1.0.1 lacked arrived in the v1.2.0 artifact,
    // so the under-claim they used to represent is over — assert they ARE
    // advertised, or a future re-pin that silently narrows the list goes
    // unnoticed.
    for capability in [
        Capability::People,
        Capability::Chunks,
        Capability::Retrieval,
        Capability::Profile,
    ] {
        assert!(
            advertised.contains(capability),
            "{capability:?} has a bus member in the pinned {} artifact but is not advertised — \
             the host is under-claiming and hiding a family it can reach",
            super::ARTIFACT_CAPABILITIES_PIN,
        );
    }

    // `Episodic` is the one that must still be absent, and for a different
    // reason than before: the artifact DOES serve it, but `ModuleMemoryProvider`
    // has no `as_episodic`, so it inherits the trait default and returns `None`.
    // Advertising a family this host cannot reach is the #5598 over-claim in a
    // different coat. Flip this to the loop above in the same change that
    // implements the accessor.
    assert!(
        !advertised.contains(Capability::Episodic),
        "Episodic is advertised but ModuleMemoryProvider has no `as_episodic`, so the accessor \
         returns None — implement it before widening ARTIFACT_CAPABILITIES",
    );

    assert_ne!(
        advertised,
        Capabilities::all(),
        "advertising the whole contract is the bug this test exists to prevent",
    );
}

/// `recall_namespace_recent` reports itself unsupported instead of claiming an
/// empty namespace.
///
/// The pinned artifact has no `RecallNamespaceRecent` member — the contract
/// gained the method after that release. `Ok(vec![])` would be indistinguishable
/// from a genuinely empty namespace, which is the over-claim shape #5641/#5623
/// were about; a bare forward would come back as an untyped `Other` after a
/// round trip (#5598). Both failure modes are silent, so this pins the typed
/// answer and that the message names the pin a reader has to change.
#[tokio::test]
async fn recall_namespace_recent_reports_unsupported_against_the_pinned_artifact() {
    use crate::openhuman::memory::api::provider::retrieval::MemoryRetrieval;

    let provider = provider();
    let err = provider
        .recall_namespace_recent("ns", 10)
        .await
        .expect_err("the pinned artifact cannot serve this member");

    match &err {
        MemoryError::Unsupported { capability } => {
            assert!(
                capability.contains("retrieval.recall_namespace_recent"),
                "the error must name the method a caller asked for, got {capability:?}"
            );
            assert!(
                capability.contains(super::ARTIFACT_CAPABILITIES_PIN),
                "the error must name the artifact pin to change, got {capability:?}"
            );
        }
        other => panic!("expected a typed Unsupported so callers can branch on it, got {other:?}"),
    }
}
