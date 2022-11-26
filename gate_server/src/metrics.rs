use prometheus::{self, IntCounter, register_int_counter, TextEncoder, Encoder};
use lazy_static::lazy_static;

lazy_static! {
  pub static ref CONNECTIONS_COUNTER: IntCounter = register_int_counter!("connections", "number of connections to the GateServer").unwrap();
  pub static ref TOTAL_CONNECTIONS: IntCounter = register_int_counter!("connections", "number of connections to the GateServer").unwrap();
}


pub fn gather_metrics() -> String {
  let mut buffer = vec![];
  let encoder = TextEncoder::new();

  let metric_families = prometheus::gather();

  encoder.encode(&metric_families, &mut buffer).unwrap();
  String::from_utf8(buffer).unwrap()
}