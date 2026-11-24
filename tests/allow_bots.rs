// `allow_bots` through a real axum router, built as the README builds it: the
// request's own User-Agent reaches the API, and a verified crawler passes the
// condition it otherwise meets. Every peer is a PUBLIC address, since a bogon
// is answered locally and would pass all of this vacuously.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::Body;
use axum::extract::{ConnectInfo, Path, Query, State};
use axum::routing::get;
use http::{Request, StatusCode};
use serde_json::{Value, json};
use tower::ServiceExt;
use vpndetection::middleware::{Condition, Options};
use vpndetection_axum::{MaybeFound, VPNDetection};

const PEER: &str = "66.249.66.1";
const GOOGLEBOT: &str = "Mozilla/5.0 (compatible; Googlebot/2.1; +http://www.google.com/bot.html)";

/// The `user_agent` and the key each lookup carried.
type Seen = Arc<Mutex<Vec<(Option<String>, Option<String>)>>>;

/// A stub API answering every lookup with one verdict, recording what each
/// lookup carried.
#[derive(Clone)]
struct Api {
    verdict: Option<&'static str>,
    seen: Seen,
}

impl Api {
    async fn start(verdict: Option<&'static str>) -> (String, Self) {
        let api = Self { verdict, seen: Arc::new(Mutex::new(Vec::new())) };
        let app = Router::new().route("/{ip}", get(lookup)).with_state(api.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        tokio::spawn(async move { axum::serve(listener, app).await.expect("serve") });
        (format!("http://{addr}"), api)
    }

    fn agents(&self) -> Vec<Option<String>> {
        self.seen.lock().unwrap().iter().map(|(agent, _)| agent.clone()).collect()
    }
}

async fn lookup(
    State(api): State<Api>,
    Path(ip): Path<String>,
    Query(query): Query<HashMap<String, String>>,
    headers: http::HeaderMap,
) -> axum::Json<Value> {
    let key = headers.get(http::header::AUTHORIZATION).and_then(|v| v.to_str().ok());
    api.seen.lock().unwrap().push((query.get("user_agent").cloned(), key.map(str::to_owned)));
    let mut body = json!({ "ip": ip, "is_vpn": false, "is_hosting": true });
    if let Some(verdict) = api.verdict {
        body["user_agent"] = json!({ "agent": "Googlebot", "kind": "bot", "verdict": verdict });
    }
    axum::Json(body)
}

/// The README's build, pointed at the stub: a key, `allow_bots` as given, and
/// a condition refusing a hosting address.
fn app(base_url: &str, allow_bots: bool) -> Router {
    let options = Options::new()
        .api_key("key")
        .allow_bots(allow_bots)
        .block_condition(vec![Condition::from([("is_hosting", true)])])
        .base_url(base_url);
    Router::new()
        .route(
            "/",
            get(|MaybeFound(found): MaybeFound| async move {
                let result = found.as_ref().and_then(|f| f.result.as_ref());
                let verdict =
                    result.and_then(|r| r.user_agent.as_ref()).map(|ua| ua.verdict.to_string());
                axum::Json(json!({ "attached": result.is_some(), "verdict": verdict }))
            }),
        )
        .layer(VPNDetection::new(options).expect("build"))
}

async fn call(app: Router, user_agent: Option<&str>) -> (StatusCode, Value) {
    let mut request = Request::builder().uri("/");
    if let Some(agent) = user_agent {
        request = request.header(http::header::USER_AGENT, agent);
    }
    let mut request = request.body(Body::empty()).expect("request");
    // The request is built by hand, so no client put a User-Agent of its own on it.
    assert_eq!(request.headers().contains_key(http::header::USER_AGENT), user_agent.is_some());
    let addr: SocketAddr = format!("{PEER}:54321").parse().expect("peer");
    request.extensions_mut().insert(ConnectInfo(addr));
    let answer = app.oneshot(request).await.expect("call");
    let status = answer.status();
    let bytes = axum::body::to_bytes(answer.into_body(), 64 * 1024).await.expect("body");
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

#[tokio::test]
async fn a_verified_crawler_passes_the_condition_it_otherwise_meets() {
    let (base_url, api) = Api::start(Some("verified")).await;
    let (status, body) = call(app(&base_url, true), Some(GOOGLEBOT)).await;
    assert_eq!(status, StatusCode::OK, "a verified crawler was refused");
    assert_eq!(body, json!({ "attached": true, "verdict": "verified" }));
    assert_eq!(
        *api.seen.lock().unwrap(),
        [(Some(GOOGLEBOT.to_owned()), Some("Bearer key".to_owned()))],
        "the request's own agent and the key reach the API"
    );
}

#[tokio::test]
async fn a_spoofed_or_not_bot_verdict_is_refused() {
    for verdict in ["spoofed", "not_bot"] {
        let (base_url, api) = Api::start(Some(verdict)).await;
        let (status, _) = call(app(&base_url, true), Some(GOOGLEBOT)).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{verdict}");
        assert_eq!(api.agents(), [Some(GOOGLEBOT.to_owned())], "{verdict}");
    }
}

#[tokio::test]
async fn off_sends_no_agent() {
    let (base_url, api) = Api::start(Some("verified")).await;
    let (status, _) = call(app(&base_url, false), Some(GOOGLEBOT)).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "a verdict never asked for let the request through");
    assert_eq!(api.agents(), [None]);
}

#[tokio::test]
async fn a_request_without_a_user_agent_sends_none() {
    let (base_url, api) = Api::start(None).await;
    let (status, _) = call(app(&base_url, true), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(api.agents(), [None]);
}
