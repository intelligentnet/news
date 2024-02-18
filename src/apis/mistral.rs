use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::Client;
use crate::apis::openai::Message;
//use dotenv::dotenv;
use std::env;
use serde_derive::{Deserialize, Serialize};

// Input structures
// Chat
#[derive(Debug, Serialize, Clone)]
pub struct ChatCompletion {
    pub model: String,
    pub messages: Vec<Message>,
    pub temperature: f32,
    //pub top_p: f32,
    //pub max_tokens: u32,
    //pub stream: bool,
    //pub safe_mode: bool,
    //pub random_seed: i32,
}

/*
#[derive(Debug, Serialize, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}
*/

// Output structures
// Chat
#[derive(Debug, Deserialize)]
pub struct APIResponse {
    pub id: String,
    //pub object: String,
    pub created: usize,
    pub model: String,
    pub choices: Option<Vec<APIChoice>>,
    //pub usage: String,
}

#[derive(Debug, Deserialize)]
pub struct APIChoice {
    //pub index: usize,
    pub message: APIMessage,
    pub finish_reason: String,
}

#[derive(Debug, Deserialize)]
pub struct APIMessage {
    pub role: String,
    pub content: String,
}

// Call Large Language Model
pub async fn call_mistral(messages: Vec<Message>) -> Result<String, Box<dyn std::error::Error + Send>> {
    let mistral_version: String = std::env::var("MISTRAL_VERSION").map_err(anyhow::Error::new)?;
    call_mistral_model(&mistral_version, messages).await
}

pub async fn call_mistral_model(model: &str, messages: Vec<Message>) -> Result<String, Box<dyn std::error::Error + Send>> {
    // Extract API Key information
    let api_key: String =
        env::var("MISTRAL_API_KEY").expect("MISTRAL_API_KEY not found in enviornment variables");
    //let api_org: String =
    //    env::var("OPEN_AI_ORG").expect("OPEN_AI_ORG not found in enviornment variables");

    // Confirm endpoint
    let url: &str = "https://api.mistral.ai/v1/chat/completions";

    // Create headers
    let mut headers: HeaderMap = HeaderMap::new();

    // We would like json
    headers.insert(
        "Content-Type",
        HeaderValue::from_str("appication/json")
            .map_err(|e| -> Box<dyn std::error::Error + Send> { Box::new(e) })?,
    );
    // Create api key header
    headers.insert(
        "Authorization",
        HeaderValue::from_str(&format!("Bearer {}", api_key))
            .map_err(|e| -> Box<dyn std::error::Error + Send> { Box::new(e) })?,
    );

    // Create client
    let client: Client = Client::builder()
        .user_agent("TargetR")
        .timeout(std::time::Duration::new(90, 0))
        .gzip(true)
        .default_headers(headers)
        .build()
        .map_err(|e| -> Box<dyn std::error::Error + Send> { Box::new(e) })?;

    // Create chat completion
    let chat_completion: ChatCompletion = ChatCompletion {
        model: model.into(),
        messages,
        temperature: 0.2,
    };

//println!("{:?}", serde_json::to_string(&chat_completion));
    // Extract API Response
    let res = client
        .post(url)
        .json(&chat_completion)
        .send()
        .await;
//println!("### {:?}", res);
    let res: APIResponse = res
        .map_err(|e| -> Box<dyn std::error::Error + Send> { Box::new(e) })?
        .json()
        .await
        .map_err(|e| -> Box<dyn std::error::Error + Send> { Box::new(e) })?;
//println!("### {:?}", res);

    // Send Response
    match res.choices {
        Some(choices) => {
//println!("choices {:?}", choices);
            let text = choices[0].message.content.clone();
            let text = text.lines().filter(|l| !l.starts_with("```")).fold(String::new(), |s, l| s + l + "\n");

            Ok(text)
        },
        None => {
            Err(anyhow::Error::msg("No Choice found").into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_call_mistral() {
        let messages: Vec<Message> = vec![Message { role: "user".into(), content: "What is the meaining of life?".into() }];
        match call_mistral(messages).await {
            Ok(answer) => { println!("{answer}"); assert!(true) },
            Err(e) => { println!("{e}"); assert!(false) },
        }
    }
}
