//! Resolved model / voice IDs from [`crate::openhuman::config::Config`].
//!
//! Most `effective_*` functions enforce the MVP model allowlist: if a resolved
//! model ID is not in the allowlist the function silently falls back to the
//! default MVP model and logs a warning. `effective_chat_model_id` and
//! `effective_embedding_model_id` intentionally bypass that allowlist for LM
//! Studio so user-managed model IDs (e.g. an LM-Studio-served
//! `text-embedding-bge-m3`) are passed through unchanged; the generic
//! `effective_*` helpers still enforce the MVP tier restriction for
//! OpenHuman-managed Ollama assets.

use crate::openhuman::config::Config;
use crate::openhuman::inference::local::provider::{provider_from_config, LocalAiProvider};
use crate::openhuman::inference::vision_models::{self, VISION_MODEL_SUGGESTIONS};

pub(crate) const DEFAULT_OLLAMA_MODEL: &str = "gemma3:1b-it-qat";

/// Default local vision model. Must name a genuinely vision-capable model:
/// it is the fallback whenever a configured vision id turns out to be
/// chat-only, so a chat model here would defeat the whole guard (#5146).
///
/// Moondream is the smallest vision model that is pullable with no extra
/// setup (~1.7 GB across model + projector layers), which keeps the fallback
/// affordable on the low-RAM tiers where vision is most likely to be enabled
/// on demand.
pub(crate) const DEFAULT_OLLAMA_VISION_MODEL: &str = "moondream:1.8b-v2-q4_K_S";
pub(crate) const DEFAULT_LOW_VISION_MODEL: &str = "moondream:1.8b-v2-q4_K_S";
pub(crate) const DEFAULT_OLLAMA_EMBED_MODEL: &str = "bge-m3";

/// Chat models allowed in the current local Ollama build.
/// Any resolved chat model ID not listed here is redirected to `MVP_DEFAULT_CHAT_MODEL`.
///
/// Every id here must be pullable from the public Ollama library as written —
/// an entry that does not resolve makes the allowlist silently redirect the
/// user back to the default, or leaves them with a model that `ollama pull`
/// cannot fetch (GH #5055).
///
/// This list must also cover every `chat_model_id` in
/// [`crate::openhuman::inference::presets`]: a preset whose model is missing
/// here is silently downgraded to `MVP_DEFAULT_CHAT_MODEL`, so the user picks
/// a tier and quietly gets the 1B model.
/// `preset_chat_models_are_allowlisted_and_resolve_unchanged` pins that
/// invariant.
///
/// Verified against the live registry (#5146 §1.3):
/// `GET https://registry.ollama.ai/v2/library/<name>/manifests/<tag>` returns
/// `200` for all five entries. Note that `gemma4` **does** now exist on the
/// Ollama library (it did not when #5055 removed it) and is multimodal at
/// every size, which is why the 16 GB+ tier can use one model for chat and
/// vision. `gemma3n:e4b-it-q8_0` stays allowlisted for back-compat with users
/// who already pulled it under the previous default.
const MVP_ALLOWED_CHAT_MODELS: &[&str] = &[
    "gemma3:270m-it-qat",
    "gemma3:1b-it-qat",
    "gemma3:4b-it-qat",
    "gemma4:e4b-it-q8_0",
    "gemma3n:e4b-it-q8_0",
];
const MVP_DEFAULT_CHAT_MODEL: &str = "gemma3:1b-it-qat";

/// Embedding models allowed in MVP (2–4 GB tier uses all-minilm).
// bge-m3 (1024-dim, 8192-token context) is the canonical local embedder
// for memory tree's fixed on-disk format. all-minilm (384-dim) is kept
// for back-compat with users who pulled it under an older default, but
// new selections should default to bge-m3.
const MVP_ALLOWED_EMBEDDING_MODELS: &[&str] = &["bge-m3", "all-minilm:latest"];

