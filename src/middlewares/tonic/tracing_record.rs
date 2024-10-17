use axum::{
    body::Body,
    http::{HeaderMap, Request, Response},
};

use opentelemetry::{global, propagation::Extractor, trace::TraceContextExt};
use tonic::body::BoxBody;
use tower::Service;
use tracing::{warn, Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;

/// accept the trace context from the request.
pub fn accept_trace(request: Request<Body>) -> Request<Body> {
    // current context, if no or invalid data is received.
    let parent_context = global::get_text_map_propagator(|propagator| {
        propagator.extract(&HeaderExtractor(request.headers()))
    });
    Span::current().set_parent(parent_context);

    request
}

/// record the otlp trace ID of the given request as "trace_id" field in the current span.
pub fn record_trace_id(request: Request<Body>) -> Request<Body> {
    let span = Span::current();
    let trace_id = span.context().span().span_context().trace_id();
    span.record("trace_id", trace_id.to_string());

    request
}

pub struct HeaderExtractor<'a>(&'a HeaderMap);

impl<'a> Extractor for HeaderExtractor<'a> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|v| {
            let s = v.to_str();
            if let Err(ref error) = s {
                warn!(%error, ?v, "cannot convert header value to ASCII")
            };
            s.ok()
        })
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|k| k.as_str()).collect()
    }
}

#[derive(Debug, Clone, Default)]
pub struct TracingRecord;

impl<S> tower::Layer<S> for TracingRecord {
    type Service = TracingRecordService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        TracingRecordService { inner }
    }
}

#[derive(Debug, Clone)]
pub struct TracingRecordService<S> {
    inner: S,
}

type BoxFuture<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;

impl<S> Service<Request<Body>> for TracingRecordService<S>
where
    S: Service<Request<Body>, Response = Response<BoxBody>> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let request = accept_trace(req);
        let request = record_trace_id(request);

        let fut = self.inner.call(request);
        Box::pin(async move {
            let res = fut.await;
            res
        })
    }
}
