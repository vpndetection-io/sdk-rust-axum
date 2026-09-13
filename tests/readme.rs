// Every code block in README.md, compiled but never run.
//
// The README is the API contract customers actually read, so a rename or a
// signature change that invalidates it should fail the build rather than reach
// a reader. Mirror any README edit here.
#![allow(unused, path_statements, clippy::no_effect)]

use std::sync::Arc;

use axum::response::IntoResponse;
use axum::{Router, routing::get};
use vpndetection::middleware::{
    Bound, Condition, IpSelector, OnMissingField, Options, header, xff,
};
use vpndetection_axum::{Found, MaybeFound, VPNDetection};

async fn getting_started() -> Result<(), Box<dyn std::error::Error>> {
    let guard = VPNDetection::new(Options::new().api_key(std::env::var("VPNDETECTION_API_KEY")?))?;

    let app = Router::new().route("/", get(hello)).layer(guard);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    axum::serve(listener, app.into_make_service_with_connect_info::<std::net::SocketAddr>())
        .await?;
    Ok(())
}

async fn hello(Found(found): Found) -> &'static str {
    match found.result.as_ref().map(|r| r.is_vpn) {
        Some(true) => "Hello, VPN user",
        _ => "Hello",
    }
}

fn blocking() -> Result<(), Box<dyn std::error::Error>> {
    Options::new().block_condition(vec![Condition::from([("is_vpn", true)])]);

    // one provider
    Condition::new().with("is_vpn", true).with("vpn", Condition::from([("provider", "nordvpn")]));

    // a numeric threshold
    Condition::from([("resproxy", Condition::from([("hits", Bound::gte(5.0))]))]);

    // any of these
    Condition::from([("vpn", Condition::from([("confidence", vec!["high", "medium"])]))]);

    // a list is OR
    vec![Condition::from([("is_tor", true)]), Condition::from([("is_resproxy", true)])];

    Bound::gte(5.0).and_lt(100.0);

    let condition = vpndetection::middleware::from_json(&serde_json::json!({
        "is_vpn": true,
        "vpn": { "provider": ["nordvpn", "mullvad"] },
    }))?;

    VPNDetection::new(Options::new())?.on_blocked(|_parts, _found| {
        (axum::http::StatusCode::FORBIDDEN, "VPN not allowed").into_response()
    });
    Ok(())
}

fn selectors() -> Result<(), vpndetection::Error> {
    Options::new().ip_selector(header("CF-Connecting-IP")); // or True-Client-IP
    Options::new().ip_selector(xff(0));
    Options::new().ip_selector(xff(1));

    let mine: IpSelector = Arc::new(|view| (view.header)("x-real-ip"));
    Options::new().ip_selector(mine);

    Options::new().fail_closed(true);
    Options::new().on_missing_field(OnMissingField::Fail);

    VPNDetection::new(Options::new())?.skip(|parts| parts.uri.path().starts_with("/healthz"));
    Ok(())
}

async fn failure(Found(found): Found) -> &'static str {
    if let Some(error) = &found.error {
        // tracing::warn!(%error, "vpndetection unavailable") in the README; the
        // adapter does not depend on tracing, so this stands in for it.
        eprintln!("vpndetection unavailable: {error}");
    }
    "ok"
}

fn absent_is_not_false(result: &vpndetection::Lookup) -> bool {
    result.is_hosting.unwrap_or(false)
}
