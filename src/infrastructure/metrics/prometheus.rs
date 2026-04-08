//! Prometheus exporter для метрик smith-rust.
//!
//! Запускает HTTP-сервер на указанном порту с endpoint `/metrics`.

use metrics::counter;
use tracing::{info, warn};

/// Имена метрик.
pub mod names {
    pub const REQUESTS_TOTAL: &str = "smith_requests_total";
    pub const REQUEST_DURATION_SECONDS: &str = "smith_request_duration_seconds";
    pub const ACTIVE_SESSIONS: &str = "smith_active_sessions";
    pub const TOOLS_EXECUTED_TOTAL: &str = "smith_tools_executed_total";
    pub const ERRORS_TOTAL: &str = "smith_errors_total";
}

/// Инициализирует Prometheus exporter на заданном порту.
///
/// # Arguments
///
/// * `port` — порт для HTTP-сервера метрик.
///
/// Возвращает JoinHandle фоновой задачи.
pub fn init_exporter(port: u16) -> tokio::task::JoinHandle<()> {
    info!(port, "initializing prometheus metrics exporter");

    let handle = tokio::spawn(async move {
        // metrics-exporter-prometheus запускает свой HTTP-сервер
        // В рамках этого шага используем упрощённую реализацию.
        // Для production: metrics_exporter_prometheus::PrometheusBuilder::new()
        //     .with_http_listener(([0, 0, 0, 0], port))
        //     .install()
        match metrics_exporter_prometheus::PrometheusBuilder::new()
            .with_http_listener(([0, 0, 0, 0], port))
            .install()
        {
            Ok(_) => info!(port, "prometheus exporter started"),
            Err(e) => warn!(port, error = %e, "failed to start prometheus exporter"),
        }

        // Держим задачу живой
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        }
    });

    handle
}

/// Инкрементирует счётчик запросов.
pub fn record_request(status: &str) {
    counter!(names::REQUESTS_TOTAL, "status" => status.to_string()).increment(1);
}

/// Инкрементирует счётчик ошибок.
pub fn record_error(error_type: &str) {
    counter!(names::ERRORS_TOTAL, "type" => error_type.to_string()).increment(1);
}

/// Инкрементирует счётчик выполненных инструментов.
pub fn record_tool_execution(tool_name: &str) {
    counter!(names::TOOLS_EXECUTED_TOTAL, "tool" => tool_name.to_string()).increment(1);
}

/// Записывает длитель запроса.
pub fn record_request_duration(seconds: f64) {
    let histogram = metrics::histogram!(names::REQUEST_DURATION_SECONDS);
    histogram.record(seconds);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metric_names() {
        assert_eq!(names::REQUESTS_TOTAL, "smith_requests_total");
        assert_eq!(names::ACTIVE_SESSIONS, "smith_active_sessions");
    }
}
