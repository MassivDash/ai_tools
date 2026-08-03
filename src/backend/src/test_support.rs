//! Shared, test-only fixtures.
//!
//! Everything in here is compiled under `#[cfg(test)]` only and is meant to be
//! reused across the crate's unit tests, so the same stand-ins are not rebuilt
//! per module.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// Canned responses for [`MockLlm`].
#[derive(Clone)]
pub(crate) struct MockLlmConfig {
    pub props_status: u16,
    pub chat_status: u16,
    /// Body served for every completion request that the `chat_bodies` queue
    /// does not cover.
    pub chat_body: String,
    /// Bodies served in order, one per completion request. Once exhausted,
    /// `chat_body` is served instead. Empty by default.
    pub chat_bodies: Vec<String>,
}

impl MockLlmConfig {
    /// Reachable, and every completion returns `content` as assistant text.
    pub(crate) fn replying(content: &str) -> Self {
        Self {
            props_status: 200,
            chat_status: 200,
            chat_body: assistant_completion(content),
            chat_bodies: Vec::new(),
        }
    }

    /// Reachable, and the Nth completion request gets the Nth body. Requests
    /// past the end of the list fall back to the last body in the list.
    pub(crate) fn replying_with_bodies(bodies: Vec<String>) -> Self {
        let last = bodies
            .last()
            .cloned()
            .unwrap_or_else(|| assistant_completion(""));
        Self {
            props_status: 200,
            chat_status: 200,
            chat_body: last,
            chat_bodies: bodies,
        }
    }
}

/// An OpenAI-compatible completion whose single choice is plain assistant text.
pub(crate) fn assistant_completion(content: &str) -> String {
    serde_json::json!({
        "id": "test",
        "object": "chat.completion",
        "created": 0,
        "model": "test-model",
        "choices": [{
            "index": 0,
            "message": { "role": "assistant", "content": content },
            "finish_reason": "stop"
        }]
    })
    .to_string()
}

/// An OpenAI-compatible completion whose single choice asks for one tool call.
///
/// Only ever point this at a tool name that is *not* registered in the registry
/// under test, or at a tool that is proven not to touch the network - the agent
/// loop really does execute what it is handed.
pub(crate) fn tool_call_completion(id: &str, name: &str, arguments: &str) -> String {
    serde_json::json!({
        "id": "test",
        "object": "chat.completion",
        "created": 0,
        "model": "test-model",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": null,
                "tool_calls": [{
                    "id": id,
                    "type": "function",
                    "function": { "name": name, "arguments": arguments }
                }]
            },
            "finish_reason": "tool_calls"
        }]
    })
    .to_string()
}

struct MockLlmState {
    config: MockLlmConfig,
    calls: AtomicUsize,
    requests: Mutex<Vec<serde_json::Value>>,
}

/// A throwaway HTTP server that stands in for the local llama.cpp server.
///
/// Serves the two endpoints the LLM-facing code talks to - `GET /props` (the
/// reachability probe) and `POST /v1/chat/completions` - with canned statuses
/// and bodies, so those code paths can be driven end to end without a real
/// model and without any network access beyond loopback.
pub(crate) struct MockLlm {
    pub base_url: String,
    state: Arc<MockLlmState>,
    handle: actix_web::dev::ServerHandle,
}

