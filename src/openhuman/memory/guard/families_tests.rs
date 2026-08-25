//! The wrapped-accessor property — the reason this milestone exists — plus
//! step 2, which lives on `GuardedTree::query_source`.

use crate::openhuman::memory::api::provider::chunks::ChunkQuery;
use crate::openhuman::memory::api::provider::retrieval::{CoverWindowQuery, FastRetrieveQuery};
use crate::openhuman::memory::api::provider::types::SourceScope;
use crate::openhuman::memory::api::provider::{MemoryProvider, MemoryTree};
use crate::openhuman::memory::api::tree::IngestRequest;
use crate::openhuman::memory::api::types::MemoryTaint;

use crate::openhuman::memory::guard::test_support::{
    document, embedded_policy, external_policy, guarded,
};
use crate::openhuman::memory::source_scope::with_source_scope;
use crate::openhuman::security::live_policy;
use crate::openhuman::security::policy::{AutonomyLevel, SecurityPolicy};

fn ingest_request(content: &str) -> IngestRequest {
    IngestRequest {
        namespace: "ns".into(),
        content: content.into(),
        timestamp: None,
        metadata: None,
    }
}

// ── The wrapped-accessor property ───────────────────────────────────────────

#[tokio::test]
async fn guard_as_tree_is_not_the_raw_driver_handle() {
    let (driver, guard) = guarded(embedded_policy());
    let via_guard = guard.as_tree().expect("tree family") as *const dyn MemoryTree;
    let raw = driver.as_tree().expect("tree family") as *const dyn MemoryTree;
    assert!(
        !std::ptr::eq(via_guard, raw),
        "the accessor handed out the driver's own handle — the guard is bypassable"
    );
}

/// The assertion that actually matters. Pointer inequality only proves *some*
/// wrapper exists; this proves the wrapper still enforces.
#[tokio::test]
async fn guard_as_tree_still_applies_policy_reached_through_the_accessor() {
    let dir = std::env::temp_dir();
    let _tier = live_policy::install_scoped(
        std::sync::Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::ReadOnly,
            ..SecurityPolicy::default()
        }),
        dir.clone(),
        dir,
    );

    let (driver, guard) = guarded(embedded_policy());
    let err = guard
        .as_tree()
        .expect("tree family")
        .append(ingest_request("hello"))
        .await
        .expect_err("a readonly tier must refuse a tree write");
    assert!(err.to_string().contains("memory guard: "), "{err}");
    assert_eq!(
        driver.call_count(),
        0,
        "the driver must not be reached at all"
    );
}

#[tokio::test]
async fn every_optional_family_accessor_enforces_the_tier() {
    let dir = std::env::temp_dir();
    let _tier = live_policy::install_scoped(
        std::sync::Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::ReadOnly,
            ..SecurityPolicy::default()
        }),
        dir.clone(),
        dir,
    );
    let (driver, guard) = guarded(embedded_policy());

    // One representative *write* per optional family. Each must be refused
    // before the driver sees it — a family whose decorator forwarded raw would
    // record a call here.
    guard
        .as_ingest()
        .unwrap()
        .ingest_chat(vec![])
        .await
        .expect_err("ingest");
    guard
        .as_documents()
        .unwrap()
        .put_document(document("x", MemoryTaint::Internal))
        .await
        .expect_err("documents");
    guard.as_tree().unwrap().seal("ns").await.expect_err("tree");
    guard
        .as_entities()
        .unwrap()
        .touch_entities("ns", &[])
        .await
        .expect_err("entities");
    guard
        .as_graph()
        .unwrap()
        .kv_put(None, "k", serde_json::Value::Null)
        .await
        .expect_err("graph");
    guard
        .as_diff()
        .unwrap()
        .capture_snapshot("src")
        .await
        .expect_err("diff");
    guard
        .as_goals()
        .unwrap()
        .set_goals(Default::default())
        .await
        .expect_err("goals");
    guard
        .as_tool_memory()
        .unwrap()
        .delete_tool_rule("t", "r")
        .await
        .expect_err("tool_memory");
    guard
        .as_sources()
        .unwrap()
        .forget_source("src")
        .await
        .expect_err("sources");
    guard
        .as_maintenance()
        .unwrap()
        .compact()
        .await
        .expect_err("maintenance");

    assert_eq!(
        driver.call_count(),
        0,
        "at least one family decorator forwarded an unguarded handle: {:?}",
        driver.calls()
    );
}

