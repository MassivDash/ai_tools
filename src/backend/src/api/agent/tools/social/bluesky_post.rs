use crate::api::agent::core::types::{ToolCall, ToolCallResult, ToolType};
use crate::api::agent::tools::framework::agent_tool::{AgentTool, ToolCategory, ToolMetadata};
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use regex::Regex;
use reqwest::Client;
use serde_json::json;
use std::env;
use std::sync::LazyLock;

static URL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:^|\s)(https?://[^\s]+)").unwrap());
static TAG_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?:^|\s)(#[^\s#]+)").unwrap());
static MENTION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:^|\s)(@[a-zA-Z0-9](?:[a-zA-Z0-9-]*[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]*[a-zA-Z0-9])?)+)")
        .unwrap()
});

/// Strip trailing punctuation that's likely sentence formatting rather than
/// part of a URL or hashtag (e.g. "check #rust." -> "#rust"). Closing
/// brackets are only trimmed when unbalanced within the match, so URLs that
/// legitimately end in a paren (e.g. Wikipedia's `..._(disambiguation)`)
/// are left intact.
fn trim_trailing_punctuation(s: &str) -> &str {
    let mut end = s.len();
    while end > 0 {
        let ch = match s[..end].chars().next_back() {
            Some(c) => c,
            None => break,
        };
        let should_trim = match ch {
            '.' | ',' | ';' | ':' | '!' | '?' | '\'' | '"' => true,
            ')' => count_matches(&s[..end], '(') < count_matches(&s[..end], ')'),
            ']' => count_matches(&s[..end], '[') < count_matches(&s[..end], ']'),
            '}' => count_matches(&s[..end], '{') < count_matches(&s[..end], '}'),
            _ => false,
        };
        if !should_trim {
            break;
        }
        end -= ch.len_utf8();
    }
    &s[..end]
}

fn count_matches(s: &str, c: char) -> usize {
    s.matches(c).count()
}

/// A facet paired with its byte start, kept alongside for sorting once
/// mentions (resolved separately, async) are merged in.
type PositionedFacet = (usize, serde_json::Value);
/// A byte range in the post text (start, end) already claimed by a facet.
type ByteRange = (usize, usize);

/// Scan post text for URLs and #hashtags and build facets for them. Byte
/// offsets (not char offsets) are required by the app.bsky.richtext.facet
/// spec; regex match positions on a &str are already UTF-8 byte indices, so
/// no extra conversion is needed. Also returns the byte ranges consumed, so
/// mention detection (which needs an extra async lookup) can avoid them.
fn build_link_and_tag_facets(text: &str) -> (Vec<PositionedFacet>, Vec<ByteRange>) {
    let mut occupied: Vec<ByteRange> = Vec::new();
    let mut facets: Vec<PositionedFacet> = Vec::new();

    for caps in URL_RE.captures_iter(text) {
        let m = caps.get(1).unwrap();
        let trimmed = trim_trailing_punctuation(m.as_str());
        if trimmed.is_empty() {
            continue;
        }
        let start = m.start();
        let end = start + trimmed.len();
        occupied.push((start, end));
        facets.push((
            start,
            json!({
                "index": {"byteStart": start, "byteEnd": end},
                "features": [{"$type": "app.bsky.richtext.facet#link", "uri": trimmed}]
            }),
        ));
    }

    for caps in TAG_RE.captures_iter(text) {
        let m = caps.get(1).unwrap();
        let start = m.start();
        if occupied.iter().any(|&(s, e)| start >= s && start < e) {
            continue;
        }
        let trimmed = trim_trailing_punctuation(m.as_str());
        if trimmed.len() <= 1 {
            continue;
        }
        let end = start + trimmed.len();
        let tag = &trimmed[1..];
        occupied.push((start, end));
        facets.push((
            start,
            json!({
                "index": {"byteStart": start, "byteEnd": end},
                "features": [{"$type": "app.bsky.richtext.facet#tag", "tag": tag}]
            }),
        ));
    }

    (facets, occupied)
}

/// Find `@handle.domain`-style mentions, skipping any byte range already
/// claimed by a link or hashtag. Only returns candidates (start, end, handle)
/// — resolving each handle to a DID requires a network call, done separately.
fn find_mention_candidates(text: &str, occupied: &[ByteRange]) -> Vec<(usize, usize, String)> {
    let mut mentions = Vec::new();
    for caps in MENTION_RE.captures_iter(text) {
        let m = caps.get(1).unwrap();
        let start = m.start();
        if occupied.iter().any(|&(s, e)| start >= s && start < e) {
            continue;
        }
        let raw = m.as_str();
        let end = start + raw.len();
        mentions.push((start, end, raw[1..].to_string()));
    }
    mentions
}