impl MockLlm {
    pub(crate) async fn start(config: MockLlmConfig) -> Self {
        use actix_web::http::StatusCode;
        use actix_web::{web, App, HttpResponse, HttpServer};

        let state = Arc::new(MockLlmState {
            config,
            calls: AtomicUsize::new(0),
            requests: Mutex::new(Vec::new()),
        });
        let data = web::Data::from(Arc::clone(&state));

        let server = HttpServer::new(move || {
            App::new()
                .app_data(data.clone())
                .route(
                    "/props",
                    web::get().to(|state: web::Data<MockLlmState>| async move {
                        HttpResponse::build(
                            StatusCode::from_u16(state.config.props_status).unwrap(),
                        )
                        .content_type("application/json")
                        .body("{}")
                    }),
                )
                .route(
                    "/v1/chat/completions",
                    web::post().to(
                        |state: web::Data<MockLlmState>,
                         body: Option<web::Json<serde_json::Value>>| async move {
                            if let Some(body) = body {
                                state.requests.lock().unwrap().push(body.into_inner());
                            }
                            let nth = state.calls.fetch_add(1, Ordering::SeqCst);
                            let body = state
                                .config
                                .chat_bodies
                                .get(nth)
                                .cloned()
                                .unwrap_or_else(|| state.config.chat_body.clone());
                            HttpResponse::build(
                                StatusCode::from_u16(state.config.chat_status).unwrap(),
                            )
                            .content_type("application/json")
                            .body(body)
                        },
                    ),
                )
        })
        .workers(1)
        .bind("127.0.0.1:0")
        .expect("Failed to bind mock LLM server");

        let base_url = format!("http://{}", server.addrs()[0]);
        let server = server.run();
        let handle = server.handle();
        tokio::spawn(server);

        Self {
            base_url,
            state,
            handle,
        }
    }

    /// The completions endpoint, which is what most callers pass around.
    pub(crate) fn chat_url(&self) -> String {
        format!("{}/v1/chat/completions", self.base_url)
    }

    /// The loopback port the mock is listening on, for callers that build the URL
    /// themselves out of a host and a port.
    pub(crate) fn port(&self) -> u16 {
        self.base_url
            .rsplit(':')
            .next()
            .and_then(|port| port.parse().ok())
            .expect("The mock LLM base URL should end in a port")
    }

    /// Number of completion requests served so far.
    pub(crate) fn call_count(&self) -> usize {
        self.state.calls.load(Ordering::SeqCst)
    }

    /// The JSON bodies of the completion requests received so far, in order.
    pub(crate) fn requests(&self) -> Vec<serde_json::Value> {
        self.state.requests.lock().unwrap().clone()
    }

    pub(crate) async fn stop(self) {
        self.handle.stop(false).await;
    }
}

/// A base URL that is guaranteed to refuse connections (port 1 is privileged and
/// never bound), for exercising the "LLM is switched off" paths.
pub(crate) const UNREACHABLE_LLM_URL: &str = "http://127.0.0.1:1";

/// An endpoint string that `reqwest::Url` cannot parse, so `ChromaDBClient::new`
/// itself fails instead of the request it would later make. This is the only way
/// to reach the "could not build a client" arms of the ChromaDB handlers.
pub(crate) const UNPARSEABLE_CHROMA_ENDPOINT: &str = "not-a-url";

/// Guards the process-global `CHROMA_ENDPOINT` environment variable.
///
/// `ChromaDBClient::new` works by writing that variable and then having the
/// `chroma` crate read it straight back out of the process environment, so it is
/// not a per-client setting. Two tests pointing clients at two different mock
/// servers at the same time can therefore read each other's endpoint. Every test
/// that builds a `ChromaDBClient` - directly, or indirectly through a handler or
/// an agent tool - takes this lock first, which serialises them against each
/// other.
static CHROMA_ENDPOINT_LOCK: Mutex<()> = Mutex::new(());

/// Exclusive access to `CHROMA_ENDPOINT`; see [`lock_chroma_endpoint`].
pub(crate) struct ChromaEndpointGuard(#[allow(dead_code)] std::sync::MutexGuard<'static, ()>);

/// Takes the process-wide `CHROMA_ENDPOINT` lock, and keeps holding it until the
/// returned guard is dropped.
///
/// Hold it across the whole of the code under test, not just client
/// construction: handlers build their client inside the request, so the guard has
/// to outlive the `call_service`. Poisoning is ignored, so one panicking test
/// does not cascade into every other test that needs the lock.
pub(crate) fn lock_chroma_endpoint() -> ChromaEndpointGuard {
    ChromaEndpointGuard(
        CHROMA_ENDPOINT_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()),
    )
}

