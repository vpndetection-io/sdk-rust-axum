// The adapter through a real axum router, plus the shared conformance corpus
// asserted end to end rather than against the core directly.
//
// What is NOT here is the condition matrix, fail-open and the selectors: those
// are framework-independent and are asserted once for Rust, in the base SDK's
// tests/middleware.rs. Repeating them per adapter is how two adapters end up
// disagreeing about which copy is right.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::Body;
use axum::extract::{ConnectInfo, State};
use axum::response::IntoResponse;
use axum::routing::get;
use http::{Request, StatusCode};
use serde_json::{Value, json};
use tower::ServiceExt;
use vpndetection::middleware::{Condition, Options, from_json, header, xff};
use vpndetection_axum::{Found, MaybeFound, VPNDetection};

const PUBLIC_IP: &str = "45.83.91.1";

/// A stub API, served by axum on a random port, that answers every lookup from
/// one body and records what it was asked about - so "never touched the
/// network" is asserted rather than assumed.
struct Api {
    base_url: String,
    asked: Arc<Mutex<Vec<String>>>,
}

impl Api {
    async fn serving(body: Value, status: StatusCode) -> Self {
        let asked = Arc::new(Mutex::new(Vec::new()));
        let app = Router::new().route("/{ip}", get(answer)).with_state(Canned {
            body,
            status,
            asked: Arc::clone(&asked),
        });

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        tokio::spawn(async move { axum::serve(listener, app).await.expect("serve") });
        Self { base_url: format!("http://{addr}"), asked }
    }

    async fn ok(body: Value) -> Self {
        Self::serving(body, StatusCode::OK).await
    }

    fn asked(&self) -> Vec<String> {
        self.asked.lock().unwrap().clone()
    }

    fn options(&self) -> Options {
        Options::new().base_url(&self.base_url)
    }
}

#[derive(Clone)]
struct Canned {
    body: Value,
    status: StatusCode,
    asked: Arc<Mutex<Vec<String>>>,
}

async fn answer(
    State(canned): State<Canned>,
    axum::extract::Path(ip): axum::extract::Path<String>,
) -> impl IntoResponse {
    canned.asked.lock().unwrap().push(ip.clone());
    let mut body = canned.body.clone();
    if let Some(object) = body.as_object_mut() {
        object.insert("ip".into(), json!(ip));
    }
    (canned.status, axum::Json(body))
}

/// An app whose one handler reports back what the middleware attached.
fn app(guard: VPNDetection) -> Router {
    Router::new()
        .route(
            "/{*rest}",
            get(|found: MaybeFound| async move {
                let MaybeFound(found) = found;
                axum::Json(json!({
                    "attached": found.is_some(),
                    "ip": found.as_ref().and_then(|f| f.ip.clone()),
                    "is_vpn": found.as_ref().and_then(|f| f.result.as_ref()).map(|r| r.is_vpn),
                    "is_bogon": found.as_ref().and_then(|f| f.result.as_ref()).map(|r| r.is_bogon),
                    "error": found.as_ref().and_then(|f| f.error.as_ref()).map(|e| e.to_string()),
                }))
            }),
        )
        .layer(guard)
}

/// One request through the whole stack. `peer` is what
/// `into_make_service_with_connect_info` would have put there.
async fn call(
    app: Router,
    path: &str,
    peer: Option<&str>,
    headers: &[(&str, &str)],
) -> (StatusCode, Value) {
    let mut request = Request::builder().uri(path);
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let mut request = request.body(Body::empty()).expect("request");
    if let Some(peer) = peer {
        let addr: SocketAddr = format!("{peer}:54321").parse().expect("peer");
        request.extensions_mut().insert(ConnectInfo(addr));
    }
    let answer = app.oneshot(request).await.expect("call");
    let status = answer.status();
    let bytes = axum::body::to_bytes(answer.into_body(), 64 * 1024).await.expect("body");
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

fn guard(options: Options) -> VPNDetection {
    VPNDetection::new(options).expect("build")
}

#[tokio::test]
async fn enriches_the_request_and_leaves_the_decision_to_the_handler() {
    let api = Api::ok(json!({ "is_vpn": true })).await;
    let app = app(guard(api.options()));

    let (status, body) = call(app, "/anything", Some(PUBLIC_IP), &[]).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["attached"], json!(true));
    assert_eq!(body["is_vpn"], json!(true));
    assert_eq!(body["ip"], json!(PUBLIC_IP));
    assert_eq!(api.asked(), vec![PUBLIC_IP]);
}

