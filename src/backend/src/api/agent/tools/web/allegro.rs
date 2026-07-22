use crate::api::agent::core::types::{ToolCall, ToolCallResult, ToolType};
use crate::api::agent::tools::framework::agent_tool::{AgentTool, ToolCategory, ToolMetadata};
use anyhow::{Context, Result};
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};

const TOKEN_URL: &str = "https://allegro.pl/auth/oauth/token";
const API_BASE: &str = "https://api.allegro.pl";
const ACCEPT_HEADER: &str = "application/vnd.allegro.public.v1+json";

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
}

/// A single offer returned by Allegro's `/offers/listing` endpoint
#[derive(Debug, Clone, PartialEq)]
struct AllegroOffer {
    name: String,
    price: Option<String>,
    currency: Option<String>,
    url: Option<String>,
}

impl AllegroOffer {
    fn from_json(item: &serde_json::Value) -> Self {
        let name = item
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown offer")
            .to_string();

        let price = item
            .pointer("/sellingMode/price/amount")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let currency = item
            .pointer("/sellingMode/price/currency")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // The listing endpoint only returns the offer id, not a direct URL.
        let url = item
            .get("id")
            .and_then(|v| v.as_str())
            .map(|id| format!("https://allegro.pl/oferta/{}", id));

        Self {
            name,
            price,
            currency,
            url,
        }
    }

    fn price_value(&self) -> Option<f64> {
        self.price.as_ref().and_then(|p| p.parse::<f64>().ok())
    }

    fn format(&self, index: usize) -> String {
        let price = match (&self.price, &self.currency) {
            (Some(p), Some(c)) => format!("{} {}", p, c),
            (Some(p), None) => p.clone(),
            _ => "price unavailable".to_string(),
        };
        let url = self.url.as_deref().unwrap_or("no link");
        format!("{}. **{}** — {}\n   {}", index, self.name, price, url)
    }
}

/// Shared OAuth2 (client-credentials) client for Allegro's public REST API.
/// Client-credentials tokens are enough for read-only browsing endpoints like
/// `/offers/listing`, but that endpoint is restricted to "verified applications" —
/// a status Allegro grants manually after reviewing a registered app.
pub struct AllegroClient {
    client_id: String,
    client_secret: String,
    http: reqwest::Client,
    token: RwLock<Option<(String, Instant)>>,
}

impl AllegroClient {
    pub fn from_env() -> Option<Self> {
        let client_id = env::var("ALLEGRO_CLIENT_ID")
            .ok()
            .filter(|s| !s.is_empty())?;
        let client_secret = env::var("ALLEGRO_CLIENT_SECRET")
            .ok()
            .filter(|s| !s.is_empty())?;

        Some(Self {
            client_id,
            client_secret,
            http: reqwest::Client::new(),
            token: RwLock::new(None),
        })
    }

