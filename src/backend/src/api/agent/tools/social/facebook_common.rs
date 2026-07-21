use anyhow::Result;
use std::env;

/// Default Graph API version, used when FACEBOOK_GRAPH_API_VERSION isn't
/// set. Meta sunsets versions on a ~2-year cycle; overriding via env var
/// avoids needing a code change/redeploy when this one is retired.
pub const DEFAULT_GRAPH_API_VERSION: &str = "v21.0";

/// Shared Facebook Graph API credentials, loaded once per tool instance.
/// Centralizes env var names and error messages so every Facebook tool
/// doesn't repeat the same "which var, what error" boilerplate.
pub struct FacebookCredentials {
    page_id: Option<String>,
    access_token: Option<String>,
    business_id: Option<String>,
    graph_api_version: String,
}

impl FacebookCredentials {
    pub fn from_env() -> Self {
        Self {
            page_id: env::var("FACEBOOK_PAGE_ID").ok(),
            access_token: env::var("FACEBOOK_PAGE_ACCESS_TOKEN").ok(),
            business_id: env::var("FACEBOOK_BUSINESS_ID").ok(),
            graph_api_version: env::var("FACEBOOK_GRAPH_API_VERSION")
                .unwrap_or_else(|_| DEFAULT_GRAPH_API_VERSION.to_string()),
        }
    }

    pub fn page_id(&self) -> Result<&str> {
        self.page_id
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("FACEBOOK_PAGE_ID environment variable not set"))
    }

    pub fn access_token(&self) -> Result<&str> {
        self.access_token.as_deref().ok_or_else(|| {
            anyhow::anyhow!("FACEBOOK_PAGE_ACCESS_TOKEN environment variable not set")
        })
    }

    pub fn business_id(&self) -> Result<&str> {
        self.business_id
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("FACEBOOK_BUSINESS_ID environment variable not set"))
    }

    /// Builds a full Graph API URL for `path` (e.g. "123456/posts") using
    /// the configured (or default) API version.
    pub fn graph_url(&self, path: &str) -> String {
        format!(
            "https://graph.facebook.com/{}/{}",
            self.graph_api_version, path
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn credentials_with_version(version: &str) -> FacebookCredentials {
        FacebookCredentials {
            page_id: None,
            access_token: None,
            business_id: None,
            graph_api_version: version.to_string(),
        }
    }

    #[test]
    fn graph_url_uses_configured_version() {
        let creds = credentials_with_version("v99.0");
        assert_eq!(
            creds.graph_url("123/posts"),
            "https://graph.facebook.com/v99.0/123/posts"
        );
    }

    #[test]
    fn missing_page_id_gives_clear_error() {
        let creds = credentials_with_version(DEFAULT_GRAPH_API_VERSION);
        let err = creds.page_id().unwrap_err().to_string();
        assert!(err.contains("FACEBOOK_PAGE_ID"));
    }

    #[test]
    fn missing_business_id_gives_clear_error() {
        let creds = credentials_with_version(DEFAULT_GRAPH_API_VERSION);
        let err = creds.business_id().unwrap_err().to_string();
        assert!(err.contains("FACEBOOK_BUSINESS_ID"));
    }
}
