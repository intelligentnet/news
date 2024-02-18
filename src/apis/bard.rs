use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::Client;
use std::process::Command;
use serde_derive::{Deserialize, Serialize};
use crate::template::Template;
use crate::apis::openai::Message;

// Input structures
// Chat
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BardCompletion {
    pub contents: Vec<Content>,
    pub safety_settings: Vec<Safety>,
    pub generation_config: Config,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Content {
    pub role: String,
    pub parts: Vec<Part>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Part {
    pub text: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct Safety {
    pub category: String,
    pub threshold: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    temperature: f32,
    candidate_count: usize,
    max_output_tokens: usize,
}

// Output structures
// Chat
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BardResponse {
    pub candidates: Vec<Candidate>,
    //pub usage_metadata: Metadata, // TODO: Fix as not parsing!
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub content: Content,
    pub safety_ratings: Vec<OutSafety>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OutSafety {
    pub category: String,
    pub probability: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    pub prompt_token_count: usize,
    pub candidates_token_count: usize,
    pub total_token_count: usize,
}

// Call Large Language Model (i.e. Bard Gemini)
pub async fn call_bard(messages: Vec<Content>) -> Result<String, Box<dyn std::error::Error + Send>> {
    call_bard_model(messages).await
}

pub async fn call_bard_model(contents: Vec<Content>) -> Result<String, Box<dyn std::error::Error + Send>> {
    let url: String = Template::new("${BARD_URL}").render_env();
    let client = get_client().await?;

    // Create chat completion
    let bard_completion: BardCompletion = BardCompletion {
        contents, 
        safety_settings: vec![
            Safety { category: "HARM_CATEGORY_HARASSMENT".into(), threshold: "BLOCK_NONE".into() },
            Safety { category: "HARM_CATEGORY_HATE_SPEECH".into(), threshold: "BLOCK_NONE".into() },
            Safety { category: "HARM_CATEGORY_SEXUALLY_EXPLICIT".into(), threshold: "BLOCK_NONE".into() },
            Safety { category: "HARM_CATEGORY_DANGEROUS_CONTENT".into(), threshold: "BLOCK_NONE".into() }
        ],
        generation_config: Config { temperature: 0.2, candidate_count: 1, max_output_tokens: 8192 }

    };

//println!("{:?}", serde_json::to_string(&bard_completion));
    // Extract Response
    let res = client
        .post(url)
        .json(&bard_completion)
        .send()
        .await;
//let res = res.unwrap().text().await.unwrap();
//println!("{:?}", res);
//let res: Vec<BardResponse> = serde_json::from_str(&res).unwrap();
    let res: Vec<BardResponse> = res
        .map_err(|e| -> Box<dyn std::error::Error + Send> { Box::new(e) })?
        .json()
        .await
        .map_err(|e| -> Box<dyn std::error::Error + Send> { Box::new(e) })?;

    // Consolidate the candidates into a single string!!!
    let mut text = String::new();
    for br in res {
        for c in br.candidates {
            for p in c.content.parts {
                text.push_str(p.text.trim());
            }
        }
    }

    // Remove any comments
    let text = text.lines().filter(|l| !l.starts_with("```")).fold(String::new(), |s, l| s + l + "\n");
//println!("-- {text}");

    Ok(text)
}

async fn get_client() -> Result<Client, Box<dyn std::error::Error + Send>> {
    // Extract API Key information
    let output = Command::new("gcloud")
        .arg("auth")
        .arg("print-access-token")
        .output()
        .expect("Failed to execute command");

    let api_key: String = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Create headers
    let mut headers: HeaderMap = HeaderMap::new();

    // We would like json
    headers.insert(
        "Content-Type",
        HeaderValue::from_str("appication/json; charset=utf-8")
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
        .timeout(std::time::Duration::new(120, 0))
        //.gzip(true)
        .default_headers(headers)
        .build()
        .map_err(|e| -> Box<dyn std::error::Error + Send> { Box::new(e) })?;

    Ok(client)
}

pub fn message_to_content(messages: &[Message]) -> Vec<Content> {
    let parts: Vec<Part> = messages.iter()
        .map(|m| Part { text: m.content.clone() })
        .collect();

    vec![Content { role: "user".into(), parts }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_call_bard() {
        let messages: Vec<Content> = vec![Content { role: "user".into(), 
            parts: vec![Part { text: "What is the meaining of life?".into() }]}];
        match call_bard(messages.clone()).await {
            Ok(answer) => { println!("{answer}"); assert!(true) },
            Err(e) => { println!("{e}"); println!("{:?}", messages); assert!(false) },
        }
    }
}
