use crate::api::agent::core::types::{ToolCall, ToolCallResult, ToolType};
use crate::api::agent::tools::framework::agent_tool::{AgentTool, ToolCategory, ToolMetadata};
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use reqwest::{header, Client};
use serde_json::json;
use std::env;

fn create_github_client(token: &str) -> Client {
    let mut headers = header::HeaderMap::new();
    if !token.is_empty() {
        let mut auth_value = header::HeaderValue::from_str(&format!("Bearer {}", token))
            .expect("Invalid header value for GITHUB_TOKEN");
        auth_value.set_sensitive(true);
        headers.insert(header::AUTHORIZATION, auth_value);
    }
    headers.insert(
        header::USER_AGENT,
        header::HeaderValue::from_static("ai-agent-tool/1.0"),
    );
    headers.insert(
        header::ACCEPT,
        header::HeaderValue::from_static("application/vnd.github.v3+json"),
    );

    Client::builder()
        .default_headers(headers)
        .build()
        .expect("Failed to build reqwest client")
}

// ============================================================================================
// Public GitHub Tool
// ============================================================================================

pub struct GitHubPublicTool {
    metadata: ToolMetadata,
    client: Client,
}

impl GitHubPublicTool {
    pub fn new() -> Self {
        // Public tool doesn't strictly need a token, but good to have if available for rate limits
        let token = env::var("GITHUB_TOKEN").unwrap_or_default();
        Self {
            metadata: ToolMetadata {
                id: "github_public".to_string(),
                name: "github_public".to_string(),
                category: ToolCategory::Development,
                tool_type: ToolType::GitHubPublic,
            },
            client: create_github_client(&token),
        }
    }

    async fn search_repos(&self, query: &str, sort: Option<&str>) -> Result<serde_json::Value> {
        let url = "https://api.github.com/search/repositories";
        let sort_param = sort.unwrap_or("stars");

        let response = self
            .client
            .get(url)
            .query(&[("q", query), ("sort", sort_param), ("per_page", "5")])
            .send()
            .await
            .context("Failed to search repositories")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("GitHub API error: {}", response.status()));
        }

        response
            .json()
            .await
            .context("Failed to parse search response")
    }

    async fn get_trending(
        &self,
        language: Option<&str>,
        timeframe: Option<&str>,
    ) -> Result<serde_json::Value> {
        let now = Utc::now();
        let date_filter = match timeframe {
            Some("weekly") => now - chrono::Duration::weeks(1),
            Some("monthly") => now - chrono::Duration::days(30),
            _ => now - chrono::Duration::days(1), // daily default
        };
        let date_str = date_filter.format("%Y-%m-%d").to_string();

        let mut query = format!("created:>{}", date_str);
        if let Some(lang) = language {
            query.push_str(&format!(" language:{}", lang));
        }

        self.search_repos(&query, Some("stars")).await
    }

    async fn list_user_repos(&self, username: &str) -> Result<serde_json::Value> {
        let url = format!("https://api.github.com/users/{}/repos", username);
        let response = self
            .client
            .get(&url)
            .query(&[("sort", "updated"), ("per_page", "10")])
            .send()
            .await
            .context("Failed to fetch user repositories")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("GitHub API error: {}", response.status()));
        }

        response
            .json()
            .await
            .context("Failed to parse repositories")
    }

    fn format_repo_list(&self, data: &serde_json::Value) -> String {
        let items = if let Some(items) = data.get("items").and_then(|i| i.as_array()) {
            items
        } else if let Some(items) = data.as_array() {
            items
        } else {
            return "No repositories found.".to_string();
        };

        if items.is_empty() {
            return "No repositories found.".to_string();
        }

        let mut output = String::new();
        for item in items {
            let name = item["full_name"].as_str().unwrap_or("unknown");
            let desc = item["description"].as_str().unwrap_or("No description");
            let stars = item["stargazers_count"].as_u64().unwrap_or(0);
            let url = item["html_url"].as_str().unwrap_or("");
            let lang = item["language"].as_str().unwrap_or("Unknown");

            output.push_str(&format!(
                "- **[{}]({})** (⭐ {} | {})\n  {}\n\n",
                name, url, stars, lang, desc
            ));
        }
        output
    }
}