/// Resolve a Bluesky handle (without the leading '@') to its DID via the
/// public com.atproto.identity.resolveHandle endpoint.
async fn resolve_handle(client: &Client, handle: &str) -> Result<String> {
    let response = client
        .get("https://bsky.social/xrpc/com.atproto.identity.resolveHandle")
        .query(&[("handle", handle)])
        .send()
        .await
        .with_context(|| format!("Failed to call resolveHandle for @{}", handle))?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!(
            "Could not resolve mention @{}: {}",
            handle,
            error_text
        ));
    }

    let data: serde_json::Value = response.json().await?;
    data["did"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("resolveHandle response for @{} missing did", handle))
}

/// Build the full `facets` array (links, hashtags, and mentions) for a post.
/// Mentions that fail to resolve (typo'd or nonexistent handle) are dropped
/// rather than failing the whole post.
async fn build_facets(client: &Client, text: &str) -> Vec<serde_json::Value> {
    let (mut facets, occupied) = build_link_and_tag_facets(text);

    for (start, end, handle) in find_mention_candidates(text, &occupied) {
        match resolve_handle(client, &handle).await {
            Ok(did) => facets.push((
                start,
                json!({
                    "index": {"byteStart": start, "byteEnd": end},
                    "features": [{"$type": "app.bsky.richtext.facet#mention", "did": did}]
                }),
            )),
            Err(e) => eprintln!("⚠️  {}", e),
        }
    }

    facets.sort_by_key(|(start, _)| *start);
    facets.into_iter().map(|(_, facet)| facet).collect()
}

/// Bluesky Post Tool implementation
/// Allows the agent to post to Bluesky
pub struct BlueskyPostTool {
    metadata: ToolMetadata,
    handle: Option<String>,
    password: Option<String>,
}

impl BlueskyPostTool {
    /// Create a new Bluesky tool
    pub fn new() -> Self {
        let handle = env::var("BLUESKY_HANDLE").ok();
        let password = env::var("BLUESKY_PASSWORD").ok();

        Self {
            metadata: ToolMetadata {
                id: "bluesky_post".to_string(),
                name: "Bluesky Post".to_string(),
                description: "Post a message to the user's Bluesky account (Max 300 characters)"
                    .to_string(),
                category: ToolCategory::Social,
                tool_type: ToolType::BlueskyPost,
            },
            handle,
            password,
        }
    }

    /// Authenticate and post to Bluesky
    async fn post_to_bluesky(&self, text: &str) -> Result<String> {
        let handle = self
            .handle
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("BLUESKY_HANDLE environment variable not set"))?;
        let password = self
            .password
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("BLUESKY_PASSWORD environment variable not set"))?;

        let client = Client::new();

        // 1. Create session
        let session_response = client
            .post("https://bsky.social/xrpc/com.atproto.server.createSession")
            .json(&json!({
                "identifier": handle,
                "password": password
            }))
            .send()
            .await
            .context("Failed to connect to Bluesky API to create session")?;

        if !session_response.status().is_success() {
            let error_text = session_response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Failed to authenticate with Bluesky: {}",
                error_text
            ));
        }

        let session_data: serde_json::Value = session_response.json().await?;
        let access_token = session_data["accessJwt"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid session response: missing accessJwt"))?;
        let did = session_data["did"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid session response: missing did"))?;

        // 2. Create post record
        let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        let facets = build_facets(&client, text).await;
        let mut post_record = json!({
            "$type": "app.bsky.feed.post",
            "text": text,
            "createdAt": now
        });
        if !facets.is_empty() {
            post_record["facets"] = json!(facets);
        }

        let record = json!({
            "repo": did,
            "collection": "app.bsky.feed.post",
            "record": post_record
        });

        let post_response = client
            .post("https://bsky.social/xrpc/com.atproto.repo.createRecord")
            .header("Authorization", format!("Bearer {}", access_token))
            .json(&record)
            .send()
            .await
            .context("Failed to post record to Bluesky")?;

        if !post_response.status().is_success() {
            let error_text = post_response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Failed to post to Bluesky: {}", error_text));
        }

        Ok(format!("Successfully posted to Bluesky: '{}'", text))
    }
}

