pub struct SorobanClient {
    rpc_url: String,
    http_client: reqwest::Client,
}

impl SorobanClient {
    pub fn new(rpc_url: String, http_client: reqwest::Client) -> Self {
        Self { rpc_url, http_client }
    }

    pub async fn get_events(
        &self,
        contract_address: &str,
        start_ledger: u64,
        limit: u32,
    ) -> anyhow::Result<serde_json::Value> {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getEvents",
            "params": {
                "startLedger": start_ledger,
                "filters": [{
                    "type": "contract",
                    "contractIds": [contract_address]
                }],
                "limit": limit
            }
        });

        let resp = self.http_client
            .post(&self.rpc_url)
            .json(&body)
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        Ok(resp)
    }
}
