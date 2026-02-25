use prometheus::{
    Encoder, GaugeVec, HistogramVec, IntCounterVec, IntGauge, Opts, Registry, TextEncoder,
};

#[derive(Clone)]
pub struct Metrics {
    registry: Registry,
    pub assignments_total: IntCounterVec,
    pub orders_in_queue: IntGauge,
    pub assignment_latency_seconds: HistogramVec,
    pub courier_utilization: GaugeVec,
}

impl Metrics {
    pub fn new() -> Self {
        let registry = Registry::new();

        let assignments_total = IntCounterVec::new(
            Opts::new("assignments_total", "Total assignments by outcome"),
            &["outcome"],
        )
        .expect("valid assignments_total metric");

        let orders_in_queue = IntGauge::new("orders_in_queue", "Current number of orders in queue")
            .expect("valid orders_in_queue metric");

        let assignment_latency_seconds = HistogramVec::new(
            prometheus::HistogramOpts::new(
                "assignment_latency_seconds",
                "Latency of assignment processing in seconds",
            ),
            &["outcome"],
        )
        .expect("valid assignment_latency_seconds metric");

        let courier_utilization = GaugeVec::new(
            Opts::new("courier_utilization", "Courier utilization ratio [0..1]"),
            &["courier_id"],
        )
        .expect("valid courier_utilization metric");

        registry
            .register(Box::new(assignments_total.clone()))
            .expect("register assignments_total");
        registry
            .register(Box::new(orders_in_queue.clone()))
            .expect("register orders_in_queue");
        registry
            .register(Box::new(assignment_latency_seconds.clone()))
            .expect("register assignment_latency_seconds");
        registry
            .register(Box::new(courier_utilization.clone()))
            .expect("register courier_utilization");

        Self {
            registry,
            assignments_total,
            orders_in_queue,
            assignment_latency_seconds,
            courier_utilization,
        }
    }

    pub fn encode(&self) -> Result<String, String> {
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();

        TextEncoder::new()
            .encode(&metric_families, &mut buffer)
            .map_err(|err| format!("failed to encode metrics: {err}"))?;

        String::from_utf8(buffer).map_err(|err| format!("metrics are not valid utf8: {err}"))
    }
}
