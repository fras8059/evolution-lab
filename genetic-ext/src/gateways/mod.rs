mod graphite_gateway;
mod statsd_gateway;

pub use graphite_gateway::GraphiteGateway;
pub use statsd_gateway::StatsdGateway;

use dipstick::*;

metrics! {
    MY_PROXY: Proxy = "Graphite_Proxy" => {
        BEST_EVAL: Gauge = "best-eval";
    }
}
