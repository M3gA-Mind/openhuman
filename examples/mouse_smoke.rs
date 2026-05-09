//! Manual smoke test for humanized MouseTool (#682).
//!
//! Run with: `cargo run --example mouse_smoke --release`
//!
//! Watch your cursor — it should curve, not teleport. Keep your hand
//! off the mouse during the run.

use openhuman_core::openhuman::security::SecurityPolicy;
use openhuman_core::openhuman::tools::{MouseTool, Tool};
use serde_json::json;
use std::sync::Arc;
use std::time::Instant;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "debug".into()),
        )
        .init();

    let policy = Arc::new(SecurityPolicy {
        max_actions_per_hour: 1000,
        ..SecurityPolicy::default()
    });
    let tool = MouseTool::new(policy);

    println!("\n=== smoke 1: humanized move (default) ===");
    let t0 = Instant::now();
    let res = tool
        .execute(json!({ "action": "move", "x": 800, "y": 500 }))
        .await?;
    println!("elapsed = {:?}", t0.elapsed());
    println!("result = {res:?}");
    assert!(!res.is_error, "humanized move should succeed");

    tokio::time::sleep(std::time::Duration::from_millis(800)).await;

    println!("\n=== smoke 2: instant teleport (human_like=false) ===");
    let t0 = Instant::now();
    let res = tool
        .execute(json!({ "action": "move", "x": 200, "y": 200, "human_like": false }))
        .await?;
    println!("elapsed = {:?}", t0.elapsed());
    println!("result = {res:?}");
    assert!(!res.is_error, "teleport move should succeed");

    tokio::time::sleep(std::time::Duration::from_millis(800)).await;

    println!("\n=== smoke 3: humanized move long distance ===");
    let t0 = Instant::now();
    let res = tool
        .execute(json!({ "action": "move", "x": 1400, "y": 800 }))
        .await?;
    println!("elapsed = {:?}", t0.elapsed());
    println!("result = {res:?}");
    assert!(!res.is_error);

    tokio::time::sleep(std::time::Duration::from_millis(800)).await;

    println!("\n=== smoke 4: humanized drag ===");
    let t0 = Instant::now();
    let res = tool
        .execute(json!({
            "action": "drag",
            "start_x": 600, "start_y": 400,
            "x": 1000, "y": 600,
        }))
        .await?;
    println!("elapsed = {:?}", t0.elapsed());
    println!("result = {res:?}");
    assert!(!res.is_error, "drag should succeed");

    tokio::time::sleep(std::time::Duration::from_millis(800)).await;

    println!("\n=== smoke 5: humanized click ===");
    // Click in dead screen area to avoid collateral.
    let t0 = Instant::now();
    let res = tool
        .execute(json!({ "action": "click", "x": 50, "y": 50 }))
        .await?;
    println!("elapsed = {:?}", t0.elapsed());
    println!("result = {res:?}");
    assert!(!res.is_error);

    println!("\n✓ smoke complete — verify visually that motion was curved + paced for human-like runs and instant for the teleport.");
    Ok(())
}
