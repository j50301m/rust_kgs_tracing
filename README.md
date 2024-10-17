# Kgs Tracing

方便在Rust專案中 初始化open telemetry的包 讓我們統一初始化格式

## Usage

Init kgs_tracing

Call Builder is going to set the opentelemetry related source to  global

``` rust
 Builder::new("service_name")
    .set_log_level(LogLevel::Debug) // 如果不加這行預設為 info
    .enable_tracing("http://localhost:4317") // 如果不要啟動 tracing 就不要加這行
    .enable_metrics("http://localhost:4317") // 如果不要啟動 metrics 就不要加這行
    .enable_log("http://localhost:3100") // 如果不要啟動 log 就不要加這行
    .build();
```

Use it

```rust
 #[tracing::instrument]
 async fn foo() {
    // Do your something
 }
```