fn enforce_mvp_chat_allowlist(resolved: &str) -> String {
    let lower = resolved.to_ascii_lowercase();
    for allowed in MVP_ALLOWED_CHAT_MODELS {
        if lower == allowed.to_ascii_lowercase() {
            return resolved.to_string();
        }
    }
    tracing::warn!(
        resolved,
        fallback = MVP_DEFAULT_CHAT_MODEL,
        "[local_ai] chat model not in MVP allowlist, redirecting to default"
    );
    MVP_DEFAULT_CHAT_MODEL.to_string()
}

/// Guarantee a vision request never reaches a chat-only model.
///
/// This replaces the previous `MVP_ALLOWED_VISION_MODELS = &[""]` allowlist,
/// which matched only the empty string and therefore rewrote *every*
/// configured vision model to `""` — including genuinely vision-capable ones.
/// Callers then sent an empty model name to Ollama, which is not a clean
/// failure: `ensure_ollama_model_available` tried to `POST /api/pull` a
/// nameless model and retried three times before surfacing an opaque error
/// (#5146 §Part 1).
///
/// The tier restriction that allowlist was standing in for is enforced
/// upstream by [`crate::openhuman::inference::presets::vision_mode_for_config`],
/// which reports `VisionMode::Disabled` for the tiers that ship no vision
/// model. What is left for this function is the capability question alone.
fn enforce_vision_capability(resolved: &str) -> String {
    if vision_models::is_vision_capable(resolved) {
        return resolved.to_string();
    }
    tracing::warn!(
        resolved,
        fallback = DEFAULT_OLLAMA_VISION_MODEL,
        "[local_ai] configured vision model is chat-only, falling back to a vision-capable default"
    );
    DEFAULT_OLLAMA_VISION_MODEL.to_string()
}

fn enforce_mvp_embedding_allowlist(resolved: &str) -> String {
    let lower = resolved.to_ascii_lowercase();
    for allowed in MVP_ALLOWED_EMBEDDING_MODELS {
        if lower == allowed.to_ascii_lowercase() {
            return resolved.to_string();
        }
    }
    tracing::warn!(
        resolved,
        fallback = MVP_ALLOWED_EMBEDDING_MODELS[0],
        "[local_ai] embedding model not in MVP allowlist, redirecting to default"
    );
    MVP_ALLOWED_EMBEDDING_MODELS[0].to_string()
}

pub(crate) fn effective_chat_model_id(config: &Config) -> String {
    let provider = provider_from_config(config);
    if provider == LocalAiProvider::LmStudio {
        let model_id = raw_chat_model_id(config);
        tracing::debug!(
            provider = provider.as_str(),
            has_model = !model_id.is_empty(),
            "[local_ai] effective_chat_model_id: using provider-managed model id"
        );
        return model_id;
    }

    let raw = if !config.local_ai.chat_model_id.trim().is_empty() {
        config.local_ai.chat_model_id.trim()
    } else {
        config.local_ai.model_id.trim()
    };
    if raw.is_empty() {
        return enforce_mvp_chat_allowlist(DEFAULT_OLLAMA_MODEL);
    }
    let lower = raw.to_ascii_lowercase();
    if lower.ends_with(".gguf")
        || lower.contains("huggingface.co/")
        || lower == "qwen3-1.7b"
        || lower == "qwen2.5-1.5b-instruct"
    {
        return enforce_mvp_chat_allowlist(DEFAULT_OLLAMA_MODEL);
    }
    enforce_mvp_chat_allowlist(raw)
}

fn raw_chat_model_id(config: &Config) -> String {
    // For LM Studio the user must set `local_ai.chat_model_id` explicitly —
    // there is no sensible Ollama-branded default to fall back to. Return an
    // empty string so callers (diagnostics, status) surface the missing-model
    // warning rather than silently requesting "gemma3:1b-it-qat" from LM Studio.
    let raw = if !config.local_ai.chat_model_id.trim().is_empty() {
        config.local_ai.chat_model_id.trim()
    } else {
        config.local_ai.model_id.trim()
    };
    if raw.is_empty() {
        tracing::debug!(
            provider = "lm_studio",
            "[local_ai] raw_chat_model_id: no LM Studio chat model configured"
        );
    }
    raw.to_string()
}

