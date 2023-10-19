use prometheus::{self, IntCounter, register_int_counter, TextEncoder, Encoder, register_int_gauge, IntGauge};
use lazy_static::lazy_static;

lazy_static! {
  pub static ref CONNECTIONS_COUNTER: IntGauge = register_int_gauge!("current_connections", "number of connections to the GateServer").unwrap();
  pub static ref TOTAL_CONNECTIONS: IntCounter = register_int_counter!("total_connections", "number of connections to the GateServer").unwrap();
}


pub fn gather_metrics() -> String {
  let mut buffer = vec![];
  let encoder = TextEncoder::new();

  let metric_families = prometheus::gather();

  encoder.encode(&metric_families, &mut buffer).unwrap();
  String::from_utf8(buffer).unwrap()
}