
pub mod strategy;
pub mod scan_strategy;
pub mod exchange_strategy;
pub mod join_strategy;
pub mod aggregate_strategy;
pub mod result_sink_strategy;
pub mod olap_table_sink_strategy;

pub use strategy::SpecializedMetricsStrategy;

#[derive(Debug, Clone)]
pub struct SpecializedMetricsParser {
    scan: ScanStrategy,
    exchange_sink: ExchangeSinkStrategy,
    exchange_source: ExchangeSourceStrategy,
    join: JoinStrategy,
    aggregate: AggregateStrategy,
    result_sink: ResultSinkStrategy,
    olap_table_sink: OlapTableSinkStrategy,
}

impl Default for SpecializedMetricsParser {
    fn default() -> Self {
        Self::new()
    }
}

impl SpecializedMetricsParser {
    pub fn new() -> Self {
        Self {
            scan: ScanStrategy,
            exchange_sink: ExchangeSinkStrategy,
            exchange_source: ExchangeSourceStrategy,
            join: JoinStrategy,
            aggregate: AggregateStrategy,
            result_sink: ResultSinkStrategy,
            olap_table_sink: OlapTableSinkStrategy,
        }
    }
    
    pub fn parse(&self, operator_name: &str, text: &str) -> crate::models::OperatorSpecializedMetrics {
        use crate::models::OperatorSpecializedMetrics;
        
        println!("DEBUG: specialized_parser.parse called with operator_name: '{}', text length: {}", operator_name, text.len());
        
        match operator_name {
            "OLAP_SCAN" | "CONNECTOR_SCAN" => self.scan.parse(text),
            "EXCHANGE_SINK" => self.exchange_sink.parse(text),
            "EXCHANGE_SOURCE" => self.exchange_source.parse(text),
            "JOIN" | "HASH_JOIN" | "NEST_LOOP_JOIN" => self.join.parse(text),
            "AGGREGATE" | "AGGREGATION" => self.aggregate.parse(text),
            "RESULT_SINK" => self.result_sink.parse(text),
            "OLAP_TABLE_SINK" => self.olap_table_sink.parse(text),
            _ => {
                println!("DEBUG: No specialized parser for operator: '{}'", operator_name);
                OperatorSpecializedMetrics::None
            }
        }
    }
}
pub use scan_strategy::ScanStrategy;
pub use exchange_strategy::{ExchangeSinkStrategy, ExchangeSourceStrategy};
pub use join_strategy::JoinStrategy;
pub use aggregate_strategy::AggregateStrategy;
pub use result_sink_strategy::ResultSinkStrategy;
pub use olap_table_sink_strategy::OlapTableSinkStrategy;