/// Resolve the vision model for status / reporting surfaces.
///
/// An empty return means "vision is not configured" and is a legitimate
/// state (the low tiers ship no vision model). A non-empty return is
/// **always** a vision-capable id. Call [`resolve_vision_model_choice`] instead
/// when about to issue an actual vision request — it turns the
/// not-configured case into an actionable error rather than an empty string
/// that downstream code would send to Ollama verbatim.
pub(crate) fn effective_vision_model_id(config: &Config) -> String {
    let raw = config.local_ai.vision_model_id.trim();
    if raw.is_empty() {
        return String::new();
    }
    let lower = raw.to_ascii_lowercase();
    let resolved = if lower == "moondream:1.8b" || lower == "moondream" {
        DEFAULT_LOW_VISION_MODEL
    } else {
        raw
    };
    enforce_vision_capability(resolved)
}

/// The vision model a request will actually use, plus what it displaced.
pub(crate) struct VisionModelChoice {
    /// The vision-capable model id to send to Ollama.
    pub(crate) model: String,
    /// The configured id that was swapped out because it is chat-only.
    ///
    /// `Some` means the user asked for one model and is getting another, which
    /// every downstream *error* must say out loud: a bare "`moondream:...` is
    /// not available, pull it" is actively misleading when the user configured
    /// `gemma3:1b-it-qat` and never mentioned moondream (greptile, #5253).
    pub(crate) replaced: Option<String>,
}

/// Resolve the vision model for a real vision request, reporting any
/// capability substitution.
///
/// Never returns an empty id: when no vision model is configured the caller
/// gets a message naming what to set and which models to pull, instead of
/// silently shipping an empty model name to Ollama (#5146 §Part 1).
pub(crate) fn resolve_vision_model_choice(config: &Config) -> Result<VisionModelChoice, String> {
    let resolved = effective_vision_model_id(config);
    if resolved.trim().is_empty() {
        let suggestions = VISION_MODEL_SUGGESTIONS.join("`, `");
        tracing::warn!("[local_ai] vision request with no vision model configured");
        return Err(format!(
            "no local vision model is configured. Set `local_ai.vision_model_id` to a \
             vision-capable model (for example `{suggestions}`) and pull it with \
             `ollama pull <model>`, or route the vision workload to a cloud provider \
             with `vision_provider`."
        ));
    }

    // Report only a *capability* substitution. An alias rewrite (`moondream` ->
    // the pinned tag) resolves to a different string but is the same model the
    // user asked for, so it is not something they need to be told about.
    let configured = config.local_ai.vision_model_id.trim();
    let replaced = (!configured.is_empty()
        && !vision_models::is_vision_capable(configured)
        && !resolved.eq_ignore_ascii_case(configured))
    .then(|| configured.to_string());

    Ok(VisionModelChoice {
        model: resolved,
        replaced,
    })
}

pub(crate) fn effective_embedding_model_id(config: &Config) -> String {
    let raw = config.local_ai.embedding_model_id.trim();

    // LM Studio serves embeddings under user-managed names (e.g.
    // `text-embedding-bge-m3`) that are deliberately outside the
    // OpenHuman-managed Ollama MVP allowlist. Mirror `effective_chat_model_id`
    // and pass a configured id through unchanged so the user can target the
    // exact served model instead of having it rewritten back to `bge-m3`
    // (#3920). The allowlist remains in force for the managed Ollama path
    // below, where the ids are OpenHuman-pulled assets.
    if provider_from_config(config) == LocalAiProvider::LmStudio {
        if raw.is_empty() {
            // No configured id — fall back to the canonical default so the
            // memory tree still has an embedder to request, rather than
            // sending an empty model name to the LM Studio server.
            tracing::debug!(
                provider = LocalAiProvider::LmStudio.as_str(),
                "[local_ai] effective_embedding_model_id: no LM Studio embedding model configured, using default"
            );
            return DEFAULT_OLLAMA_EMBED_MODEL.to_string();
        }
        tracing::debug!(
            provider = LocalAiProvider::LmStudio.as_str(),
            "[local_ai] effective_embedding_model_id: using provider-managed embedding id"
        );
        return raw.to_string();
    }

    if raw.is_empty() {
        return enforce_mvp_embedding_allowlist(DEFAULT_OLLAMA_EMBED_MODEL);
    }
    enforce_mvp_embedding_allowlist(raw)
}

