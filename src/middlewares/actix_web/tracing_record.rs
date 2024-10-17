use std::future::{ready, Ready};

use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage,
};
use futures_util::future::LocalBoxFuture;
use opentelemetry::trace::TraceContextExt;
use tracing_actix_web::RootSpan;
use tracing_opentelemetry::OpenTelemetrySpanExt;

pub struct TracingRecord;

impl<S, B> Transform<S, ServiceRequest> for TracingRecord
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = TracingRecordMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(TracingRecordMiddleware { service }))
    }
}

pub struct TracingRecordMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for TracingRecordMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);
    fn call(&self, req: ServiceRequest) -> Self::Future {
        // if span is exist, set trace_id to span for `CustomRootSpanBuilder`
        if let Some(span) = req.extensions().get::<RootSpan>().cloned() {
            let trace_id = span.context().span().span_context().trace_id();
            span.record("trace_id", trace_id.to_string());
        }

        let fut = self.service.call(req);
        Box::pin(async move {
            let res = fut.await?;
            Ok(res)
        })
    }
}
