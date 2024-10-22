mod graphite_gateway;
mod statsd_gateway;

use const_format::concatcp;
pub use graphite_gateway::GraphiteGateway;
pub use statsd_gateway::StatsdGateway;

use dipstick::*;

const METRICS_PREFIX: &str = "evolution-lab.";
const METRICS_MAX: &str = concatcp!(METRICS_PREFIX, "max");
const METRICS_MEAN: &str = concatcp!(METRICS_PREFIX, "mean");
const METRICS_MIN: &str = concatcp!(METRICS_PREFIX, "min");
const METRICS_STD_DEV: &str = concatcp!(METRICS_PREFIX, "std-dev");

metrics! {
    MY_PROXY: Proxy = "Graphite_Proxy" => {
        MAX: Gauge = METRICS_MAX;
        MEAN: Gauge = METRICS_MEAN;
        MIN: Gauge = METRICS_MIN;
        STD_DEV: Gauge = METRICS_STD_DEV;
    }
}
