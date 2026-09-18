//! [`NarratedToolCallMiddleware`]: run a tool call the model wrote as text
//! (#6344).
//!
//! The loop only executes `response.message.tool_calls`. A model with a
//! structured channel sometimes emits its *own* wire syntax as plain text
//! instead — Gemma-style `<|tool_call>call:NAME{…}<tool_call|>` or DeepSeek
//! `<｜DSML｜invoke name="…">`. That text used to become the final message:
//! the tool never ran, and the markup reached the user or, from a sub-agent,
//! came back as the delegation's "result".
//!
//! Only these provider control-token dialects are recognised. They never occur
//! in prose, so recognising them cannot misfire on an answer that merely
//! *explains* tool calling; the plain `<tool_call>` XML form can, and is left to
//! the text-mode dialects that actually teach it.

use std::sync::OnceLock;

use async_trait::async_trait;
use regex::Regex;
use serde_json::{Map, Value};

use tinyagents_harness::context::RunContext;
use tinyagents_harness::error::{Result as TaResult, TinyAgentsError};
use tinyagents_harness::middleware::Middleware;
use tinyinference::message::ContentBlock;
use tinyinference::model::ModelResponse;
use tinyinference::tool::ToolCall as TaToolCall;

/// Opening tokens of the recognised dialects. Any left in the text after
/// [`recover`] is markup that could not be parsed.
const MARKERS: [&str; 2] = ["<|tool_call>", "<｜DSML｜"];

/// A narrated call, recovered as a tool name and its arguments.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NarratedCall {
    pub name: String,
    pub arguments: Value,
}

/// Text with every parsed call span removed, and the calls in order.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct Recovered {
    pub prose: String,
    pub calls: Vec<NarratedCall>,
}

impl Recovered {
    /// Markup remains that matched no call — a dialect variant this parser does
    /// not know, or a span the model cut off.
    pub fn has_unparsed_markup(&self) -> bool {
        contains_marker(&self.prose)
    }
}

pub(crate) fn contains_marker(text: &str) -> bool {
    MARKERS.iter().any(|m| text.contains(m))
}

fn gemma_span() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?s)<\|tool_call>(.*?)<tool_call\|>").expect("gemma span"))
}

fn dsml_block() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?s)<｜DSML｜(?:tool_calls|function_calls)>.*?</｜DSML｜(?:tool_calls|function_calls)>")
            .expect("dsml block")
    })
}

fn dsml_invoke() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?s)<｜DSML｜invoke name="([^"]+)">(.*?)</｜DSML｜invoke>"#)
            .expect("dsml invoke")
    })
}

fn dsml_parameter() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r#"(?s)<｜DSML｜parameter name="([^"]+)"(?: string="(true|false)")?>(.*?)</｜DSML｜parameter>"#,
        )
        .expect("dsml parameter")
    })
}

/// Parse every narrated call in `text`. Spans that parse are removed from the
/// prose; spans that do not are left in place so [`Recovered::has_unparsed_markup`]
/// reports them.
pub(crate) fn recover(text: &str) -> Recovered {
    if !contains_marker(text) {
        return Recovered {
            prose: text.to_string(),
            calls: Vec::new(),
        };
    }
    let mut calls = Vec::new();

    let text =
        gemma_span().replace_all(text, |caps: &regex::Captures<'_>| {
            match parse_gemma_call(&caps[1]) {
                Some(call) => {
                    calls.push(call);
                    String::new()
                }
                None => caps[0].to_string(),
            }
        });

    let text = dsml_block().replace_all(&text, |caps: &regex::Captures<'_>| {
        let block = &caps[0];
        let parsed: Vec<NarratedCall> = dsml_invoke()
            .captures_iter(block)
            .map(|invoke| NarratedCall {
                name: invoke[1].to_string(),
                arguments: parse_dsml_parameters(&invoke[2]),
            })
            .collect();
        if parsed.is_empty() {
            return block.to_string();
        }
        calls.extend(parsed);
        String::new()
    });

    Recovered {
        prose: text.trim().to_string(),
        calls,
    }
}