#[tokio::test]
async fn blocks_before_the_handler_runs() {
    let api = Api::ok(json!({ "is_vpn": true })).await;
    let app = app(guard(api.options().block_condition(vec![Condition::from([("is_vpn", true)])])));

    let (status, body) = call(app, "/anything", Some(PUBLIC_IP), &[]).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body, json!({ "error": "access denied" }));
    assert!(body.get("attached").is_none(), "the handler answered a blocked request");
}

#[tokio::test]
async fn on_blocked_replaces_the_refusal() {
    let api = Api::ok(json!({ "is_vpn": true, "vpn": { "provider": "nordvpn" } })).await;
    let app = app(guard(api.options().block_condition(vec![Condition::from([("is_vpn", true)])]))
        .on_blocked(|_parts, found| {
            let provider = found
                .result
                .as_ref()
                .and_then(|r| r.vpn.as_ref())
                .and_then(|v| v.provider.clone())
                .unwrap_or_default();
            (StatusCode::from_u16(451).expect("status"), provider).into_response()
        }));

    let answer = app
        .oneshot({
            let mut request = Request::builder().uri("/x").body(Body::empty()).expect("request");
            request.extensions_mut().insert(ConnectInfo(SocketAddr::from(([45, 83, 91, 1], 1))));
            request
        })
        .await
        .expect("call");

    assert_eq!(answer.status().as_u16(), 451);
    let bytes = axum::body::to_bytes(answer.into_body(), 1024).await.expect("body");
    assert_eq!(&bytes[..], b"nordvpn");
}

#[tokio::test]
async fn skip_leaves_the_request_untouched() {
    let api = Api::ok(json!({ "is_vpn": true })).await;
    let app = app(guard(api.options().block_condition(vec![Condition::from([("is_vpn", true)])]))
        .skip(|parts| parts.uri.path().starts_with("/healthz")));

    let (status, body) = call(app, "/healthz/live", Some(PUBLIC_IP), &[]).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["attached"], json!(false));
    assert!(api.asked().is_empty());
}

#[tokio::test]
async fn a_failing_lookup_lets_the_visitor_through() {
    let api = Api::serving(json!({ "error": "boom" }), StatusCode::INTERNAL_SERVER_ERROR).await;
    let app = app(guard(
        api.options().retries(0).block_condition(vec![Condition::from([("is_vpn", true)])]),
    ));

    let (status, body) = call(app, "/x", Some(PUBLIC_IP), &[]).await;

    assert_eq!(status, StatusCode::OK, "our outage must not become theirs");
    assert!(body["error"].as_str().expect("an error").contains("server_error"));
}

// The test that matters. Every other assertion here would pass whether or not
// the selector is right, because a direct connection has nothing to confuse.
#[tokio::test]
async fn a_forged_x_forwarded_for_is_ignored_by_default() {
    let api = Api::ok(json!({ "is_vpn": true })).await;
    let default = app(guard(api.options()));

    let (_, body) = call(default, "/x", Some("10.0.0.7"), &[("X-Forwarded-For", PUBLIC_IP)]).await;

    assert_eq!(body["ip"], json!("10.0.0.7"), "ConnectInfo is the socket peer, not the header");
    assert_eq!(body["is_bogon"], json!(true));
    assert!(api.asked().is_empty(), "and a bogon is answered locally, so nothing was asked");

    let trusting = Api::ok(json!({ "is_vpn": true })).await;
    let explicit = app(guard(trusting.options().ip_selector(xff(0))));
    let (_, body) = call(explicit, "/x", Some("10.0.0.7"), &[("X-Forwarded-For", PUBLIC_IP)]).await;
    assert_eq!(body["ip"], json!(PUBLIC_IP));
    assert_eq!(trusting.asked(), vec![PUBLIC_IP]);
}

