use crate::api::agent::core::types::{ToolCall, ToolCallResult, ToolType};
use crate::api::agent::tools::framework::agent_tool::{AgentTool, ToolCategory, ToolMetadata};
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use reqwest::{header, Client};
use serde_json::json;
use std::env;

/// The real GitHub REST API root, used unless a test overrides it.
const GITHUB_API_URL: &str = "https://api.github.com";

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
    headers.insert(
        "X-GitHub-Api-Version",
        header::HeaderValue::from_static("2022-11-28"),
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
    /// API root to talk to. Always the real GitHub one in production; tests point
    /// it at a loopback mock instead.
    base_url: String,
}

impl GitHubPublicTool {
    pub fn new() -> Self {
        // Public tool doesn't strictly need a token, but good to have if available for rate limits
        let mut token = env::var("GITHUB_TOKEN")
            .unwrap_or_default()
            .trim()
            .trim_matches(|c| c == '"' || c == '\'')
            .to_string();

        if token.starts_with("Bearer ") {
            token = token["Bearer ".len()..].trim().to_string();
        } else if token.starts_with("token ") {
            token = token["token ".len()..].trim().to_string();
        }

        Self {
            metadata: ToolMetadata {
                id: "github_public".to_string(),
                name: "GitHub Public".to_string(),
                description: "Search public repositories and users".to_string(),
                category: ToolCategory::Development,
                tool_type: ToolType::GitHubPublic,
            },
            client: create_github_client(&token),
            base_url: GITHUB_API_URL.to_string(),
        }
    }

    /// A tool pointed at `base_url` instead of the real GitHub API, so the
    /// request/response handling can be driven without the network.
    #[cfg(test)]
    pub(crate) fn with_base_url(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            ..Self::new()
        }
    }

    async fn search_repos(&self, query: &str, sort: Option<&str>) -> Result<serde_json::Value> {
        let url = format!("{}/search/repositories", self.base_url);
        let sort_param = sort.unwrap_or("stars");

        let response = self
            .client
            .get(&url)
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
        let url = format!("{}/users/{}/repos", self.base_url, username);
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
            tool_call_id: None,
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
    owner: String,
    /// Default repository used when an action's `repo` argument is omitted,
    /// mirroring `owner`'s `GITHUB_OWNER` fallback. Empty means "no default".
    repo: String,
    /// API root to talk to. Always the real GitHub one in production; tests point
    /// it at a loopback mock instead.
    base_url: String,
}

impl GitHubAuthenticatedTool {
    pub fn new() -> Self {
        let mut token = env::var("GITHUB_TOKEN")
            .unwrap_or_default()
            .trim()
            .trim_matches(|c| c == '"' || c == '\'')
            .to_string();

        let owner = env::var("GITHUB_OWNER")
            .unwrap_or_default()
            .trim()
            .to_string();

        let repo = env::var("GITHUB_REPO")
            .unwrap_or_default()
            .trim()
            .to_string();

        if token.starts_with("Bearer ") {
            token = token["Bearer ".len()..].trim().to_string();
        } else if token.starts_with("token ") {
            token = token["token ".len()..].trim().to_string();
        }

        Self {
            metadata: ToolMetadata {
                id: "github_authenticated".to_string(),
                name: "GitHub Authenticated".to_string(),
                description: "Manage issues, PRs, and notifications".to_string(),
                category: ToolCategory::Development,
                tool_type: ToolType::GitHubAuthenticated,
            },
            client: create_github_client(&token),
            token,
            owner,
            repo,
            base_url: GITHUB_API_URL.to_string(),
        }
    }

    /// A tool with a canned token and owner pointed at `base_url` instead of the
    /// real GitHub API, so the authenticated paths can be driven without the
    /// network and without `GITHUB_TOKEN`/`GITHUB_OWNER` being set. `repo` starts
    /// empty (not read from the real `GITHUB_REPO`) so tests stay hermetic; use
    /// [`Self::with_repo`] to opt into a default explicitly.
    #[cfg(test)]
    pub(crate) fn with_base_url(base_url: impl Into<String>, token: &str, owner: &str) -> Self {
        Self {
            client: create_github_client(token),
            token: token.to_string(),
            owner: owner.to_string(),
            repo: String::new(),
            base_url: base_url.into(),
            ..Self::new()
        }
    }

    /// Sets the default repo, as if `GITHUB_REPO` had been configured.
    #[cfg(test)]
    pub(crate) fn with_repo(mut self, repo: impl Into<String>) -> Self {
        self.repo = repo.into();
        self
    }

    async fn check_notifications(&self) -> Result<serde_json::Value> {
        if self.token.is_empty() {
            return Err(anyhow::anyhow!(
                "GITHUB_TOKEN is required for notifications"
            ));
        }
        let url = format!("{}/notifications", self.base_url);
        let response = self
            .client
            .get(&url)
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

    async fn list_my_repos(&self, page: u32) -> Result<serde_json::Value> {
        if self.token.is_empty() {
            return Err(anyhow::anyhow!(
                "GITHUB_TOKEN is required to list your repositories"
            ));
        }
        let url = format!("{}/user/repos", self.base_url);
        let response = self
            .client
            .get(&url)
            .query(&[
                ("sort", "updated"),
                ("per_page", "100"),
                ("type", "owner"),
                ("page", &page.to_string()),
            ])
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

    async fn list_org_repos(&self, org: &str, page: u32) -> Result<serde_json::Value> {
        if self.token.is_empty() {
            return Err(anyhow::anyhow!(
                "GITHUB_TOKEN is required to list organization repositories"
            ));
        }
        let url = format!("{}/orgs/{}/repos", self.base_url, org);
        let response = self
            .client
            .get(&url)
            .query(&[
                ("sort", "updated"),
                ("per_page", "100"),
                ("page", &page.to_string()),
            ])
            .send()
            .await
            .context("Failed to fetch organization repositories")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("GitHub API error: {}", response.status()));
        }
        response
            .json()
            .await
            .context("Failed to parse organization repositories")
    }

    async fn check_workflow_runs(&self, owner: &str, repo: &str) -> Result<serde_json::Value> {
        // Requires GITHUB_TOKEN for private repos or higher limits
        let url = format!("{}/repos/{}/{}/actions/runs", self.base_url, owner, repo);
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
        let url = format!("{}/repos/{}/{}/issues", self.base_url, owner, repo);
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
        let url = format!("{}/users/{}/events", self.base_url, username);
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
        let url = format!("{}/repos/{}/{}/pulls", self.base_url, owner, repo);
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

    /// Resolve `owner`/`repo`/`pr_number` shared by all the `pr_*`/`update_pr` actions.
    /// `owner` falls back to `GITHUB_OWNER`; `repo` and `pr_number` are always required.
    fn resolve_pr_target<'a>(
        &'a self,
        args: &'a serde_json::Value,
    ) -> Result<(&'a str, &'a str, u64)> {
        let mut owner = args.get("owner").and_then(|v| v.as_str()).unwrap_or("");
        if owner.is_empty() {
            owner = &self.owner;
        }
        let mut repo = args.get("repo").and_then(|v| v.as_str()).unwrap_or("");
        if repo.is_empty() {
            repo = &self.repo;
        }
        let pr_number = args.get("pr_number").and_then(|v| v.as_u64());

        if owner.is_empty() || repo.is_empty() {
            return Err(anyhow::anyhow!(
                "Owner and repo required (add GITHUB_OWNER to .env if owner omitted)"
            ));
        }
        let pr_number =
            pr_number.ok_or_else(|| anyhow::anyhow!("'pr_number' is required for this action"))?;

        Ok((owner, repo, pr_number))
    }

    async fn get_pull_request(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> Result<serde_json::Value> {
        let url = format!(
            "{}/repos/{}/{}/pulls/{}",
            self.base_url, owner, repo, pr_number
        );
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch pull request")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("GitHub API error: {}", response.status()));
        }
        response
            .json()
            .await
            .context("Failed to parse pull request")
    }