// ── Step 2 ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn guard_fills_query_source_scope_from_the_task_local() {
    let (driver, guard) = guarded(embedded_policy());
    with_source_scope(Some(vec!["slack:#eng".into()]), async {
        guard
            .as_tree()
            .unwrap()
            .query_source("ns", "src", 10, None)
            .await
            .expect("query_source");
    })
    .await;
    let call = driver.only_call();
    assert_eq!(call.scoped, Some(true));
    assert_eq!(call.content.as_deref(), Some("slack:#eng"));
}

#[tokio::test]
async fn guard_explicit_scope_is_intersected_with_the_ambient_one() {
    let (driver, guard) = guarded(embedded_policy());
    let explicit = SourceScope::new(["gmail:me"]);
    with_source_scope(Some(vec!["slack:#eng".into()]), async {
        guard
            .as_tree()
            .unwrap()
            .query_source("ns", "src", 10, Some(&explicit))
            .await
            .expect("query_source");
    })
    .await;
    assert_eq!(
        driver.only_call().content.as_deref(),
        Some(""),
        "a request outside the ambient allowlist must fail closed"
    );
}

#[tokio::test]
async fn guard_leaves_query_source_unscoped_outside_a_source_scope() {
    let (driver, guard) = guarded(embedded_policy());
    guard
        .as_tree()
        .unwrap()
        .query_source("ns", "src", 10, None)
        .await
        .expect("query_source");
    assert_eq!(driver.only_call().scoped, Some(false));
}

// ── Steps 3 + 4 through a family accessor ───────────────────────────────────

#[tokio::test]
async fn family_writes_are_taint_stamped_too() {
    let (driver, guard) = guarded(embedded_policy());
    with_source_scope(Some(vec!["slack:#eng".into()]), async {
        guard
            .as_documents()
            .unwrap()
            .put_document(document("body", MemoryTaint::Internal))
            .await
            .expect("put_document");
    })
    .await;
    assert_eq!(driver.only_call().taint, Some(MemoryTaint::ExternalSync));
}

#[tokio::test]
async fn family_writes_are_not_redacted_for_an_embedded_driver() {
    let secrety = "Authorization: Bearer abcdefghijklmnop";
    let (driver, guard) = guarded(embedded_policy());
    guard
        .as_documents()
        .unwrap()
        .put_document(document(secrety, MemoryTaint::Internal))
        .await
        .expect("put_document");
    assert_eq!(driver.only_call().content.as_deref(), Some(secrety));
}

#[tokio::test]
async fn family_calls_are_refused_for_an_untrusted_external_driver() {
    let (driver, guard) = guarded(external_policy("untrusted"));
    guard
        .as_tree()
        .unwrap()
        .query_source("ns", "src", 10, None)
        .await
        .expect_err("fail-closed");
    assert_eq!(driver.call_count(), 0);
}

// ── Scope narrowing on the chunk and retrieval families ─────────────────────
//
// These mirror `guard_explicit_scope_is_intersected_with_the_ambient_one` for
// the two families added by the module port. They exist because the first
// implementation of both forwarded the caller's scope **unchanged**, which is
// the widening leak `GuardPolicy::narrow_scope` was written to close: a
// source-restricted turn could name a collection outside its restriction and
// have that become the sole query predicate.

#[tokio::test]
async fn chunk_listing_intersects_an_explicit_scope_with_the_ambient_one() {
    let (driver, guard) = guarded(embedded_policy());
    let explicit = SourceScope::new(["gmail:me"]);
    with_source_scope(Some(vec!["slack:#eng".into()]), async {
        guard
            .as_chunks()
            .unwrap()
            .list_chunks(&ChunkQuery::default(), Some(&explicit))
            .await
            .expect("list_chunks");
    })
    .await;
    assert_eq!(
        driver.only_call().content.as_deref(),
        Some(""),
        "a chunk query outside the ambient allowlist must fail closed"
    );
}

