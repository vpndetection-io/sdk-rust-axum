//! The official [axum](https://github.com/tokio-rs/axum) middleware for the
//! [VPNDetection](https://vpndetection.io) API.
//!
//! It classifies the visitor behind each request - VPN, residential proxy, Tor,
//! hosting, CDN, relay - and puts the answer on the request. Blocking is
//! opt-in.
//!
//! It is a [`tower_layer::Layer`], so it works in any tower stack, not only
//! axum.
//!
//! ```no_run
//! use axum::{Router, routing::get};
//! use vpndetection_axum::{Found, VPNDetection};
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let guard = VPNDetection::new(
//!     vpndetection::middleware::Options::new().api_key(std::env::var("VPNDETECTION_API_KEY")?),
//! )?;
//!
//! let app: Router = Router::new().route("/", get(hello)).layer(guard);
//!
//! async fn hello(Found(found): Found) -> &'static str {
//!     match found.result.as_ref().map(|r| r.is_vpn) {
//!         Some(true) => "Hello, VPN user",
//!         _ => "Hello",
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! By default nothing is blocked. Every request carries a
//! [`vpndetection::middleware::Lookup`] and your own handlers decide what that
//! means - which is usually what you want, because whether a VPN visitor is a
//! problem depends entirely on what they are doing.
//!
//! # Where the client address comes from
//!
//! By default the middleware reads axum's [`axum::extract::ConnectInfo`], which
//! is the socket peer - and which is **absent unless you served the app with
//! [`axum::Router::into_make_service_with_connect_info`]**. Behind nginx, a
//! load balancer or a CDN, that peer is your proxy: a datacenter address, so a
//! hosting rule would block every visitor you have. Name your edge's header
//! instead:
//!
//! ```
//! # use vpndetection::middleware::Options;
//! Options::new().ip_selector(vpndetection::middleware::header("CF-Connecting-IP"));
//! ```

use std::net::SocketAddr;
use std::sync::Arc;
use std::task::{Context, Poll};

use axum::extract::ConnectInfo;
use axum::response::{IntoResponse, Response};
use futures_util::future::BoxFuture;
use http::{Request, StatusCode};
use vpndetection::middleware::{Core, Lookup, Options, RequestView};

pub use vpndetection::middleware;

/// How a blocked request is answered, and which requests are classified at all.
///
/// Both are here rather than in [`Options`] because both need the framework's
/// own request type, which the shared core deliberately does not know.
type Blocked = Arc<dyn Fn(&http::request::Parts, &Lookup) -> Response + Send + Sync>;
type Skip = Arc<dyn Fn(&http::request::Parts) -> bool + Send + Sync>;

/// The middleware, as a [`tower_layer::Layer`].
///
/// `.layer(guard)` on a [`axum::Router`], or anywhere a tower layer goes.
#[derive(Clone)]
pub struct VPNDetection {
    core: Core,
    on_blocked: Blocked,
    skip: Option<Skip>,
}

impl VPNDetection {
    /// Builds the middleware, refusing a condition that constrains nothing and
    /// building a client when [`Options::client`] was not set.
    pub fn new(options: Options) -> Result<Self, vpndetection::Error> {
        Ok(Self { core: Core::new(options)?, on_blocked: Arc::new(refuse), skip: None })
    }

    /// Answers a request the condition matched, replacing the `403`.
    ///
    /// The handler never runs either way.
    ///
    /// ```
    /// # use axum::response::IntoResponse;
    /// # use vpndetection::middleware::Options;
    /// # fn f() -> Result<(), vpndetection::Error> {
    /// vpndetection_axum::VPNDetection::new(Options::new())?
    ///     .on_blocked(|_parts, found| {
    ///         let why = found.result.as_ref().and_then(|r| r.vpn.as_ref());
    ///         (axum::http::StatusCode::FORBIDDEN, format!("{why:?}")).into_response()
    ///     });
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn on_blocked<F>(mut self, on_blocked: F) -> Self
    where
        F: Fn(&http::request::Parts, &Lookup) -> Response + Send + Sync + 'static,
    {
        self.on_blocked = Arc::new(on_blocked);
        self
    }

    /// Skip classification for a request entirely - health checks, static
    /// assets, anything an app-wide layer would otherwise pay for.
    #[must_use]
    pub fn skip<F>(mut self, skip: F) -> Self
    where
        F: Fn(&http::request::Parts) -> bool + Send + Sync + 'static,
    {
        self.skip = Some(Arc::new(skip));
        self
    }
}

impl<S> tower_layer::Layer<S> for VPNDetection {
    type Service = VPNDetectionService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        VPNDetectionService {
            inner,
            core: self.core.clone(),
            on_blocked: Arc::clone(&self.on_blocked),
            skip: self.skip.clone(),
        }
    }
}

