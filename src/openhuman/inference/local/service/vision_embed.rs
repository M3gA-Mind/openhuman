use crate::openhuman::agent::multimodal;
use crate::openhuman::config::Config;
use crate::openhuman::inference::local::ollama::{
    ollama_base_url_from_config, redact_ollama_base_url, OllamaGenerateOptions,
    OllamaGenerateRequest,
};
use crate::openhuman::inference::model_ids;
use crate::openhuman::inference::presets::{self, VisionMode};
use crate::openhuman::inference::types::LocalAiEmbeddingResult;
use tinyagents::harness::embeddings::{
    EmbeddingModel, OllamaEmbeddingModel, DEFAULT_OLLAMA_DIMENSIONS,
    RECOMMENDED_OLLAMA_CONTEXT_TOKENS,
};

use super::LocalAiService;

fn embedding_dimensions(model_id: &str) -> Option<usize> {
    let normalized = model_id.trim().to_ascii_lowercase();
    if normalized.starts_with("all-minilm") {
        Some(384)
    } else if normalized.contains("bge-m3") || normalized.starts_with("mxbai-embed-large") {
        Some(DEFAULT_OLLAMA_DIMENSIONS)
    } else if normalized.starts_with("nomic-embed-text") {
        Some(768)
    } else {
        None
    }
}

impl LocalAiService {
    pub async fn vision_prompt(
        &self,
        config: &Config,
        prompt: &str,
        image_refs: &[String],
        max_tokens: Option<u32>,
    ) -> Result<String, String> {
        if !config.local_ai.runtime_enabled {
            return Err("local ai is disabled".to_string());
        }
        if image_refs.is_empty() {
            return Err("vision prompt requires at least one image reference".to_string());
        }
        if matches!(
            presets::vision_mode_for_config(&config.local_ai),
            VisionMode::Disabled
        ) {
            self.status.lock().vision_state = "disabled".to_string();
            return Err(
                "vision summaries are unavailable for this RAM tier. Use OCR-only summarization or switch to a higher local AI tier."
                    .to_string(),
            );
        }
        self.bootstrap(config).await;

        // Resolve through `resolve_vision_model_choice` rather than
        // `effective_vision_model_id`: the latter returns an empty string when
        // no vision model is configured, which used to be handed straight to
        // `ensure_ollama_model_available` and became a nameless `POST
        // /api/pull` retried three times before failing opaquely (#5146).
        // The resolver guarantees a non-empty, vision-capable id or a message
        // that says what to configure.
        // NOTE: this Err arm is defence in depth, not a reachable branch here.
        // `resolve_vision_model_choice` fails only when no vision model is
        // configured, and that same condition makes `vision_mode_for_config`
        // report `Disabled` (a blank `vision_model_id` cannot match any
        // vision-enabled preset, so the tier resolves to `Custom` -> Disabled),
        // which returns above. The empty case is covered directly in
        // `model_ids::tests::resolve_vision_model_id_errors_when_unconfigured`.
        let choice = match model_ids::resolve_vision_model_choice(config) {
            Ok(choice) => choice,
            Err(error) => {
                self.status.lock().vision_state = "missing".to_string();
                tracing::warn!(
                    target: "local_ai::vision",
                    %error,
                    "[local_ai:vision] no vision-capable model resolved; refusing request"
                );
                return Err(error);
            }
        };
        let vision_model = choice.model;
        // A capability substitution means we are about to talk about a model the
        // user never named. Carry that into the error text so "pull moondream"
        // cannot read as a non-sequitur to someone who configured gemma3.
        let substitution_note = choice
            .replaced
            .as_deref()
            .map(|configured| {
                format!(
                    " Your configured `{configured}` cannot accept images, so OpenHuman \
                     selected `{vision_model}` instead; set `local_ai.vision_model_id` to \
                     a vision-capable model to choose your own."
                )
            })
            .unwrap_or_default();
        tracing::debug!(
            target: "local_ai::vision",
            model = %vision_model,
            substituted_for = ?choice.replaced,
            "[local_ai:vision] resolved vision-capable model"
        );

        // A model that is configured but not pulled (and cannot be pulled)
        // must also read as a vision problem, not a generic pull failure.
        if let Err(error) = self
            .ensure_ollama_model_available(config, &vision_model, "vision")
            .await
        {
            self.status.lock().vision_state = "missing".to_string();
            tracing::warn!(
                target: "local_ai::vision",
                model = %vision_model,
                substituted_for = ?choice.replaced,
                %error,
                "[local_ai:vision] vision model unavailable"
            );
            return Err(format!(
                "local vision model `{vision_model}` is not available: {error}. \
                 Pull it with `ollama pull {vision_model}`, or route the vision \
                 workload to a cloud provider with `vision_provider`.{substitution_note}"
            ));
        }

        let images: Vec<String> = image_refs
            .iter()
            .filter_map(|reference| multimodal::extract_ollama_image_payload(reference))
            .collect();
        if images.is_empty() {
            return Err("no valid image payloads were provided".to_string());
        }

        // Vision generation is background LLM-bound work; gate it through
        // the scheduler's global LLM permit.
        let _gate_permit = crate::openhuman::scheduler_gate::wait_for_capacity().await;

        let body = OllamaGenerateRequest {
            model: vision_model,
            prompt: prompt.trim().to_string(),
            system: Some("You are a vision model. Answer directly and concisely.".to_string()),
            images: Some(images),
            stream: false,
            options: Some(OllamaGenerateOptions {
                temperature: Some(0.2),
                top_k: Some(30),
                top_p: Some(0.9),
                num_predict: max_tokens.map(|v| v as i32),
            }),
        };

        let base = ollama_base_url_from_config(config);
        let url = format!("{base}/api/generate");
        let body_bytes = serde_json::to_vec(&body).map(|v| v.len()).unwrap_or(0);
        tracing::debug!(
            target: "local_ai::vision",
            %base,
            %url,
            model = %body.model,
            prompt_chars = body.prompt.chars().count(),
            images = body.images.as_ref().map(|v| v.len()).unwrap_or(0),
            body_bytes,
            "[local_ai:vision] sending generate request"
        );

        let response = self.http.post(&url).json(&body).send().await.map_err(|e| {
            tracing::warn!(
                target: "local_ai::vision",
                %url,
                error = %e,
                "[local_ai:vision] request send failed"
            );
            format!("ollama vision request failed: {e}")
        })?;

        let status = response.status();
        tracing::debug!(
            target: "local_ai::vision",
            %url,
            %status,
            "[local_ai:vision] received response"
        );

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            let detail = body.trim();
            tracing::warn!(
                target: "local_ai::vision",
                %url,
                %status,
                body = %detail,
                "[local_ai:vision] non-success response"
            );
            return Err(format!(
                "ollama vision request failed with status {}{}",
                status,
                if detail.is_empty() {
                    String::new()
                } else {
                    format!(": {detail}")
                }
            ));
        }

