use base64::{engine::general_purpose, Engine as _};
use reqwest::Client;
use sha2::{Digest, Sha256};
use sigh::alg::RsaSha256;
use sigh::{Key, PrivateKey, SigningConfig};
use url::Url;

pub struct SignedRequester {
    private_key: PrivateKey,
    key_id: String,
    client: Client,
}

impl SignedRequester {
    pub fn new(pem: &str, key_id: &str) -> Self {
        let private_key = PrivateKey::from_pem(pem.as_bytes()).unwrap();
        Self {
            private_key,
            key_id: key_id.to_owned(),
            client: reqwest::Client::new(),
        }
    }

    pub async fn get(&self, url: Url) -> Result<String, anyhow::Error> {
        let host = url.host_str().unwrap();
        let signing_config = SigningConfig::new(RsaSha256, &self.private_key, &self.key_id, false);
        let mut req: http::Request<reqwest::Body> = http::Request::builder()
            .method("GET")
            .uri(url.as_str())
            .header("Accept", "application/activity+json")
            .header(
                "Date",
                chrono::Utc::now()
                    .format("%a, %d %b %Y %H:%M:%S GMT")
                    .to_string(),
            )
            .header("Host", host)
            .body(reqwest::Body::from(""))
            .unwrap();

        signing_config.sign(&mut req)?;
        let result = self.client.execute(req.try_into().unwrap()).await?;

        Ok(result.text().await?)
    }

    pub async fn post(
        &self,
        url: Url,
        payload: serde_json::Value,
    ) -> Result<String, anyhow::Error> {
        let host = url.host_str().unwrap();
        let payload_str = payload.to_string();

        let mut hasher = Sha256::new();
        hasher.update(payload_str.as_bytes());
        let digest = general_purpose::STANDARD.encode(hasher.finalize());

        // Mastodon only supports rsa-sha256
        let signing_config = SigningConfig::new(RsaSha256, &self.private_key, &self.key_id, true);

        let mut req: http::Request<reqwest::Body> = http::Request::builder()
            .method("POST")
            .uri(url.as_str())
            .header("Accept", "application/activity+json")
            .header(
                "Date",
                chrono::Utc::now()
                    .format("%a, %d %b %Y %H:%M:%S GMT")
                    .to_string(),
            )
            .header("Host", host)
            .header("Content-Type", "application/activity+json")
            .header("Digest", format!("SHA-256={}", digest))
            .body(reqwest::Body::from(payload_str))
            .unwrap();

        signing_config.sign(&mut req)?;
        let result = self.client.execute(req.try_into().unwrap()).await?;

        Ok(result.text().await?)
    }
}