#[async_trait]
impl AgentTool for GitHubPublicTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn get_function_definition(&self) -> serde_json::Value {
        json!({
            "name": "github_public",
            "description": "Access PUBLIC GitHub information: search repositories, check trending projects, or list specific user's repositories. Does NOT require authentication, but uses it if available.",
            "parameters": {
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["trending", "search", "user_repos"],
                        "description": "The action to perform."
                    },
                    "query": {
                        "type": "string",
                        "description": "Search query (required for 'search')."
                    },
                    "language": {
                        "type": "string",
                        "description": "Programming language filter (optional for 'trending'/'search')."
                    },
                    "timeframe": {
                        "type": "string",
                        "enum": ["daily", "weekly", "monthly"],
                        "description": "Timeframe for trending (default: daily)."
                    },
                    "username": {
                        "type": "string",
                        "description": "Target username (required for 'user_repos')."
                    }
                },
                "required": ["action"]
            }
        })
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolCallResult> {
        let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
            .context("Failed to parse arguments")?;

        let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("");

        println!("\x1b[36m🐙 GitHub Public Tool executing: {}\x1b[0m", action);

        let result = match action {
            "trending" => {
                let lang = args.get("language").and_then(|v| v.as_str());
                let timeframe = args.get("timeframe").and_then(|v| v.as_str());
                let data = self.get_trending(lang, timeframe).await?;
                format!(
                    "🔥 **Trending Repositories**\n\n{}",
                    self.format_repo_list(&data)
                )
            }
            "search" => {
                let query = args
                    .get("query")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("'query' is required for search"))?;
                let data = self.search_repos(query, None).await?;
                format!(
                    "🔍 **GitHub Search Results**\n\n{}",
                    self.format_repo_list(&data)
                )
            }
            "user_repos" => {
                let username = args
                    .get("username")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("'username' is required for user_repos"))?;
                let data = self.list_user_repos(username).await?;
                format!(
                    "📂 **Repositories for {}**\n\n{}",
                    username,
                    self.format_repo_list(&data)
                )
            }
            _ => return Err(anyhow::anyhow!("Unknown action: {}", action)),
        };

        Ok(ToolCallResult {
            tool_name: "github_public".to_string(),
            result,
        })
    }

    fn is_available(&self) -> bool {
        true
    }
}

// ============================================================================================
// Authenticated GitHub Tool
// ============================================================================================

pub struct GitHubAuthenticatedTool {
    metadata: ToolMetadata,
    client: Client,
    token: String,
}

impl GitHubAuthenticatedTool {
    pub fn new() -> Self {
        let token = env::var("GITHUB_TOKEN").unwrap_or_default();
        Self {
            metadata: ToolMetadata {
                id: "github_authenticated".to_string(),
                name: "github_authenticated".to_string(),
                category: ToolCategory::Development,
                tool_type: ToolType::GitHubAuthenticated,
            },
            client: create_github_client(&token),
            token,
        }
    }

