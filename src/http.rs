use anyhow::{Context, Result};
use reqwest::blocking::Client;
use reqwest::Method;
use serde_json::Value;

pub struct HttpClient {
    base_url: String,
    api_token: String,
    client: Client,
}

pub struct ResponseData {
    pub status: u16,
    pub body: Value,
}

impl HttpClient {
    pub fn new(base_url: String, api_token: String) -> Result<Self> {
        let client = Client::builder()
            .user_agent("cloudflare-cli")
            .build()
            .context("build http client")?;
        Ok(Self {
            base_url,
            api_token,
            client,
        })
    }

    pub fn execute(
        &self,
        method: Method,
        path: &str,
        query: &[(String, String)],
        headers: &[(String, String)],
        body: Option<Value>,
    ) -> Result<ResponseData> {
        let mut url = build_url(&self.base_url, path)?;
        {
            let mut pairs = url.query_pairs_mut();
            for (k, v) in query {
                pairs.append_pair(k, v);
            }
        }

        let mut req = self
            .client
            .request(method, url)
            .header("authorization", format!("Bearer {}", self.api_token));

        if let Some(value) = body {
            req = req.header("content-type", "application/json").json(&value);
        }

        for (k, v) in headers {
            req = req.header(k, v);
        }

        let resp = req.send().context("send request")?;
        let status = resp.status();
        let text = resp.text().context("read response body")?;
        let body = serde_json::from_str(&text).unwrap_or_else(|_| Value::String(text));

        Ok(ResponseData {
            status: status.as_u16(),
            body,
        })
    }
}

fn build_url(base: &str, path: &str) -> Result<reqwest::Url> {
    let base = base.trim_end_matches('/');
    let path = path.trim_start_matches('/');
    let full = format!("{}/{}", base, path);
    reqwest::Url::parse(&full).context("invalid url")
}