pub(crate) fn effective_stt_model_id(config: &Config) -> String {
    let raw = config.local_ai.stt_model_id.trim();
    if raw.is_empty() {
        "ggml-base-q5_1.bin".to_string()
    } else {
        raw.to_string()
    }
}

pub(crate) fn effective_tts_voice_id(config: &Config) -> String {
    let raw = config.local_ai.tts_voice_id.trim();
    if raw.is_empty() {
        "en_US-lessac-medium".to_string()
    } else {
        raw.to_string()
    }
}

pub(crate) fn effective_quantization(config: &Config) -> String {
    let raw = config.local_ai.quantization.trim();
    if raw.is_empty() {
        "q4".to_string()
    } else {
        raw.to_ascii_lowercase()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> Config {
        Config::default()
    }

    #[test]
    fn chat_model_falls_back_for_empty_and_unsupported_ids() {
        let mut config = test_config();

        config.local_ai.chat_model_id = String::new();
        config.local_ai.model_id = String::new();
        assert_eq!(effective_chat_model_id(&config), MVP_DEFAULT_CHAT_MODEL);

        config.local_ai.chat_model_id = "custom.gguf".to_string();
        assert_eq!(effective_chat_model_id(&config), MVP_DEFAULT_CHAT_MODEL);

        config.local_ai.chat_model_id = "qwen3-1.7b".to_string();
        assert_eq!(effective_chat_model_id(&config), MVP_DEFAULT_CHAT_MODEL);
    }

    #[test]
    fn chat_model_allows_mvp_model() {
        let mut config = test_config();
        config.local_ai.chat_model_id = "gemma3:1b-it-qat".to_string();
        assert_eq!(effective_chat_model_id(&config), "gemma3:1b-it-qat");
    }

    #[test]
    fn chat_model_allows_requested_ollama_gemma3n_q8() {
        let mut config = test_config();
        config.local_ai.chat_model_id = "gemma3n:e4b-it-q8_0".to_string();
        assert_eq!(effective_chat_model_id(&config), "gemma3n:e4b-it-q8_0");
    }

    #[test]
    fn chat_model_allows_custom_ids_for_lm_studio() {
        let mut config = test_config();
        config.local_ai.provider = "lm_studio".to_string();
        config.local_ai.chat_model_id = "publisher/custom-model-7b".to_string();
        assert_eq!(
            effective_chat_model_id(&config),
            "publisher/custom-model-7b"
        );
    }

    #[test]
    fn lm_studio_chat_model_returns_empty_when_no_model_configured() {
        // LM Studio has no sensible Ollama-branded default — an empty model ID
        // surfaces the missing-model warning in diagnostics / status rather than
        // silently sending "gemma3:1b-it-qat" to an LM Studio server.
        let mut config = test_config();
        config.local_ai.provider = "lm_studio".to_string();
        config.local_ai.chat_model_id = String::new();
        config.local_ai.model_id = String::new();
        assert_eq!(effective_chat_model_id(&config), "");
    }

    #[test]
    fn chat_model_rejects_non_mvp_models() {
        let mut config = test_config();

        // Bare `gemma3n:e4b` is a real Ollama tag but is NOT the allowlisted
        // quantization, so it still redirects to the default.
        config.local_ai.chat_model_id = "gemma3n:e4b".to_string();
        assert_eq!(effective_chat_model_id(&config), MVP_DEFAULT_CHAT_MODEL);

        // Arbitrary non-preset models stay rejected.
        config.local_ai.chat_model_id = "llama3.1:8b".to_string();
        assert_eq!(effective_chat_model_id(&config), MVP_DEFAULT_CHAT_MODEL);

        config.local_ai.chat_model_id = "totally-made-up-model:v0".to_string();
        assert_eq!(effective_chat_model_id(&config), MVP_DEFAULT_CHAT_MODEL);
    }

    /// #5146 §1.3: the allowlist must cover every preset chat model.
    ///
    /// `gemma3:270m-it-qat` (1 GB tier) and `gemma3:4b-it-qat` (8-16 GB tier)
    /// were previously absent, so applying either preset resolved straight
    /// back to the 1B default — the user picked a tier and silently got a
    /// different model than the one the preset advertised.
    #[test]
    fn preset_chat_models_are_allowlisted_and_resolve_unchanged() {
        let mut config = test_config();
        for preset in crate::openhuman::inference::presets::all_presets() {
            config.local_ai.chat_model_id = preset.chat_model_id.to_string();
            assert_eq!(
                effective_chat_model_id(&config),
                preset.chat_model_id,
                "preset {:?} chat model `{}` is not allowlisted and was redirected",
                preset.tier,
                preset.chat_model_id
            );
        }
    }

    /// GH #5055 / #5146 §1.3: every allowlisted chat model must be a real,
    /// fully-qualified Ollama id.
    ///
    /// The #5055 form of this test asserted "no entry may start with
    /// `gemma4:`", because no `gemma4` namespace existed at the time. Gemma 4
    /// has since been published and `gemma4:e4b-it-q8_0` resolves against
    /// `registry.ollama.ai`, so that assertion was pinning an expired fact.
    /// The durable invariant is the `<model>:<tag>` shape plus the
    /// preset cross-check above.
    #[test]
    fn mvp_chat_allowlist_entries_are_fully_qualified() {
        for model in MVP_ALLOWED_CHAT_MODELS {
            assert!(
                model.contains(':'),
                "`{model}` must be a fully-qualified `<model>:<tag>` id"
            );
        }
    }

    #[test]
    fn vision_model_normalizes_legacy_moondream_values() {
        let mut config = test_config();

        // Empty stays empty: "vision not configured" is a real state.
        config.local_ai.vision_model_id = String::new();
        assert_eq!(effective_vision_model_id(&config), "");

        // Legacy shorthands normalize to the pinned Moondream build. Before
        // #5146 these resolved to "" (vision silently disabled) because the
        // vision allowlist contained only the empty string.
        config.local_ai.vision_model_id = "moondream".to_string();
        assert_eq!(effective_vision_model_id(&config), DEFAULT_LOW_VISION_MODEL);
        config.local_ai.vision_model_id = "moondream:1.8b".to_string();
        assert_eq!(effective_vision_model_id(&config), DEFAULT_LOW_VISION_MODEL);
    }

    /// #5146 §Part 1: a genuinely vision-capable model must survive resolution
    /// unchanged. The previous `MVP_ALLOWED_VISION_MODELS = &[""]` allowlist
    /// rewrote every one of these to `""`.
    #[test]
    fn vision_capable_models_pass_through_unchanged() {
        let mut config = test_config();
        for model in ["llava:7b", "gemma3:4b-it-qat", "gemma4:e4b-it-q8_0"] {
            config.local_ai.vision_model_id = model.to_string();
            assert_eq!(effective_vision_model_id(&config), model);
        }
    }

    /// #5146 §Part 1: a chat-only model must never be returned as the vision
    /// model. Ollama silently drops the `images` array for such a model, so
    /// passing it through would produce a fabricated description instead of an
    /// error.
    #[test]
    fn chat_only_vision_model_falls_back_to_a_vision_capable_default() {
        let mut config = test_config();
        for chat_only in ["gemma3n:e4b-it-q8_0", "gemma3:1b-it-qat", "llama3.1:8b"] {
            config.local_ai.vision_model_id = chat_only.to_string();
            let resolved = effective_vision_model_id(&config);
            assert_eq!(resolved, DEFAULT_OLLAMA_VISION_MODEL);
            assert!(vision_models::is_vision_capable(&resolved));
        }
    }

    /// The default must itself be vision-capable — it is the fallback the
    /// guard above lands on, so a chat-only default would defeat the guard.
    #[test]
    fn default_vision_model_is_vision_capable() {
        assert!(!DEFAULT_OLLAMA_VISION_MODEL.is_empty());
        assert!(vision_models::is_vision_capable(
            DEFAULT_OLLAMA_VISION_MODEL
        ));
        assert!(vision_models::is_vision_capable(DEFAULT_LOW_VISION_MODEL));
    }

    /// #5146 §Part 1: an unconfigured vision model must produce an actionable
    /// error, not an empty model id that downstream code sends to Ollama.
    #[test]
    fn resolve_vision_model_id_errors_when_unconfigured() {
        let mut config = test_config();
        config.local_ai.vision_model_id = String::new();

        let err = resolve_vision_model_choice(&config)
            .err()
            .expect("expected a vision error");
        assert!(
            err.contains("vision_model_id"),
            "error should name the config key to set: {err}"
        );
        assert!(
            err.contains("ollama pull"),
            "error should say how to install a model: {err}"
        );
        // Whitespace-only is the same "not configured" state.
        config.local_ai.vision_model_id = "   ".to_string();
        assert!(resolve_vision_model_choice(&config).is_err());
    }

    #[test]
    fn resolve_vision_model_id_returns_a_vision_capable_model_when_configured() {
        let mut config = test_config();
        config.local_ai.vision_model_id = "llava:7b".to_string();
        assert_eq!(
            resolve_vision_model_choice(&config).unwrap().model,
            "llava:7b"
        );

        // Even a chat-only configured id resolves to something that can see.
        config.local_ai.vision_model_id = "gemma3n:e4b-it-q8_0".to_string();
        let resolved = resolve_vision_model_choice(&config).unwrap().model;
        assert!(vision_models::is_vision_capable(&resolved));
    }

    #[test]
    fn resolve_vision_model_choice_reports_only_capability_substitutions() {
        let mut config = test_config();

        // A vision-capable id is used as-is, with nothing to report.
        config.local_ai.vision_model_id = "llava:7b".to_string();
        let choice = resolve_vision_model_choice(&config).unwrap();
        assert_eq!(choice.model, "llava:7b");
        assert_eq!(choice.replaced, None);

        // A chat-only id is replaced, and the configured id is reported so the
        // caller can explain the swap instead of naming a model out of nowhere.
        config.local_ai.vision_model_id = "gemma3n:e4b-it-q8_0".to_string();
        let choice = resolve_vision_model_choice(&config).unwrap();
        assert!(vision_models::is_vision_capable(&choice.model));
        assert_eq!(choice.replaced.as_deref(), Some("gemma3n:e4b-it-q8_0"));

        // An alias rewrite resolves to a different string but is the same model
        // the user asked for, so it must not be reported as a substitution.
        config.local_ai.vision_model_id = "moondream".to_string();
        let choice = resolve_vision_model_choice(&config).unwrap();
        assert!(vision_models::is_vision_capable(&choice.model));
        assert_eq!(choice.replaced, None);
    }

    #[test]
    fn embedding_model_empty_falls_back_to_bge_m3() {
        // After the cloud-embeddings unification PR, the default embedder
        // for the local Ollama path is bge-m3 (1024 dim) to match memory
        // tree's fixed on-disk format. Empty / whitespace input must
        // resolve to that default, not the prior all-minilm:latest.
        let mut config = test_config();
        config.local_ai.embedding_model_id = String::new();
        assert_eq!(effective_embedding_model_id(&config), "bge-m3");

        config.local_ai.embedding_model_id = "   ".to_string();
        assert_eq!(effective_embedding_model_id(&config), "bge-m3");
    }

    #[test]
    fn embedding_model_passes_through_allowlisted_legacy() {
        // all-minilm:latest is kept in MVP_ALLOWED_EMBEDDING_MODELS for
        // back-compat with users who already pulled it under the prior
        // default. It is NOT 1024-dim — memory tree's post-call validator
        // will surface that mismatch at embed time — but the allowlist
        // enforcer itself must let the value pass through unchanged.
        let mut config = test_config();
        config.local_ai.embedding_model_id = "all-minilm:latest".to_string();
        assert_eq!(effective_embedding_model_id(&config), "all-minilm:latest");
    }

    #[test]
    fn embedding_model_rejects_non_allowlisted_and_redirects_to_default() {
        // Any non-allowlisted value (including legacy nomic-embed-text:latest
        // and arbitrary user input) is silently redirected to the canonical
        // default. This is the path that fired the "embedding model not in
        // MVP allowlist, redirecting to default" warning on every embed
        // resolution before bge-m3 was added to the allowlist.
        let mut config = test_config();
        config.local_ai.embedding_model_id = "nomic-embed-text:latest".to_string();
        assert_eq!(effective_embedding_model_id(&config), "bge-m3");

        config.local_ai.embedding_model_id = "totally-made-up-model:v0".to_string();
        assert_eq!(effective_embedding_model_id(&config), "bge-m3");
    }

    #[test]
    fn lm_studio_embedding_model_passes_through_served_name() {
        // The native local-runtime fix for #3920: LM Studio serves embeddings
        // under user-managed names that are not in the MVP allowlist. A
        // configured id must reach the runtime unchanged rather than being
        // rewritten back to bge-m3 (which the LM Studio server would not have
        // under that exact name).
        let mut config = test_config();
        config.local_ai.provider = "lm_studio".to_string();
        config.local_ai.embedding_model_id = "text-embedding-bge-m3".to_string();
        assert_eq!(
            effective_embedding_model_id(&config),
            "text-embedding-bge-m3"
        );
    }

    #[test]
    fn lm_studio_embedding_model_passes_through_arbitrary_id() {
        // Contrast with `embedding_model_rejects_non_allowlisted_and_redirects_to_default`:
        // the SAME non-allowlisted id is rewritten to bge-m3 on the managed
        // Ollama path but passes through unchanged on the LM Studio path.
        let mut config = test_config();
        config.local_ai.provider = "lm_studio".to_string();
        config.local_ai.embedding_model_id = "nomic-embed-text:latest".to_string();
        assert_eq!(
            effective_embedding_model_id(&config),
            "nomic-embed-text:latest"
        );
    }

    #[test]
    fn lm_studio_embedding_model_empty_falls_back_to_default() {
        // With no configured embedding id, fall back to the canonical default
        // so the memory tree still has an embedder to request.
        let mut config = test_config();
        config.local_ai.provider = "lm_studio".to_string();
        config.local_ai.embedding_model_id = String::new();
        assert_eq!(effective_embedding_model_id(&config), "bge-m3");

        config.local_ai.embedding_model_id = "   ".to_string();
        assert_eq!(effective_embedding_model_id(&config), "bge-m3");
    }

    #[test]
    fn ollama_embedding_path_still_enforces_allowlist_after_lm_studio_bypass() {
        // Guard: the LM Studio bypass must not weaken the managed Ollama path.
        // Default provider (Ollama) still rewrites a non-allowlisted id.
        let mut config = test_config();
        config.local_ai.embedding_model_id = "text-embedding-bge-m3".to_string();
        assert_eq!(effective_embedding_model_id(&config), "bge-m3");
    }

    #[test]
    fn stt_tts_and_quantization_defaults_are_applied() {
        let mut config = test_config();
        config.local_ai.stt_model_id.clear();
        config.local_ai.tts_voice_id.clear();
        config.local_ai.quantization = "Q5_K_M".to_string();

        assert_eq!(effective_stt_model_id(&config), "ggml-base-q5_1.bin");
        assert_eq!(effective_tts_voice_id(&config), "en_US-lessac-medium");
        assert_eq!(effective_quantization(&config), "q5_k_m");
    }
}