#[tokio::test]
async fn a_header_selector_reads_the_edge_that_writes_it() {
    let api = Api::ok(json!({ "is_vpn": true })).await;
    let app = app(guard(api.options().ip_selector(header("CF-Connecting-IP"))));

    call(app, "/x", Some("10.0.0.7"), &[("CF-Connecting-IP", "45.83.91.9")]).await;

    assert_eq!(api.asked(), vec!["45.83.91.9"]);
}

#[tokio::test]
async fn depth_counts_trusted_hops_from_the_right() {
    let api = Api::ok(json!({ "is_vpn": true })).await;
    let app = app(guard(api.options().ip_selector(xff(1))));

    call(
        app,
        "/x",
        Some("10.0.0.7"),
        &[("X-Forwarded-For", "45.83.91.1, 70.41.3.18, 150.172.238.178")],
    )
    .await;

    assert_eq!(api.asked(), vec!["150.172.238.178"]);
}

// Serving without into_make_service_with_connect_info leaves no peer at all,
// which is a wiring mistake the middleware has to survive rather than panic on.
#[tokio::test]
async fn no_connect_info_warns_and_lets_the_visitor_through() {
    let api = Api::ok(json!({ "is_vpn": true })).await;
    let warned = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&warned);
    let app = app(guard(
        api.options().block_condition(vec![Condition::from([("is_vpn", true)])]).on_warn(Arc::new(
            move |_| {
                counter.fetch_add(1, Ordering::SeqCst);
            },
        )),
    ));

    let (status, body) = call(app, "/x", None, &[]).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body["error"].as_str().expect("an error").contains("no client address"));
    assert_eq!(warned.load(Ordering::SeqCst), 1);
    assert!(api.asked().is_empty());
}

#[tokio::test]
async fn found_rejects_where_the_layer_is_not_mounted() {
    let unguarded: Router =
        Router::new().route("/", get(|Found(_): Found| async { "unreachable" }));

    let (status, _) = call(unguarded, "/", Some(PUBLIC_IP), &[]).await;

    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "a wiring mistake, not a visitor's");
}

#[tokio::test]
async fn corpus_conditions() {
    let corpus: Value =
        serde_json::from_str(include_str!("../testdata/testdata.json")).expect("the corpus");
    for case in corpus["middleware"]["conditions"].as_array().expect("conditions") {
        let why = format!("{}: {}", case["name"], case["why"]);
        let bogon = case["bogon"].as_str();
        let ip = bogon
            .or_else(|| case["body"]["ip"].as_str())
            .expect("every case names an address")
            .to_owned();
        let mut body = case["body"].clone();
        if let Some(object) = body.as_object_mut() {
            object.remove("ip");
        }
        let api = Api::ok(if body.is_object() { body } else { json!({}) }).await;

        let warnings = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&warnings);
        let app = app(guard(
            api.options()
                .block_condition(from_json(&case["condition"]).expect("a readable condition"))
                .ip_selector({
                    let ip = ip.clone();
                    Arc::new(move |_: &vpndetection::middleware::RequestView<'_>| Some(ip.clone()))
                })
                .on_warn(Arc::new(move |m: &str| sink.lock().unwrap().push(m.to_owned()))),
        ));

        let (status, _) = call(app, "/x", None, &[]).await;

        let blocked = case["expect"]["blocked"].as_bool().expect("blocked");
        assert_eq!(status, if blocked { StatusCode::FORBIDDEN } else { StatusCode::OK }, "{why}");

        let missing = case["expect"]["missing"].as_array().expect("missing");
        let reported: Vec<String> = warnings
            .lock()
            .unwrap()
            .iter()
            .filter(|w| w.contains("does not include"))
            .cloned()
            .collect();
        assert_eq!(reported.len(), usize::from(!missing.is_empty()), "{why}");
        for member in missing {
            assert!(reported[0].contains(member.as_str().expect("a member")), "{why}");
        }
    }
}
