use std::{process, time::Duration};

use opentelemetry::{trace::TraceError, KeyValue};
use opentelemetry_otlp::{ExportConfig, Protocol, WithExportConfig};
use opentelemetry_sdk::{
    metrics::{
        reader::{DefaultAggregationSelector, DefaultTemporalitySelector},
        SdkMeterProvider,
    },
    propagation::TraceContextPropagator,
    runtime,
    trace::{self as sdktrace},
    Resource,
};
use tracing_loki::{url::Url, BackgroundTask, Layer};
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

use crate::enums::LogLevel;

/// telemetry builder
///
///  ### 參數:
///  - `service_name`: 服務名稱
/// - `log_level`: log 等級 預設為 info
/// - `trace_export_url`: 接收 trace 的 服務位置
/// - `metrics_export_url`: 接收 metrics 的 服務位置
/// - `loki_export_url`: 接收 log 的 服務位置
///
/// ### 使用範例:
/// ```rust
/// use common_lib::components::telemetry_initializer::Builder;
/// use common_lib::enums::LogLevel;
///
/// Builder::new("service_name")
///    .set_log_level(LogLevel::Debug) // 如果不加這行預設為 info
///    .enable_tracing("http://localhost:4317") // 如果不要啟動 tracing 就不要加這行
///    .enable_metrics("http://localhost:4317") // 如果不要啟動 metrics 就不要加這行
///    .enable_log("http://localhost:3100") // 如果不要啟動 log 就不要加這行
///    .build();
/// ```
/// ### `panic`:
/// - 如果初始化失敗會 panic
/// - 如果設定的 url 有誤會panic
pub struct Builder<'a> {
    service_name: &'a str,
    log_level: LogLevel,
    trace_export_url: Option<&'a str>,
    metrics_export_url: Option<&'a str>,
    loki_export_url: Option<&'a str>,
}

impl<'a> Builder<'a> {
    pub fn new(service_name: &'a str) -> Self {
        Self {
            service_name,
            log_level: LogLevel::Info,
            trace_export_url: None,
            metrics_export_url: None,
            loki_export_url: None,
        }
    }

    pub fn set_log_level(self, level: LogLevel) -> Self {
        Self {
            log_level: level,
            ..self
        }
    }

    pub fn enable_tracing(self, export_url: &'a str) -> Self {
        Self {
            trace_export_url: Some(export_url),
            ..self
        }
    }

    pub fn enable_metrics(self, export_url: &'a str) -> Self {
        Self {
            metrics_export_url: Some(export_url),
            ..self
        }
    }

    pub fn enable_log(self, export_url: &'a str) -> Self {
        Self {
            loki_export_url: Some(export_url),
            ..self
        }
    }

    pub fn build(self) {
        // init tracing
        let trace_layer = if let Some(trace_export_url) = self.trace_export_url {
            let tracer = self
                .init_tracer(trace_export_url)
                .expect("Failed to initialize OpenTelemetry tracer.");
            Some(tracing_opentelemetry::layer().with_tracer(tracer))
        } else {
            None
        };

        // init loki
        let loki_layer = if let Some(loki_export_url) = self.loki_export_url {
            let (loki_layer, upload_log_task) = self
                .init_loki(loki_export_url)
                .expect("Failed to init Loki.");
            let _loki_task: tokio::task::JoinHandle<()> = tokio::spawn(upload_log_task);
            Some(loki_layer)
        } else {
            None
        };

        // init metrics
        let _metrics_provider = if let Some(metrics_export_url) = self.metrics_export_url {
            Some(
                self.init_metrics(metrics_export_url)
                    .expect("Failed to initialize OpenTelemetry metrics."),
            )
        } else {
            None
        };

        // set log level
        let env_filter =
            EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new(self.log_level.to_string()));

        // init subscriber
        let subscriber = Registry::default()
            .with(env_filter)
            .with(super::ConsoleLogLayer)
            .with(TraceIdLayer)
            .with(trace_layer)
            .with(loki_layer);

        // Set the global subscriber
        tracing::subscriber::set_global_default(subscriber)
            .expect("Failed to set global subscriber.");
    }

    fn init_tracer(&self, export_url: &'a str) -> Result<sdktrace::Tracer, TraceError> {
        opentelemetry::global::set_text_map_propagator(TraceContextPropagator::new());
        opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(
                opentelemetry_otlp::new_exporter()
                    .tonic()
                    .with_endpoint(export_url),
            )
            .with_trace_config(sdktrace::config().with_resource(Resource::new(vec![
                KeyValue::new("service.name", self.service_name.to_string()),
            ])))
            .install_batch(runtime::Tokio)
    }

    fn init_loki(
        &self,
        export_url: &'a str,
    ) -> Result<(Layer, BackgroundTask), tracing_loki::Error> {
        let (layer, task) = tracing_loki::builder()
            .label("service_name", self.service_name)?
            .extra_field("process_id", format!("{}", process::id()))?
            .build_url(Url::parse(export_url).unwrap())?;
        Ok((layer, task))
    }

    fn init_metrics(
        &self,
        export_url: &'a str,
    ) -> Result<SdkMeterProvider, opentelemetry::metrics::MetricsError> {
        let export_config = ExportConfig {
            endpoint: export_url.to_string(),
            timeout: Duration::from_secs(3),
            protocol: Protocol::Grpc,
        };

        opentelemetry_otlp::new_pipeline()
            .metrics(opentelemetry_sdk::runtime::Tokio)
            .with_exporter(
                opentelemetry_otlp::new_exporter()
                    .tonic()
                    .with_export_config(export_config),
            )
            .with_resource(Resource::new(vec![KeyValue::new(
                "service_name",
                self.service_name.to_string(),
            )]))
            .with_period(Duration::from_secs(3))
            .with_timeout(Duration::from_secs(10))
            .with_aggregation_selector(DefaultAggregationSelector::new())
            .with_temporality_selector(DefaultTemporalitySelector::new())
            .build()
    }
}

struct TraceIdLayer;

impl<S> tracing_subscriber::Layer<S> for TraceIdLayer
where
    S: tracing::Subscriber + for<'span> tracing_subscriber::registry::LookupSpan<'span>,
{
    fn on_new_span(
        &self,
        _: &tracing::span::Attributes<'_>,
        id: &tracing::Id,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        if let Some(span) = ctx.span(id) {
            let trace_id = id.into_u64().to_string(); // 获取 trace_id
            span.extensions_mut().insert(trace_id.clone()); // 将 trace_id 存储到 span 的扩展字段中
        }
    }
}