    async fn access_token(&self) -> Result<String> {
        {
            let cache = self.token.read().await;
            if let Some((token, expires_at)) = &*cache {
                if Instant::now() < *expires_at {
                    return Ok(token.clone());
                }
            }
        }

        let credentials = STANDARD.encode(format!("{}:{}", self.client_id, self.client_secret));
        let res = self
            .http
            .post(TOKEN_URL)
            .header("Authorization", format!("Basic {}", credentials))
            .query(&[("grant_type", "client_credentials")])
            .send()
            .await
            .context("Failed to request Allegro OAuth token")?;

        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Allegro token request failed with {}: {}",
                status,
                text
            ));
        }

        let token_data: TokenResponse = res
            .json()
            .await
            .context("Failed to parse Allegro token response")?;

        let expires_at =
            Instant::now() + Duration::from_secs(token_data.expires_in.saturating_sub(60));

        let mut cache = self.token.write().await;
        *cache = Some((token_data.access_token.clone(), expires_at));

        Ok(token_data.access_token)
    }

    /// Search Allegro offer listings for a phrase, optionally bounded by price.
    /// `sort` accepts Allegro's raw sort values, e.g. "+price", "-price", "-relevance".
    async fn search(
        &self,
        phrase: &str,
        min_price: Option<f64>,
        max_price: Option<f64>,
        limit: u64,
        sort: Option<&str>,
    ) -> Result<Vec<AllegroOffer>> {
        let token = self.access_token().await?;

        let mut query: Vec<(String, String)> = vec![
            ("phrase".to_string(), phrase.to_string()),
            ("limit".to_string(), limit.to_string()),
        ];
        if let Some(min) = min_price {
            query.push(("price.from".to_string(), format!("{:.2}", min)));
        }
        if let Some(max) = max_price {
            query.push(("price.to".to_string(), format!("{:.2}", max)));
        }
        if let Some(sort) = sort {
            query.push(("sort".to_string(), sort.to_string()));
        }

        let res = self
            .http
            .get(format!("{}/offers/listing", API_BASE))
            .bearer_auth(token)
            .header("Accept", ACCEPT_HEADER)
            .query(&query)
            .send()
            .await
            .context("Failed to search Allegro offers")?;

        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            if status.as_u16() == 403 {
                return Err(anyhow::anyhow!(
                    "Allegro search failed with 403: {}. This usually means the app isn't a \
                     'verified application' yet — GET /offers/listing requires Allegro to \
                     manually approve the app for that status even when the OAuth token itself \
                     is valid.",
                    text
                ));
            }
            return Err(anyhow::anyhow!(
                "Allegro search failed with {}: {}",
                status,
                text
            ));
        }

        let doc: serde_json::Value = res
            .json()
            .await
            .context("Failed to parse Allegro search response")?;

        let mut offers = Vec::new();
        if let Some(items) = doc.get("items") {
            for key in ["promoted", "regular"] {
                if let Some(arr) = items.get(key).and_then(|v| v.as_array()) {
                    for item in arr {
                        offers.push(AllegroOffer::from_json(item));
                    }
                }
            }
        }

        Ok(offers)
    }
}

/// Tool for one-off gift/present searches on Allegro
pub struct AllegroSearchTool {
    metadata: ToolMetadata,
    client: Option<AllegroClient>,
}

impl AllegroSearchTool {
    pub fn new() -> Self {
        Self {
            metadata: ToolMetadata {
                id: "allegro_search".to_string(),
                name: "Allegro Gift Search".to_string(),
                description: "Search Allegro (Polish marketplace) for products and gift ideas"
                    .to_string(),
                category: ToolCategory::Search,
                tool_type: ToolType::AllegroSearch,
            },
            client: AllegroClient::from_env(),
        }
    }
}

impl Default for AllegroSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgentTool for AllegroSearchTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn get_function_definition(&self) -> serde_json::Value {
        json!({
            "name": "search_allegro",
            "description": "Search Allegro (the Polish e-commerce marketplace) for products. Use this to find gift ideas or specific things the user wants to buy, optionally filtered by price range and sorted by relevance or price.",
            "parameters": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "What to search for, e.g. 'mechaniczna klawiatura' or 'wireless headphones'"
                    },
                    "min_price": {
                        "type": "number",
                        "description": "Minimum price in PLN (optional)"
                    },
                    "max_price": {
                        "type": "number",
                        "description": "Maximum price in PLN (optional)"
                    },
                    "sort": {
                        "type": "string",
                        "description": "Sort order. Defaults to relevance.",
                        "enum": ["relevance", "price_asc", "price_desc"]
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Max number of results to return (1-20). Defaults to 10."
                    }
                },
                "required": ["query"]
            }
        })
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolCallResult> {
        let args: serde_json::Value =
            serde_json::from_str(&tool_call.function.arguments).unwrap_or_else(|_| json!({}));

        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: query"))?;

        let min_price = args.get("min_price").and_then(|v| v.as_f64());
        let max_price = args.get("max_price").and_then(|v| v.as_f64());
        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(10)
            .clamp(1, 20);
        let sort = match args.get("sort").and_then(|v| v.as_str()) {
            Some("price_asc") => Some("+price"),
            Some("price_desc") => Some("-price"),
            _ => None,
        };

        let client = self.client.as_ref().ok_or_else(|| {
            anyhow::anyhow!(
                "Allegro is not configured (missing ALLEGRO_CLIENT_ID/ALLEGRO_CLIENT_SECRET)"
            )
        })?;

        println!("🎁 Searching Allegro for: {}", query);
        let offers = client
            .search(query, min_price, max_price, limit, sort)
            .await?;
        println!(
            "✅ Allegro search completed for: {} ({} results)",
            query,
            offers.len()
        );

        let result = if offers.is_empty() {
            format!("No Allegro offers found for '{}'.", query)
        } else {
            let lines: Vec<String> = offers
                .iter()
                .enumerate()
                .map(|(i, offer)| offer.format(i + 1))
                .collect();
            format!(
                "Found {} Allegro offer(s) for '{}':\n\n{}",
                offers.len(),
                query,
                lines.join("\n\n")
            )
        };

        Ok(ToolCallResult {
            tool_call_id: None,
            tool_name: "search_allegro".to_string(),
            result,
        })
    }

    fn is_available(&self) -> bool {
        self.client.is_some()
    }
}