    async fn list_pull_commits(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> Result<serde_json::Value> {
        let url = format!(
            "{}/repos/{}/{}/pulls/{}/commits",
            self.base_url, owner, repo, pr_number
        );
        let response = self
            .client
            .get(&url)
            .query(&[("per_page", "100")])
            .send()
            .await
            .context("Failed to fetch pull request commits")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("GitHub API error: {}", response.status()));
        }
        response
            .json()
            .await
            .context("Failed to parse pull request commits")
    }

    async fn get_combined_status(
        &self,
        owner: &str,
        repo: &str,
        sha: &str,
    ) -> Result<serde_json::Value> {
        let url = format!(
            "{}/repos/{}/{}/commits/{}/status",
            self.base_url, owner, repo, sha
        );
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch combined status")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("GitHub API error: {}", response.status()));
        }
        response
            .json()
            .await
            .context("Failed to parse combined status")
    }

    async fn get_check_runs(
        &self,
        owner: &str,
        repo: &str,
        sha: &str,
    ) -> Result<serde_json::Value> {
        let url = format!(
            "{}/repos/{}/{}/commits/{}/check-runs",
            self.base_url, owner, repo, sha
        );
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch check runs")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("GitHub API error: {}", response.status()));
        }
        response.json().await.context("Failed to parse check runs")
    }

    async fn update_pull_request(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
        title: Option<&str>,
        body: Option<&str>,
    ) -> Result<serde_json::Value> {
        let url = format!(
            "{}/repos/{}/{}/pulls/{}",
            self.base_url, owner, repo, pr_number
        );

        let mut payload = serde_json::Map::new();
        if let Some(title) = title {
            payload.insert("title".to_string(), json!(title));
        }
        if let Some(body) = body {
            payload.insert("body".to_string(), json!(body));
        }

        let response = self
            .client
            .patch(&url)
            .json(&payload)
            .send()
            .await
            .context("Failed to update pull request")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("GitHub API error: {}", response.status()));
        }
        response
            .json()
            .await
            .context("Failed to parse updated pull request")
    }

    async fn get_my_profile(&self) -> Result<serde_json::Value> {
        if self.token.is_empty() {
            return Err(anyhow::anyhow!(
                "GITHUB_TOKEN is required to check followers"
            ));
        }
        let url = format!("{}/user", self.base_url);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch authenticated user profile")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("GitHub API error: {}", response.status()));
        }
        response
            .json()
            .await
            .context("Failed to parse user profile")
    }

    async fn list_authenticated_issues(
        &self,
        filter: &str,
        state: &str,
        page: u32,
    ) -> Result<serde_json::Value> {
        let url = format!("{}/issues", self.base_url);
        let response = self
            .client
            .get(&url)
            .query(&[
                ("filter", filter),
                ("state", state),
                ("sort", "updated"),
                ("per_page", "100"),
                ("page", &page.to_string()),
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

    fn format_followers(&self, data: &serde_json::Value) -> String {
        let login = data["login"].as_str().unwrap_or("unknown");
        let followers = data["followers"].as_u64().unwrap_or(0);
        let following = data["following"].as_u64().unwrap_or(0);
        format!(
            "**@{}** has **{}** followers (following {}).",
            login, followers, following
        )
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

    fn format_pr_details(&self, data: &serde_json::Value) -> String {
        let title = data["title"].as_str().unwrap_or("No title");
        let state = data["state"].as_str().unwrap_or("unknown");
        let draft = data["draft"].as_bool().unwrap_or(false);
        let state_label = if draft {
            format!("{} (draft)", state)
        } else {
            state.to_string()
        };
        let url = data["html_url"].as_str().unwrap_or("");
        let author = data["user"]["login"].as_str().unwrap_or("unknown");
        let head = data["head"]["ref"].as_str().unwrap_or("?");
        let base = data["base"]["ref"].as_str().unwrap_or("?");
        let body = data["body"]
            .as_str()
            .map(|b| b.trim())
            .filter(|b| !b.is_empty())
            .unwrap_or("No description.");

        format!(
            "**{}** ({})\n{}\nby @{}: `{}` → `{}`\n\n{}",
            title, state_label, url, author, head, base, body
        )
    }

    fn format_pr_checks(&self, status: &serde_json::Value, checks: &serde_json::Value) -> String {
        let overall = status["state"].as_str().unwrap_or("unknown");
        let statuses = status["statuses"].as_array();
        let check_runs = checks["check_runs"].as_array();

        let statuses_empty = statuses.map(|s| s.is_empty()).unwrap_or(true);
        let checks_empty = check_runs.map(|c| c.is_empty()).unwrap_or(true);
        if statuses_empty && checks_empty {
            return format!(
                "Overall status: **{}**\n\nNo status checks or check runs found.",
                overall
            );
        }

        let mut output = format!("Overall status: **{}**\n\n", overall);

        if let Some(statuses) = statuses.filter(|s| !s.is_empty()) {
            output.push_str("**Status checks:**\n");
            for s in statuses {
                let context = s["context"].as_str().unwrap_or("unknown");
                let state = s["state"].as_str().unwrap_or("unknown");
                let description = s["description"].as_str().unwrap_or("");
                let icon = match state {
                    "success" => "✅",
                    "failure" | "error" => "❌",
                    "pending" => "⏳",
                    _ => "❓",
                };
                output.push_str(&format!(
                    "- {} **{}**: {} — {}\n",
                    icon, context, state, description
                ));
            }
            output.push('\n');
        }

        if let Some(runs) = check_runs.filter(|c| !c.is_empty()) {
            output.push_str("**Check runs:**\n");
            for run in runs {
                let name = run["name"].as_str().unwrap_or("unnamed");
                let run_status = run["status"].as_str().unwrap_or("unknown");
                let conclusion = run["conclusion"].as_str().unwrap_or("pending");
                let icon = match conclusion {
                    "success" => "✅",
                    "failure" => "❌",
                    "cancelled" => "🚫",
                    "pending" => "⏳",
                    _ => "❓",
                };
                output.push_str(&format!(
                    "- {} **{}**: {} ({})\n",
                    icon, name, run_status, conclusion
                ));
            }
        }

        output.trim_end().to_string()
    }

    fn format_pr_commits(&self, data: &serde_json::Value) -> String {
        let items = match data.as_array() {
            Some(i) => i,
            None => return "No commits found.".to_string(),
        };
        if items.is_empty() {
            return "No commits found.".to_string();
        }

        let mut output = String::new();
        for item in items {
            let sha = item["sha"].as_str().unwrap_or("???");
            let short_sha = &sha[..sha.len().min(7)];
            let message = item["commit"]["message"]
                .as_str()
                .and_then(|m| m.lines().next())
                .unwrap_or("");
            let author = item["commit"]["author"]["name"]
                .as_str()
                .unwrap_or("unknown");
            let url = item["html_url"].as_str().unwrap_or("");
            output.push_str(&format!(
                "- **[{}]({})** {} — {}\n",
                short_sha, url, message, author
            ));
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
            "description": "Access PRIVATE/AUTHENTICATED GitHub features: notifications, your repos, workflow runs, issues, events, pull requests (list, view details, check CI status/checks, list commits, edit title/description), and follower count. REQUIRED: GITHUB_TOKEN env variable.",
            "parameters": {
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["notifications", "list_my_repos", "list_org_repos", "actions", "issues", "events", "pulls", "followers", "pr_details", "pr_checks", "pr_commits", "update_pr"],
                        "description": "The action to perform."
                    },
                    "owner": { "type": "string", "description": "Repository owner (optional, falls back to GITHUB_OWNER in .env)." },
                    "repo": { "type": "string", "description": "Repository name (optional for issues/pulls; for pr_details/pr_checks/pr_commits/update_pr, falls back to GITHUB_REPO in .env if omitted)." },
                    "org": { "type": "string", "description": "Organization name (required for list_org_repos)." },
                    "username": { "type": "string", "description": "Username for events check." },
                    "page": { "type": "integer", "description": "Page number for pagination (default: 1)." },
                    "filter": {
                        "type": "string",
                        "enum": ["assigned", "created", "mentioned", "subscribed", "repos", "all"],
                        "description": "Filter for listing issues (default: assigned)."
                    },
                    "state": {
                        "type": "string",
                        "enum": ["open", "closed", "all"],
                        "description": "State of issues to return (default: open)."
                    },
                    "pr_number": { "type": "integer", "description": "Pull request number. Required for pr_details, pr_checks, pr_commits, and update_pr." },
                    "title": { "type": "string", "description": "New PR title (optional, for update_pr)." },
                    "description": { "type": "string", "description": "New PR description/body (optional, for update_pr). At least one of title/description is required." }
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
                tool_call_id: None,
                tool_name: "github_authenticated".to_string(),
                result: "GITHUB_TOKEN is not set. This tool requires authentication.".to_string(),
            });
        }

        let result = match action {
            "notifications" => match self.check_notifications().await {
                Ok(data) => format!(
                    "🔔 **Your Notifications**\n\n{}",
                    self.format_notifications(&data)
                ),
                Err(e) => format!("Failed: {}", e),
            },
            "list_my_repos" => {
                let page = args.get("page").and_then(|v| v.as_u64()).unwrap_or(1) as u32;
                match self.list_my_repos(page).await {
                    Ok(data) => format!(
                        "📂 **Your Managed Repositories (Page {})**\n\n{}",
                        page,
                        self.format_repo_list(&data)
                    ),
                    Err(e) => format!("Failed: {}", e),
                }
            }
            "list_org_repos" => {
                let org = args
                    .get("org")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("'org' is required for list_org_repos"))?;
                let page = args.get("page").and_then(|v| v.as_u64()).unwrap_or(1) as u32;
                match self.list_org_repos(org, page).await {
                    Ok(data) => format!(
                        "🏢 **Repositories for Organization: {} (Page {})**\n\n{}",
                        org,
                        page,
                        self.format_repo_list(&data)
                    ),
                    Err(e) => format!("Failed: {}", e),
                }
            }
            "actions" => {
                let mut owner = args.get("owner").and_then(|v| v.as_str()).unwrap_or("");
                if owner.is_empty() {
                    owner = &self.owner;
                }
                let repo = args.get("repo").and_then(|v| v.as_str()).unwrap_or("");
                if owner.is_empty() || repo.is_empty() {
                    return Err(anyhow::anyhow!(
                        "Owner and repo required (add GITHUB_OWNER to .env if owner omitted)"
                    ));
                }

                match self.check_workflow_runs(owner, repo).await {
                    Ok(data) => format!(
                        "🏃 **Workflows for {}/{}**\n\n{}",
                        owner,
                        repo,
                        self.format_workflow_runs(&data)
                    ),
                    Err(e) => format!("Failed: {}", e),
                }
            }
            "issues" => {
                let mut owner = args.get("owner").and_then(|v| v.as_str()).unwrap_or("");
                if owner.is_empty() {
                    owner = &self.owner;
                }
                let repo = args.get("repo").and_then(|v| v.as_str()).unwrap_or("");
                let page = args.get("page").and_then(|v| v.as_u64()).unwrap_or(1) as u32;

                if owner.is_empty() || repo.is_empty() {
                    // List issues assigned to authenticated user
                    let filter = args
                        .get("filter")
                        .and_then(|v| v.as_str())
                        .unwrap_or("assigned");
                    let state = args.get("state").and_then(|v| v.as_str()).unwrap_or("open");

                    match self.list_authenticated_issues(filter, state, page).await {
                        Ok(data) => format!(
                            "🐛 **Issues ({}, {}) Page {}**\n\n{}",
                            filter,
                            state,
                            page,
                            self.format_issues(&data)
                        ),
                        Err(e) => format!("Failed: {}", e),
                    }
                } else {
                    match self.list_issues(owner, repo).await {
                        Ok(data) => format!(
                            "🐛 **Issues for {}/{}**\n\n{}",
                            owner,
                            repo,
                            self.format_issues(&data)
                        ),
                        Err(e) => format!("Failed: {}", e),
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
                    Err(e) => format!("Failed: {}", e),
                }
            }
            "pulls" => {
                let mut owner = args.get("owner").and_then(|v| v.as_str()).unwrap_or("");
                if owner.is_empty() {
                    owner = &self.owner;
                }
                let repo = args.get("repo").and_then(|v| v.as_str()).unwrap_or("");

                if owner.is_empty() || repo.is_empty() {
                    // Use issues endpoint but perhaps filter differently?
                    // For now reusing list_authenticated_issues ("assigned") but user might want "created" or "mentioned"
                    // Implementation choice: list default (assigned) for 'pulls' context too, or we can use "all"
                    let page = args.get("page").and_then(|v| v.as_u64()).unwrap_or(1) as u32;
                    match self
                        .list_authenticated_issues("assigned", "open", page)
                        .await
                    {
                        Ok(data) => format!(
                            "🔃 **Your Pull Requests & Issues**\n\n{}",
                            self.format_issues(&data)
                        ),
                        Err(e) => format!("Failed: {}", e),
                    }
                } else {
                    match self.list_pulls(owner, repo).await {
                        Ok(data) => format!(
                            "🔃 **Pull Requests for {}/{}**\n\n{}",
                            owner,
                            repo,
                            self.format_pulls(&data)
                        ),
                        Err(e) => format!("Failed: {}", e),
                    }
                }
            }
            "followers" => match self.get_my_profile().await {
                Ok(data) => format!("👥 **Followers**\n\n{}", self.format_followers(&data)),
                Err(e) => format!("Failed: {}", e),
            },
            "pr_details" => {
                let (owner, repo, pr_number) = self.resolve_pr_target(&args)?;
                match self.get_pull_request(owner, repo, pr_number).await {
                    Ok(data) => format!(
                        "🔃 **PR #{} for {}/{}**\n\n{}",
                        pr_number,
                        owner,
                        repo,
                        self.format_pr_details(&data)
                    ),
                    Err(e) => format!("Failed: {}", e),
                }
            }
            "pr_checks" => {
                let (owner, repo, pr_number) = self.resolve_pr_target(&args)?;
                match self.get_pull_request(owner, repo, pr_number).await {
                    Ok(pr) => {
                        let sha = pr["head"]["sha"].as_str().unwrap_or("").to_string();
                        if sha.is_empty() {
                            "Failed: could not determine the PR's head commit SHA".to_string()
                        } else {
                            let status = self.get_combined_status(owner, repo, &sha).await;
                            let checks = self.get_check_runs(owner, repo, &sha).await;
                            match (status, checks) {
                                (Ok(status), Ok(checks)) => format!(
                                    "✅ **CI Status for PR #{} ({}/{})**\n\n{}",
                                    pr_number,
                                    owner,
                                    repo,
                                    self.format_pr_checks(&status, &checks)
                                ),
                                (Err(e), _) | (_, Err(e)) => format!("Failed: {}", e),
                            }
                        }
                    }
                    Err(e) => format!("Failed: {}", e),
                }
            }
            "pr_commits" => {
                let (owner, repo, pr_number) = self.resolve_pr_target(&args)?;
                match self.list_pull_commits(owner, repo, pr_number).await {
                    Ok(data) => format!(
                        "📜 **Commits for PR #{} ({}/{})**\n\n{}",
                        pr_number,
                        owner,
                        repo,
                        self.format_pr_commits(&data)
                    ),
                    Err(e) => format!("Failed: {}", e),
                }
            }
            "update_pr" => {
                let (owner, repo, pr_number) = self.resolve_pr_target(&args)?;
                let title = args.get("title").and_then(|v| v.as_str());
                let description = args.get("description").and_then(|v| v.as_str());
                if title.is_none() && description.is_none() {
                    return Err(anyhow::anyhow!(
                        "At least one of 'title' or 'description' is required for update_pr"
                    ));
                }
                match self
                    .update_pull_request(owner, repo, pr_number, title, description)
                    .await
                {
                    Ok(data) => format!(
                        "✏️ **Updated PR #{} ({}/{})**\n\n{}",
                        pr_number,
                        owner,
                        repo,
                        self.format_pr_details(&data)
                    ),
                    Err(e) => format!("Failed: {}", e),
                }
            }
            _ => return Err(anyhow::anyhow!("Unknown action: {}", action)),
        };

        Ok(ToolCallResult {
            tool_call_id: None,
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
        assert_eq!(tool.metadata().name, "GitHub Public");
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

    use crate::api::agent::core::types::FunctionCall;
    use crate::test_support::{MockHttpApi, MockResponse};

    fn public_call(arguments: &str) -> ToolCall {
        ToolCall {
            id: "call_gh_public".to_string(),
            tool_type: "function".to_string(),
            function: FunctionCall {
                name: "github_public".to_string(),
                arguments: arguments.to_string(),
            },
        }
    }

    fn auth_call(arguments: &str) -> ToolCall {
        ToolCall {
            id: "call_gh_auth".to_string(),
            tool_type: "function".to_string(),
            function: FunctionCall {
                name: "github_authenticated".to_string(),
                arguments: arguments.to_string(),
            },
        }
    }

    /// An authenticated tool with a known token and default owner.
    fn auth_tool(api: &MockHttpApi) -> GitHubAuthenticatedTool {
        GitHubAuthenticatedTool::with_base_url(api.base_url(), "test-token", "default-owner")
    }

    fn repo(name: &str, stars: u64) -> serde_json::Value {
        json!({
            "full_name": name,
            "description": format!("{} description", name),
            "stargazers_count": stars,
            "html_url": format!("https://github.example/{}", name),
            "language": "Rust"
        })
    }

    /// The three headers `create_github_client` always sets.
    fn assert_standard_headers(request: &crate::test_support::MockRequest) {
        assert_eq!(request.header("user-agent"), Some("ai-agent-tool/1.0"));
        assert_eq!(
            request.header("accept"),
            Some("application/vnd.github.v3+json")
        );
        assert_eq!(request.header("x-github-api-version"), Some("2022-11-28"));
    }

    // ========================================================================
    // Public tool
    // ========================================================================

    #[tokio::test]
    async fn public_search_sends_the_query_and_renders_the_items() {
        let api = MockHttpApi::serving(
            "GET",
            "/search/repositories",
            MockResponse::json(json!({"items": [repo("rust-lang/rust", 95000)]})),
        )
        .await;

        let result = GitHubPublicTool::with_base_url(api.base_url())
            .execute(&public_call(
                r#"{"action": "search", "query": "language:rust cli"}"#,
            ))
            .await
            .expect("The search should succeed");

        let request = api.only_request();
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/search/repositories");
        assert_eq!(
            request.query_params(),
            vec![
                ("q".to_string(), "language:rust cli".to_string()),
                ("sort".to_string(), "stars".to_string()),
                ("per_page".to_string(), "5".to_string()),
            ]
        );
        assert_standard_headers(&request);

        assert_eq!(result.tool_name, "github_public");
        assert!(result.tool_call_id.is_none());
        assert!(result.result.starts_with("🔍 **GitHub Search Results**"));
        assert!(result.result.contains(
            "- **[rust-lang/rust](https://github.example/rust-lang/rust)** (⭐ 95000 | Rust)"
        ));
        assert!(result.result.contains("rust-lang/rust description"));
        api.stop().await;
    }

    #[tokio::test]
    async fn public_trending_filters_by_creation_date_and_language() {
        let api = MockHttpApi::start().await;
        api.on(
            "GET",
            "/search/repositories",
            MockResponse::json(json!({"items": []})),
        );
        let tool = GitHubPublicTool::with_base_url(api.base_url());

        let before = Utc::now();
        for timeframe in ["daily", "weekly", "monthly", "hourly"] {
            tool.execute(&public_call(
                &json!({"action": "trending", "timeframe": timeframe, "language": "rust"})
                    .to_string(),
            ))
            .await
            .expect("Trending should succeed");
        }
        let after = Utc::now();

        // An unknown timeframe falls back to the daily window.
        let windows = [
            chrono::Duration::days(1),
            chrono::Duration::weeks(1),
            chrono::Duration::days(30),
            chrono::Duration::days(1),
        ];
        let requests = api.requests();
        assert_eq!(requests.len(), 4);
        for (request, window) in requests.iter().zip(windows) {
            let query = request.query_param("q").expect("a q parameter");
            // The date is computed from "now", so accept either side of a
            // midnight rollover mid-test.
            let acceptable = [
                format!(
                    "created:>{} language:rust",
                    (before - window).format("%Y-%m-%d")
                ),
                format!(
                    "created:>{} language:rust",
                    (after - window).format("%Y-%m-%d")
                ),
            ];
            assert!(
                acceptable.contains(&query),
                "{} not in {:?}",
                query,
                acceptable
            );
            assert_eq!(request.query_param("sort").as_deref(), Some("stars"));
        }
        api.stop().await;
    }

    #[tokio::test]
    async fn public_trending_without_a_language_sends_only_the_date_filter() {
        let api = MockHttpApi::serving(
            "GET",
            "/search/repositories",
            MockResponse::json(json!({"items": []})),
        )
        .await;

        let result = GitHubPublicTool::with_base_url(api.base_url())
            .execute(&public_call(r#"{"action": "trending"}"#))
            .await
            .expect("Trending should succeed");

        let query = api.only_request().query_param("q").expect("a q parameter");
        assert!(query.starts_with("created:>"), "{}", query);
        assert!(!query.contains("language:"), "{}", query);
        // An empty item list is reported rather than rendered as nothing.
        assert!(result.result.contains("No repositories found."));
        api.stop().await;
    }

    #[tokio::test]
    async fn public_user_repos_reads_the_users_repo_list() {
        // This endpoint returns a bare array rather than a search envelope.
        let api = MockHttpApi::serving(
            "GET",
            "/users/octocat/repos",
            MockResponse::json(json!([repo("octocat/hello", 12), {"full_name": "octocat/bare"}])),
        )
        .await;

        let result = GitHubPublicTool::with_base_url(api.base_url())
            .execute(&public_call(
                r#"{"action": "user_repos", "username": "octocat"}"#,
            ))
            .await
            .expect("Listing user repos should succeed");

        let request = api.only_request();
        assert_eq!(request.path, "/users/octocat/repos");
        assert_eq!(
            request.query_params(),
            vec![
                ("sort".to_string(), "updated".to_string()),
                ("per_page".to_string(), "10".to_string()),
            ]
        );

        assert!(result.result.starts_with("📂 **Repositories for octocat**"));
        assert!(result.result.contains("octocat/hello"));
        // Missing fields fall back rather than failing.
        assert!(result
            .result
            .contains("**[octocat/bare]()** (⭐ 0 | Unknown)"));
        assert!(result.result.contains("No description"));
        api.stop().await;
    }

    #[tokio::test]
    async fn public_non_list_payloads_report_no_repositories() {
        let api = MockHttpApi::serving(
            "GET",
            "/search/repositories",
            MockResponse::json(json!({"message": "something else"})),
        )
        .await;

        let result = GitHubPublicTool::with_base_url(api.base_url())
            .execute(&public_call(r#"{"action": "search", "query": "x"}"#))
            .await
            .expect("An unexpected shape is not an error");

        assert!(result.result.contains("No repositories found."));
        api.stop().await;
    }

    #[tokio::test]
    async fn public_http_errors_and_malformed_bodies_fail_the_call() {
        let api = MockHttpApi::start().await;
        api.on_sequence(
            "GET",
            "/search/repositories",
            vec![
                MockResponse::error(422, "Validation Failed"),
                MockResponse::raw(200, "application/json", "{\"items\":"),
            ],
        );
        api.on(
            "GET",
            "/users/ghost/repos",
            MockResponse::error(404, "Not Found"),
        );
        let tool = GitHubPublicTool::with_base_url(api.base_url());

        assert_eq!(
            tool.execute(&public_call(r#"{"action": "search", "query": "x"}"#))
                .await
                .expect_err("A 422 must fail the call")
                .to_string(),
            "GitHub API error: 422 Unprocessable Entity"
        );
        assert_eq!(
            tool.execute(&public_call(r#"{"action": "search", "query": "x"}"#))
                .await
                .expect_err("A truncated body must fail the call")
                .to_string(),
            "Failed to parse search response"
        );
        assert_eq!(
            tool.execute(&public_call(
                r#"{"action": "user_repos", "username": "ghost"}"#
            ))
            .await
            .expect_err("A 404 must fail the call")
            .to_string(),
            "GitHub API error: 404 Not Found"
        );
        api.stop().await;
    }

    #[tokio::test]
    async fn public_bad_arguments_fail_before_any_request() {
        let api = MockHttpApi::serving(
            "GET",
            "/search/repositories",
            MockResponse::json(json!({"items": []})),
        )
        .await;
        let tool = GitHubPublicTool::with_base_url(api.base_url());

        assert_eq!(
            tool.execute(&public_call("not json"))
                .await
                .expect_err("Unparseable arguments must fail")
                .to_string(),
            "Failed to parse arguments"
        );
        assert_eq!(
            tool.execute(&public_call(r#"{"action": "delete_everything"}"#))
                .await
                .expect_err("An unknown action must fail")
                .to_string(),
            "Unknown action: delete_everything"
        );
        assert_eq!(
            tool.execute(&public_call("{}"))
                .await
                .expect_err("A missing action must fail")
                .to_string(),
            "Unknown action: "
        );
        assert_eq!(
            tool.execute(&public_call(r#"{"action": "search"}"#))
                .await
                .expect_err("Search without a query must fail")
                .to_string(),
            "'query' is required for search"
        );
        assert_eq!(
            tool.execute(&public_call(r#"{"action": "user_repos"}"#))
                .await
                .expect_err("user_repos without a username must fail")
                .to_string(),
            "'username' is required for user_repos"
        );

        assert_eq!(api.call_count(), 0, "Nothing should have reached GitHub");
        api.stop().await;
    }

    // ========================================================================
    // Authenticated tool
    // ========================================================================

    #[tokio::test]
    async fn authenticated_tool_without_a_token_refuses_without_calling_out() {
        let api = MockHttpApi::start().await;
        let tool = GitHubAuthenticatedTool::with_base_url(api.base_url(), "", "");

        assert!(!tool.is_available());
        let result = tool
            .execute(&auth_call(r#"{"action": "notifications"}"#))
            .await
            .expect("A tokenless call reports rather than errors");

        assert_eq!(
            result.result,
            "GITHUB_TOKEN is not set. This tool requires authentication."
        );
        assert_eq!(api.call_count(), 0, "Nothing should have reached GitHub");
        api.stop().await;
    }

    #[tokio::test]
    async fn notifications_are_fetched_with_the_bearer_token_and_rendered() {
        let api = MockHttpApi::serving(
            "GET",
            "/notifications",
            MockResponse::json(json!([{
                "subject": {"title": "Fix the build", "type": "PullRequest"},
                "repository": {"full_name": "acme/widgets"}
            }])),
        )
        .await;

        let result = auth_tool(&api)
            .execute(&auth_call(r#"{"action": "notifications"}"#))
            .await
            .expect("Notifications should succeed");

        let request = api.only_request();
        assert_eq!(request.path, "/notifications");
        assert_eq!(
            request.query_params(),
            vec![
                ("all".to_string(), "false".to_string()),
                ("per_page".to_string(), "10".to_string()),
            ]
        );
        assert_eq!(request.header("authorization"), Some("Bearer test-token"));
        assert_standard_headers(&request);

        assert_eq!(result.tool_name, "github_authenticated");
        assert!(result.result.starts_with("🔔 **Your Notifications**"));
        assert!(result.result.contains("- **PullRequest**: Fix the build"));
        assert!(result.result.contains("Repo: acme/widgets"));
        api.stop().await;
    }

    #[tokio::test]
    async fn an_empty_or_unexpected_notification_payload_is_described() {
        let api = MockHttpApi::start().await;
        api.on_sequence(
            "GET",
            "/notifications",
            vec![
                MockResponse::json(json!([])),
                MockResponse::json(json!({"message": "not an array"})),
            ],
        );
        let tool = auth_tool(&api);
        let call = auth_call(r#"{"action": "notifications"}"#);

        assert!(tool
            .execute(&call)
            .await
            .expect("An empty inbox is not an error")
            .result
            .contains("No new notifications! 🎉"));
        assert!(tool
            .execute(&call)
            .await
            .expect("A non-array payload is not an error")
            .result
            .contains("No notifications found."));
        api.stop().await;
    }

    #[tokio::test]
    async fn list_my_repos_paginates_and_reports_a_403_as_a_scope_hint() {
        let api = MockHttpApi::start().await;
        api.on_sequence(
            "GET",
            "/user/repos",
            vec![
                MockResponse::json(json!([repo("me/one", 1)])),
                MockResponse::error(403, "Forbidden"),
            ],
        );
        let tool = auth_tool(&api);

        let ok = tool
            .execute(&auth_call(r#"{"action": "list_my_repos", "page": 3}"#))
            .await
            .expect("Listing repos should succeed");
        assert!(ok
            .result
            .starts_with("📂 **Your Managed Repositories (Page 3)**"));
        assert!(ok
            .result
            .contains("- **[me/one](https://github.example/me/one)** (⭐ 1)"));

        let requests = api.requests();
        assert_eq!(
            requests[0].query_params(),
            vec![
                ("sort".to_string(), "updated".to_string()),
                ("per_page".to_string(), "100".to_string()),
                ("type".to_string(), "owner".to_string()),
                ("page".to_string(), "3".to_string()),
            ]
        );

        // Transport-level failures are folded into the result text, not returned
        // as errors.
        let forbidden = tool
            .execute(&auth_call(r#"{"action": "list_my_repos"}"#))
            .await
            .expect("A 403 is reported in the result, not as an error");
        assert!(
            forbidden.result.contains("Failed: Access Forbidden (403)"),
            "{}",
            forbidden.result
        );
        assert!(forbidden.result.contains("'metadata' or 'contents' scope"));
        // The default page is 1.
        assert_eq!(api.requests()[1].query_param("page").as_deref(), Some("1"));
        api.stop().await;
    }

    #[tokio::test]
    async fn list_org_repos_requires_an_org_and_reports_api_errors() {
        let api = MockHttpApi::start().await;
        api.on_sequence(
            "GET",
            "/orgs/acme/repos",
            vec![
                MockResponse::json(json!([repo("acme/widgets", 7)])),
                MockResponse::error(404, "Not Found"),
            ],
        );
        let tool = auth_tool(&api);

        let ok = tool
            .execute(&auth_call(r#"{"action": "list_org_repos", "org": "acme"}"#))
            .await
            .expect("Listing org repos should succeed");
        assert!(ok
            .result
            .starts_with("🏢 **Repositories for Organization: acme (Page 1)**"));
        assert!(ok.result.contains("acme/widgets"));
        assert_eq!(
            api.only_request().query_params(),
            vec![
                ("sort".to_string(), "updated".to_string()),
                ("per_page".to_string(), "100".to_string()),
                ("page".to_string(), "1".to_string()),
            ]
        );

        let failed = tool
            .execute(&auth_call(r#"{"action": "list_org_repos", "org": "acme"}"#))
            .await
            .expect("A 404 is reported in the result");
        assert!(
            failed
                .result
                .contains("Failed: GitHub API error: 404 Not Found"),
            "{}",
            failed.result
        );

        assert_eq!(
            tool.execute(&auth_call(r#"{"action": "list_org_repos"}"#))
                .await
                .expect_err("Without an org the call must fail")
                .to_string(),
            "'org' is required for list_org_repos"
        );
        api.stop().await;
    }

    #[tokio::test]
    async fn workflow_runs_use_the_configured_owner_and_render_every_conclusion() {
        // No "owner" argument, so GITHUB_OWNER's stand-in is used.
        let api = MockHttpApi::serving(
            "GET",
            "/repos/default-owner/widgets/actions/runs",
            MockResponse::json(json!({"workflow_runs": [
                {"name": "ci", "status": "completed", "conclusion": "success", "html_url": "https://github.example/1"},
                {"name": "release", "status": "completed", "conclusion": "failure", "html_url": "https://github.example/2"},
                {"name": "nightly", "status": "completed", "conclusion": "cancelled", "html_url": "https://github.example/3"},
                {"name": "queued", "status": "queued", "html_url": "https://github.example/4"},
                {"name": "odd", "status": "completed", "conclusion": "neutral", "html_url": "https://github.example/5"}
            ]})),
        )
        .await;

        let result = auth_tool(&api)
            .execute(&auth_call(r#"{"action": "actions", "repo": "widgets"}"#))
            .await
            .expect("Reading workflow runs should succeed");

        let request = api.only_request();
        assert_eq!(request.path, "/repos/default-owner/widgets/actions/runs");
        assert_eq!(
            request.query_params(),
            vec![("per_page".to_string(), "5".to_string())]
        );

        assert!(result
            .result
            .starts_with("🏃 **Workflows for default-owner/widgets**"));
        assert!(result
            .result
            .contains("- ✅ **[ci](https://github.example/1)**"));
        assert!(result
            .result
            .contains("Status: completed | Result: success"));
        assert!(result.result.contains("- ❌ **[release]"));
        assert!(result.result.contains("- 🚫 **[nightly]"));
        // A run with no conclusion yet reads as pending.
        assert!(result.result.contains("- ⏳ **[queued]"));
        assert!(result.result.contains("Result: pending"));
        assert!(result.result.contains("- ❓ **[odd]"));
        api.stop().await;
    }

    #[tokio::test]
    async fn workflow_runs_report_403s_and_empty_payloads() {
        let api = MockHttpApi::start().await;
        api.on_sequence(
            "GET",
            "/repos/o/r/actions/runs",
            vec![
                MockResponse::error(403, "Forbidden"),
                MockResponse::json(json!({"workflow_runs": []})),
                MockResponse::json(json!({})),
            ],
        );
        let tool = auth_tool(&api);
        let call = auth_call(r#"{"action": "actions", "owner": "o", "repo": "r"}"#);

        let forbidden = tool.execute(&call).await.expect("A 403 is reported");
        assert!(
            forbidden
                .result
                .contains("ensure your GITHUB_TOKEN has the 'actions' scope"),
            "{}",
            forbidden.result
        );
        assert!(tool
            .execute(&call)
            .await
            .expect("An empty run list is not an error")
            .result
            .contains("No workflow runs found."));
        assert!(tool
            .execute(&call)
            .await
            .expect("A payload without workflow_runs is not an error")
            .result
            .contains("No workflow runs found."));

        // With neither an argument nor a configured owner there is nothing to ask
        // about, and the call fails outright.
        let ownerless = GitHubAuthenticatedTool::with_base_url(api.base_url(), "test-token", "");
        assert_eq!(
            ownerless
                .execute(&auth_call(r#"{"action": "actions", "repo": "r"}"#))
                .await
                .expect_err("Without an owner the call must fail")
                .to_string(),
            "Owner and repo required (add GITHUB_OWNER to .env if owner omitted)"
        );
        api.stop().await;
    }

    #[tokio::test]
    async fn issues_come_from_the_repo_when_one_is_named() {
        let api = MockHttpApi::serving(
            "GET",
            "/repos/acme/widgets/issues",
            MockResponse::json(json!([
                {"number": 7, "title": "Broken", "html_url": "https://github.example/i/7", "user": {"login": "jane"}},
                {}
            ])),
        )
        .await;

        let result = auth_tool(&api)
            .execute(&auth_call(
                r#"{"action": "issues", "owner": "acme", "repo": "widgets"}"#,
            ))
            .await
            .expect("Listing repo issues should succeed");

        let request = api.only_request();
        assert_eq!(request.path, "/repos/acme/widgets/issues");
        assert_eq!(
            request.query_params(),
            vec![
                ("state".to_string(), "open".to_string()),
                ("sort".to_string(), "updated".to_string()),
                ("per_page".to_string(), "5".to_string()),
            ]
        );

        assert!(result.result.starts_with("🐛 **Issues for acme/widgets**"));
        assert!(result
            .result
            .contains("- **#7 [Broken](https://github.example/i/7)** by @jane"));
        assert!(result.result.contains("- **#0 [No title]()** by @unknown"));
        api.stop().await;
    }

    #[tokio::test]
    async fn issues_fall_back_to_the_authenticated_users_list() {
        let api = MockHttpApi::start().await;
        api.on_sequence(
            "GET",
            "/issues",
            vec![
                MockResponse::json(json!([{"number": 1, "title": "Mine", "html_url": "https://github.example/i/1", "user": {"login": "me"}}])),
                MockResponse::json(json!([])),
                MockResponse::json(json!({"message": "not an array"})),
                MockResponse::error(500, "boom"),
            ],
        );
        let tool = auth_tool(&api);

        // An owner is configured but no repo, so this is the "my issues" path.
        let mine = tool
            .execute(&auth_call(
                r#"{"action": "issues", "filter": "created", "state": "closed", "page": 2}"#,
            ))
            .await
            .expect("Listing my issues should succeed");
        assert!(mine
            .result
            .starts_with("🐛 **Issues (created, closed) Page 2**"));
        assert!(mine.result.contains("- **#1 [Mine]"));
        assert_eq!(
            api.only_request().query_params(),
            vec![
                ("filter".to_string(), "created".to_string()),
                ("state".to_string(), "closed".to_string()),
                ("sort".to_string(), "updated".to_string()),
                ("per_page".to_string(), "100".to_string()),
                ("page".to_string(), "2".to_string()),
            ]
        );

        // Defaults: assigned + open + page 1.
        let defaults = tool
            .execute(&auth_call(r#"{"action": "issues"}"#))
            .await
            .expect("Listing my issues should succeed");
        assert!(defaults
            .result
            .starts_with("🐛 **Issues (assigned, open) Page 1**"));
        assert!(defaults.result.contains("No issues found."));

        assert!(tool
            .execute(&auth_call(r#"{"action": "issues"}"#))
            .await
            .expect("A non-array payload is not an error")
            .result
            .contains("No issues found."));
        assert!(tool
            .execute(&auth_call(r#"{"action": "issues"}"#))
            .await
            .expect("A 500 is reported in the result")
            .result
            .contains("Failed: GitHub API error: 500 Internal Server Error"));
        api.stop().await;
    }

    #[tokio::test]
    async fn events_need_a_username_and_render_the_date_only() {
        let api = MockHttpApi::start().await;
        api.on_sequence(
            "GET",
            "/users/octocat/events",
            vec![
                MockResponse::json(json!([
                    {"type": "PushEvent", "repo": {"name": "octocat/hello"}, "created_at": "2026-08-01T10:00:00Z"},
                    {}
                ])),
                MockResponse::json(json!([])),
                MockResponse::json(json!({"message": "not an array"})),
            ],
        );
        let tool = auth_tool(&api);
        let call = auth_call(r#"{"action": "events", "username": "octocat"}"#);

        let result = tool.execute(&call).await.expect("Events should succeed");
        assert_eq!(
            api.only_request().query_params(),
            vec![("per_page".to_string(), "5".to_string())]
        );
        assert!(result.result.starts_with("📅 **Events for octocat**"));
        assert!(result.result.contains("- **PushEvent** at octocat/hello"));
        // Only the date part of created_at is shown.
        assert!(result.result.contains("Date: 2026-08-01"));
        assert!(result.result.contains("- **Event** at unknown"));

        assert!(tool
            .execute(&call)
            .await
            .expect("An empty event list is not an error")
            .result
            .contains("No events found."));
        assert!(tool
            .execute(&call)
            .await
            .expect("A non-array payload is not an error")
            .result
            .contains("No events found."));

        assert_eq!(
            tool.execute(&auth_call(r#"{"action": "events"}"#))
                .await
                .expect_err("Without a username the call must fail")
                .to_string(),
            "Username required for events"
        );
        api.stop().await;
    }

    #[tokio::test]
    async fn pulls_come_from_the_repo_when_one_is_named() {
        let api = MockHttpApi::start().await;
        api.on_sequence(
            "GET",
            "/repos/acme/widgets/pulls",
            vec![
                MockResponse::json(json!([
                    {"title": "Add feature", "html_url": "https://github.example/p/1", "user": {"login": "jane"}}
                ])),
                MockResponse::json(json!([])),
                MockResponse::json(json!({"message": "not an array"})),
            ],
        );
        let tool = auth_tool(&api);
        let call = auth_call(r#"{"action": "pulls", "owner": "acme", "repo": "widgets"}"#);

        let result = tool.execute(&call).await.expect("Pulls should succeed");
        assert_eq!(
            api.only_request().query_params(),
            vec![
                ("state".to_string(), "open".to_string()),
                ("per_page".to_string(), "5".to_string()),
                ("sort".to_string(), "updated".to_string()),
                ("direction".to_string(), "desc".to_string()),
            ]
        );
        assert!(result
            .result
            .starts_with("🔃 **Pull Requests for acme/widgets**"));
        assert!(result
            .result
            .contains("- **[Add feature](https://github.example/p/1)** by @jane"));

        assert!(tool
            .execute(&call)
            .await
            .expect("An empty pull list is not an error")
            .result
            .contains("No pull requests found."));
        assert!(tool
            .execute(&call)
            .await
            .expect("A non-array payload is not an error")
            .result
            .contains("No pull requests found."));
        api.stop().await;
    }

    #[tokio::test]
    async fn pulls_without_a_repo_fall_back_to_the_issues_endpoint() {
        let api = MockHttpApi::start().await;
        api.on_sequence(
            "GET",
            "/issues",
            vec![
                MockResponse::json(json!([{"number": 4, "title": "Mine", "html_url": "https://github.example/i/4", "user": {"login": "me"}}])),
                MockResponse::error(500, "boom"),
            ],
        );
        let tool = auth_tool(&api);

        let result = tool
            .execute(&auth_call(r#"{"action": "pulls", "page": 5}"#))
            .await
            .expect("The fallback should succeed");

        assert!(result
            .result
            .starts_with("🔃 **Your Pull Requests & Issues**"));
        assert!(result.result.contains("- **#4 [Mine]"));
        // The fallback always asks for assigned+open, but honours the page.
        assert_eq!(
            api.only_request().query_params(),
            vec![
                ("filter".to_string(), "assigned".to_string()),
                ("state".to_string(), "open".to_string()),
                ("sort".to_string(), "updated".to_string()),
                ("per_page".to_string(), "100".to_string()),
                ("page".to_string(), "5".to_string()),
            ]
        );

        assert!(tool
            .execute(&auth_call(r#"{"action": "pulls"}"#))
            .await
            .expect("A 500 is reported in the result")
            .result
            .contains("Failed: GitHub API error: 500 Internal Server Error"));
        api.stop().await;
    }

    #[tokio::test]
    async fn followers_come_from_the_authenticated_user_profile() {
        let api = MockHttpApi::start().await;
        api.on_sequence(
            "GET",
            "/user",
            vec![
                MockResponse::json(json!({"login": "octocat", "followers": 12, "following": 3})),
                MockResponse::json(json!({})),
                MockResponse::error(401, "Bad credentials"),
            ],
        );
        let tool = auth_tool(&api);
        let call = auth_call(r#"{"action": "followers"}"#);

        let result = tool.execute(&call).await.expect("Followers should succeed");
        let request = api.only_request();
        assert_eq!(request.path, "/user");
        assert_eq!(request.query, "");
        assert!(result.result.starts_with("👥 **Followers**"));
        assert!(result
            .result
            .contains("**@octocat** has **12** followers (following 3)."));

        assert!(tool
            .execute(&call)
            .await
            .expect("An empty profile is not an error")
            .result
            .contains("**@unknown** has **0** followers (following 0)."));
        assert!(tool
            .execute(&call)
            .await
            .expect("A 401 is reported in the result")
            .result
            .contains("Failed: GitHub API error: 401 Unauthorized"));
        api.stop().await;
    }

    #[tokio::test]
    async fn authenticated_bad_arguments_fail_before_any_request() {
        let api = MockHttpApi::start().await;
        let tool = auth_tool(&api);

        assert_eq!(
            tool.execute(&auth_call("{{"))
                .await
                .expect_err("Unparseable arguments must fail")
                .to_string(),
            "Failed to parse arguments"
        );
        assert_eq!(
            tool.execute(&auth_call(r#"{"action": "merge_everything"}"#))
                .await
                .expect_err("An unknown action must fail")
                .to_string(),
            "Unknown action: merge_everything"
        );
        assert_eq!(api.call_count(), 0, "Nothing should have reached GitHub");
        api.stop().await;
    }

    #[test]
    fn the_authenticated_function_definition_advertises_every_action() {
        let def = GitHubAuthenticatedTool::new().get_function_definition();
        assert_eq!(def["name"], "github_authenticated");
        assert_eq!(def["parameters"]["required"], json!(["action"]));
        assert_eq!(
            def["parameters"]["properties"]["action"]["enum"],
            json!([
                "notifications",
                "list_my_repos",
                "list_org_repos",
                "actions",
                "issues",
                "events",
                "pulls",
                "followers",
                "pr_details",
                "pr_checks",
                "pr_commits",
                "update_pr"
            ])
        );
        for parameter in [
            "owner",
            "repo",
            "org",
            "username",
            "page",
            "filter",
            "state",
            "pr_number",
            "title",
            "description",
        ] {
            assert!(
                def["parameters"]["properties"].get(parameter).is_some(),
                "{} should be documented",
                parameter
            );
        }
    }

    #[tokio::test]
    async fn the_per_method_token_guards_never_call_out() {
        // execute() already refuses a tokenless call, so these guards are
        // defensive only - they are checked directly here so the refusal is
        // pinned down rather than left untested.
        let api = MockHttpApi::start().await;
        let tool = GitHubAuthenticatedTool::with_base_url(api.base_url(), "", "owner");

        assert_eq!(
            tool.check_notifications()
                .await
                .expect_err("notifications must refuse")
                .to_string(),
            "GITHUB_TOKEN is required for notifications"
        );
        assert_eq!(
            tool.list_my_repos(1)
                .await
                .expect_err("list_my_repos must refuse")
                .to_string(),
            "GITHUB_TOKEN is required to list your repositories"
        );
        assert_eq!(
            tool.list_org_repos("acme", 1)
                .await
                .expect_err("list_org_repos must refuse")
                .to_string(),
            "GITHUB_TOKEN is required to list organization repositories"
        );
        assert_eq!(
            tool.get_my_profile()
                .await
                .expect_err("get_my_profile must refuse")
                .to_string(),
            "GITHUB_TOKEN is required to check followers"
        );

        assert_eq!(api.call_count(), 0, "Nothing should have reached GitHub");
        api.stop().await;
    }

    #[tokio::test]
    async fn repo_scoped_endpoints_report_their_own_http_errors() {
        let api = MockHttpApi::start().await;
        api.on(
            "GET",
            "/repos/acme/widgets/issues",
            MockResponse::error(404, "Not Found"),
        );
        api.on(
            "GET",
            "/users/ghost/events",
            MockResponse::error(404, "Not Found"),
        );
        api.on(
            "GET",
            "/repos/acme/widgets/pulls",
            MockResponse::error(451, "Repository unavailable"),
        );
        let tool = auth_tool(&api);

        for (arguments, expected) in [
            (
                r#"{"action": "issues", "owner": "acme", "repo": "widgets"}"#,
                "Failed: GitHub API error: 404 Not Found",
            ),
            (
                r#"{"action": "events", "username": "ghost"}"#,
                "Failed: GitHub API error: 404 Not Found",
            ),
            (
                r#"{"action": "pulls", "owner": "acme", "repo": "widgets"}"#,
                "Failed: GitHub API error: 451 Unavailable For Legal Reasons",
            ),
        ] {
            let result = tool
                .execute(&auth_call(arguments))
                .await
                .expect("Transport failures are reported in the result");
            assert!(
                result.result.contains(expected),
                "{} -> {}",
                arguments,
                result.result
            );
        }
        api.stop().await;
    }

    #[tokio::test]
    async fn the_authenticated_repo_formatter_handles_envelopes_and_empty_lists() {
        let api = MockHttpApi::start().await;
        api.on_sequence(
            "GET",
            "/user/repos",
            vec![
                // A search-style envelope rather than a bare array.
                MockResponse::json(json!({"items": [repo("me/enveloped", 4)]})),
                MockResponse::json(json!([])),
                MockResponse::json(json!({"message": "not a list at all"})),
            ],
        );
        let tool = auth_tool(&api);
        let call = auth_call(r#"{"action": "list_my_repos"}"#);

        assert!(tool
            .execute(&call)
            .await
            .expect("An enveloped payload should render")
            .result
            .contains("- **[me/enveloped](https://github.example/me/enveloped)** (⭐ 4)"));
        assert!(tool
            .execute(&call)
            .await
            .expect("An empty list is not an error")
            .result
            .contains("No repositories found."));
        assert!(tool
            .execute(&call)
            .await
            .expect("A non-list payload is not an error")
            .result
            .contains("No repositories found."));
        api.stop().await;
    }

    #[tokio::test]
    async fn non_forbidden_failures_are_reported_for_every_authenticated_endpoint() {
        let api = MockHttpApi::start().await;
        api.on("GET", "/notifications", MockResponse::error(500, "boom"));
        api.on(
            "GET",
            "/user/repos",
            MockResponse::error(502, "bad gateway"),
        );
        api.on(
            "GET",
            "/repos/o/r/actions/runs",
            MockResponse::error(404, "Not Found"),
        );
        let tool = auth_tool(&api);

        for (arguments, expected) in [
            (
                r#"{"action": "notifications"}"#,
                "Failed: GitHub API error: 500 Internal Server Error",
            ),
            (
                r#"{"action": "list_my_repos"}"#,
                "Failed: GitHub API error: 502 Bad Gateway",
            ),
            (
                r#"{"action": "actions", "owner": "o", "repo": "r"}"#,
                "Failed: GitHub API error: 404 Not Found",
            ),
        ] {
            let result = tool
                .execute(&auth_call(arguments))
                .await
                .expect("Transport failures are reported in the result");
            assert!(
                result.result.contains(expected),
                "{} -> {}",
                arguments,
                result.result
            );
        }
        api.stop().await;
    }

    #[tokio::test]
    async fn list_org_repos_honours_an_explicit_page() {
        let api =
            MockHttpApi::serving("GET", "/orgs/acme/repos", MockResponse::json(json!([]))).await;

        let result = auth_tool(&api)
            .execute(&auth_call(
                r#"{"action": "list_org_repos", "org": "acme", "page": 4}"#,
            ))
            .await
            .expect("Listing org repos should succeed");

        assert!(result
            .result
            .starts_with("🏢 **Repositories for Organization: acme (Page 4)**"));
        assert_eq!(api.only_request().query_param("page").as_deref(), Some("4"));
        api.stop().await;
    }

    // ---------------------------------------------------------------------
    // pr_details / pr_checks / pr_commits / update_pr
    // ---------------------------------------------------------------------

    fn pull_request_fixture() -> serde_json::Value {
        json!({
            "number": 42,
            "title": "Add feature",
            "body": "This adds a feature.",
            "state": "open",
            "draft": false,
            "html_url": "https://github.example/p/42",
            "user": {"login": "jane"},
            "head": {"ref": "feature-branch", "sha": "abc123"},
            "base": {"ref": "main"}
        })
    }

    #[tokio::test]
    async fn pr_details_renders_title_state_branches_and_description() {
        let api = MockHttpApi::serving(
            "GET",
            "/repos/acme/widgets/pulls/42",
            MockResponse::json(pull_request_fixture()),
        )
        .await;

        let result = auth_tool(&api)
            .execute(&auth_call(
                r#"{"action": "pr_details", "owner": "acme", "repo": "widgets", "pr_number": 42}"#,
            ))
            .await
            .expect("Fetching PR details should succeed");

        assert!(result.result.starts_with("🔃 **PR #42 for acme/widgets**"));
        assert!(result.result.contains("Add feature"));
        assert!(result.result.contains("open"));
        assert!(result.result.contains("feature-branch"));
        assert!(result.result.contains("main"));
        assert!(result.result.contains("jane"));
        assert!(result.result.contains("This adds a feature."));
        api.stop().await;
    }

    #[tokio::test]
    async fn pr_details_reports_a_missing_pr() {
        let api = MockHttpApi::serving(
            "GET",
            "/repos/acme/widgets/pulls/999",
            MockResponse::error(404, r#"{"message": "Not Found"}"#),
        )
        .await;

        let result = auth_tool(&api)
            .execute(&auth_call(
                r#"{"action": "pr_details", "owner": "acme", "repo": "widgets", "pr_number": 999}"#,
            ))
            .await
            .expect("A 404 is reported in the result, not an Err");

        assert!(result.result.contains("Failed: GitHub API error: 404"));
        api.stop().await;
    }

    #[tokio::test]
    async fn pr_details_requires_a_pr_number() {
        let api = MockHttpApi::start().await;

        let err = auth_tool(&api)
            .execute(&auth_call(
                r#"{"action": "pr_details", "owner": "acme", "repo": "widgets"}"#,
            ))
            .await
            .expect_err("A missing pr_number must be rejected before any request");

        assert!(err.to_string().contains("pr_number"));
        assert_eq!(api.call_count(), 0);
        api.stop().await;
    }

    #[tokio::test]
    async fn pr_checks_combines_combined_status_and_check_runs() {
        let api = MockHttpApi::start().await;
        api.on(
            "GET",
            "/repos/acme/widgets/pulls/42",
            MockResponse::json(pull_request_fixture()),
        );
        api.on(
            "GET",
            "/repos/acme/widgets/commits/abc123/status",
            MockResponse::json(json!({
                "state": "failure",
                "statuses": [
                    {"context": "ci/build", "state": "success", "description": "Build passed"},
                    {"context": "ci/lint", "state": "failure", "description": "Lint failed"}
                ]
            })),
        );
        api.on(
            "GET",
            "/repos/acme/widgets/commits/abc123/check-runs",
            MockResponse::json(json!({
                "check_runs": [
                    {"name": "test-suite", "status": "completed", "conclusion": "success"}
                ]
            })),
        );

        let result = auth_tool(&api)
            .execute(&auth_call(
                r#"{"action": "pr_checks", "owner": "acme", "repo": "widgets", "pr_number": 42}"#,
            ))
            .await
            .expect("Fetching PR checks should succeed");

        assert!(result.result.contains("failure"));
        assert!(result.result.contains("ci/build"));
        assert!(result.result.contains("Build passed"));
        assert!(result.result.contains("ci/lint"));
        assert!(result.result.contains("Lint failed"));
        assert!(result.result.contains("test-suite"));
        assert_eq!(api.call_count(), 3);
        api.stop().await;
    }

    #[tokio::test]
    async fn pr_checks_reports_no_status_or_checks_found() {
        let api = MockHttpApi::start().await;
        api.on(
            "GET",
            "/repos/acme/widgets/pulls/42",
            MockResponse::json(pull_request_fixture()),
        );
        api.on(
            "GET",
            "/repos/acme/widgets/commits/abc123/status",
            MockResponse::json(json!({"state": "pending", "statuses": []})),
        );
        api.on(
            "GET",
            "/repos/acme/widgets/commits/abc123/check-runs",
            MockResponse::json(json!({"check_runs": []})),
        );

        let result = auth_tool(&api)
            .execute(&auth_call(
                r#"{"action": "pr_checks", "owner": "acme", "repo": "widgets", "pr_number": 42}"#,
            ))
            .await
            .expect("An empty status/checks payload is not an error");

        assert!(result
            .result
            .contains("No status checks or check runs found"));
        api.stop().await;
    }

    #[tokio::test]
    async fn pr_checks_fails_if_the_pr_itself_cannot_be_fetched() {
        let api = MockHttpApi::serving(
            "GET",
            "/repos/acme/widgets/pulls/42",
            MockResponse::error(404, r#"{"message": "Not Found"}"#),
        )
        .await;

        let result = auth_tool(&api)
            .execute(&auth_call(
                r#"{"action": "pr_checks", "owner": "acme", "repo": "widgets", "pr_number": 42}"#,
            ))
            .await
            .expect("A failed PR lookup is reported in the result");

        assert!(result.result.contains("Failed: GitHub API error: 404"));
        // Never got far enough to ask for status/checks.
        assert_eq!(api.call_count(), 1);
        api.stop().await;
    }

    #[tokio::test]
    async fn pr_commits_renders_the_commit_list() {
        let api = MockHttpApi::serving(
            "GET",
            "/repos/acme/widgets/pulls/42/commits",
            MockResponse::json(json!([
                {
                    "sha": "abc1234567890",
                    "html_url": "https://github.example/c/abc1234567890",
                    "commit": {
                        "message": "Fix the bug\n\nLonger description here.",
                        "author": {"name": "Jane Doe"}
                    }
                }
            ])),
        )
        .await;

        let result = auth_tool(&api)
            .execute(&auth_call(
                r#"{"action": "pr_commits", "owner": "acme", "repo": "widgets", "pr_number": 42}"#,
            ))
            .await
            .expect("Fetching PR commits should succeed");

        assert!(result.result.contains("abc1234"));
        assert!(result.result.contains("Fix the bug"));
        assert!(result.result.contains("Jane Doe"));
        // Only the first line of a multi-line commit message is rendered.
        assert!(!result.result.contains("Longer description here."));
        api.stop().await;
    }

    #[tokio::test]
    async fn pr_commits_reports_an_empty_list() {
        let api = MockHttpApi::serving(
            "GET",
            "/repos/acme/widgets/pulls/42/commits",
            MockResponse::json(json!([])),
        )
        .await;

        let result = auth_tool(&api)
            .execute(&auth_call(
                r#"{"action": "pr_commits", "owner": "acme", "repo": "widgets", "pr_number": 42}"#,
            ))
            .await
            .expect("An empty commit list is not an error");

        assert!(result.result.contains("No commits found"));
        api.stop().await;
    }

    #[tokio::test]
    async fn update_pr_sends_title_and_description_and_reports_success() {
        let api = MockHttpApi::serving(
            "PATCH",
            "/repos/acme/widgets/pulls/42",
            MockResponse::json(json!({
                "number": 42,
                "title": "New title",
                "body": "New description.",
                "state": "open",
                "draft": false,
                "html_url": "https://github.example/p/42",
                "user": {"login": "jane"},
                "head": {"ref": "feature-branch", "sha": "abc123"},
                "base": {"ref": "main"}
            })),
        )
        .await;

        let result = auth_tool(&api)
            .execute(&auth_call(
                r#"{"action": "update_pr", "owner": "acme", "repo": "widgets", "pr_number": 42, "title": "New title", "description": "New description."}"#,
            ))
            .await
            .expect("Updating a PR should succeed");

        let sent = api.only_request().json();
        assert_eq!(sent["title"], "New title");
        assert_eq!(sent["body"], "New description.");
        assert!(result
            .result
            .starts_with("✏️ **Updated PR #42 (acme/widgets)**"));
        assert!(result.result.contains("New title"));
        api.stop().await;
    }

    #[tokio::test]
    async fn update_pr_can_update_only_the_description() {
        let api = MockHttpApi::serving(
            "PATCH",
            "/repos/acme/widgets/pulls/42",
            MockResponse::json(pull_request_fixture()),
        )
        .await;

        auth_tool(&api)
            .execute(&auth_call(
                r#"{"action": "update_pr", "owner": "acme", "repo": "widgets", "pr_number": 42, "description": "Only the body changes."}"#,
            ))
            .await
            .expect("Updating just the description should succeed");

        let sent = api.only_request().json();
        assert_eq!(sent["body"], "Only the body changes.");
        // 'title' must be entirely absent, not sent as null/empty, so GitHub
        // doesn't interpret it as "clear the title".
        assert!(sent.get("title").is_none());
        api.stop().await;
    }

    #[tokio::test]
    async fn update_pr_requires_at_least_title_or_description() {
        let api = MockHttpApi::start().await;

        let err = auth_tool(&api)
            .execute(&auth_call(
                r#"{"action": "update_pr", "owner": "acme", "repo": "widgets", "pr_number": 42}"#,
            ))
            .await
            .expect_err("Neither title nor description must be rejected before any request");

        assert!(err.to_string().contains("title") || err.to_string().contains("description"));
        assert_eq!(api.call_count(), 0);
        api.stop().await;
    }

    #[tokio::test]
    async fn pr_details_falls_back_to_the_configured_default_repo_when_omitted() {
        // auth_tool() configures owner "default-owner" (see its definition above);
        // this test only omits `repo`, exercising the new GITHUB_REPO-equivalent
        // fallback via `.with_repo(...)`.
        let api = MockHttpApi::serving(
            "GET",
            "/repos/default-owner/widgets/pulls/42",
            MockResponse::json(pull_request_fixture()),
        )
        .await;

        let result = auth_tool(&api)
            .with_repo("widgets")
            .execute(&auth_call(r#"{"action": "pr_details", "pr_number": 42}"#))
            .await
            .expect("The configured default repo should be used when 'repo' is omitted");

        assert!(result
            .result
            .starts_with("🔃 **PR #42 for default-owner/widgets**"));
        api.stop().await;
    }

    #[tokio::test]
    async fn pr_details_requires_a_repo_when_none_is_configured() {
        let api = MockHttpApi::start().await;

        let err = auth_tool(&api)
            .execute(&auth_call(r#"{"action": "pr_details", "pr_number": 42}"#))
            .await
            .expect_err("With no 'repo' argument and no configured default, this must fail");

        assert!(err.to_string().contains("Owner and repo required"));
        assert_eq!(api.call_count(), 0);
        api.stop().await;
    }

    #[tokio::test]
    async fn update_pr_reports_a_github_error() {
        let api = MockHttpApi::serving(
            "PATCH",
            "/repos/acme/widgets/pulls/42",
            MockResponse::error(422, r#"{"message": "Validation Failed"}"#),
        )
        .await;

        let result = auth_tool(&api)
            .execute(&auth_call(
                r#"{"action": "update_pr", "owner": "acme", "repo": "widgets", "pr_number": 42, "title": "x"}"#,
            ))
            .await
            .expect("A 422 is reported in the result, not an Err");

        assert!(result.result.contains("Failed: GitHub API error: 422"));
        api.stop().await;
    }
}
