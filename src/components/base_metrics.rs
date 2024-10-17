use opentelemetry::{global, metrics::Unit, KeyValue};
use tokio::time::Duration;

/// 基礎指標: 量測 系統CPU 與 RAM 使用率
pub fn base_metrics(service_name: &str) {
    let meter = global::meter("metrics-example");

    // measure cpu
    let cpu_gauge = meter
        .f64_gauge("cpu_usage")
        .with_description("CPU usage percentage")
        .with_unit(Unit::new("percent"))
        .init();

    // measure ram
    let ram_gauge = meter
        .i64_gauge("ram_usage")
        .with_description("RAM usage in bytes")
        .init();

    let mut system = sysinfo::System::new_all();
    let mut interval = tokio::time::interval(Duration::from_secs(10));

    let service_name = service_name.to_string();
    tokio::spawn(async move {
        loop {
            interval.tick().await;
            system.refresh_all();

            let cpu_usage = system.global_cpu_info().cpu_usage();
            let ram_usage = system.used_memory() as i64;

            cpu_gauge.record(
                cpu_usage as f64,
                &[KeyValue::new("service.name", service_name.clone())],
            );
            ram_gauge.record(
                ram_usage,
                &[KeyValue::new("service.name", service_name.clone())],
            );
        }
    });
}
