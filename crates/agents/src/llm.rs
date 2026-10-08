//! Real LLM backend behind the [`LlmProvider`] seam, gated behind the
//! `llm` feature so the default build stays dependency-free and offline.
//!
//! The provider shells out to `curl` (found on virtually every system)
//! rather than pulling in an HTTP client + TLS stack. Configure it with:
//!
//! - `SIMULACRA_LLM_ENDPOINT` (default `https://api.openai.com/v1/chat/completions`)
//! - `SIMULACRA_LLM_MODEL` (default `gpt-4o-mini`)
//! - `SIMULACRA_LLM_API_KEY` (required; the provider returns `None` without it)

use std::process::Command;

use crate::LlmProvider;

/// An OpenAI-compatible chat-completions backend invoked through `curl`.
#[derive(Debug, Clone)]
pub struct CurlProvider {
    endpoint: String,
    model: String,
    api_key: Option<String>,
}

impl Default for CurlProvider {
    fn default() -> Self {
        CurlProvider {
            endpoint: std::env::var("SIMULACRA_LLM_ENDPOINT")
                .unwrap_or_else(|_| "https://api.openai.com/v1/chat/completions".into()),
            model: std::env::var("SIMULACRA_LLM_MODEL").unwrap_or_else(|_| "gpt-4o-mini".into()),
            api_key: std::env::var("SIMULACRA_LLM_API_KEY").ok(),
        }
    }
}

impl CurlProvider {
    /// Build a provider from explicit settings instead of the environment.
    pub fn with_settings(endpoint: impl Into<String>, model: impl Into<String>, api_key: impl Into<String>) -> Self {
        CurlProvider {
            endpoint: endpoint.into(),
            model: model.into(),
            api_key: Some(api_key.into()),
        }
    }
}

impl LlmProvider for CurlProvider {
    fn complete(&self, prompt: &str) -> Option<String> {
        let key = self.api_key.as_ref()?;
        // The prompt is embedded as a JSON string; escape what matters.
        let escaped = prompt.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n");
        let body = format!(
            "{{\"model\":\"{}\",\"messages\":[{{\"role\":\"user\",\"content\":\"{}\"}}]}}",
            self.model, escaped
        );
        let output = Command::new("curl")
            .args(["-sS", "--max-time", "60", "-X", "POST"])
            .arg(&self.endpoint)
            .arg("-H")
            .arg(format!("Authorization: Bearer {key}"))
            .arg("-H")
            .arg("Content-Type: application/json")
            .arg("-d")
            .arg(&body)
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let text = String::from_utf8(output.stdout).ok()?;
        extract_content(&text)
    }
}

/// Crudely extract `"content":"..."` from the first choice of the response.
fn extract_content(json: &str) -> Option<String> {
    let idx = json.find("\"content\"")?;
    let rest = &json[idx..];
    let start = rest.find(':')? + 1;
    let rest = rest[start..].trim_start();
    let quote = rest.find('"')?;
    let rest = &rest[quote + 1..];
    let mut out = String::new();
    let mut chars = rest.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next()? {
                'n' => out.push('\n'),
                't' => out.push('\t'),
                'r' => out.push('\r'),
                other => out.push(other),
            },
            '"' => return Some(out),
            other => out.push(other),
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_without_key_returns_none() {
        let p = CurlProvider::with_settings("http://localhost:9/x", "m", " ");
        // A live key would still try the network; an unreachable endpoint
        // must yield None, never panic.
        let mut no_key = p.clone();
        no_key.api_key = None;
        assert!(no_key.complete("hello").is_none());
    }

    #[test]
    fn extract_content_pulls_first_choice() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"hi \"there\"\nfriend"}}]}"#;
        assert_eq!(extract_content(json).as_deref(), Some("hi \"there\"\nfriend"));
    }

    #[test]
    fn extract_content_handles_missing_field() {
        assert!(extract_content("{}").is_none());
    }
}
