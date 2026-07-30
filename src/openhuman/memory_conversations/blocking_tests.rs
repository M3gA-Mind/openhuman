//! Tests for the blocking-pool conversation-store wrappers (#5156).
//!
//! The property under test is that the wrappers are a faithful, `.await`-able
//! stand-in for the synchronous store: same results, same errors, and safe to
//! drive concurrently from several tasks on a runtime with a single async worker
//! — which is exactly the shape that used to starve (a sync call parks the
//! worker on the store's `parking_lot` mutex, so with every worker parked the
//! runtime stops polling the HTTP task that owes the client its response).

use serde_json::json;
use tempfile::TempDir;

use super::*;

fn message(id: &str, content: &str) -> ConversationMessage {
    ConversationMessage {
        id: id.to_string(),
        content: content.to_string(),
        message_type: "text".to_string(),
        extra_metadata: json!({}),
        sender: "user".to_string(),
        created_at: "2026-07-30T00:00:00Z".to_string(),
    }
}

fn create(id: &str) -> CreateConversationThread {
    CreateConversationThread {
        id: id.to_string(),
        title: format!("Title {id}"),
        created_at: "2026-07-30T00:00:00Z".to_string(),
        parent_thread_id: None,
        labels: None,
        personality_id: None,
    }
}

#[tokio::test]
async fn create_append_read_round_trips_through_the_blocking_pool() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().to_path_buf();

    let thread = ensure_thread(dir.clone(), create("thread-a"))
        .await
        .expect("ensure_thread");
    assert_eq!(thread.id, "thread-a");

    append_message(dir.clone(), "thread-a".to_string(), message("m1", "hello"))
        .await
        .expect("append_message");

    let messages = get_messages(dir.clone(), "thread-a".to_string())
        .await
        .expect("get_messages");
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].content, "hello");

    let threads = list_threads(dir.clone()).await.expect("list_threads");
    assert_eq!(threads.len(), 1);
    assert_eq!(threads[0].message_count, 1);
}

#[tokio::test]
async fn store_errors_surface_unchanged() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().to_path_buf();

    // Appending to a thread that was never created is the store's own error,
    // not a join failure — the wrapper must pass it through verbatim so the
    // RPC layer's thread-scoped error mapping still recognises it.
    let error = append_message(dir, "missing".to_string(), message("m1", "hello"))
        .await
        .expect_err("append to a missing thread must fail");
    assert!(
        error.contains("not found"),
        "expected the store's own error, got: {error}"
    );
}

/// The starvation shape from #5156: several conversation operations in flight at
/// once on a runtime with a **single** async worker. Each one contends for the
/// store's process-global mutex, so with the calls made inline the single worker
/// is parked in `lock()` and nothing else on the runtime can be polled. Off the
/// blocking pool they all complete, and a plain cooperative task keeps being
/// polled while they do.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn concurrent_operations_complete_without_stalling_the_single_async_worker() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().to_path_buf();

    // A cooperative task that only ever yields: it can make progress solely
    // because no store call is holding the worker hostage.
    let ticker = tokio::spawn(async {
        let mut ticks = 0_u32;
        for _ in 0..64 {
            tokio::task::yield_now().await;
            ticks += 1;
        }
        ticks
    });

    let mut handles = Vec::new();
    for idx in 0..8 {
        let dir = dir.clone();
        handles.push(tokio::spawn(async move {
            let thread = ensure_thread(dir.clone(), create(&format!("thread-{idx}")))
                .await
                .expect("ensure_thread");
            append_message(
                dir,
                thread.id.clone(),
                message(&format!("m{idx}"), "concurrent"),
            )
            .await
            .expect("append_message");
            thread.id
        }));
    }

    let mut ids = Vec::new();
    for handle in handles {
        ids.push(handle.await.expect("join create task"));
    }
    ids.sort();
    ids.dedup();
    assert_eq!(
        ids.len(),
        8,
        "every concurrent create must land its own thread"
    );
    assert_eq!(ticker.await.expect("join ticker"), 64);

    let threads = list_threads(dir).await.expect("list_threads");
    assert_eq!(threads.len(), 8);
    assert!(threads.iter().all(|thread| thread.message_count == 1));
}

#[tokio::test]
async fn title_labels_delete_and_purge_round_trip() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().to_path_buf();
    ensure_thread(dir.clone(), create("thread-a"))
        .await
        .expect("ensure_thread");

    let retitled = update_thread_title(
        dir.clone(),
        "thread-a".to_string(),
        "Renamed".to_string(),
        "2026-07-30T00:01:00Z".to_string(),
    )
    .await
    .expect("update_thread_title");
    assert_eq!(retitled.title, "Renamed");

    let relabelled = update_thread_labels(
        dir.clone(),
        "thread-a".to_string(),
        vec!["general".to_string()],
        "2026-07-30T00:02:00Z".to_string(),
    )
    .await
    .expect("update_thread_labels");
    assert_eq!(relabelled.labels, vec!["general".to_string()]);
    assert_eq!(relabelled.title, "Renamed", "labels update preserves title");

    assert!(
        delete_thread(
            dir.clone(),
            "thread-a".to_string(),
            "2026-07-30T00:03:00Z".to_string()
        )
        .await
        .expect("delete_thread"),
        "deleting a live thread reports true"
    );
    assert!(list_threads(dir.clone()).await.unwrap().is_empty());

    ensure_thread(dir.clone(), create("thread-b"))
        .await
        .expect("ensure_thread");
    purge_threads(dir.clone()).await.expect("purge_threads");
    assert!(list_threads(dir).await.unwrap().is_empty());
}

#[tokio::test]
async fn update_message_patches_metadata_and_search_finds_content() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().to_path_buf();
    ensure_thread(dir.clone(), create("thread-a"))
        .await
        .expect("ensure_thread");
    append_message(
        dir.clone(),
        "thread-a".to_string(),
        message("m1", "quarterly roadmap review"),
    )
    .await
    .expect("append_message");

    let patched = update_message(
        dir.clone(),
        "thread-a".to_string(),
        "m1".to_string(),
        ConversationMessagePatch {
            extra_metadata: Some(json!({"pinned": true})),
        },
    )
    .await
    .expect("update_message");
    assert_eq!(patched.extra_metadata, json!({"pinned": true}));

    let hits = search_cross_thread_messages(dir, "roadmap".to_string(), 10, None)
        .await
        .expect("search_cross_thread_messages");
    assert!(
        hits.iter().any(|hit| hit.thread_id == "thread-a"),
        "cross-thread search must find the appended message, got: {hits:?}"
    );
}