/// One collection served by [`MockChroma`].
#[derive(Clone)]
pub(crate) struct MockChromaCollection {
    /// The server-assigned UUID. Record-level endpoints are addressed by this,
    /// collection-level ones by name, exactly as in the real API.
    pub id: String,
    pub name: String,
    pub metadata: Option<std::collections::HashMap<String, String>>,
    /// What `GET .../collections/{id}/count` reports.
    pub count: u32,
}

impl MockChromaCollection {
    /// An empty collection with no metadata and a fresh id.
    pub(crate) fn new(name: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            metadata: None,
            count: 0,
        }
    }

    pub(crate) fn with_metadata(mut self, entries: &[(&str, &str)]) -> Self {
        self.metadata = Some(
            entries
                .iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect(),
        );
        self
    }

    pub(crate) fn with_count(mut self, count: u32) -> Self {
        self.count = count;
        self
    }

    /// The collection encoded the way a real ChromaDB server encodes it.
    ///
    /// This serialises the `chroma` crate's own `Collection` type rather than
    /// hand-written JSON, so the mock's wire format cannot drift from the shape
    /// the client under test deserialises (including the `configuration_json`
    /// blob and the `id` rename).
    fn to_json(&self, tenant: &str, database: &str) -> serde_json::Value {
        let collection = chroma::types::Collection {
            name: self.name.clone(),
            tenant: tenant.to_string(),
            database: database.to_string(),
            metadata: self.metadata.as_ref().map(|metadata| {
                metadata
                    .iter()
                    .map(|(key, value)| {
                        (
                            key.clone(),
                            chroma::types::MetadataValue::Str(value.clone()),
                        )
                    })
                    .collect()
            }),
            ..Default::default()
        };

        let mut json =
            serde_json::to_value(&collection).expect("A chroma Collection should always serialise");
        json["id"] = serde_json::Value::String(self.id.clone());
        json
    }
}

/// Canned behaviour for [`MockChroma`].
///
/// The `*_status` fields force the matching operation to fail with that HTTP
/// status instead of doing its normal thing. Prefer 4xx statuses: the `chroma`
/// client retries GETs that come back 5xx three times with backoff, which makes
/// a test that wants a failure take seconds instead of milliseconds.
#[derive(Clone, Default)]
pub(crate) struct MockChromaConfig {
    pub collections: Vec<MockChromaCollection>,
    pub list_status: Option<u16>,
    pub get_status: Option<u16>,
    pub create_status: Option<u16>,
    pub delete_status: Option<u16>,
    pub count_status: Option<u16>,
}

impl MockChromaConfig {
    /// A healthy server holding exactly `collections`.
    pub(crate) fn holding(collections: Vec<MockChromaCollection>) -> Self {
        Self {
            collections,
            ..Default::default()
        }
    }

    /// A healthy server with no collections at all.
    pub(crate) fn empty() -> Self {
        Self::default()
    }
}

/// One request [`MockChroma`] served, for asserting on what the client actually
/// put on the wire.
#[derive(Clone, Debug)]
pub(crate) struct MockChromaRequest {
    pub method: String,
    /// Path only, e.g. `/api/v2/tenants/default_tenant/databases/default_database/collections`.
    pub path: String,
    /// Raw query string, e.g. `limit=100`. Empty when there was none.
    pub query: String,
    /// Parsed request body, if the request had one.
    pub body: Option<serde_json::Value>,
}

struct MockChromaState {
    config: MockChromaConfig,
    /// Live collections; `create`/`delete` mutate this so a test can assert the
    /// server-side effect of a call.
    collections: Mutex<Vec<MockChromaCollection>>,
    requests: Mutex<Vec<MockChromaRequest>>,
}

impl MockChromaState {
    fn record(&self, req: &actix_web::HttpRequest, body: Option<serde_json::Value>) {
        self.requests.lock().unwrap().push(MockChromaRequest {
            method: req.method().to_string(),
            path: req.path().to_string(),
            query: req.query_string().to_string(),
            body,
        });
    }
}