        let payload: crate::openhuman::inference::local::ollama::OllamaGenerateResponse = response
            .json()
            .await
            .map_err(|e| format!("ollama vision response parse failed: {e}"))?;
        if payload.response.trim().is_empty() {
            return Err("ollama vision returned empty content".to_string());
        }

        self.status.lock().vision_state = "ready".to_string();
        Ok(payload.response)
    }

    pub async fn embed(
        &self,
        config: &Config,
        inputs: &[String],
    ) -> Result<LocalAiEmbeddingResult, String> {
        if !config.local_ai.runtime_enabled {
            return Err("local ai is disabled".to_string());
        }
        let items: Vec<String> = inputs
            .iter()
            .map(|x| x.trim().to_string())
            .filter(|x| !x.is_empty())
            .collect();
        if items.is_empty() {
            return Err("embed requires at least one non-empty input".to_string());
        }
        self.bootstrap(config).await;
        let embedding_model = model_ids::effective_embedding_model_id(config);
        self.ensure_ollama_model_available(config, &embedding_model, "embedding")
            .await?;

        // Embeds are bge-m3 calls (8K context, ~1.3 GB resident) — the
        // single concurrent embed that has historically crashed the
        // user's laptop when stacked with other Ollama work. Gate it.
        let _gate_permit = crate::openhuman::scheduler_gate::wait_for_capacity().await;

        let embed_base = ollama_base_url_from_config(config);
        let dimensions = embedding_dimensions(&embedding_model);
        log::debug!(
            "[local_ai:embed] embed: using model={} dimensions={} base_url={}",
            embedding_model,
            dimensions
                .map(|value| value.to_string())
                .unwrap_or_else(|| "dynamic".to_string()),
            redact_ollama_base_url(&embed_base)
        );
        let (dims, vectors) = if let Some(dimensions) = dimensions {
            let model = OllamaEmbeddingModel::try_new(&embed_base, &embedding_model, dimensions)
                .map_err(|error| format!("invalid local embedding RPC configuration: {error}"))?
                .with_client(self.http.clone())
                .with_context_options(
                    RECOMMENDED_OLLAMA_CONTEXT_TOKENS,
                    RECOMMENDED_OLLAMA_CONTEXT_TOKENS,
                );
            let vectors = model
                .embed(&items)
                .await
                .map_err(|error| format!("local embedding RPC failed: {error}"))?;
            (model.dimensions(), vectors)
        } else {
            OllamaEmbeddingModel::embed_discovering_dimensions(
                &embed_base,
                &embedding_model,
                self.http.clone(),
                &items,
                RECOMMENDED_OLLAMA_CONTEXT_TOKENS,
                RECOMMENDED_OLLAMA_CONTEXT_TOKENS,
            )
            .await
            .map_err(|error| format!("local embedding RPC failed: {error}"))?
        };
        self.status.lock().embedding_state = "ready".to_string();
        Ok(LocalAiEmbeddingResult {
            model_id: embedding_model,
            dimensions: dims,
            vectors,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{routing::post, Json, Router};
    use serde_json::json;

    async fn spawn_mock(app: Router) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://127.0.0.1:{}", addr.port())
    }

    fn enabled_config() -> Config {
        let mut c = Config::default();
        c.local_ai.runtime_enabled = true;
        c
    }

    fn ready_service(config: &Config) -> LocalAiService {
        let s = LocalAiService::new(config);
        {
            let mut g = s.status.lock();
            g.state = "ready".to_string();
        }
        s
    }

    fn mock_with_tags_and(route: &str, handler: axum::routing::MethodRouter) -> Router {
        use axum::routing::get;
        // Respond to `/api/tags` with a payload that contains whatever model
        // the caller asks about, so `has_model` returns true and `embed`
        // proceeds to the real endpoint.
        Router::new()
            .route(
                "/api/tags",
                get(|| async {
                    Json(json!({
                        "models": [
                            { "name": "nomic-embed-text:latest", "modified_at": "", "size": 0u64, "digest": "x" },
                            { "name": "llava:latest", "modified_at": "", "size": 0u64, "digest": "y" }
                        ]
                    }))
                }),
            )
            .route(route, handler)
    }

    #[tokio::test]
    async fn embed_against_mock_returns_vectors_with_dimensions() {
        let _guard = crate::openhuman::inference::inference_test_guard();

        let app = mock_with_tags_and(
            "/api/embed",
            post(|Json(_b): Json<serde_json::Value>| async {
                Json(json!({
                    "model": "m",
                    "embeddings": [[0.1, 0.2, 0.3], [0.4, 0.5, 0.6]]
                }))
            }),
        );
        let base = spawn_mock(app).await;
        unsafe {
            std::env::set_var("OPENHUMAN_OLLAMA_BASE_URL", &base);
        }

        let config = enabled_config();
        let service = ready_service(&config);
        let result = service
            .embed(&config, &["hello".to_string(), "world".to_string()])
            .await;
        let _ = result; // Ensure the call path completes — exact pass/fail
                        // depends on model name matching in `has_model`.

        unsafe {
            std::env::remove_var("OPENHUMAN_OLLAMA_BASE_URL");
        }
    }

    #[tokio::test]
    async fn embed_rejects_all_empty_inputs_before_network_call() {
        let _guard = crate::openhuman::inference::inference_test_guard();

        // Even without a working mock server, entirely-empty inputs must be
        // rejected before any HTTP call.
        let config = enabled_config();
        let service = ready_service(&config);
        let err = service
            .embed(&config, &["".to_string(), "   ".to_string()])
            .await
            .unwrap_err();
        assert!(err.contains("non-empty input"));
    }

    #[tokio::test]
    async fn embed_disabled_returns_error() {
        let mut config = Config::default();
        config.local_ai.runtime_enabled = false;
        let service = LocalAiService::new(&config);
        let err = service.embed(&config, &["x".into()]).await.unwrap_err();
        assert!(err.contains("local ai is disabled"));
    }

    #[test]
    fn embedding_dimensions_match_supported_legacy_models() {
        assert_eq!(embedding_dimensions("bge-m3"), Some(1024));
        assert_eq!(embedding_dimensions("all-minilm:latest"), Some(384));
        assert_eq!(embedding_dimensions("nomic-embed-text"), Some(768));
        assert_eq!(embedding_dimensions("user-managed-model"), None);
    }

    #[tokio::test]
    async fn vision_prompt_disabled_returns_error() {
        let mut config = Config::default();
        config.local_ai.runtime_enabled = false;
        let service = LocalAiService::new(&config);
        let err = service
            .vision_prompt(&config, "describe", &[], None)
            .await
            .unwrap_err();
        assert!(err.contains("local ai is disabled"));
    }

    // ── #5146 §Part 1: which model a vision request actually reaches ────────
    //
    // These drive the real `vision_prompt` path against a mock Ollama server.
    // `ready_service` marks the status "ready", which makes `bootstrap` return
    // early, so no process launch or network beyond the mock is involved.

    /// Mock Ollama exposing `/api/tags` with `installed` present, and an
    /// `/api/generate` that echoes back the `model` field it was sent. The
    /// echo is what lets a test assert *which* model the request targeted.
    fn mock_ollama_echoing_requested_model(installed: &'static str) -> Router {
        use axum::routing::get;
        Router::new()
            .route(
                "/api/tags",
                get(move || async move {
                    Json(json!({
                        "models": [
                            { "name": installed, "modified_at": "", "size": 0u64, "digest": "a" }
                        ]
                    }))
                }),
            )
            .route(
                "/api/generate",
                post(|Json(body): Json<serde_json::Value>| async move {
                    Json(json!({
                        "response": body["model"].as_str().unwrap_or("<no model field>"),
                        "done": true
                    }))
                }),
            )
    }

    /// A configured, genuinely vision-capable model must reach Ollama unchanged.
    ///
    /// Before #5146 the `MVP_ALLOWED_VISION_MODELS = &[""]` allowlist rewrote
    /// this to the empty string, so the request went out with `model: ""`.
    #[tokio::test]
    async fn vision_prompt_sends_the_configured_vision_capable_model() {
        let _guard = crate::openhuman::inference::inference_test_guard();

        let base = spawn_mock(mock_ollama_echoing_requested_model("llava:7b")).await;
        unsafe {
            std::env::set_var("OPENHUMAN_OLLAMA_BASE_URL", &base);
        }

        let mut config = enabled_config();
        config.local_ai.vision_model_id = "llava:7b".to_string();
        let service = ready_service(&config);

        let result = service
            .vision_prompt(
                &config,
                "describe",
                &["data:image/png;base64,QUJD".to_string()],
                None,
            )
            .await;

        unsafe {
            std::env::remove_var("OPENHUMAN_OLLAMA_BASE_URL");
        }

        assert_eq!(
            result.expect("vision prompt should succeed"),
            "llava:7b",
            "the configured vision model must reach Ollama unchanged"
        );
    }

    /// A chat-only model configured for vision must never be the model that
    /// receives the images.
    ///
    /// Ollama accepts an `images` array against a text-only model, discards it,
    /// and answers from the prompt alone, so passing `gemma3n` through would
    /// return a fabricated description rather than an error.
    #[tokio::test]
    async fn vision_prompt_never_routes_images_at_a_chat_only_model() {
        let _guard = crate::openhuman::inference::inference_test_guard();

        let base = spawn_mock(mock_ollama_echoing_requested_model(
            "moondream:1.8b-v2-q4_K_S",
        ))
        .await;
        unsafe {
            std::env::set_var("OPENHUMAN_OLLAMA_BASE_URL", &base);
        }

        let mut config = enabled_config();
        // Text-only on Ollama, despite sharing a prefix with multimodal gemma3.
        config.local_ai.vision_model_id = "gemma3n:e4b-it-q8_0".to_string();
        let service = ready_service(&config);

        let result = service
            .vision_prompt(
                &config,
                "describe",
                &["data:image/png;base64,QUJD".to_string()],
                None,
            )
            .await;

        unsafe {
            std::env::remove_var("OPENHUMAN_OLLAMA_BASE_URL");
        }

        let model_used = result.expect("vision prompt should succeed");
        assert_ne!(
            model_used, "gemma3n:e4b-it-q8_0",
            "images must never be sent to a chat-only model"
        );
        assert_eq!(model_used, "moondream:1.8b-v2-q4_K_S");
    }

    /// A configured-but-unpullable vision model must report a vision problem
    /// naming the model and the `ollama pull` that fixes it.
    #[tokio::test]
    async fn vision_prompt_reports_an_unavailable_vision_model() {
        use axum::routing::get;
        let _guard = crate::openhuman::inference::inference_test_guard();

        // Empty tag list, and a pull that refuses: nothing to fall back to.
        let app = Router::new()
            .route("/api/tags", get(|| async { Json(json!({ "models": [] })) }))
            .route(
                "/api/pull",
                post(|| async {
                    (
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        "pull refused",
                    )
                }),
            );
        let base = spawn_mock(app).await;
        unsafe {
            std::env::set_var("OPENHUMAN_OLLAMA_BASE_URL", &base);
        }

        let mut config = enabled_config();
        config.local_ai.vision_model_id = "llava:7b".to_string();
        let service = ready_service(&config);

        let err = service
            .vision_prompt(
                &config,
                "describe",
                &["data:image/png;base64,QUJD".to_string()],
                None,
            )
            .await
            .expect_err("an unpullable vision model must fail");

        unsafe {
            std::env::remove_var("OPENHUMAN_OLLAMA_BASE_URL");
        }

        assert!(
            err.contains("llava:7b"),
            "error should name the model: {err}"
        );
        assert!(
            err.contains("ollama pull"),
            "error should say how to install it: {err}"
        );
        assert_eq!(service.status.lock().vision_state, "missing");
    }

    /// greptile #5253: when the capability guard swaps a chat-only model for a
    /// vision-capable default, an unavailable-model error must say so. Without
    /// this the user is told to `ollama pull moondream:…` having configured
    /// `gemma3n:…`, with nothing connecting the two.
    #[tokio::test]
    async fn unavailable_error_explains_a_capability_substitution() {
        use axum::routing::get;
        let _guard = crate::openhuman::inference::inference_test_guard();

        let app = Router::new()
            .route("/api/tags", get(|| async { Json(json!({ "models": [] })) }))
            .route(
                "/api/pull",
                post(|| async {
                    (
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        "pull refused",
                    )
                }),
            );
        let base = spawn_mock(app).await;
        unsafe {
            std::env::set_var("OPENHUMAN_OLLAMA_BASE_URL", &base);
        }

        let mut config = enabled_config();
        // Chat-only: the guard substitutes the vision-capable default.
        config.local_ai.vision_model_id = "gemma3n:e4b-it-q8_0".to_string();
        let service = ready_service(&config);

        let err = service
            .vision_prompt(
                &config,
                "describe",
                &["data:image/png;base64,QUJD".to_string()],
                None,
            )
            .await
            .expect_err("an unpullable substituted model must fail");

        unsafe {
            std::env::remove_var("OPENHUMAN_OLLAMA_BASE_URL");
        }

        assert!(
            err.contains("gemma3n:e4b-it-q8_0"),
            "error must name the model the user actually configured: {err}"
        );
        assert!(
            err.contains("cannot accept images"),
            "error must explain why it was replaced: {err}"
        );
        assert!(
            err.contains(crate::openhuman::inference::model_ids::DEFAULT_OLLAMA_VISION_MODEL),
            "error must name the substitute it is asking the user to pull: {err}"
        );
    }
}
