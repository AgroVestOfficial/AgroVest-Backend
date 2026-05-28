use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SorobanEvent {
    pub topic: Vec<String>,
    pub value: serde_json::Value,
    pub ledger: u64,
}

pub fn parse_event_domain(events: &serde_json::Value) -> Vec<SorobanEvent> {
    // Parse Soroban RPC getEvents response
    events
        .get("result")
        .and_then(|r| r.get("events"))
        .and_then(|e| e.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|ev| serde_json::from_value(ev.clone()).ok())
                .collect()
        })
        .unwrap_or_default()
}