/// `call:NAME{key:<|"|>v<|"|>,n:1}` or `call:{"name":…,"arguments":{…}}`.
fn parse_gemma_call(inner: &str) -> Option<NarratedCall> {
    let body = inner.trim().strip_prefix("call:")?.trim();
    // `<|"|>` is the dialect's string delimiter.
    let body = body.replace("<|\"|>", "\"");
    if body.starts_with('{') {
        let value: Value = serde_json::from_str(&body).ok()?;
        let name = value.get("name")?.as_str()?.to_string();
        let arguments = value
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| Value::Object(Map::new()));
        return arguments
            .is_object()
            .then_some(NarratedCall { name, arguments });
    }
    let brace = body.find('{')?;
    let name = body[..brace].trim();
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return None;
    }
    let arguments: Value = serde_json::from_str(&quote_bare_keys(&body[brace..])).ok()?;
    arguments.is_object().then(|| NarratedCall {
        name: name.to_string(),
        arguments,
    })
}

/// Quote the unquoted object keys of a JSON-like body (`{fetch_type:"pages"}`).
/// A bare identifier is a key only where a key can start — after `{` or `,`,
/// outside a string — and only when a `:` follows it.
fn quote_bare_keys(body: &str) -> String {
    let chars: Vec<char> = body.chars().collect();
    let mut out = String::with_capacity(body.len() + 8);
    let mut in_string = false;
    let mut escaped = false;
    let mut last_significant = ' ';
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if in_string {
            out.push(c);
            match (escaped, c) {
                (true, _) => escaped = false,
                (false, '\\') => escaped = true,
                (false, '"') => in_string = false,
                _ => {}
            }
            i += 1;
            continue;
        }
        if (c.is_ascii_alphabetic() || c == '_') && matches!(last_significant, '{' | ',') {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let ident: String = chars[start..i].iter().collect();
            let mut j = i;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            if chars.get(j) == Some(&':') {
                out.push('"');
                out.push_str(&ident);
                out.push('"');
            } else {
                out.push_str(&ident);
            }
            last_significant = 'a';
            continue;
        }
        if c == '"' {
            in_string = true;
        }
        if !c.is_whitespace() {
            last_significant = c;
        }
        out.push(c);
        i += 1;
    }
    out
}

/// `string="true"` values are literal text; `string="false"` values are JSON.
fn parse_dsml_parameters(invoke_body: &str) -> Value {
    let mut arguments = Map::new();
    for param in dsml_parameter().captures_iter(invoke_body) {
        let raw = &param[3];
        let is_string = param.get(2).is_none_or(|flag| flag.as_str() == "true");
        let value = if is_string {
            Value::String(raw.to_string())
        } else {
            serde_json::from_str(raw.trim()).unwrap_or_else(|_| Value::String(raw.to_string()))
        };
        arguments.insert(param[1].to_string(), value);
    }
    Value::Object(arguments)
}

/// `after_model`: turn a narrated call into the structured call the loop runs,
/// or fail the run when the markup cannot be parsed. Never lets the markup
/// through as the message.
pub(crate) struct NarratedToolCallMiddleware;

#[async_trait]
impl Middleware<()> for NarratedToolCallMiddleware {
    fn name(&self) -> &str {
        "narrated_tool_call"
    }

    async fn after_model(
        &self,
        _ctx: &mut RunContext<()>,
        _state: &(),
        response: &mut ModelResponse,
    ) -> TaResult<()> {
        if !response.message.tool_calls.is_empty() {
            return Ok(());
        }
        let text: String = response
            .message
            .content
            .iter()
            .filter_map(ContentBlock::as_text)
            .collect();
        if !contains_marker(&text) {
            return Ok(());
        }
        let recovered = recover(&text);
        if recovered.has_unparsed_markup() {
            tracing::warn!(
                chars = text.len(),
                "[tinyagents::mw] narrated tool call could not be parsed — failing the run"
            );
            return Err(TinyAgentsError::Middleware(
                "the model wrote a tool call as text in a format that could not be parsed, \
                 so the call did not run"
                    .to_string(),
            ));
        }
        let names: Vec<&str> = recovered.calls.iter().map(|c| c.name.as_str()).collect();
        tracing::warn!(
            ?names,
            "[tinyagents::mw] model narrated tool call(s) as text — recovered as structured calls"
        );

        response
            .message
            .content
            .retain(|block| !matches!(block, ContentBlock::Text(_)));
        if !recovered.prose.is_empty() {
            response
                .message
                .content
                .insert(0, ContentBlock::Text(recovered.prose));
        }
        response.message.tool_calls = recovered
            .calls
            .into_iter()
            .map(|call| TaToolCall {
                id: format!("call_narrated_{}", uuid::Uuid::new_v4().simple()),
                name: call.name,
                arguments: call.arguments,
                invalid: None,
            })
            .collect();
        response.finish_reason = Some("tool_calls".to_string());
        Ok(())
    }
}

#[cfg(test)]
#[path = "narrated_tool_call_tests.rs"]
mod tests;
