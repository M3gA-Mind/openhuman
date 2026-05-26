//! Indexing: enumerate a git tree's blobs → for each unseen `(content, model)`
//! extract a structural-aug doc + BM25 tokens, embed it, and cache by blob SHA;
//! then write the `(repo, ref)` manifest. Content-addressed + incremental: a
//! branch switch / new commit / pull only (re)embeds the blobs that changed.
//!
//! The structural extractor here is a dependency-free heuristic (signatures +
//! imports + call identifiers + leading doc/comments) — the same *content* the
//! validated prototype's `ast` pass produced. A tree-sitter upgrade (better
//! extraction + the repo-map call graph) slots in behind [`structural_doc`].
//!
//! The embedder is injected (`&dyn EmbeddingProvider`) so the flow unit-tests
//! with a fake; production passes the configured (cloud-default) provider, and
//! its `signature()` becomes the blob cache's `model` key.

use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

use crate::openhuman::embeddings::EmbeddingProvider;

use super::store::CodegraphStore;

const CODE_EXTS: &[&str] = &[
    "rs", "py", "js", "jsx", "ts", "tsx", "go", "java", "rb", "c", "cc", "cpp", "h", "hpp", "cs",
    "php", "kt", "swift", "scala", "sh",
];
const MAX_FILE_BYTES: u64 = 100_000;
const MAX_CALLS: usize = 200;

/// Per-index outcome. On a branch switch, `computed` is just the changed blobs.
#[derive(Debug, Clone, serde::Serialize)]
pub struct IndexReport {
    pub repo_id: String,
    pub git_ref: String,
    pub files: usize,
    pub computed: usize,
    pub cached: usize,
    pub skipped: usize,
}

fn git(repo_dir: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo_dir)
        .args(args)
        .output()
        .with_context(|| format!("git {args:?}"))?;
    if !out.status.success() {
        anyhow::bail!("git {args:?} failed: {}", String::from_utf8_lossy(&out.stderr));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Branch name if on a branch, else the short commit SHA (detached).
pub fn current_ref(repo_dir: &Path) -> Result<String> {
    if let Ok(s) = git(repo_dir, &["symbolic-ref", "--quiet", "--short", "HEAD"]) {
        let s = s.trim();
        if !s.is_empty() {
            return Ok(s.to_string());
        }
    }
    Ok(git(repo_dir, &["rev-parse", "--short", "HEAD"])?.trim().to_string())
}

/// `(path, blob_sha)` for tracked code files at the current checkout.
fn tree_blobs(repo_dir: &Path) -> Result<Vec<(String, String)>> {
    let mut out = Vec::new();
    for line in git(repo_dir, &["ls-files", "-s"])?.lines() {
        // `<mode> <sha> <stage>\t<path>`
        let (meta, path) = match line.split_once('\t') {
            Some(p) => p,
            None => continue,
        };
        let sha = match meta.split_whitespace().nth(1) {
            Some(s) => s,
            None => continue,
        };
        let ext = Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("");
        if CODE_EXTS.contains(&ext) {
            out.push((path.to_string(), sha.to_string()));
        }
    }
    Ok(out)
}

/// Lexical tokens with identifier splitting (camelCase / snake_case), so the
/// BM25 arm matches `__floordiv__` and `floordiv`/`floor`/`div` alike.
pub fn code_tokens(text: &str) -> Vec<String> {
    let mut toks = Vec::new();
    for raw in text.split(|c: char| !c.is_ascii_alphanumeric()) {
        if raw.is_empty() {
            continue;
        }
        let low = raw.to_ascii_lowercase();
        toks.push(low.clone());
        // split camelCase / snake (already split on non-alnum) into sub-words
        let mut cur = String::new();
        let mut prev_lower = false;
        for ch in raw.chars() {
            if ch.is_ascii_uppercase() && prev_lower && !cur.is_empty() {
                toks.push(cur.to_ascii_lowercase());
                cur.clear();
            }
            cur.push(ch);
            prev_lower = ch.is_ascii_lowercase();
        }
        let sub = cur.to_ascii_lowercase();
        if !sub.is_empty() && sub != low {
            toks.push(sub);
        }
    }
    toks
}

/// Heuristic, content-only "intent" text: definition signatures + imports +
/// called-symbol identifiers + leading doc/comment lines. Path is excluded so
/// the result is purely content-derived (cacheable by blob SHA).
pub fn structural_doc(text: &str) -> String {
    let mut sigs = Vec::new();
    let mut imports = Vec::new();
    let mut docs = Vec::new();
    let mut calls: Vec<String> = Vec::new();
    let mut seen_calls = std::collections::HashSet::new();

    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        let lead = t.split_whitespace().next().unwrap_or("");
        match lead {
            // definition keywords across the supported languages
            "fn" | "def" | "class" | "struct" | "impl" | "trait" | "enum" | "interface"
            | "type" | "func" | "function" | "module" | "public" | "private" | "protected"
            | "pub" | "async" | "export" | "const" => {
                sigs.push(t.trim_end_matches('{').trim().to_string());
            }
            "import" | "use" | "from" | "require" | "#include" | "package" => {
                imports.push(t.to_string());
            }
            _ => {}
        }
        if t.starts_with("//") || t.starts_with("///") || t.starts_with('#') || t.starts_with('*')
            || t.starts_with("\"\"\"")
        {
            docs.push(t.trim_start_matches(['/', '#', '*', ' ', '"']).to_string());
        }
        // naive call extraction: `ident(`
        for (i, _) in line.match_indices('(') {
            let prefix = &line[..i];
            let name: String = prefix
                .chars()
                .rev()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect::<String>()
                .chars()
                .rev()
                .collect();
            if name.len() >= 2 && seen_calls.insert(name.clone()) && calls.len() < MAX_CALLS {
                calls.push(name);
            }
        }
    }

    let mut parts = Vec::new();
    if !sigs.is_empty() {
        parts.push(format!("symbols: {}", sigs.join(" ")));
    }
    if !imports.is_empty() {
        parts.push(format!("imports: {}", imports.join(" ")));
    }
    if !calls.is_empty() {
        parts.push(format!("calls: {}", calls.join(" ")));
    }
    if !docs.is_empty() {
        parts.push(format!("docs: {}", docs.join(" ")));
    }
    parts.join("\n")
}