/// A ChromaDB error body, which the client turns into
/// `ChromaHttpClientError::ApiError("{error}: {message}", status)`.
fn chroma_error(status: u16, error: &str, message: &str) -> actix_web::HttpResponse {
    actix_web::HttpResponse::build(
        actix_web::http::StatusCode::from_u16(status).expect("a valid HTTP status"),
    )
    .json(serde_json::json!({ "error": error, "message": message }))
}

/// A throwaway HTTP server that stands in for a ChromaDB instance.
///
/// It implements the slice of the ChromaDB v2 REST API that this crate's client
/// actually calls, as determined by reading the vendored `chroma` 0.9.0 sources
/// (`client/chroma_http_client.rs` and `collection.rs`):
///
/// - `GET    /api/v2/tenants/{tenant}/databases/{db}/collections` (`?limit&offset`)
/// - `POST   /api/v2/tenants/{tenant}/databases/{db}/collections`
/// - `GET    /api/v2/tenants/{tenant}/databases/{db}/collections/{name}`
/// - `DELETE /api/v2/tenants/{tenant}/databases/{db}/collections/{name}`
/// - `GET    /api/v2/tenants/{tenant}/databases/{db}/collections/{id}/count`
///
/// There is deliberately no tenant/database bootstrap handshake: `from_env()`
/// defaults `CHROMA_TENANT`/`CHROMA_DATABASE` to `default_tenant`/
/// `default_database`, so the client never calls `/api/v2/auth/identity`. The
/// tenant and database are captured from the path instead of being validated, so
/// a test can assert which ones the client addressed. Anything else 404s and is
/// still recorded, which is what proves no unexpected endpoint was hit.
pub(crate) struct MockChroma {
    pub base_url: String,
    state: Arc<MockChromaState>,
    handle: actix_web::dev::ServerHandle,
}