    async fn check_notifications(&self) -> Result<serde_json::Value> {
        if self.token.is_empty() {
            return Err(anyhow::anyhow!(
                "GITHUB_TOKEN is required for notifications"
            ));
        }
        let url = "https://api.github.com/notifications";
        let response = self
            .client
            .get(url)
            .query(&[("all", "false"), ("per_page", "10")])
            .send()
            .await
            .context("Failed to fetch notifications")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("GitHub API error: {}", response.status()));
        }
        response
            .json()
            .await
            .context("Failed to parse notifications")
    }

    async fn list_my_repos(&self) -> Result<serde_json::Value> {
        if self.token.is_empty() {
            return Err(anyhow::anyhow!(
                "GITHUB_TOKEN is required to list your repositories"
            ));
        }
        let url = "https://api.github.com/user/repos";
        let response = self
            .client
            .get(url)
            .query(&[("sort", "updated"), ("per_page", "10"), ("type", "owner")])
            .send()
            .await
            .context("Failed to fetch repositories")?;

        if response.status() == reqwest::StatusCode::FORBIDDEN {
            return Err(anyhow::anyhow!(
                "Access Forbidden (403). Check that your GITHUB_TOKEN has the 'metadata' or 'contents' scope enabled."
            ));
        }
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("GitHub API error: {}", response.status()));
        }
        response
            .json()
            .await
            .context("Failed to parse repositories")
    }

    async fn check_workflow_runs(&self, owner: &str, repo: &str) -> Result<serde_json::Value> {
        // Requires GITHUB_TOKEN for private repos or higher limits
        let url = format!(
            "https://api.github.com/repos/{}/{}/actions/runs",
            owner, repo
        );
        let response = self
            .client
            .get(&url)
            .query(&[("per_page", "5")])
            .send()
            .await
            .context("Failed to fetch workflow runs")?;

        if response.status() == reqwest::StatusCode::FORBIDDEN {
            return Err(anyhow::anyhow!(
                "Access Forbidden (403). For Actions, ensure your GITHUB_TOKEN has the 'actions' scope (Read-only)."
            ));
        }
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("GitHub API error: {}", response.status()));
        }
        response
            .json()
            .await
            .context("Failed to parse workflow runs")
    }

    async fn list_issues(&self, owner: &str, repo: &str) -> Result<serde_json::Value> {
        let url = format!("https://api.github.com/repos/{}/{}/issues", owner, repo);
        let response = self
            .client
            .get(&url)
            .query(&[("state", "open"), ("sort", "updated"), ("per_page", "5")])
            .send()
            .await
            .context("Failed to fetch issues")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("GitHub API error: {}", response.status()));
        }
        response.json().await.context("Failed to parse issues")
    }

    async fn list_events(&self, username: &str) -> Result<serde_json::Value> {
        let url = format!("https://api.github.com/users/{}/events", username);
        let response = self
            .client
            .get(&url)
            .query(&[("per_page", "5")])
            .send()
            .await
            .context("Failed to fetch events")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("GitHub API error: {}", response.status()));
        }
        response.json().await.context("Failed to parse events")
    }

    async fn list_pulls(&self, owner: &str, repo: &str) -> Result<serde_json::Value> {
        let url = format!("https://api.github.com/repos/{}/{}/pulls", owner, repo);
        let response = self
            .client
            .get(&url)
            .query(&[
                ("state", "open"),
                ("per_page", "5"),
                ("sort", "updated"),
                ("direction", "desc"),
            ])
            .send()
            .await
            .context("Failed to list pull requests")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("GitHub API error: {}", response.status()));
        }
        response.json().await.context("Failed to parse pulls")
    }

    async fn list_authenticated_issues(&self, filter: &str) -> Result<serde_json::Value> {
        let url = "https://api.github.com/issues";
        let response = self
            .client
            .get(url)
            .query(&[
                ("filter", filter),
                ("state", "open"),
                ("sort", "updated"),
                ("per_page", "10"),
            ])
            .send()
            .await
            .context("Failed to fetch authenticated issues")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("GitHub API error: {}", response.status()));
        }
        response.json().await.context("Failed to parse issues")
    }

    // Formatters reused logic could be shared but for now duplication is safer than complexity
    fn format_repo_list(&self, data: &serde_json::Value) -> String {
        let items = if let Some(items) = data.get("items").and_then(|i| i.as_array()) {
            items
        } else if let Some(items) = data.as_array() {
            items
        } else {
            return "No repositories found.".to_string();
        };

        if items.is_empty() {
            return "No repositories found.".to_string();
        }
        let mut output = String::new();
        for item in items {
            let name = item["full_name"].as_str().unwrap_or("unknown");
            let stars = item["stargazers_count"].as_u64().unwrap_or(0);
            let url = item["html_url"].as_str().unwrap_or("");
            output.push_str(&format!("- **[{}]({})** (⭐ {})\n", name, url, stars));
        }
        output
    }

    fn format_notifications(&self, data: &serde_json::Value) -> String {
        let items = match data.as_array() {
            Some(i) => i,
            None => return "No notifications found.".to_string(),
        };
        if items.is_empty() {
            return "No new notifications! 🎉".to_string();
        }
        let mut output = String::new();
        for item in items {
            let title = item["subject"]["title"].as_str().unwrap_or("No title");
            let _type = item["subject"]["type"].as_str().unwrap_or("Notification");
            let repo = item["repository"]["full_name"]
                .as_str()
                .unwrap_or("unknown");
            output.push_str(&format!("- **{}**: {}\n  Repo: {}\n\n", _type, title, repo));
        }
        output
    }

    fn format_workflow_runs(&self, data: &serde_json::Value) -> String {
        let runs = match data.get("workflow_runs").and_then(|i| i.as_array()) {
            Some(i) => i,
            None => return "No workflow runs found.".to_string(),
        };
        if runs.is_empty() {
            return "No workflow runs found.".to_string();
        }

        let mut output = String::new();
        for run in runs {
            let name = run["name"].as_str().unwrap_or("unnamed");
            let status = run["status"].as_str().unwrap_or("unknown");
            let conclusion = run["conclusion"].as_str().unwrap_or("pending");
            let url = run["html_url"].as_str().unwrap_or("");
            let icon = match conclusion {
                "success" => "✅",
                "failure" => "❌",
                "cancelled" => "🚫",
                "pending" => "⏳",
                _ => "❓",
            };
            output.push_str(&format!(
                "- {} **[{}]({})**\n  Status: {} | Result: {}\n\n",
                icon, name, url, status, conclusion
            ));
        }
        output
    }

    fn format_issues(&self, data: &serde_json::Value) -> String {
        let items = match data.as_array() {
            Some(i) => i,
            None => return "No issues found.".to_string(),
        };
        if items.is_empty() {
            return "No issues found.".to_string();
        }

        let mut output = String::new();
        for item in items {
            let title = item["title"].as_str().unwrap_or("No title");
            let url = item["html_url"].as_str().unwrap_or("");
            let number = item["number"].as_i64().unwrap_or(0);
            let user = item["user"]["login"].as_str().unwrap_or("unknown");
            output.push_str(&format!(
                "- **#{} [{}]({})** by @{}\n",
                number, title, url, user
            ));
        }
        output
    }

    fn format_events(&self, data: &serde_json::Value) -> String {
        let items = match data.as_array() {
            Some(i) => i,
            None => return "No events found.".to_string(),
        };
        if items.is_empty() {
            return "No events found.".to_string();
        }

        let mut output = String::new();
        for item in items {
            let _type = item["type"].as_str().unwrap_or("Event");
            let repo = item["repo"]["name"].as_str().unwrap_or("unknown");
            let date = item["created_at"]
                .as_str()
                .unwrap_or("")
                .split('T')
                .next()
                .unwrap_or("");
            output.push_str(&format!("- **{}** at {}\n  Date: {}\n", _type, repo, date));
        }
        output
    }

    fn format_pulls(&self, data: &serde_json::Value) -> String {
        let items = match data.as_array() {
            Some(i) => i,
            None => return "No pull requests found.".to_string(),
        };

        if items.is_empty() {
            return "No pull requests found.".to_string();
        }

        let mut output = String::new();
        for item in items {
            let title = item["title"].as_str().unwrap_or("No title");
            let url = item["html_url"].as_str().unwrap_or("");
            let user = item["user"]["login"].as_str().unwrap_or("unknown");
            output.push_str(&format!("- **[{}]({})** by @{}\n", title, url, user));
        }
        output
    }
}