#[tokio::test]
async fn chunk_listing_inherits_the_ambient_scope_when_none_is_requested() {
    let (driver, guard) = guarded(embedded_policy());
    with_source_scope(Some(vec!["slack:#eng".into()]), async {
        guard
            .as_chunks()
            .unwrap()
            .list_chunks(&ChunkQuery::default(), None)
            .await
            .expect("list_chunks");
    })
    .await;
    assert_eq!(
        driver.only_call().content.as_deref(),
        Some("slack:#eng"),
        "the ambient allowlist must reach the driver as a query predicate"
    );
}

#[tokio::test]
async fn fast_retrieve_intersects_an_explicit_scope_with_the_ambient_one() {
    let (driver, guard) = guarded(embedded_policy());
    let explicit = SourceScope::new(["gmail:me"]);
    with_source_scope(Some(vec!["slack:#eng".into()]), async {
        guard
            .as_retrieval()
            .unwrap()
            .fast_retrieve(
                "q",
                FastRetrieveQuery {
                    limit: 10,
                    max_hops: 2,
                    time_window_days: None,
                },
                Some(&explicit),
            )
            .await
            .expect("fast_retrieve");
    })
    .await;
    assert_eq!(driver.only_call().content.as_deref(), Some(""));
}

#[tokio::test]
async fn cover_window_intersects_an_explicit_scope_with_the_ambient_one() {
    let (driver, guard) = guarded(embedded_policy());
    let explicit = SourceScope::new(["gmail:me"]);
    with_source_scope(Some(vec!["slack:#eng".into()]), async {
        guard
            .as_retrieval()
            .unwrap()
            .cover_window(&CoverWindowQuery::default(), Some(&explicit))
            .await
            .expect("cover_window");
    })
    .await;
    assert_eq!(driver.only_call().content.as_deref(), Some(""));
}

// ── Scope narrowing on the two id-addressed retrieval primitives ────────────
//
// `retrieve_children` and `retrieve_leaves` took no scope argument until the
// review of the module port pointed out what that meant. In-process they were
// still restricted, because the engine reads the ambient task-local — but the
// task-local belongs to the *host's* task and does not cross a bus, so the same
// two methods reached over the module transport were unrestricted. A source
// gate that holds embedded and fails open over a transport is worse than one
// that does neither, because nothing about the call site says which you have.
//
// The scope is an argument now, and these pin that it arrives.

#[tokio::test]
async fn retrieve_children_inherits_the_ambient_scope_when_none_is_requested() {
    let (driver, guard) = guarded(embedded_policy());
    with_source_scope(Some(vec!["slack:#eng".into()]), async {
        guard
            .as_retrieval()
            .unwrap()
            .retrieve_children("node", 2, None, None, None)
            .await
            .expect("retrieve_children");
    })
    .await;
    assert_eq!(
        driver.only_call().content.as_deref(),
        Some("slack:#eng"),
        "the ambient allowlist must reach the driver as an explicit argument"
    );
}

#[tokio::test]
async fn retrieve_children_intersects_an_explicit_scope_with_the_ambient_one() {
    let (driver, guard) = guarded(embedded_policy());
    let explicit = SourceScope::new(["gmail:me"]);
    with_source_scope(Some(vec!["slack:#eng".into()]), async {
        guard
            .as_retrieval()
            .unwrap()
            .retrieve_children("node", 2, None, None, Some(&explicit))
            .await
            .expect("retrieve_children");
    })
    .await;
    assert_eq!(
        driver.only_call().content.as_deref(),
        Some(""),
        "a walk outside the ambient allowlist must fail closed, not widen"
    );
}

