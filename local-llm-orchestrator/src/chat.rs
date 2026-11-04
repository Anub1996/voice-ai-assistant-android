//! The orchestrated chat pipeline:
//! recall memories -> inject context -> route to runtime -> forward ->
//! remember the turn.

use axum::http::HeaderMap;
use serde_json::{Value, json};

use crate::error::{Error, Result};
use crate::memory::{MIN_STORE_CHARS, MemoryService, NewMemory};
use crate::runtime::Lease;
use crate::server::AppState;

pub const H_NAMESPACE: &str = "x-llmo-namespace";
pub const H_SESSION: &str = "x-llmo-session";
pub const H_MEMORY: &str = "x-llmo-memory";
pub const H_RUNTIME: &str = "x-llmo-runtime";
pub const H_INJECTED: &str = "x-llmo-memories";

/// Per-request memory behaviour, taken from `x-llmo-*` headers so that any
/// stock OpenAI client can use it via "default headers".
#[derive(Debug, Clone, PartialEq)]
pub struct MemoryOptions {
    pub namespace: String,
    pub session: Option<String>,
    pub recall: bool,
    pub store: bool,
}

impl Default for MemoryOptions {
    fn default() -> Self {
        Self {
            namespace: "default".into(),
            session: None,
            recall: true,
            store: true,
        }
    }
}

pub fn valid_ident(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | ':' | '@'))
}

impl MemoryOptions {
    pub fn from_headers(h: &HeaderMap) -> Result<Self> {
        let get = |k: &str| h.get(k).and_then(|v| v.to_str().ok()).map(str::trim);
        let mut o = MemoryOptions::default();
        if let Some(ns) = get(H_NAMESPACE) {
            if !valid_ident(ns) {
                return Err(Error::BadRequest(format!("invalid {H_NAMESPACE}")));
            }
            o.namespace = ns.to_string();
        }
        if let Some(s) = get(H_SESSION) {
            if !valid_ident(s) {
                return Err(Error::BadRequest(format!("invalid {H_SESSION}")));
            }
            o.session = Some(s.to_string());
        }
        if let Some(mode) = get(H_MEMORY) {
            (o.recall, o.store) = match mode.to_ascii_lowercase().as_str() {
                "on" | "" => (true, true),
                "off" => (false, false),
                "recall" => (true, false),
                "store" => (false, true),
                other => {
                    return Err(Error::BadRequest(format!(
                        "{H_MEMORY} must be on|off|recall|store, got `{other}`"
                    )));
                }
            };
        }
        Ok(o)
    }
}

