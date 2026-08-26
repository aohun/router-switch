use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use serde_json::{json, Value};
use tokio_util::sync::CancellationToken;

use crate::GatewayError;

use super::{ModelEvent, ModelInvocation, Provider, ProviderStream};

pub struct OpenAiResponsesProvider {
    pub base_url: String,
    pub api_key: String,
    pub default_model: String,
    client: reqwest::Client,
}

impl OpenAiResponsesProvider {
    pub fn new(base_url: String, api_key: String, default_model: String) -> Self {
        Self {
            base_url,
            api_key,
            default_model,
            client: reqwest::Client::builder().build().unwrap_or_default(),
        }
    }
}

fn join_url(base: &str, path: &str) -> String {
    format!(
        "{}{}",
        base.trim_end_matches('/'),
        if path.starts_with('/') {
            path.to_string()
        } else {
            format!("/{path}")
        }
    )
}

impl Provider for OpenAiResponsesProvider {
    fn stream(
        &self,
        invocation: ModelInvocation,
        cancellation: CancellationToken,
    ) -> ProviderStream {
        let client = self.client.clone();
        let url = join_url(&self.base_url, "/responses");
        let api_key = self.api_key.clone();
        let model = if !invocation.model.is_empty() {
            invocation.model
        } else {
            self.default_model.clone()
        };

        let body = json!({
            "model": model,
            "stream": true,
            "input": invocation.prompt
        });

        let stream = async_stream::stream! {
            let request = client
                .post(&url)
                .header("authorization", format!("Bearer {}", api_key))
                .header("content-type", "application/json")
                .json(&body);

            let response = match request.send().await {
                Ok(res) => res,
                Err(err) => {
                    yield Err(GatewayError::Provider(format!("OpenAI Responses request failed: {err}")));
                    return;
                }
            };

            if !response.status().is_success() {
                let status = response.status();
                let error_text = response.text().await.unwrap_or_default();
                yield Err(GatewayError::Provider(format!(
                    "Responses error {status}: {error_text}"
                )));
                return;
            }

            let mut event_stream = response.bytes_stream().eventsource();
            while let Some(event_res) = event_stream.next().await {
                if cancellation.is_cancelled() {
                    return;
                }
                match event_res {
                    Ok(event) => {
                        let data = event.data.trim();
                        if data == "[DONE]" {
                            yield Ok(ModelEvent::Done);
                            break;
                        }
                        if let Ok(val) = serde_json::from_str::<Value>(data) {
                            if let Some(delta) = val.get("delta").and_then(Value::as_str) {
                                if !delta.is_empty() {
                                    yield Ok(ModelEvent::TextDelta(delta.to_string()));
                                }
                            } else if let Some(text) = val.pointer("/output_text").and_then(Value::as_str) {
                                if !text.is_empty() {
                                    yield Ok(ModelEvent::TextDelta(text.to_string()));
                                }
                            }
                        }
                    }
                    Err(err) => {
                        yield Err(GatewayError::Provider(format!("Responses stream error: {err}")));
                        return;
                    }
                }
            }
        };

        Box::pin(stream)
    }
}
