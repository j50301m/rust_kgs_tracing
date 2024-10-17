use axum::{
    body::Body,
    http::{Request, Uri},
};

use tower_http::{
    classify::{GrpcErrorsAsFailures, SharedClassifier},
    trace::TraceLayer,
};
use tracing::{field, info_span, Span};

/// create a new span for each request.
pub fn root_span_builder(
) -> TraceLayer<SharedClassifier<GrpcErrorsAsFailures>, fn(&Request<Body>) -> Span> {
    let layer: TraceLayer<SharedClassifier<GrpcErrorsAsFailures>, fn(&Request<Body>) -> Span> =
        TraceLayer::new_for_grpc().make_span_with(make_root_span);
    layer
}

fn make_root_span(request: &Request<Body>) -> Span {
    let headers = request.headers();
    let (service, method) = extract_service_method(request.uri());
    info_span!(
        "incoming request",
        ?headers,
        trace_id = field::Empty,
        otel.kind = "server",
        otel.name = format!("{}/{}", service, method),
    )
}

pub fn extract_service_method(uri: &Uri) -> (&str, &str) {
    let path = uri.path();
    let mut parts = path.split('/').filter(|x| !x.is_empty());
    let service = parts
        .next()
        .unwrap_or_default()
        .split_once('.')
        .map(|(_, s)| s)
        .unwrap_or_default();
    let method = parts.next().unwrap_or_default();
    (service, method)
}