/// Text of an OpenAI message `content` (string or array of parts).
pub fn content_text(content: &Value) -> String {
    match content {
        Value::String(s) => s.clone(),
        Value::Array(parts) => parts
            .iter()
            .filter(|p| p["type"] == "text")
            .filter_map(|p| p["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

pub struct Prepared {
    pub body: Value,
    pub lease: Lease,
    pub injected: usize,
    pub user_text: Option<String>,
    pub stream: bool,
    pub opts: MemoryOptions,
}

/// Validate the request, inject memory context and acquire a runtime.
pub async fn prepare(state: &AppState, mut body: Value, opts: MemoryOptions) -> Result<Prepared> {
    let model = body["model"]
        .as_str()
        .ok_or_else(|| Error::BadRequest("`model` is required".into()))?
        .to_string();
    let messages = body["messages"]
        .as_array_mut()
        .ok_or_else(|| Error::BadRequest("`messages` must be an array".into()))?;

    let user_text = messages
        .iter()
        .rev()
        .find(|m| m["role"] == "user")
        .map(|m| content_text(&m["content"]))
        .filter(|t| !t.trim().is_empty());

    let mut injected = 0;
    if opts.recall
        && let Some(query) = &user_text
    {
        let seen: Vec<String> = messages
            .iter()
            .map(|m| content_text(&m["content"]))
            .collect();
        if let Some((ctx, used)) = state
            .memory
            .context_for(&opts.namespace, query, &seen)
            .await?
        {
            injected = used.len();
            inject_system(messages, &ctx);
        }
    }

    let stream = body["stream"].as_bool().unwrap_or(false);
    let lease = state.runtimes.acquire(&model).await?;
    body["model"] = Value::String(lease.model.clone());
    Ok(Prepared {
        body,
        lease,
        injected,
        user_text,
        stream,
        opts,
    })
}

/// Merge memory context into the leading system message, or add one.
/// (Several chat templates reject a second system message.)
pub fn inject_system(messages: &mut Vec<Value>, ctx: &str) {
    if let Some(first) = messages.first_mut()
        && first["role"] == "system"
    {
        if let Some(s) = first["content"].as_str() {
            first["content"] = Value::String(format!("{s}\n\n{ctx}"));
            return;
        }
        if let Some(parts) = first["content"].as_array_mut() {
            parts.push(json!({ "type": "text", "text": ctx }));
            return;
        }
    }
    messages.insert(0, json!({ "role": "system", "content": ctx }));
}

/// Persist the user turn and the assistant reply in the background.
pub fn remember_turn(
    state: &AppState,
    opts: &MemoryOptions,
    user_text: Option<String>,
    assistant_text: Option<String>,
) {
    if !opts.store {
        return;
    }
    let memory = state.memory.clone();
    let opts = opts.clone();
    tokio::spawn(async move { store_turn(&memory, &opts, user_text, assistant_text).await });
}

/// Persist the user turn and the assistant reply; returns how many were kept.
pub async fn store_turn(
    memory: &MemoryService,
    opts: &MemoryOptions,
    user_text: Option<String>,
    assistant_text: Option<String>,
) -> usize {
    if !opts.store {
        return 0;
    }
    let turns = [("user", user_text), ("assistant", assistant_text)]
        .into_iter()
        .filter_map(|(role, t)| t.map(|t| (role, t)))
        .filter(|(_, t)| t.trim().chars().count() >= MIN_STORE_CHARS);
    let mut stored = 0;
    for (role, content) in turns {
        let new = NewMemory {
            namespace: opts.namespace.clone(),
            session_id: opts.session.clone(),
            role: role.into(),
            content,
            tags: vec![],
            pinned: false,
        };
        match memory.remember(new).await {
            Ok(_) => stored += 1,
            Err(e) => tracing::warn!(error = %e, "failed to store conversation turn"),
        }
    }
    stored
}

/// Reply text from a non-streaming chat completion.
pub fn completion_text(v: &Value) -> Option<String> {
    let c = &v["choices"][0]["message"]["content"];
    let t = content_text(c);
    (!t.is_empty()).then_some(t)
}

/// Incrementally parses an SSE byte stream of chat completion chunks and
/// accumulates `choices[0].delta.content`.
#[derive(Default)]
pub struct SseTap {
    buf: Vec<u8>,
    pub content: String,
}

impl SseTap {
    pub fn feed(&mut self, chunk: &[u8]) {
        self.buf.extend_from_slice(chunk);
        while let Some(pos) = self.buf.iter().position(|b| *b == b'\n') {
            let line: Vec<u8> = self.buf.drain(..=pos).collect();
            self.line(&line);
        }
    }

    pub fn finish(mut self) -> String {
        let rest = std::mem::take(&mut self.buf);
        self.line(&rest);
        self.content
    }

    fn line(&mut self, line: &[u8]) {
        let Ok(line) = std::str::from_utf8(line) else {
            return;
        };
        let Some(data) = line.trim().strip_prefix("data:") else {
            return;
        };
        let data = data.trim();
        if data == "[DONE]" {
            return;
        }
        if let Ok(v) = serde_json::from_str::<Value>(data)
            && let Some(s) = v["choices"][0]["delta"]["content"].as_str()
        {
            self.content.push_str(s);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn headers_to_options() {
        let mut h = HeaderMap::new();
        assert_eq!(
            MemoryOptions::from_headers(&h).unwrap(),
            MemoryOptions::default()
        );
        h.insert(H_NAMESPACE, HeaderValue::from_static("work"));
        h.insert(H_SESSION, HeaderValue::from_static("s-1"));
        h.insert(H_MEMORY, HeaderValue::from_static("recall"));
        let o = MemoryOptions::from_headers(&h).unwrap();
        assert_eq!(o.namespace, "work");
        assert_eq!(o.session.as_deref(), Some("s-1"));
        assert!(o.recall && !o.store);
        h.insert(H_MEMORY, HeaderValue::from_static("sometimes"));
        assert!(MemoryOptions::from_headers(&h).is_err());
        h.insert(H_MEMORY, HeaderValue::from_static("off"));
        h.insert(H_NAMESPACE, HeaderValue::from_static("../etc"));
        assert!(MemoryOptions::from_headers(&h).is_err());
    }

    #[test]
    fn injects_into_existing_system_message() {
        let mut m = vec![
            json!({"role":"system","content":"Be terse."}),
            json!({"role":"user","content":"hi"}),
        ];
        inject_system(&mut m, "CTX");
        assert_eq!(m.len(), 2);
        assert_eq!(m[0]["content"], "Be terse.\n\nCTX");

        let mut m = vec![json!({"role":"user","content":"hi"})];
        inject_system(&mut m, "CTX");
        assert_eq!(m[0], json!({"role":"system","content":"CTX"}));
    }

    #[test]
    fn content_parts() {
        let c = json!([{"type":"text","text":"a"},{"type":"image_url","image_url":{}},{"type":"text","text":"b"}]);
        assert_eq!(content_text(&c), "a\nb");
    }

    #[test]
    fn sse_tap_handles_split_chunks() {
        let stream = "data: {\"choices\":[{\"delta\":{\"role\":\"assistant\"}}]}\n\n\
                      data: {\"choices\":[{\"delta\":{\"content\":\"Hel\"}}]}\n\n\
                      data: {\"choices\":[{\"delta\":{\"content\":\"lo ✓\"}}]}\n\n\
                      data: [DONE]\n\n";
        let mut tap = SseTap::default();
        for chunk in stream.as_bytes().chunks(7) {
            tap.feed(chunk);
        }
        assert_eq!(tap.finish(), "Hello ✓");
    }
}
