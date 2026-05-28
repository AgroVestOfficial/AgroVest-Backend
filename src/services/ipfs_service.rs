use reqwest::Client;

#[derive(Clone)]
pub struct IpfsService {
    client: Client,
    pinata_api_key: String,
    pinata_secret_key: String,
    gateway_url: String,
}

impl IpfsService {
    pub fn new(
        client: Client,
        pinata_api_key: String,
        pinata_secret_key: String,
        gateway_url: String,
    ) -> Self {
        Self {
            client,
            pinata_api_key,
            pinata_secret_key,
            gateway_url,
        }
    }

    pub async fn pin_file(
        &self,
        file_name: &str,
        file_bytes: Vec<u8>,
        mime_type: &str,
    ) -> Result<PinResult, anyhow::Error> {
        let form = reqwest::multipart::Form::new().part(
            "file",
            reqwest::multipart::Part::bytes(file_bytes)
                .file_name(file_name.to_string())
                .mime_str(mime_type)?,
        );

        let resp = self
            .client
            .post("https://api.pinata.cloud/pinning/pinFileToIPFS")
            .header("pinata_api_key", &self.pinata_api_key)
            .header("pinata_secret_api_key", &self.pinata_secret_key)
            .multipart(form)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Pinata upload failed ({}): {}", status, body));
        }

        let data: serde_json::Value = resp.json().await?;

        let cid = data["IpfsHash"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing IpfsHash"))?
            .to_string();

        let url = format!("{}/{}", self.gateway_url, cid);

        Ok(PinResult { cid, url })
    }
}

#[derive(serde::Serialize)]
pub struct PinResult {
    pub cid: String,
    pub url: String,
}