impl MockChroma {
    pub(crate) async fn start(config: MockChromaConfig) -> Self {
        use actix_web::{web, HttpRequest, HttpResponse, HttpServer};

        let state = Arc::new(MockChromaState {
            collections: Mutex::new(config.collections.clone()),
            config,
            requests: Mutex::new(Vec::new()),
        });
        let data = web::Data::from(Arc::clone(&state));

        /// `?limit&offset` as `list_collections` sends them.
        #[derive(serde::Deserialize)]
        struct ListQuery {
            limit: Option<usize>,
            offset: Option<usize>,
        }

        let server = HttpServer::new(move || {
            actix_web::App::new()
                .app_data(data.clone())
                .service(
                    web::resource("/api/v2/tenants/{tenant}/databases/{database}/collections")
                        .route(web::get().to(
                            |state: web::Data<MockChromaState>,
                             path: web::Path<(String, String)>,
                             query: web::Query<ListQuery>,
                             req: HttpRequest| async move {
                                state.record(&req, None);
                                if let Some(status) = state.config.list_status {
                                    return chroma_error(
                                        status,
                                        "ListCollectionsError",
                                        "could not list collections",
                                    );
                                }
                                let (tenant, database) = path.into_inner();
                                let collections = state.collections.lock().unwrap();
                                let page: Vec<serde_json::Value> = collections
                                    .iter()
                                    .skip(query.offset.unwrap_or(0))
                                    .take(query.limit.unwrap_or(usize::MAX))
                                    .map(|collection| collection.to_json(&tenant, &database))
                                    .collect();
                                HttpResponse::Ok().json(page)
                            },
                        ))
                        .route(web::post().to(
                            |state: web::Data<MockChromaState>,
                             path: web::Path<(String, String)>,
                             body: web::Json<serde_json::Value>,
                             req: HttpRequest| async move {
                                let body = body.into_inner();
                                state.record(&req, Some(body.clone()));
                                if let Some(status) = state.config.create_status {
                                    return chroma_error(
                                        status,
                                        "CreateCollectionError",
                                        "could not create collection",
                                    );
                                }
                                let (tenant, database) = path.into_inner();
                                let name = body
                                    .get("name")
                                    .and_then(|name| name.as_str())
                                    .unwrap_or_default()
                                    .to_string();

                                let mut collections = state.collections.lock().unwrap();
                                if collections.iter().any(|c| c.name == name) {
                                    return chroma_error(
                                        409,
                                        "UniqueConstraintError",
                                        &format!("Collection {} already exists", name),
                                    );
                                }

                                let metadata = body.get("metadata").and_then(|metadata| {
                                    metadata.as_object().map(|metadata| {
                                        metadata
                                            .iter()
                                            .map(|(key, value)| {
                                                let value = match value {
                                                    serde_json::Value::String(text) => text.clone(),
                                                    other => other.to_string(),
                                                };
                                                (key.clone(), value)
                                            })
                                            .collect()
                                    })
                                });

                                let created = MockChromaCollection {
                                    id: uuid::Uuid::new_v4().to_string(),
                                    name,
                                    metadata,
                                    count: 0,
                                };
                                let json = created.to_json(&tenant, &database);
                                collections.push(created);
                                HttpResponse::Ok().json(json)
                            },
                        )),
                )
                .service(
                    web::resource(
                        "/api/v2/tenants/{tenant}/databases/{database}/collections/{name}",
                    )
                    .route(web::get().to(
                        |state: web::Data<MockChromaState>,
                         path: web::Path<(String, String, String)>,
                         req: HttpRequest| async move {
                            state.record(&req, None);
                            if let Some(status) = state.config.get_status {
                                return chroma_error(
                                    status,
                                    "GetCollectionError",
                                    "could not get collection",
                                );
                            }
                            let (tenant, database, name) = path.into_inner();
                            let collections = state.collections.lock().unwrap();
                            match collections.iter().find(|c| c.name == name) {
                                Some(collection) => {
                                    HttpResponse::Ok().json(collection.to_json(&tenant, &database))
                                }
                                None => chroma_error(
                                    404,
                                    "NotFoundError",
                                    &format!("Collection {} does not exist", name),
                                ),
                            }
                        },
                    ))
                    .route(web::delete().to(
                        |state: web::Data<MockChromaState>,
                         path: web::Path<(String, String, String)>,
                         req: HttpRequest| async move {
                            state.record(&req, None);
                            if let Some(status) = state.config.delete_status {
                                return chroma_error(
                                    status,
                                    "DeleteCollectionError",
                                    "could not delete collection",
                                );
                            }
                            let (_, _, name) = path.into_inner();
                            let mut collections = state.collections.lock().unwrap();
                            match collections.iter().position(|c| c.name == name) {
                                Some(index) => {
                                    collections.remove(index);
                                    HttpResponse::Ok().json(serde_json::json!({}))
                                }
                                None => chroma_error(
                                    404,
                                    "NotFoundError",
                                    &format!("Collection {} does not exist", name),
                                ),
                            }
                        },
                    )),
                )
                .service(
                    web::resource(
                        "/api/v2/tenants/{tenant}/databases/{database}/collections/{id}/count",
                    )
                    .route(web::get().to(
                        |state: web::Data<MockChromaState>,
                         path: web::Path<(String, String, String)>,
                         req: HttpRequest| async move {
                            state.record(&req, None);
                            if let Some(status) = state.config.count_status {
                                return chroma_error(
                                    status,
                                    "CountError",
                                    "could not count records",
                                );
                            }
                            let (_, _, id) = path.into_inner();
                            let collections = state.collections.lock().unwrap();
                            match collections.iter().find(|c| c.id == id) {
                                Some(collection) => HttpResponse::Ok().json(collection.count),
                                None => chroma_error(
                                    404,
                                    "NotFoundError",
                                    &format!("Collection {} does not exist", id),
                                ),
                            }
                        },
                    )),
                )
                .default_service(web::to(
                    |state: web::Data<MockChromaState>, req: HttpRequest| async move {
                        state.record(&req, None);
                        chroma_error(404, "NotFoundError", "unimplemented mock endpoint")
                    },
                ))
        })
        .workers(1)
        .bind("127.0.0.1:0")
        .expect("Failed to bind mock ChromaDB server");

        let base_url = format!("http://{}", server.addrs()[0]);
        let server = server.run();
        let handle = server.handle();
        tokio::spawn(server);

        Self {
            base_url,
            state,
            handle,
        }
    }

