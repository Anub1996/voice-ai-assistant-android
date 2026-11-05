//! Chat prompt formatting: the model's own Jinja chat template
//! (`tokenizer.chat_template`), with known-format fallbacks.

use minijinja::{Environment, Error, ErrorKind, Value, context};
use serde::Serialize;

use super::tokenizer::Tokenizer;

#[derive(Debug, Clone, Serialize)]
pub struct Msg {
    pub role: String,
    pub content: String,
}

/// Render the conversation into the model's prompt format, ending with the
/// assistant turn opener.
pub fn render(template: Option<&str>, messages: &[Msg], tok: &Tokenizer) -> String {
    if let Some(t) = template {
        match render_jinja(t, messages, tok) {
            Ok(s) => return s,
            Err(e) => tracing::warn!(error = %e, "chat template failed; using a built-in format"),
        }
    }
    fallback(messages, tok)
}

fn render_jinja(t: &str, messages: &[Msg], tok: &Tokenizer) -> Result<String, Error> {
    let mut env = Environment::new();
    env.set_unknown_method_callback(minijinja_contrib::pycompat::unknown_method_callback);
    env.add_function("raise_exception", |msg: String| -> Result<Value, Error> {
        Err(Error::new(ErrorKind::InvalidOperation, msg))
    });
    env.add_function("strftime_now", |fmt: String| -> String {
        strftime_now(&fmt)
    });
    env.add_template("chat", t)?;
    let name = |id: Option<u32>| id.and_then(|i| tok.token_text(i)).unwrap_or("").to_string();
    env.get_template("chat")?.render(context! {
        messages => Value::from(minijinja::value::Serde(messages)),
        add_generation_prompt => true,
        bos_token => name(tok.bos),
        eos_token => name(tok.eos),
        enable_thinking => false,
        date_string => strftime_now("%d %b %Y"),
        tools => Value::from(()),
    })
}

/// Minimal `strftime` for the formats chat templates use (`%d %b %Y`).
fn strftime_now(fmt: &str) -> String {
    let date = crate::memory::civil_date(crate::memory::store::now_secs());
    let (y, m, d) = (&date[0..4], &date[5..7], &date[8..10]);
    let months = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let mi: usize = m.parse().unwrap_or(1);
    fmt.replace("%d", d)
        .replace("%m", m)
        .replace("%Y", y)
        .replace("%b", months[mi.clamp(1, 12) - 1])
        .replace("%B", months[mi.clamp(1, 12) - 1])
}

fn fallback(messages: &[Msg], tok: &Tokenizer) -> String {
    let has = |t: &str| tok.id_of(t).is_some();
    let mut s = String::new();
    if has("<|im_start|>") {
        for m in messages {
            s += &format!("<|im_start|>{}\n{}<|im_end|>\n", m.role, m.content);
        }
        s += "<|im_start|>assistant\n";
    } else if has("<|start_header_id|>") {
        s += "<|begin_of_text|>";
        for m in messages {
            s += &format!(
                "<|start_header_id|>{}<|end_header_id|>\n\n{}<|eot_id|>",
                m.role,
                m.content.trim()
            );
        }
        s += "<|start_header_id|>assistant<|end_header_id|>\n\n";
    } else if has("<start_of_turn>") {
        let mut system = String::new();
        for m in messages {
            match m.role.as_str() {
                "system" => system = format!("{}\n\n", m.content.trim()),
                "assistant" => {
                    s += &format!("<start_of_turn>model\n{}<end_of_turn>\n", m.content.trim())
                }
                _ => {
                    s += &format!(
                        "<start_of_turn>user\n{}{}<end_of_turn>\n",
                        system,
                        m.content.trim()
                    );
                    system.clear();
                }
            }
        }
        s += "<start_of_turn>model\n";
    } else if has("<|user|>") && has("<|assistant|>") {
        for m in messages {
            s += &format!("<|{}|>\n{}<|end|>\n", m.role, m.content);
        }
        s += "<|assistant|>\n";
    } else {
        for m in messages {
            let who = match m.role.as_str() {
                "system" => "System",
                "assistant" => "Assistant",
                _ => "User",
            };
            s += &format!("{who}: {}\n", m.content.trim());
        }
        s += "Assistant:";
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lite::tokenizer::tests::bpe_metadata;

    fn msgs() -> Vec<Msg> {
        vec![
            Msg {
                role: "system".into(),
                content: "Be brief.".into(),
            },
            Msg {
                role: "user".into(),
                content: "Hi".into(),
            },
        ]
    }

    /// Qwen2.5-style template (uses `.startswith`-free subset plus loop vars).
    const CHATML: &str = "{%- for message in messages %}{{- '<|im_start|>' + message['role'] + '\\n' + message['content'].strip() + '<|im_end|>\\n' }}{%- endfor %}{%- if add_generation_prompt %}{{- '<|im_start|>assistant\\n' }}{%- endif %}";

    #[test]
    fn renders_model_template_with_pycompat() {
        let tok = Tokenizer::from_gguf(&bpe_metadata("qwen2")).unwrap();
        let out = render(Some(CHATML), &msgs(), &tok);
        assert_eq!(
            out,
            "<|im_start|>system\nBe brief.<|im_end|>\n<|im_start|>user\nHi<|im_end|>\n<|im_start|>assistant\n"
        );
    }

    #[test]
    fn broken_template_falls_back_to_chatml() {
        let tok = Tokenizer::from_gguf(&bpe_metadata("qwen2")).unwrap();
        let out = render(Some("{{ raise_exception('nope') }}"), &msgs(), &tok);
        assert!(out.ends_with("<|im_start|>assistant\n"));
        assert!(out.contains("<|im_start|>user\nHi<|im_end|>"));
    }

    #[test]
    fn strftime_formats() {
        let s = strftime_now("%d %b %Y");
        assert_eq!(s.len(), 11, "{s}");
    }
}
