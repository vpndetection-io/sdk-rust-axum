# [<img src="https://s3.vpndetection.io/vpndetection-public/brand/mark.svg" alt="VPNDetection" width="24"/>](https://vpndetection.io/) VPNDetection axum Middleware

[![crates.io](https://img.shields.io/crates/v/vpndetection-axum.svg)](https://crates.io/crates/vpndetection-axum)
[![docs.rs](https://img.shields.io/docsrs/vpndetection-axum)](https://docs.rs/vpndetection-axum)
[![license](https://img.shields.io/github/license/vpndetection-io/sdk-rust-axum.svg)](LICENSE)

The official [axum](https://github.com/tokio-rs/axum) middleware for the [VPNDetection](https://vpndetection.io) API.

It classifies the visitor behind each request — VPN, residential proxy, Tor, hosting, CDN, relay — and puts the answer on the request. Blocking is opt-in.

It is a `tower::Layer`, so it works in any tower stack, not only axum.

## Getting Started

```bash
cargo add vpndetection-axum
```

Requires Rust 1.85 or newer, and axum 0.8.

You need an API key. Create one in the [console](https://app.vpndetection.io); the free tier's allowance is counted per source address, and a server is a single source address, so a key is what makes this usable in production rather than optional.

```rust
use axum::{Router, routing::get};
use vpndetection::middleware::Options;
use vpndetection_axum::{Found, VPNDetection};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let guard = VPNDetection::new(
        Options::new().api_key(std::env::var("VPNDETECTION_API_KEY")?),
    )?;

    let app = Router::new().route("/", get(hello)).layer(guard);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await?;
    Ok(())
}

async fn hello(Found(found): Found) -> &'static str {
    match found.result.as_ref().map(|r| r.is_vpn) {
        Some(true) => "Hello, VPN user",
        _ => "Hello",
    }
}
```

By default nothing is blocked. Every request carries a `Lookup` and your own handlers decide what that means — which is usually what you want, because whether a VPN visitor is a problem depends entirely on what they are doing.

`Found` rejects with `500` on a route the layer does not cover, which is a wiring mistake rather than something a visitor can cause. Use `MaybeFound` where that is deliberate.

## Blocking

Set a `block_condition` and a matching request is answered with `403` and never reaches your handlers.

```rust
use vpndetection::middleware::{Condition, Options};

Options::new().block_condition(vec![Condition::from([("is_vpn", true)])]);
```

A condition is written in the shape of a result, keyed by the same names the API uses, and only the members you name are considered. That lets it reach the evidence, not just the flags:

```rust
use vpndetection::middleware::{Bound, Condition};

// one provider
Condition::new()
    .with("is_vpn", true)
    .with("vpn", Condition::from([("provider", "nordvpn")]));

// a numeric threshold
Condition::from([("resproxy", Condition::from([("hits", Bound::gte(5.0))]))]);

// any of these
Condition::from([("vpn", Condition::from([("confidence", vec!["high", "medium"])]))]);

// a list is OR
vec![
    Condition::from([("is_tor", true)]),
    Condition::from([("is_resproxy", true)]),
];
```

Values are matched by equality, strings without regard to case. A `Vec` means any-of. `Bound::gte`, `gt`, `lte` and `lt` compare numbers and chain into a range (`Bound::gte(5.0).and_lt(100.0)`); every bound you give must hold. Members set to `false` or `Value::Null` are ignored, so a condition states the signals you act on; one that constrains nothing would match every request, and is refused when the middleware is built rather than silently blocking all your traffic.

A policy that lives in your app's config rather than in its source is read with `from_json`, which takes the same shape:

```rust
let condition = vpndetection::middleware::from_json(&serde_json::json!({
    "is_vpn": true,
    "vpn": { "provider": ["nordvpn", "mullvad"] },
}))?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Replace the refusal with `on_blocked`:

```rust
# use axum::response::IntoResponse;
# use vpndetection::middleware::Options;
# fn f() -> Result<(), vpndetection::Error> {
vpndetection_axum::VPNDetection::new(Options::new())?
    .on_blocked(|_parts, _found| {
        (axum::http::StatusCode::FORBIDDEN, "VPN not allowed").into_response()
    });
# Ok(())
# }
```

## Where the client address comes from

This is the setting that decides whether any of the above works, and it is the one thing only you can get right.

By default the middleware reads axum's `ConnectInfo<SocketAddr>`, which is the socket peer — and which is **absent entirely unless you serve the app with `into_make_service_with_connect_info`**, as the example above does. Behind nginx, a load balancer, or a CDN, that peer is your proxy: a datacenter address, so a hosting rule would block every visitor you have.

For an edge that writes the address into its own header, name the header:

```rust
use vpndetection::middleware::{Options, header};

Options::new().ip_selector(header("CF-Connecting-IP")); // or True-Client-IP
```

`xff(0)` reads the left-most `X-Forwarded-For` entry. Be aware that the left-most entry is whatever the caller sent, because proxies append to that header — it is only trustworthy when an edge you control overwrites it. If you know how many proxies sit in front, count from the right instead: `xff(1)` is the address your nearest proxy saw.

Anything else, pass your own closure. It receives a `RequestView` and returns an address:

```rust
use std::sync::Arc;
use vpndetection::middleware::{IpSelector, Options};

let mine: IpSelector = Arc::new(|view| (view.header)("x-real-ip"));
Options::new().ip_selector(mine);
```

If the address resolves to a private one, the middleware says so once through `on_warn`. That is expected on localhost and is the signal to fix your configuration anywhere else.

## When a lookup fails

The request is let through, and the reason is on `Lookup::error`. Our outage should not become yours, so a network failure, an exhausted quota or a rejected key all fail open.

```rust
# use vpndetection_axum::Found;
async fn handler(Found(found): Found) -> &'static str {
    if let Some(error) = &found.error {
        tracing::warn!(%error, "vpndetection unavailable");
    }
    "ok"
}
```

Set `fail_closed(true)` to block instead. Private addresses are answered locally and never fail, so this will not lock you out in development.

## Cost and latency

Answers are cached per middleware for an hour, so a returning visitor costs nothing, and private addresses never leave the process. A cache miss is one request to our API, bounded at 2500 ms by default and not retried — on a request path, failing open quickly beats holding a visitor while we try again. Both are adjustable, and so is the cache, through a client you build yourself and pass as `client`.

Layer the routes that matter rather than the whole router, or skip what you do not care about:

```rust
# use vpndetection::middleware::Options;
# fn f() -> Result<(), vpndetection::Error> {
vpndetection_axum::VPNDetection::new(Options::new())?
    .skip(|parts| parts.uri.path().starts_with("/healthz"));
# Ok(())
# }
```

If you already hold a `vpndetection::Client`, pass it as `client` and the middleware will share it rather than building a second cache.

Beyond a few million distinct visitors a day, stop calling the API per request: [download the dataset](https://vpndetection.io/databases) and look addresses up locally instead.

## Absent is not false

Only `ip` and `is_vpn` come back on every plan. A field your plan does not include is `None`, which means "not in your plan" rather than "checked, and no".

```rust
# fn f(result: &vpndetection::Lookup) -> bool {
result.is_hosting.unwrap_or(false)   // when you only want the flag
# }
```

A `block_condition` naming a member your plan does not serve can never match, so the middleware warns once instead of failing silently. Set `on_missing_field(OnMissingField::Fail)` to make it an error.

## Other Libraries

There are official VPNDetection client libraries available for many languages including PHP, Python, Go, Java, Ruby, and many popular frameworks such as Django, Rails, and Laravel. See our GitHub at https://github.com/vpndetection-io for more.

## About VPNDetection

VPN Detection API: Accurate anonymity detection identifying VPNs, residential proxies, hosting servers, Tor nodes, CDNs, relays and more.

[<img src="https://s3.vpndetection.io/vpndetection-public/brand/mark.svg" alt="VPNDetection" width="96"/>](https://vpndetection.io/)

## License

This project is licensed under the [MIT License](LICENSE).