    /// The requests served so far, in order.
    pub(crate) fn requests(&self) -> Vec<MockChromaRequest> {
        self.state.requests.lock().unwrap().clone()
    }

    /// The names of the collections the server currently holds, in insertion
    /// order, so a test can see what a create or delete actually did.
    pub(crate) fn collection_names(&self) -> Vec<String> {
        self.state
            .collections
            .lock()
            .unwrap()
            .iter()
            .map(|collection| collection.name.clone())
            .collect()
    }

    pub(crate) async fn stop(self) {
        self.handle.stop(false).await;
    }
}

/// A registrable agent tool that echoes a canned result back.
///
/// It performs no I/O at all, so it is safe to let an agent loop actually run it -
/// unlike the real tools, which talk to third-party services.
pub(crate) struct EchoTool {
    metadata: crate::api::agent::tools::framework::agent_tool::ToolMetadata,
    result: String,
}

impl EchoTool {
    /// A tool registered and advertised under `name`, whose every call returns
    /// `result`.
    pub(crate) fn new(name: &str, result: &str) -> Self {
        use crate::api::agent::core::types::ToolType;
        use crate::api::agent::tools::framework::agent_tool::{ToolCategory, ToolMetadata};

        Self {
            metadata: ToolMetadata {
                id: name.to_string(),
                name: name.to_string(),
                tool_type: ToolType::AskHuman,
                description: "A test tool that echoes a canned result".to_string(),
                category: ToolCategory::Utility,
            },
            result: result.to_string(),
        }
    }

    /// The same tool, advertised under `tool_type`, for code that branches on a
    /// tool's type rather than its name.
    pub(crate) fn with_tool_type(
        mut self,
        tool_type: crate::api::agent::core::types::ToolType,
    ) -> Self {
        self.metadata.tool_type = tool_type;
        self
    }
}

#[async_trait::async_trait]
impl crate::api::agent::tools::framework::agent_tool::AgentTool for EchoTool {
    fn metadata(&self) -> &crate::api::agent::tools::framework::agent_tool::ToolMetadata {
        &self.metadata
    }

    fn get_function_definition(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.metadata.name,
            "description": self.metadata.description,
            "parameters": { "type": "object", "properties": {} }
        })
    }

    async fn execute(
        &self,
        tool_call: &crate::api::agent::core::types::ToolCall,
    ) -> anyhow::Result<crate::api::agent::core::types::ToolCallResult> {
        Ok(crate::api::agent::core::types::ToolCallResult {
            tool_call_id: Some(tool_call.id.clone()),
            tool_name: self.metadata.name.clone(),
            result: self.result.clone(),
        })
    }
}

/// A canned response for one [`MockHttpApi`] route.
#[derive(Clone, Debug)]
pub(crate) struct MockResponse {
    pub status: u16,
    pub content_type: String,
    pub body: String,
}

impl MockResponse {
    /// `200 application/json` carrying `value`.
    pub(crate) fn json(value: serde_json::Value) -> Self {
        Self {
            status: 200,
            content_type: "application/json".to_string(),
            body: value.to_string(),
        }
    }

    /// `200 text/html` carrying `body`, for the tools that read the body as text
    /// instead of JSON.
    pub(crate) fn html(body: &str) -> Self {
        Self {
            status: 200,
            content_type: "text/html; charset=utf-8".to_string(),
            body: body.to_string(),
        }
    }

    /// An error status with a plain-text body, which is what every tool under
    /// test funnels into its own error message via `response.text()`.
    pub(crate) fn error(status: u16, body: &str) -> Self {
        Self {
            status,
            content_type: "text/plain; charset=utf-8".to_string(),
            body: body.to_string(),
        }
    }