/// What [`VPNDetection`] wraps a service in. You never name this type.
#[derive(Clone)]
pub struct VPNDetectionService<S> {
    inner: S,
    core: Core,
    on_blocked: Blocked,
    skip: Option<Skip>,
}

impl<S, B> tower_service::Service<Request<B>> for VPNDetectionService<S>
where
    S: tower_service::Service<Request<B>, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
    B: Send + 'static,
{
    type Response = Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Response, S::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<B>) -> Self::Future {
        // The CLONE is the one that was polled ready; `self.inner` may not be.
        // Swapping rather than cloning in place is the tower idiom, and getting
        // it backwards is how a middleware calls a service that was never
        // readied.
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);
        let core = self.core.clone();
        let on_blocked = Arc::clone(&self.on_blocked);
        let skip = self.skip.clone();

        Box::pin(async move {
            let (mut parts, body) = request.into_parts();
            if skip.is_some_and(|skip| skip(&parts)) {
                return inner.call(Request::from_parts(parts, body)).await;
            }

            let found = {
                let header = |name: &str| {
                    parts.headers.get(name).and_then(|v| v.to_str().ok()).map(str::to_owned)
                };
                let framework_ip = || {
                    parts
                        .extensions
                        .get::<ConnectInfo<SocketAddr>>()
                        .map(|ConnectInfo(addr)| addr.ip().to_string())
                };
                core.evaluate(&RequestView { header: &header, framework_ip: &framework_ip }).await
            };
            let found = match found {
                Ok(found) => found,
                // A condition naming a member the plan does not serve, with
                // OnMissingField::Fail. A LOOKUP failure never lands here: it
                // rides on Lookup::error and the visitor is let through.
                Err(error) => {
                    return Ok(
                        (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response()
                    );
                }
            };

            if found.blocked {
                return Ok(on_blocked(&parts, &found));
            }
            parts.extensions.insert(found);
            inner.call(Request::from_parts(parts, body)).await
        })
    }
}

fn refuse(_parts: &http::request::Parts, _found: &Lookup) -> Response {
    // Written out rather than serialized: one fixed object is not worth a
    // serde_json dependency in a crate that otherwise needs none.
    (
        StatusCode::FORBIDDEN,
        [(http::header::CONTENT_TYPE, "application/json")],
        r#"{"error":"access denied"}"#,
    )
        .into_response()
}

/// What the middleware found out about this visitor.
///
/// ```
/// use vpndetection_axum::Found;
///
/// async fn handler(Found(found): Found) -> String {
///     format!("{:?}", found.result.map(|r| r.is_vpn))
/// }
/// ```
///
/// Rejects with `500` when the middleware did not run for this route, which is
/// a wiring mistake rather than something a visitor can cause. Use
/// [`MaybeFound`] on a route that is deliberately outside the layer, or one
/// `skip` claims.
#[derive(Debug, Clone)]
pub struct Found(pub Lookup);

/// [`Found`], for a route where the middleware may not have run.
#[derive(Debug, Clone)]
pub struct MaybeFound(pub Option<Lookup>);

impl<S: Send + Sync> axum::extract::FromRequestParts<S> for Found {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        parts.extensions.get::<Lookup>().cloned().map(Self).ok_or_else(|| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "vpndetection: this route is not behind the VPNDetection layer, or `skip` \
                 claimed the request; extract MaybeFound instead",
            )
                .into_response()
        })
    }
}

impl<S: Send + Sync> axum::extract::FromRequestParts<S> for MaybeFound {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        Ok(Self(parts.extensions.get::<Lookup>().cloned()))
    }
}
