use rouille::{Response};
use super::metrics;

pub fn start_webserver() {
  rouille::start_server("0.0.0.0:8888", move |_| {
    Response::text(metrics::gather_metrics())
  });
}
