use anyhow::Result;
use std::env;

/// Default Graph API version, used when FACEBOOK_GRAPH_API_VERSION isn't
/// set. Meta sunsets versions on a ~2-year cycle; overriding via env var
/// avoids needing a code change/redeploy when this one is retired.
pub const DEFAULT_GRAPH_API_VERSION: &str = "v21.0";

/// The real Graph API host, used unless a test overrides it.
const GRAPH_API_BASE_URL: &str = "https://graph.facebook.com";

/// Shared Facebook Graph API credentials, loaded once per tool instance.
/// Centralizes env var names and error messages so every Facebook tool
/// doesn't repeat the same "which var, what error" boilerplate.
pub struct FacebookCredentials {
    page_id: Option<String>,
    access_token: Option<String>,
    business_id: Option<String>,
    graph_api_version: String,
    /// Graph API host to talk to. Always the real one in production; tests point
    /// it at a loopback mock instead. Every Facebook tool builds its URLs through
    /// `graph_url`, so overriding it here redirects all of them at once.
    base_url: String,
}

impl FacebookCredentials {
    pub fn from_env() -> Self {
        Self {
            page_id: env::var("FACEBOOK_PAGE_ID").ok(),
            access_token: env::var("FACEBOOK_PAGE_ACCESS_TOKEN").ok(),
            business_id: env::var("FACEBOOK_BUSINESS_ID").ok(),
            graph_api_version: env::var("FACEBOOK_GRAPH_API_VERSION")
                .unwrap_or_else(|_| DEFAULT_GRAPH_API_VERSION.to_string()),
            base_url: GRAPH_API_BASE_URL.to_string(),
        }
    }

    /// Fully populated canned credentials pointed at `base_url` instead of the
    /// real Graph API, so every Facebook tool can be driven without the network
    /// and without any FACEBOOK_* env var being set.
    #[cfg(test)]
    pub(crate) fn for_test(base_url: impl Into<String>) -> Self {
        Self {
            page_id: Some("page_1".to_string()),
            access_token: Some("test-page-token".to_string()),
            business_id: Some("biz_1".to_string()),
            graph_api_version: DEFAULT_GRAPH_API_VERSION.to_string(),
            base_url: base_url.into(),
        }
    }

    /// The same, minus the access token, for exercising the missing-credential
    /// error paths.
    #[cfg(test)]
    pub(crate) fn without_access_token(mut self) -> Self {
        self.access_token = None;
        self
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
        format!("{}/{}/{}", self.base_url, self.graph_api_version, path)
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
            base_url: GRAPH_API_BASE_URL.to_string(),
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

    #[test]
    fn missing_access_token_gives_clear_error() {
        let creds = credentials_with_version(DEFAULT_GRAPH_API_VERSION);
        let err = creds.access_token().unwrap_err().to_string();
        assert!(err.contains("FACEBOOK_PAGE_ACCESS_TOKEN"));
    }

    #[test]
    fn graph_url_uses_the_real_host_by_default_and_the_override_in_tests() {
        let real = credentials_with_version("v21.0");
        assert_eq!(
            real.graph_url("123/feed"),
            "https://graph.facebook.com/v21.0/123/feed"
        );

        // The test-only override is what redirects every Facebook tool at a mock.
        let mocked = FacebookCredentials::for_test("http://127.0.0.1:9");
        assert_eq!(
            mocked.graph_url("123/feed"),
            format!("http://127.0.0.1:9/{}/123/feed", DEFAULT_GRAPH_API_VERSION)
        );
        assert_eq!(mocked.page_id().unwrap(), "page_1");
        assert_eq!(mocked.business_id().unwrap(), "biz_1");
        assert_eq!(mocked.access_token().unwrap(), "test-page-token");
        assert!(mocked
            .without_access_token()
            .access_token()
            .unwrap_err()
            .to_string()
            .contains("FACEBOOK_PAGE_ACCESS_TOKEN"));
    }
}