/// A single item on the gift wishlist
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct WishlistItem {
    phrase: String,
    max_price: Option<f64>,
    added_at: String,
}

async fn load_wishlist(path: &PathBuf) -> Result<Vec<WishlistItem>> {
    match tokio::fs::read_to_string(path).await {
        Ok(content) => {
            if content.trim().is_empty() {
                Ok(Vec::new())
            } else {
                serde_json::from_str(&content).context("Failed to parse wishlist file")
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(e).context("Failed to read wishlist file"),
    }
}

async fn save_wishlist(path: &PathBuf, items: &[WishlistItem]) -> Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .context("Failed to create wishlist directory")?;
    }
    let content = serde_json::to_string_pretty(items).context("Failed to serialize wishlist")?;
    tokio::fs::write(path, content)
        .await
        .context("Failed to write wishlist file")
}

/// Tool for maintaining a gift wishlist and checking Allegro for current deals on it
pub struct AllegroWishlistTool {
    metadata: ToolMetadata,
    client: Option<AllegroClient>,
    path: PathBuf,
    lock: Mutex<()>,
}

impl AllegroWishlistTool {
    pub fn new() -> Self {
        let path = env::var("ALLEGRO_WISHLIST_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("data/allegro_wishlist.json"));

        Self {
            metadata: ToolMetadata {
                id: "allegro_wishlist".to_string(),
                name: "Allegro Gift Wishlist".to_string(),
                description: "Track gift ideas and check Allegro for current best prices on them"
                    .to_string(),
                category: ToolCategory::Search,
                tool_type: ToolType::AllegroWishlist,
            },
            client: AllegroClient::from_env(),
            path,
            lock: Mutex::new(()),
        }
    }
}

impl Default for AllegroWishlistTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgentTool for AllegroWishlistTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn get_function_definition(&self) -> serde_json::Value {
        json!({
            "name": "manage_allegro_wishlist",
            "description": "Manage a wishlist of gift ideas to buy on Allegro, and check current best prices/deals for items already on the list. Use 'add' to remember something the user wants, 'list' to show the wishlist, 'remove' to drop an item, and 'check_deals' to search Allegro for current prices on every item.",
            "parameters": {
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "description": "Which wishlist action to perform",
                        "enum": ["add", "remove", "list", "check_deals"]
                    },
                    "phrase": {
                        "type": "string",
                        "description": "The gift/search phrase, e.g. 'mechaniczna klawiatura'. Required for 'add' and 'remove'."
                    },
                    "max_price": {
                        "type": "number",
                        "description": "Optional target price in PLN for 'add' — used by 'check_deals' to flag when a current offer is at or below it."
                    }
                },
                "required": ["action"]
            }
        })
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolCallResult> {
        let args: serde_json::Value =
            serde_json::from_str(&tool_call.function.arguments).unwrap_or_else(|_| json!({}));

        let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("");
        let phrase = args.get("phrase").and_then(|v| v.as_str());
        let max_price = args.get("max_price").and_then(|v| v.as_f64());

        let _guard = self.lock.lock().await;

        let result = match action {
            "add" => {
                let phrase =
                    phrase.ok_or_else(|| anyhow::anyhow!("Missing required parameter: phrase"))?;
                let mut items = load_wishlist(&self.path).await?;
                items.push(WishlistItem {
                    phrase: phrase.to_string(),
                    max_price,
                    added_at: chrono::Utc::now().to_rfc3339(),
                });
                save_wishlist(&self.path, &items).await?;
                println!("🎁 Added to Allegro wishlist: {}", phrase);
                format!("Added '{}' to the gift wishlist.", phrase)
            }
            "remove" => {
                let phrase =
                    phrase.ok_or_else(|| anyhow::anyhow!("Missing required parameter: phrase"))?;
                let mut items = load_wishlist(&self.path).await?;
                let before = items.len();
                items.retain(|i| !i.phrase.eq_ignore_ascii_case(phrase));
                let removed = before - items.len();
                save_wishlist(&self.path, &items).await?;
                if removed > 0 {
                    format!("Removed '{}' from the gift wishlist.", phrase)
                } else {
                    format!("'{}' was not found on the wishlist.", phrase)
                }
            }
            "list" => {
                let items = load_wishlist(&self.path).await?;
                if items.is_empty() {
                    "The gift wishlist is empty.".to_string()
                } else {
                    let lines: Vec<String> = items
                        .iter()
                        .enumerate()
                        .map(|(i, item)| match item.max_price {
                            Some(p) => format!("{}. {} (target: {:.2} PLN)", i + 1, item.phrase, p),
                            None => format!("{}. {}", i + 1, item.phrase),
                        })
                        .collect();
                    format!(
                        "Gift wishlist ({} item(s)):\n{}",
                        items.len(),
                        lines.join("\n")
                    )
                }
            }
            "check_deals" => {
                let items = load_wishlist(&self.path).await?;
                if items.is_empty() {
                    "The gift wishlist is empty — nothing to check.".to_string()
                } else {
                    let client = self.client.as_ref().ok_or_else(|| {
                        anyhow::anyhow!(
                            "Allegro is not configured (missing ALLEGRO_CLIENT_ID/ALLEGRO_CLIENT_SECRET)"
                        )
                    })?;

                    let mut lines = Vec::new();
                    for item in &items {
                        match client
                            .search(&item.phrase, None, None, 3, Some("+price"))
                            .await
                        {
                            Ok(offers) => {
                                if let Some(cheapest) = offers.first() {
                                    let deal = item
                                        .max_price
                                        .zip(cheapest.price_value())
                                        .map(|(target, price)| price <= target)
                                        .unwrap_or(false);
                                    let marker = if deal { " 🎉 DEAL!" } else { "" };
                                    lines.push(format!(
                                        "- {}: {}{}",
                                        item.phrase,
                                        cheapest.format(1).replace("1. ", ""),
                                        marker
                                    ));
                                } else {
                                    lines.push(format!("- {}: no offers found", item.phrase));
                                }
                            }
                            Err(e) => {
                                lines.push(format!("- {}: search failed ({})", item.phrase, e));
                            }
                        }
                    }
                    format!(
                        "Current Allegro prices for your wishlist:\n{}",
                        lines.join("\n")
                    )
                }
            }
            other => return Err(anyhow::anyhow!("Unknown wishlist action: '{}'", other)),
        };

        Ok(ToolCallResult {
            tool_call_id: None,
            tool_name: "manage_allegro_wishlist".to_string(),
            result,
        })
    }

    fn is_available(&self) -> bool {
        self.client.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_offer_json() {
        let raw = json!({
            "id": "12345",
            "name": "Test Gift",
            "sellingMode": {
                "price": { "amount": "99.99", "currency": "PLN" }
            }
        });

        let offer = AllegroOffer::from_json(&raw);
        assert_eq!(offer.name, "Test Gift");
        assert_eq!(offer.price.as_deref(), Some("99.99"));
        assert_eq!(offer.currency.as_deref(), Some("PLN"));
        assert_eq!(
            offer.url.as_deref(),
            Some("https://allegro.pl/oferta/12345")
        );
        assert_eq!(offer.price_value(), Some(99.99));
    }

    #[test]
    fn parses_offer_missing_fields() {
        let raw = json!({});
        let offer = AllegroOffer::from_json(&raw);
        assert_eq!(offer.name, "Unknown offer");
        assert!(offer.price.is_none());
        assert!(offer.url.is_none());
        assert!(offer.format(1).contains("price unavailable"));
    }

    #[test]
    fn search_tool_definition_matches_execute_name() {
        let tool = AllegroSearchTool {
            metadata: ToolMetadata {
                id: "allegro_search".to_string(),
                name: "Allegro Gift Search".to_string(),
                description: "test".to_string(),
                category: ToolCategory::Search,
                tool_type: ToolType::AllegroSearch,
            },
            client: None,
        };
        assert!(!tool.is_available());
        let def = tool.get_function_definition();
        assert_eq!(
            def.get("name").and_then(|v| v.as_str()),
            Some("search_allegro")
        );
    }

    #[test]
    fn wishlist_tool_definition_has_actions_enum() {
        let tool = AllegroWishlistTool {
            metadata: ToolMetadata {
                id: "allegro_wishlist".to_string(),
                name: "Allegro Gift Wishlist".to_string(),
                description: "test".to_string(),
                category: ToolCategory::Search,
                tool_type: ToolType::AllegroWishlist,
            },
            client: None,
            path: PathBuf::from("unused.json"),
            lock: Mutex::new(()),
        };
        assert!(!tool.is_available());
        let def = tool.get_function_definition();
        let actions = def
            .pointer("/parameters/properties/action/enum")
            .and_then(|v| v.as_array())
            .expect("action enum present");
        assert_eq!(actions.len(), 4);
    }

    #[tokio::test]
    async fn wishlist_add_list_remove_round_trip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("wishlist.json");

        let mut items = load_wishlist(&path).await.expect("load empty");
        assert!(items.is_empty());

        items.push(WishlistItem {
            phrase: "mechanical keyboard".to_string(),
            max_price: Some(250.0),
            added_at: "2026-01-01T00:00:00Z".to_string(),
        });
        save_wishlist(&path, &items).await.expect("save");

        let reloaded = load_wishlist(&path).await.expect("reload");
        assert_eq!(reloaded.len(), 1);
        assert_eq!(reloaded[0].phrase, "mechanical keyboard");
        assert_eq!(reloaded[0].max_price, Some(250.0));

        let mut items = reloaded;
        items.retain(|i| !i.phrase.eq_ignore_ascii_case("mechanical keyboard"));
        save_wishlist(&path, &items)
            .await
            .expect("save after remove");

        let final_items = load_wishlist(&path).await.expect("reload after remove");
        assert!(final_items.is_empty());
    }

    /// Live diagnostic against the real Allegro API, bypassing the agent entirely.
    /// Ignored by default (needs real credentials + network). Run explicitly with:
    ///   cd src/backend && cargo test --test-threads=1 -- --ignored --nocapture live_check_allegro_credentials
    /// Never prints the client secret or the raw access token, only status/error text.
    #[tokio::test]
    #[ignore]
    async fn live_check_allegro_credentials() {
        dotenv::dotenv().ok();

        let client = AllegroClient::from_env()
            .expect("ALLEGRO_CLIENT_ID / ALLEGRO_CLIENT_SECRET not set in environment/.env");

        match client.access_token().await {
            Ok(token) => println!("✅ token request OK (len={})", token.len()),
            Err(e) => {
                println!("❌ token request FAILED: {:#}", e);
                return;
            }
        }

        match client.search("prezent", None, None, 3, None).await {
            Ok(offers) => {
                println!("✅ search OK — {} offer(s) returned", offers.len());
                for offer in offers.iter().take(3) {
                    println!("  - {}", offer.format(1));
                }
            }
            Err(e) => println!("❌ search FAILED: {:#}", e),
        }
    }
}
