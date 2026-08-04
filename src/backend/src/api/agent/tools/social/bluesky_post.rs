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

/// The real Bluesky PDS host, used unless a test overrides it.
const BLUESKY_BASE_URL: &str = "https://bsky.social";

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
async fn resolve_handle(client: &Client, base_url: &str, handle: &str) -> Result<String> {
    let response = client
        .get(format!(
            "{}/xrpc/com.atproto.identity.resolveHandle",
            base_url
        ))
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
async fn build_facets(client: &Client, base_url: &str, text: &str) -> Vec<serde_json::Value> {
    let (mut facets, occupied) = build_link_and_tag_facets(text);

    for (start, end, handle) in find_mention_candidates(text, &occupied) {
        match resolve_handle(client, base_url, &handle).await {
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
    /// PDS host to talk to. Always the real Bluesky one in production; tests point
    /// it at a loopback mock instead.
    base_url: String,
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
            base_url: BLUESKY_BASE_URL.to_string(),
        }
    }

    /// A tool with canned credentials pointed at `base_url` instead of the real
    /// Bluesky PDS, so the session/post/mention-resolution flow can be driven
    /// without the network and without `BLUESKY_HANDLE`/`BLUESKY_PASSWORD`.
    #[cfg(test)]
    pub(crate) fn with_base_url(
        base_url: impl Into<String>,
        handle: Option<&str>,
        password: Option<&str>,
    ) -> Self {
        Self {
            handle: handle.map(|h| h.to_string()),
            password: password.map(|p| p.to_string()),
            base_url: base_url.into(),
            ..Self::new()
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
            .post(format!(
                "{}/xrpc/com.atproto.server.createSession",
                self.base_url
            ))
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

        let facets = build_facets(&client, &self.base_url, text).await;
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
            .post(format!(
                "{}/xrpc/com.atproto.repo.createRecord",
                self.base_url
            ))
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

    use crate::api::agent::core::types::FunctionCall;
    use crate::test_support::{MockHttpApi, MockResponse};

    const CREATE_SESSION: &str = "/xrpc/com.atproto.server.createSession";
    const RESOLVE_HANDLE: &str = "/xrpc/com.atproto.identity.resolveHandle";
    const CREATE_RECORD: &str = "/xrpc/com.atproto.repo.createRecord";

    fn tool_call(arguments: &str) -> ToolCall {
        ToolCall {
            id: "call_bsky".to_string(),
            tool_type: "function".to_string(),
            function: FunctionCall {
                name: "bluesky_post".to_string(),
                arguments: arguments.to_string(),
            },
        }
    }

    fn post_call(text: &str) -> ToolCall {
        tool_call(&json!({"text": text}).to_string())
    }

    fn tool_for(api: &MockHttpApi) -> BlueskyPostTool {
        BlueskyPostTool::with_base_url(api.base_url(), Some("me.bsky.social"), Some("app-password"))
    }

    /// A mock PDS that authenticates, resolves handles, and accepts records.
    async fn healthy_pds() -> MockHttpApi {
        let api = MockHttpApi::start().await;
        api.on(
            "POST",
            CREATE_SESSION,
            MockResponse::json(json!({"accessJwt": "jwt-123", "did": "did:plc:me"})),
        );
        api.on(
            "GET",
            RESOLVE_HANDLE,
            MockResponse::json(json!({"did": "did:plc:alice"})),
        );
        api.on(
            "POST",
            CREATE_RECORD,
            MockResponse::json(
                json!({"uri": "at://did:plc:me/app.bsky.feed.post/1", "cid": "bafy"}),
            ),
        );
        api
    }

    #[test]
    fn metadata_and_function_definition_describe_the_bluesky_tool() {
        let tool = BlueskyPostTool::new();
        assert_eq!(tool.metadata().id, "bluesky_post");
        assert_eq!(tool.metadata().category, ToolCategory::Social);
        assert_eq!(tool.metadata().tool_type, ToolType::BlueskyPost);

        let def = tool.get_function_definition();
        assert_eq!(def["name"], "bluesky_post");
        assert_eq!(def["parameters"]["required"], json!(["text"]));
    }

    #[tokio::test]
    async fn a_post_authenticates_then_creates_a_record_with_sorted_facets() {
        let api = healthy_pds().await;
        let text = "hi @alice.bsky.social see https://example.com/x #rust";

        let result = tool_for(&api)
            .execute(&post_call(text))
            .await
            .expect("The post should succeed");

        let requests = api.requests();
        assert_eq!(requests.len(), 3, "{:?}", requests);

        // 1. Session, with the configured identifier and password.
        assert_eq!(requests[0].method, "POST");
        assert_eq!(requests[0].path, CREATE_SESSION);
        assert_eq!(
            requests[0].json(),
            json!({"identifier": "me.bsky.social", "password": "app-password"})
        );

        // 2. One handle resolution for the single mention.
        assert_eq!(requests[1].method, "GET");
        assert_eq!(requests[1].path, RESOLVE_HANDLE);
        assert_eq!(
            requests[1].query_param("handle").as_deref(),
            Some("alice.bsky.social")
        );

        // 3. The record itself, bearing the session token.
        assert_eq!(requests[2].method, "POST");
        assert_eq!(requests[2].path, CREATE_RECORD);
        assert_eq!(requests[2].header("authorization"), Some("Bearer jwt-123"));
        let body = requests[2].json();
        assert_eq!(body["repo"], "did:plc:me");
        assert_eq!(body["collection"], "app.bsky.feed.post");
        assert_eq!(body["record"]["$type"], "app.bsky.feed.post");
        assert_eq!(body["record"]["text"], text);
        let created_at = body["record"]["createdAt"].as_str().expect("a createdAt");
        assert!(created_at.ends_with('Z'), "{}", created_at);
        assert!(
            chrono::DateTime::parse_from_rfc3339(created_at).is_ok(),
            "createdAt must be RFC3339: {}",
            created_at
        );

        // Facets are ordered by byteStart regardless of detection order, and each
        // index really does slice the matching substring out of the text.
        let facets = body["record"]["facets"].as_array().expect("facets");
        assert_eq!(facets.len(), 3, "{:#?}", facets);
        let kinds: Vec<&str> = facets
            .iter()
            .map(|facet| facet["features"][0]["$type"].as_str().unwrap())
            .collect();
        assert_eq!(
            kinds,
            vec![
                "app.bsky.richtext.facet#mention",
                "app.bsky.richtext.facet#link",
                "app.bsky.richtext.facet#tag",
            ]
        );
        assert_eq!(facets[0]["features"][0]["did"], "did:plc:alice");
        assert_eq!(facets[1]["features"][0]["uri"], "https://example.com/x");
        assert_eq!(facets[2]["features"][0]["tag"], "rust");
        for facet in facets {
            let start = facet["index"]["byteStart"].as_u64().unwrap() as usize;
            let end = facet["index"]["byteEnd"].as_u64().unwrap() as usize;
            assert!(start < end && end <= text.len(), "{:?}", facet);
        }

        assert_eq!(result.tool_name, "bluesky_post");
        assert!(result.tool_call_id.is_none());
        assert_eq!(
            result.result,
            format!("Successfully posted to Bluesky: '{}'", text)
        );
        api.stop().await;
    }

    #[tokio::test]
    async fn plain_text_carries_no_facets_key_and_resolves_nothing() {
        let api = healthy_pds().await;

        tool_for(&api)
            .execute(&post_call("just a plain sentence"))
            .await
            .expect("The post should succeed");

        let requests = api.requests();
        assert_eq!(requests.len(), 2, "No handle should be resolved");
        let body = requests[1].json();
        assert!(
            body["record"].get("facets").is_none(),
            "An empty facet list must be omitted entirely: {}",
            body
        );
        api.stop().await;
    }

    #[tokio::test]
    async fn an_unresolvable_mention_is_dropped_instead_of_failing_the_post() {
        let api = MockHttpApi::start().await;
        api.on(
            "POST",
            CREATE_SESSION,
            MockResponse::json(json!({"accessJwt": "jwt-123", "did": "did:plc:me"})),
        );
        api.on(
            "GET",
            RESOLVE_HANDLE,
            MockResponse::error(
                400,
                r#"{"error":"InvalidRequest","message":"Unable to resolve handle"}"#,
            ),
        );
        api.on(
            "POST",
            CREATE_RECORD,
            MockResponse::json(json!({"uri": "at://x"})),
        );

        let result = tool_for(&api)
            .execute(&post_call("hello @ghost.invalid.example"))
            .await
            .expect("A typo'd mention must not fail the whole post");

        let body = api.requests()[2].json();
        assert!(
            body["record"].get("facets").is_none(),
            "The unresolved mention must not become a facet: {}",
            body
        );
        assert!(result.result.contains("Successfully posted to Bluesky"));
        api.stop().await;
    }

    #[tokio::test]
    async fn a_handle_resolution_without_a_did_is_also_dropped() {
        let api = MockHttpApi::start().await;
        api.on(
            "POST",
            CREATE_SESSION,
            MockResponse::json(json!({"accessJwt": "jwt-123", "did": "did:plc:me"})),
        );
        api.on("GET", RESOLVE_HANDLE, MockResponse::json(json!({})));
        api.on(
            "POST",
            CREATE_RECORD,
            MockResponse::json(json!({"uri": "at://x"})),
        );

        tool_for(&api)
            .execute(&post_call("hi @alice.bsky.social"))
            .await
            .expect("A did-less resolution must not fail the post");

        let body = api.requests()[2].json();
        assert!(body["record"].get("facets").is_none(), "{}", body);
        api.stop().await;
    }

    #[tokio::test]
    async fn a_failed_authentication_is_reported_with_its_body() {
        let api = MockHttpApi::serving(
            "POST",
            CREATE_SESSION,
            MockResponse::error(
                401,
                r#"{"error":"AuthenticationRequired","message":"Invalid identifier or password"}"#,
            ),
        )
        .await;

        let error = tool_for(&api)
            .execute(&post_call("hello"))
            .await
            .expect_err("A 401 must fail the call");

        let message = error.to_string();
        assert!(
            message.starts_with("Failed to authenticate with Bluesky:"),
            "{}",
            message
        );
        assert!(
            message.contains("Invalid identifier or password"),
            "{}",
            message
        );
        assert_eq!(api.call_count(), 1, "The post must not be attempted");
        api.stop().await;
    }

    #[tokio::test]
    async fn a_session_missing_its_token_or_did_is_rejected() {
        let api = MockHttpApi::start().await;
        api.on_sequence(
            "POST",
            CREATE_SESSION,
            vec![
                MockResponse::json(json!({"did": "did:plc:me"})),
                MockResponse::json(json!({"accessJwt": "jwt-123"})),
            ],
        );
        let tool = tool_for(&api);

        assert_eq!(
            tool.execute(&post_call("hello"))
                .await
                .expect_err("A session without accessJwt must fail")
                .to_string(),
            "Invalid session response: missing accessJwt"
        );
        assert_eq!(
            tool.execute(&post_call("hello"))
                .await
                .expect_err("A session without did must fail")
                .to_string(),
            "Invalid session response: missing did"
        );
        api.stop().await;
    }

    #[tokio::test]
    async fn a_rejected_record_is_reported_with_its_body() {
        let api = MockHttpApi::start().await;
        api.on(
            "POST",
            CREATE_SESSION,
            MockResponse::json(json!({"accessJwt": "jwt-123", "did": "did:plc:me"})),
        );
        api.on(
            "POST",
            CREATE_RECORD,
            MockResponse::error(
                400,
                r#"{"error":"InvalidRequest","message":"Record too long"}"#,
            ),
        );

        let error = tool_for(&api)
            .execute(&post_call("hello"))
            .await
            .expect_err("A rejected record must fail the call");

        let message = error.to_string();
        assert!(
            message.starts_with("Failed to post to Bluesky:"),
            "{}",
            message
        );
        assert!(message.contains("Record too long"), "{}", message);
        api.stop().await;
    }

    #[tokio::test]
    async fn missing_credentials_fail_before_any_request() {
        let api = healthy_pds().await;

        let no_handle = BlueskyPostTool::with_base_url(api.base_url(), None, Some("app-password"));
        assert_eq!(
            no_handle
                .execute(&post_call("hello"))
                .await
                .expect_err("Without a handle the call must fail")
                .to_string(),
            "BLUESKY_HANDLE environment variable not set"
        );

        let no_password =
            BlueskyPostTool::with_base_url(api.base_url(), Some("me.bsky.social"), None);
        assert_eq!(
            no_password
                .execute(&post_call("hello"))
                .await
                .expect_err("Without a password the call must fail")
                .to_string(),
            "BLUESKY_PASSWORD environment variable not set"
        );

        assert_eq!(api.call_count(), 0, "Nothing should have reached the PDS");
        api.stop().await;
    }

    #[tokio::test]
    async fn over_long_text_and_bad_arguments_fail_before_any_request() {
        let api = healthy_pds().await;
        let tool = tool_for(&api);

        // 301 characters, counted in chars rather than bytes.
        assert_eq!(
            tool.execute(&post_call(&"é".repeat(301)))
                .await
                .expect_err("A 301-character post must be refused")
                .to_string(),
            "Post text exceeds Bluesky's 300 character limit"
        );
        assert_eq!(
            tool.execute(&tool_call("<not json>"))
                .await
                .expect_err("Unparseable arguments must fail")
                .to_string(),
            "Failed to parse tool call arguments"
        );
        assert_eq!(
            tool.execute(&tool_call(r#"{"message": "wrong field"}"#))
                .await
                .expect_err("A missing text must fail")
                .to_string(),
            "Missing required parameter: text"
        );

        assert_eq!(api.call_count(), 0, "Nothing should have reached the PDS");
        api.stop().await;
    }

    #[tokio::test]
    async fn exactly_three_hundred_characters_is_accepted() {
        let api = healthy_pds().await;
        // 300 multi-byte characters is 600 bytes: the limit is on chars.
        let text = "é".repeat(300);

        tool_for(&api)
            .execute(&post_call(&text))
            .await
            .expect("A 300-character post must be accepted");

        let body = api.requests()[1].json();
        assert_eq!(body["record"]["text"], text);
        api.stop().await;
    }

    #[test]
    fn strips_unbalanced_trailing_brackets_and_braces() {
        // The bracket rules mirror the paren rule: only trim when the match
        // itself leaves the bracket unclosed.
        assert_eq!(
            trim_trailing_punctuation("https://example.com/a]"),
            "https://example.com/a"
        );
        assert_eq!(
            trim_trailing_punctuation("https://example.com/a}"),
            "https://example.com/a"
        );
        assert_eq!(
            trim_trailing_punctuation("https://example.com/[a]"),
            "https://example.com/[a]"
        );
        assert_eq!(
            trim_trailing_punctuation("https://example.com/{a}"),
            "https://example.com/{a}"
        );
    }

    #[test]
    fn a_bare_hash_is_not_a_hashtag() {
        // After trimming, "#." is one character long and cannot be a tag.
        let (facets, occupied) = build_link_and_tag_facets("a #. b");
        assert!(facets.is_empty(), "{:?}", facets);
        assert!(occupied.is_empty());
    }

    #[test]
    fn a_mention_inside_a_link_is_not_treated_as_a_mention() {
        // The "@handle" inside the URL must not trigger a handle lookup: it is
        // not whitespace-preceded, and the bytes it occupies are in any case
        // already claimed by the link facet.
        let text = "profile at https://example.com/@alice.bsky.social today";
        let (facets, occupied) = build_link_and_tag_facets(text);
        assert_eq!(facets.len(), 1);
        assert!(find_mention_candidates(text, &occupied).is_empty());
    }
}
