//! Agent-facing codegraph tools: `codegraph_index` (start/refresh a repo's
//! index) and `codegraph_search` (the fused BM25 ∪ dense seed). Coding
//! subagents call these on a checked-out worktree; the embedder is the
//! configured (cloud-default) provider, and its `signature()` keys the cache.

use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::openhuman::codegraph::{current_ref, index_ref, search_ref, CodegraphStore};
use crate::openhuman::config::Config;
use crate::openhuman::embeddings;
use crate::openhuman::tools::traits::{Tool, ToolResult};

fn codegraph_db(workspace_dir: &Path) -> std::path::PathBuf {
    workspace_dir.join("codegraph").join("index.db")
}

/// Stable per-repo key: the canonical worktree path (manifests are per
/// `(repo_id, ref)`; the blob cache is content-addressed so it's shared anyway).
fn repo_id(repo_dir: &Path) -> String {
    std::fs::canonicalize(repo_dir)
        .unwrap_or_else(|_| repo_dir.to_path_buf())
        .to_string_lossy()
        .into_owned()
}

fn arg_str<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key).and_then(|v| v.as_str())
}

/// `codegraph_index { path, ref? }` — (re)index the worktree at `path` under its
/// current branch (or `ref`). Incremental: only changed blobs are embedded.
pub struct CodegraphIndexTool {
    config: Arc<Config>,
    workspace_dir: std::path::PathBuf,
}

impl CodegraphIndexTool {
    pub fn new(config: Arc<Config>, workspace_dir: std::path::PathBuf) -> Self {
        Self {
            config,
            workspace_dir,
        }
    }
}

#[async_trait]
impl Tool for CodegraphIndexTool {
    fn name(&self) -> &str {
        "codegraph_index"
    }

    fn description(&self) -> &str {
        "Index a checked-out repo for fast retrieval. Args: `path` (repo working dir, required), \
         `ref` (branch/commit; defaults to the current checkout). Incremental and content-addressed \
         — only files whose content changed are (re)embedded. Returns {files, computed, cached, skipped}."
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Repo working directory to index."},
                "ref": {"type": "string", "description": "Branch/commit to index (defaults to current checkout)."}
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        let path = match arg_str(&args, "path") {
            Some(p) => p,
            None => {
                return Ok(ToolResult::error(
                    "codegraph_index: `path` (repo working dir) is required",
                ))
            }
        };
        let repo_dir = Path::new(path);
        let git_ref = match arg_str(&args, "ref") {
            Some(r) => r.to_string(),
            None => current_ref(repo_dir)?,
        };
        let provider = embeddings::provider_from_config(&self.config)?;
        let mut store = CodegraphStore::open(&codegraph_db(&self.workspace_dir))?;
        let report = index_ref(
            &mut store,
            &repo_id(repo_dir),
            repo_dir,
            Some(&git_ref),
            &*provider,
        )
        .await?;
        Ok(ToolResult::success(serde_json::to_string_pretty(&report)?))
    }
}

/// `codegraph_search { query, path, ref?, k? }` — the seed: BM25 ∪ dense,
/// RRF-fused, with a `coverage` flag (`full`/`partial`/`none`). On `none`/`partial`
/// the agent should treat hits as hints and lean on grep.
pub struct CodegraphSearchTool {
    config: Arc<Config>,
    workspace_dir: std::path::PathBuf,
}

impl CodegraphSearchTool {
    pub fn new(config: Arc<Config>, workspace_dir: std::path::PathBuf) -> Self {
        Self {
            config,
            workspace_dir,
        }
    }
}

#[async_trait]
impl Tool for CodegraphSearchTool {
    fn name(&self) -> &str {
        "codegraph_search"
    }

    fn description(&self) -> &str {
        "Find the files most relevant to a query in an indexed repo (lexical + semantic, fused). \
         Args: `query` (required), `path` (repo working dir, required), `ref` (defaults to current), \
         `k` (max hits, default 10). Returns {hits:[paths], coverage:full|partial|none, indexed, total}. \
         If coverage is not `full`, treat hits as hints and also use grep."
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {"type": "string", "description": "What to find (issue text / symbols)."},
                "path": {"type": "string", "description": "Repo working directory."},
                "ref": {"type": "string", "description": "Branch/commit (defaults to current checkout)."},
                "k": {"type": "integer", "description": "Max hits to return (default 10)."}
            },
            "required": ["query", "path"]
        })
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        let query = match arg_str(&args, "query") {
            Some(q) => q,
            None => return Ok(ToolResult::error("codegraph_search: `query` is required")),
        };
        let path = match arg_str(&args, "path") {
            Some(p) => p,
            None => {
                return Ok(ToolResult::error(
                    "codegraph_search: `path` (repo working dir) is required",
                ))
            }
        };
        let repo_dir = Path::new(path);
        let git_ref = match arg_str(&args, "ref") {
            Some(r) => r.to_string(),
            None => current_ref(repo_dir)?,
        };
        let k = args.get("k").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
        let provider = embeddings::provider_from_config(&self.config)?;
        let mut store = CodegraphStore::open(&codegraph_db(&self.workspace_dir))?;
        let outcome = search_ref(
            &mut store,
            &repo_id(repo_dir),
            &git_ref,
            query,
            &*provider,
            k,
        )
        .await?;
        Ok(ToolResult::success(serde_json::to_string_pretty(&outcome)?))
    }
}
