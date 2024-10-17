use opentelemetry::{global, propagation::Injector};
use tracing::span;
use tracing_opentelemetry::OpenTelemetrySpanExt;

struct MetadataMap<'a>(&'a mut tonic::metadata::MetadataMap);

/// 創建一個 帶有 current span 的 tonic request
pub fn create_request_with_span<T>(request: T) -> tonic::Request<T> {
    // create a new request
    let mut request = tonic::Request::new(request);

    // inject the current span context into the request
    let cx: opentelemetry::Context = span::Span::current().context();
    global::get_text_map_propagator(|propagator| {
        propagator.inject_context(&cx, &mut MetadataMap(request.metadata_mut()))
    });

    request
}

impl<'a> Injector for MetadataMap<'a> {
    /// Set a key and value in the MetadataMap.  Does nothing if the key or value are not valid inputs
    fn set(&mut self, key: &str, value: String) {
        if let Ok(key) = tonic::metadata::MetadataKey::from_bytes(key.as_bytes()) {
            if let Ok(val) = tonic::metadata::MetadataValue::try_from(&value) {
                self.0.insert(key, val);
            }
        }
    }
}