#[async_trait]
impl AgentTool for GitHubAuthenticatedTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn get_function_definition(&self) -> serde_json::Value {
        json!({
            "name": "github_authenticated",
            "description": "Access PRIVATE/AUTHENTICATED GitHub features: notifications, your repos, workflow runs, issues, and events. REQUIRED: GITHUB_TOKEN env variable.",
            "parameters": {
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["notifications", "list_my_repos", "actions", "issues", "events", "pulls"],
                        "description": "The action to perform."
                    },
                    "owner": { "type": "string", "description": "Repository owner (optional for issues/pulls)." },
                    "repo": { "type": "string", "description": "Repository name (optional for issues/pulls)." },
                    "username": { "type": "string", "description": "Username for events check." }
                },
                "required": ["action"]
            }
        })
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolCallResult> {
        let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
            .context("Failed to parse arguments")?;

        let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("");
        println!("\x1b[35m🔐 GitHub Auth Tool executing: {}\x1b[0m", action);

        if self.token.is_empty() {
            return Ok(ToolCallResult {
                tool_name: "github_authenticated".to_string(),
                result: "❌ GITHUB_TOKEN is not set. This tool requires authentication."
                    .to_string(),
            });
        }

        let result = match action {
            "notifications" => match self.check_notifications().await {
                Ok(data) => format!(
                    "🔔 **Your Notifications**\n\n{}",
                    self.format_notifications(&data)
                ),
                Err(e) => format!("❌ Failed: {}", e),
            },
            "list_my_repos" => match self.list_my_repos().await {
                Ok(data) => format!(
                    "📂 **Your Managed Repositories**\n\n{}",
                    self.format_repo_list(&data)
                ),
                Err(e) => format!("❌ Failed: {}", e),
            },
            "actions" => {
                let owner = args.get("owner").and_then(|v| v.as_str()).unwrap_or("");
                let repo = args.get("repo").and_then(|v| v.as_str()).unwrap_or("");
                if owner.is_empty() || repo.is_empty() {
                    return Err(anyhow::anyhow!("Owner and repo required"));
                }

                match self.check_workflow_runs(owner, repo).await {
                    Ok(data) => format!(
                        "🏃 **Workflows for {}/{}**\n\n{}",
                        owner,
                        repo,
                        self.format_workflow_runs(&data)
                    ),
                    Err(e) => format!("❌ Failed: {}", e),
                }
            }
            "issues" => {
                let owner = args.get("owner").and_then(|v| v.as_str()).unwrap_or("");
                let repo = args.get("repo").and_then(|v| v.as_str()).unwrap_or("");

                if owner.is_empty() || repo.is_empty() {
                    // List issues assigned to authenticated user
                    match self.list_authenticated_issues("assigned").await {
                        Ok(data) => format!(
                            "🐛 **Issues Assigned to You**\n\n{}",
                            self.format_issues(&data)
                        ),
                        Err(e) => format!("❌ Failed: {}", e),
                    }
                } else {
                    match self.list_issues(owner, repo).await {
                        Ok(data) => format!(
                            "🐛 **Issues for {}/{}**\n\n{}",
                            owner,
                            repo,
                            self.format_issues(&data)
                        ),
                        Err(e) => format!("❌ Failed: {}", e),
                    }
                }
            }
            "events" => {
                let username = args.get("username").and_then(|v| v.as_str()).unwrap_or("");
                if username.is_empty() {
                    return Err(anyhow::anyhow!("Username required for events"));
                }

                match self.list_events(username).await {
                    Ok(data) => format!(
                        "📅 **Events for {}**\n\n{}",
                        username,
                        self.format_events(&data)
                    ),
                    Err(e) => format!("❌ Failed: {}", e),
                }
            }
            "pulls" => {
                let owner = args.get("owner").and_then(|v| v.as_str()).unwrap_or("");
                let repo = args.get("repo").and_then(|v| v.as_str()).unwrap_or("");

                if owner.is_empty() || repo.is_empty() {
                    // Use issues endpoint but perhaps filter differently?
                    // For now reusing list_authenticated_issues ("assigned") but user might want "created" or "mentioned"
                    // Implementation choice: list default (assigned) for 'pulls' context too, or we can use "all"
                    match self.list_authenticated_issues("all").await {
                        Ok(data) => format!(
                            "🔃 **Your Pull Requests & Issues**\n\n{}",
                            self.format_issues(&data)
                        ),
                        Err(e) => format!("❌ Failed: {}", e),
                    }
                } else {
                    match self.list_pulls(owner, repo).await {
                        Ok(data) => format!(
                            "🔃 **Pull Requests for {}/{}**\n\n{}",
                            owner,
                            repo,
                            self.format_pulls(&data)
                        ),
                        Err(e) => format!("❌ Failed: {}", e),
                    }
                }
            }
            _ => return Err(anyhow::anyhow!("Unknown action: {}", action)),
        };

        Ok(ToolCallResult {
            tool_name: "github_authenticated".to_string(),
            result,
        })
    }

    fn is_available(&self) -> bool {
        // Only available if token is present
        !self.token.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_github_public_metadata() {
        let tool = GitHubPublicTool::new();
        let metadata = tool.metadata();
        assert_eq!(metadata.id, "github_public");
        assert_eq!(metadata.name, "github_public");
        assert_eq!(metadata.category, ToolCategory::Development);
        assert_eq!(metadata.tool_type, ToolType::GitHubPublic);
    }

    #[test]
    fn test_github_public_function_definition() {
        let tool = GitHubPublicTool::new();
        let def = tool.get_function_definition();
        assert_eq!(def["name"], "github_public");
        assert!(def["parameters"]["properties"].get("action").is_some());
    }

    #[test]
    fn test_github_authenticated_metadata() {
        let tool = GitHubAuthenticatedTool::new();
        let metadata = tool.metadata();
        assert_eq!(metadata.id, "github_authenticated");
        assert_eq!(metadata.tool_type, ToolType::GitHubAuthenticated);
    }

    #[test]
    fn test_github_authenticated_availability() {
        let tool = GitHubAuthenticatedTool::new();
        // If GITHUB_TOKEN is not set, is_available should be false
        if std::env::var("GITHUB_TOKEN").is_err() {
            assert!(!tool.is_available());
        } else {
            assert!(tool.is_available());
        }
    }
}