fn l2_normalize(v: &mut [f32]) {
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in v.iter_mut() {
            *x /= norm;
        }
    }
}

/// (Re)index the checkout at `repo_dir` under `(repo_id, ref)`. Only blobs not
/// already cached for the embedder's signature are read + embedded; the rest
/// are cache hits. Then the ref's manifest is rewritten to the current tree.
pub async fn index_ref(
    store: &mut CodegraphStore,
    repo_id: &str,
    repo_dir: &Path,
    git_ref: Option<&str>,
    embedder: &dyn EmbeddingProvider,
) -> Result<IndexReport> {
    let git_ref = match git_ref {
        Some(r) => r.to_string(),
        None => current_ref(repo_dir)?,
    };
    let model = embedder.signature();
    let blobs = tree_blobs(repo_dir)?;
    let (mut computed, mut cached, mut skipped) = (0usize, 0usize, 0usize);

    for (path, sha) in &blobs {
        if store.has_blob(sha, &model)? {
            cached += 1;
            continue;
        }
        let full = repo_dir.join(path);
        match std::fs::metadata(&full) {
            Ok(m) if m.len() > MAX_FILE_BYTES => {
                skipped += 1;
                continue;
            }
            Err(_) => {
                skipped += 1;
                continue;
            }
            _ => {}
        }
        let text = match std::fs::read_to_string(&full) {
            Ok(t) => t,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };
        let doc = structural_doc(&text);
        let mut emb = embedder
            .embed(&[doc.as_str()])
            .await
            .context("codegraph: embed structural doc")?
            .into_iter()
            .next()
            .unwrap_or_default();
        l2_normalize(&mut emb);
        store.put_blob(sha, &model, &code_tokens(&text), &emb)?;
        computed += 1;
    }

    store.set_manifest(repo_id, &git_ref, &blobs)?;
    Ok(IndexReport {
        repo_id: repo_id.to_string(),
        git_ref,
        files: blobs.len(),
        computed,
        cached,
        skipped,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use tempfile::TempDir;

    #[test]
    fn code_tokens_splits_identifiers() {
        let t = code_tokens("def __floordiv__(self): TimedeltaIndex");
        assert!(t.contains(&"floordiv".to_string()));
        assert!(t.contains(&"timedelta".to_string()) || t.contains(&"timedeltaindex".to_string()));
    }

    #[test]
    fn structural_doc_pulls_signatures_imports_calls() {
        let src = "import os\nfn reconcile(charge):\n    return backoff(charge)\n";
        let d = structural_doc(src);
        assert!(d.contains("imports:") && d.contains("import os"));
        assert!(d.contains("symbols:") && d.contains("reconcile"));
        assert!(d.contains("calls:") && d.contains("backoff"));
    }

    struct FakeEmbedder;
    #[async_trait]
    impl EmbeddingProvider for FakeEmbedder {
        fn name(&self) -> &str { "fake" }
        fn model_id(&self) -> &str { "fake-1" }
        fn dimensions(&self) -> usize { 3 }
        async fn embed(&self, texts: &[&str]) -> anyhow::Result<Vec<Vec<f32>>> {
            // deterministic non-zero vector per input (length-based, just needs to be stable)
            Ok(texts.iter().map(|t| vec![t.len() as f32 + 1.0, 1.0, 0.5]).collect())
        }
    }

    fn git(dir: &std::path::Path, args: &[&str]) {
        let ok = std::process::Command::new("git")
            .arg("-C").arg(dir).args(args)
            .output().unwrap().status.success();
        assert!(ok, "git {args:?}");
    }

    #[tokio::test]
    async fn index_ref_is_content_addressed_and_incremental() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        git(&repo, &["init", "-q"]);
        git(&repo, &["config", "user.email", "t@t"]);
        git(&repo, &["config", "user.name", "t"]);
        std::fs::write(repo.join("a.rs"), "fn reconcile() { backoff(); }\n").unwrap();
        std::fs::write(repo.join("readme.md"), "not code\n").unwrap(); // non-code ext → ignored
        git(&repo, &["add", "-A"]);
        git(&repo, &["commit", "-q", "-m", "init"]);

        let mut store = CodegraphStore::open(&tmp.path().join("cg.db")).unwrap();
        let emb = FakeEmbedder;

        let r1 = index_ref(&mut store, "r", &repo, Some("main"), &emb).await.unwrap();
        assert_eq!(r1.files, 1, "only the .rs file is indexed");
        assert_eq!(r1.computed, 1);
        assert_eq!(r1.cached, 0);

        // Re-index unchanged tree → all cache hits, nothing re-embedded.
        let r2 = index_ref(&mut store, "r", &repo, Some("main"), &emb).await.unwrap();
        assert_eq!(r2.computed, 0);
        assert_eq!(r2.cached, 1);

        // The blob hydrates with tokens + a normalized embedding.
        let hits = store.hydrate("r", "main", &emb.signature()).unwrap();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].tokens.contains(&"reconcile".to_string()));
        let norm: f32 = hits[0].emb.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-3, "embedding is L2-normalized");
    }
}