    /// Fully hand-rolled, for serving bodies that are deliberately not the
    /// content type they claim - i.e. the malformed-JSON paths.
    pub(crate) fn raw(status: u16, content_type: &str, body: &str) -> Self {
        Self {
            status,
            content_type: content_type.to_string(),
            body: body.to_string(),
        }
    }
}

/// One request [`MockHttpApi`] served, for asserting on what a tool actually put
/// on the wire.
#[derive(Clone, Debug)]
pub(crate) struct MockRequest {
    pub method: String,
    /// Path only, without the query string.
    pub path: String,
    /// Raw query string, e.g. `q=rust&per_page=5`. Empty when there was none.
    pub query: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl MockRequest {
    /// A request header by (case-insensitive) name.
    pub(crate) fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    /// The body as text.
    pub(crate) fn body_text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }

    /// The body parsed as JSON. Panics if it is not JSON, which only happens if
    /// the code under test stopped sending a JSON body.
    pub(crate) fn json(&self) -> serde_json::Value {
        serde_json::from_slice(&self.body)
            .unwrap_or_else(|e| panic!("Request body was not JSON ({}): {}", e, self.body_text()))
    }

    /// Every query parameter, percent-decoded, in the order sent.
    pub(crate) fn query_params(&self) -> Vec<(String, String)> {
        url::form_urlencoded::parse(self.query.as_bytes())
            .map(|(key, value)| (key.into_owned(), value.into_owned()))
            .collect()
    }

    /// The first value of a percent-decoded query parameter.
    pub(crate) fn query_param(&self, name: &str) -> Option<String> {
        self.query_params()
            .into_iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value)
    }

    /// The first value of a percent-decoded `application/x-www-form-urlencoded`
    /// body parameter.
    pub(crate) fn form_param(&self, name: &str) -> Option<String> {
        url::form_urlencoded::parse(&self.body)
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.into_owned())
    }
}

/// One registered `(method, path) -> responses` mapping.
struct MockRoute {
    method: String,
    path: String,
    /// Served in order, one per matching request. Requests past the end of the
    /// list get the last entry again.
    responses: Vec<MockResponse>,
    /// How many requests this route has served, which is what indexes
    /// `responses`.
    hits: usize,
}

struct MockHttpApiState {
    routes: Mutex<Vec<MockRoute>>,
    requests: Mutex<Vec<MockRequest>>,
    calls: AtomicUsize,
}

/// A throwaway HTTP server that stands in for an arbitrary third-party REST API.
///
/// Unlike [`MockLlm`] and [`MockChroma`], which each implement one specific
/// upstream, this one knows nothing about any API: a test registers the exact
/// `(method, path)` pairs it expects and the canned responses to serve for them,
/// then asserts on [`MockHttpApi::requests`]. That is enough to stand in for
/// Alpha Vantage, NBP, the GitHub REST API, the Facebook Graph API, Bluesky's
/// XRPC endpoints, Google Books, or a plain website, so every tool that wraps one
/// of those can be driven end to end over loopback only.
///
/// Anything not registered is answered `404` and still recorded, which is what
/// proves a tool did not quietly call an endpoint the test does not know about.
pub(crate) struct MockHttpApi {
    base_url: String,
    state: Arc<MockHttpApiState>,
    handle: actix_web::dev::ServerHandle,
}

impl MockHttpApi {
    /// Starts an empty server on an ephemeral loopback port. Register routes with
    /// [`MockHttpApi::on`] before driving the code under test.
    pub(crate) async fn start() -> Self {
        use actix_web::http::StatusCode;
        use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};

        let state = Arc::new(MockHttpApiState {
            routes: Mutex::new(Vec::new()),
            requests: Mutex::new(Vec::new()),
            calls: AtomicUsize::new(0),
        });
        let data = web::Data::from(Arc::clone(&state));