#[tokio::test]
async fn retrieve_leaves_inherits_the_ambient_scope_when_none_is_requested() {
    let (driver, guard) = guarded(embedded_policy());
    with_source_scope(Some(vec!["slack:#eng".into()]), async {
        guard
            .as_retrieval()
            .unwrap()
            .retrieve_leaves(&["chunk-1".to_string()], None)
            .await
            .expect("retrieve_leaves");
    })
    .await;
    assert_eq!(
        driver.only_call().content.as_deref(),
        Some("slack:#eng"),
        "naming a chunk id directly must not read around a source restriction"
    );
}

#[tokio::test]
async fn retrieve_leaves_intersects_an_explicit_scope_with_the_ambient_one() {
    let (driver, guard) = guarded(embedded_policy());
    let explicit = SourceScope::new(["gmail:me"]);
    with_source_scope(Some(vec!["slack:#eng".into()]), async {
        guard
            .as_retrieval()
            .unwrap()
            .retrieve_leaves(&["chunk-1".to_string()], Some(&explicit))
            .await
            .expect("retrieve_leaves");
    })
    .await;
    assert_eq!(
        driver.only_call().content.as_deref(),
        Some(""),
        "an explicit scope outside the ambient one must fail closed"
    );
}

// ── The two family members the v1.6.0 contract added ────────────────────────

#[tokio::test]
async fn recall_namespace_recent_is_admitted_as_a_read() {
    let (driver, guard) = guarded(embedded_policy());
    guard
        .as_retrieval()
        .unwrap()
        .recall_namespace_recent("ns", 10)
        .await
        .expect("recall_namespace_recent");
    assert_eq!(
        driver.only_call().method,
        "retrieval.recall_namespace_recent"
    );
}

#[tokio::test]
async fn recall_namespace_recent_is_refused_for_an_untrusted_external_driver() {
    // The guard has to decide before the driver is touched: a refusal that
    // still reached the driver would have already disclosed the namespace.
    let (driver, guard) = guarded(external_policy("untrusted"));
    guard
        .as_retrieval()
        .unwrap()
        .recall_namespace_recent("ns", 10)
        .await
        .expect_err("fail-closed");
    assert_eq!(driver.call_count(), 0);
}

/// `insert_event` redacts, and `insert_turn` beside it does not.
///
/// This is the assertion the §2 design turns on: an `EpisodicEvent` carries
/// extracted prose *and* a namespace, so it follows `tree.append` rather than
/// `insert_turn`. `RecordingProvider` records the event content specifically so
/// a guard that skipped redaction shows up here rather than only in a live
/// store.
#[tokio::test]
async fn insert_event_redacts_its_content_on_an_external_driver() {
    let secrety = "Authorization: Bearer abcdefghijklmnop";
    let (driver, guard) = guarded(external_policy(
        crate::openhuman::memory::guard::policy::TRUSTED,
    ));
    guard
        .as_episodic()
        .unwrap()
        .insert_event(&episodic_event("ns", secrety))
        .await
        .expect("insert_event");
    let call = driver.only_call();
    assert_eq!(call.method, "episodic.insert_event");
    let content = call
        .content
        .as_deref()
        .expect("the event content is recorded");
    assert_ne!(
        content, secrety,
        "the credential reached the driver verbatim — `redact_outbound` was not applied"
    );
}

#[tokio::test]
async fn insert_event_is_refused_for_an_untrusted_external_driver() {
    let (driver, guard) = guarded(external_policy("untrusted"));
    guard
        .as_episodic()
        .unwrap()
        .insert_event(&episodic_event("ns", "anything"))
        .await
        .expect_err("fail-closed");
    assert_eq!(driver.call_count(), 0);
}

fn episodic_event(
    namespace: &str,
    content: &str,
) -> crate::openhuman::memory::api::provider::episodic::EpisodicEvent {
    use crate::openhuman::memory::api::provider::episodic::{EpisodicEvent, EventKind};
    EpisodicEvent {
        event_id: "e1".into(),
        segment_id: "seg-1".into(),
        session_id: "s1".into(),
        namespace: namespace.into(),
        kind: EventKind::Fact,
        content: content.into(),
        subject: Some(content.into()),
        timestamp_ref: None,
        confidence: 1.0,
        embedding: None,
        source_turn_ids: None,
        created_at: 0.0,
    }
}