#[async_trait]
impl AgentTool for BlueskyPostTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn get_function_definition(&self) -> serde_json::Value {
        json!({
            "name": "bluesky_post",
            "description": "Post a text message to Bluesky. IMPORTANT: The text MUST be 300 characters or less. Do not exceed this limit.",
            "parameters": {
                "type": "object",
                "properties": {
                    "text": {
                        "type": "string",
                        "description": "The text content of the post to create. STRICT LIMIT: Maximum 300 characters."
                    }
                },
                "required": ["text"]
            }
        })
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolCallResult> {
        let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
            .context("Failed to parse tool call arguments")?;

        let text = args
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: text"))?;

        if text.chars().count() > 300 {
            return Err(anyhow::anyhow!(
                "Post text exceeds Bluesky's 300 character limit"
            ));
        }

        println!("🦋 Posting to Bluesky...");
        let result = self.post_to_bluesky(text).await?;
        println!("✅ {}", result);

        Ok(ToolCallResult {
            tool_call_id: None,
            tool_name: "bluesky_post".to_string(),
            result,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_balanced_trailing_paren_in_url() {
        // Regression: a Wikipedia-style URL ending in a matched paren must
        // not be truncated by sentence-punctuation trimming.
        let url = "https://en.wikipedia.org/wiki/Rust_(programming_language)";
        assert_eq!(trim_trailing_punctuation(url), url);
    }

    #[test]
    fn strips_unbalanced_trailing_paren() {
        // "(" is not part of the captured match, so the trailing ")" is unbalanced.
        let captured = "https://example.com/foo)";
        assert_eq!(
            trim_trailing_punctuation(captured),
            "https://example.com/foo"
        );
    }

    #[test]
    fn strips_sentence_punctuation() {
        assert_eq!(
            trim_trailing_punctuation("https://example.com."),
            "https://example.com"
        );
        assert_eq!(trim_trailing_punctuation("#rustlang,"), "#rustlang");
    }

    #[test]
    fn builds_link_facet_with_correct_byte_offsets() {
        let text = "Check out https://example.com/path?q=1, great stuff";
        let (facets, occupied) = build_link_and_tag_facets(text);
        assert_eq!(facets.len(), 1);
        assert_eq!(occupied.len(), 1);
        let (start, facet) = &facets[0];
        let byte_start = facet["index"]["byteStart"].as_u64().unwrap() as usize;
        let byte_end = facet["index"]["byteEnd"].as_u64().unwrap() as usize;
        assert_eq!(*start, byte_start);
        assert_eq!(occupied[0], (byte_start, byte_end));
        // The trailing comma from the sentence must not be part of the URI.
        assert_eq!(&text[byte_start..byte_end], "https://example.com/path?q=1");
        assert_eq!(facet["features"][0]["uri"], "https://example.com/path?q=1");
    }

    #[test]
    fn builds_tag_facet_with_multibyte_text() {
        let text = "café #café über";
        let (facets, _) = build_link_and_tag_facets(text);
        assert_eq!(facets.len(), 1);
        let (_, facet) = &facets[0];
        assert_eq!(facet["features"][0]["tag"], "café");
        let byte_start = facet["index"]["byteStart"].as_u64().unwrap() as usize;
        let byte_end = facet["index"]["byteEnd"].as_u64().unwrap() as usize;
        // The '#' must line up on a real UTF-8 boundary despite the preceding
        // multibyte "café " prefix, and the slice must be exactly "#café".
        assert_eq!(&text[byte_start..byte_end], "#café");
    }

    #[test]
    fn does_not_double_count_hashtag_inside_url_fragment() {
        let text = "See https://example.com/#section for details";
        let (facets, _) = build_link_and_tag_facets(text);
        assert_eq!(facets.len(), 1);
        assert_eq!(
            facets[0].1["features"][0]["$type"],
            "app.bsky.richtext.facet#link"
        );
    }

    #[test]
    fn finds_mention_candidate_outside_occupied_ranges() {
        let text = "ping @alice.bsky.social about this";
        let (_, occupied) = build_link_and_tag_facets(text);
        let mentions = find_mention_candidates(text, &occupied);
        assert_eq!(mentions.len(), 1);
        assert_eq!(mentions[0].2, "alice.bsky.social");
    }
}