        let server = HttpServer::new(move || {
            App::new().app_data(data.clone()).default_service(web::to(
                |state: web::Data<MockHttpApiState>,
                 req: HttpRequest,
                 body: web::Bytes| async move {
                    let method = req.method().to_string();
                    let path = req.path().to_string();
                    state.requests.lock().unwrap().push(MockRequest {
                        method: method.clone(),
                        path: path.clone(),
                        query: req.query_string().to_string(),
                        headers: req
                            .headers()
                            .iter()
                            .map(|(name, value)| {
                                (
                                    name.as_str().to_string(),
                                    String::from_utf8_lossy(value.as_bytes()).into_owned(),
                                )
                            })
                            .collect(),
                        body: body.to_vec(),
                    });
                    state.calls.fetch_add(1, Ordering::SeqCst);

                    let canned = {
                        let mut routes = state.routes.lock().unwrap();
                        routes
                            .iter_mut()
                            .find(|route| route.method == method && route.path == path)
                            .map(|route| {
                                let nth = route.hits.min(route.responses.len() - 1);
                                route.hits += 1;
                                route.responses[nth].clone()
                            })
                    };

                    match canned {
                        Some(response) => HttpResponse::build(
                            StatusCode::from_u16(response.status).expect("a valid HTTP status"),
                        )
                        .content_type(response.content_type)
                        .body(response.body),
                        None => HttpResponse::NotFound()
                            .content_type("application/json")
                            .body(format!(
                                r#"{{"error":"no mock route for {} {}"}}"#,
                                method, path
                            )),
                    }
                },
            ))
        })
        .workers(1)
        .bind("127.0.0.1:0")
        .expect("Failed to bind mock HTTP API server");

        let base_url = format!("http://{}", server.addrs()[0]);
        let server = server.run();
        let handle = server.handle();
        tokio::spawn(server);

        Self {
            base_url,
            state,
            handle,
        }
    }

    /// Convenience for the common case: start a server serving exactly one route.
    pub(crate) async fn serving(method: &str, path: &str, response: MockResponse) -> Self {
        let api = Self::start().await;
        api.on(method, path, response);
        api
    }

    /// Registers `response` for every request to `(method, path)`.
    pub(crate) fn on(&self, method: &str, path: &str, response: MockResponse) -> &Self {
        self.on_sequence(method, path, vec![response])
    }

    /// Registers a sequence of responses for `(method, path)`: the Nth matching
    /// request gets the Nth response, and anything past the end repeats the last.
    pub(crate) fn on_sequence(
        &self,
        method: &str,
        path: &str,
        responses: Vec<MockResponse>,
    ) -> &Self {
        assert!(
            !responses.is_empty(),
            "A mock route needs at least one response"
        );
        self.state.routes.lock().unwrap().push(MockRoute {
            method: method.to_string(),
            path: path.to_string(),
            responses,
            hits: 0,
        });
        self
    }

    /// The `http://127.0.0.1:PORT` root, which is what the tools under test take
    /// as their base URL.
    pub(crate) fn base_url(&self) -> &str {
        &self.base_url
    }

    /// The base URL with `path` appended, for tools whose base URL is a full
    /// endpoint rather than a host.
    pub(crate) fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    /// Every request served so far, in order.
    pub(crate) fn requests(&self) -> Vec<MockRequest> {
        self.state.requests.lock().unwrap().clone()
    }

    /// The single request served so far. Panics otherwise, so a test that means
    /// "exactly one call" says so in one line.
    pub(crate) fn only_request(&self) -> MockRequest {
        let requests = self.requests();
        assert_eq!(
            requests.len(),
            1,
            "Expected exactly one request, got {:?}",
            requests
        );
        requests.into_iter().next().unwrap()
    }

    /// Number of requests served so far, matched or not.
    pub(crate) fn call_count(&self) -> usize {
        self.state.calls.load(Ordering::SeqCst)
    }

    pub(crate) async fn stop(self) {
        self.handle.stop(false).await;
    }
}
