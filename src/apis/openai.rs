use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::Client;
//use dotenv::dotenv;
use std::env;
use serde_derive::{Deserialize, Serialize};
//use crate::llm::gpt::GPTITEM_SCHEMA;

// Input structures
// Chat
#[derive(Debug, Serialize, Clone)]
pub struct ChatCompletion {
    pub model: String,
    pub messages: Vec<Message>,
    pub response_format: ResponseFormat,
    pub temperature: f32,
}

#[derive(Debug, Serialize, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct ResponseFormat {
    pub r#type: String,
}

// Image
#[derive(Debug, Serialize, Clone)]
pub struct ImageCompletion {
    pub model: String,
    pub prompt: String,
    pub n: usize,
    pub size: String,
}

// Output structures
// Chat
#[derive(Debug, Deserialize)]
pub struct APIResponse {
    pub id: String,
    pub model: String,
    pub choices: Option<Vec<APIChoice>>,
}

#[derive(Debug, Deserialize)]
pub struct APIChoice {
    pub message: APIMessage,
    pub finish_reason: String,
}

#[derive(Debug, Deserialize)]
pub struct APIMessage {
    pub role: String,
    pub content: String,
}

// Image
#[derive(Debug, Deserialize)]
pub struct ImageResponse {
    pub created: u64,
    pub data: Vec<ImageData>,
}

#[derive(Debug, Deserialize)]
pub struct ImageData {
    pub revised_prompt: String,
    pub url: String,
}

// Embeddings

#[derive(Debug, Serialize, Clone)]
pub struct Embedding {
    pub input: Vec<String>,
    pub model: String,
    pub dimensions: usize,
}

#[derive(Debug, Deserialize)]
pub struct EmbeddingResponse {
    pub data: Vec<EmbeddingData>,
    pub model: String,
}

#[derive(Debug, Deserialize)]
pub struct EmbeddingData {
    pub object: String,
    pub index: usize,
    pub embedding: Vec<f32>,
}

// Call Large Language Model (i.e. GPT-4)
pub async fn call_gpt(messages: Vec<Message>) -> Result<String, Box<dyn std::error::Error + Send>> {
    let gpt_version: String = std::env::var("GPT_VERSION").map_err(anyhow::Error::new)?;
    call_gpt_model(&gpt_version, messages, true).await
}

pub async fn call_gpt_model(model: &str, messages: Vec<Message>, is_json: bool) -> Result<String, Box<dyn std::error::Error + Send>> {
    // Confirm endpoint
    let url: String =
        env::var("GPT_CHAT_URL").expect("GPT_CHAT_URL not found in enviornment variables");

    let client = get_client().await?;

    // Create chat completion
    let chat_completion: ChatCompletion = ChatCompletion {
        model: model.into(),
        messages,
        temperature: 0.2,
        response_format: ResponseFormat { r#type: 
            if is_json {"json_object".to_string()} else {"text".to_string()}},
    };
//println!("--- {}", serde_json::to_string(&chat_completion).unwrap());

    // Extract API Response
    let res = client
        .post(url)
        .json(&chat_completion)
        .send()
        .await;
    let res: APIResponse = res
        .map_err(|e| -> Box<dyn std::error::Error + Send> { Box::new(e) })?
        .json()
        .await
        .map_err(|e| -> Box<dyn std::error::Error + Send> { Box::new(e) })?;

    // Send Response
    match res.choices {
        Some(choices) => {
            let text = choices[0].message.content.clone();
            let text = text.lines().filter(|l| !l.starts_with("```")).fold(String::new(), |s, l| s + l + "\n");

            Ok(text)
        },
        None => {
            Err(anyhow::Error::msg("No Choice found").into())
        }
    }
}

pub async fn call_gpt_image_model(model: &str, prompt: &str, size: &str, n: usize) -> Result<String, Box<dyn std::error::Error + Send>> {
    // Confirm endpoint
    let url: String =
        env::var("GPT_IMAGE_URL").expect("GPT_IMAGE_URL not found in enviornment variables");

    // Create chat completion
    let client = get_client().await?;

    let image_completion: ImageCompletion = ImageCompletion {
        model: model.into(),
        prompt: prompt.into(),
        n,
        size: size.into(),
    };

//println!("{:?}", serde_json::to_string(&chat_completion));
    // Extract API Response
    let res = client
        .post(url)
        .json(&image_completion)
        .send()
        .await;
//println!("### {:?}", res);
    let res: ImageResponse = res
        .map_err(|e| -> Box<dyn std::error::Error + Send> { Box::new(e) })?
        .json()
        .await
        .map_err(|e| -> Box<dyn std::error::Error + Send> { Box::new(e) })?;
//println!("### {:?}", res);

    // Send Response
    Ok(res.data[0].url.clone())
}

pub async fn call_embedding_model(model: &str, input: &[String]) -> Result<Vec<f32>, Box<dyn std::error::Error + Send>> {
    // Confirm endpoint
    let url: String =
        env::var("GPT_EMBEDDING_URL").expect("GPT_EMBEDDING_URL not found in enviornment variables");

    let client = get_client().await?;

    // Create chat completion
    let embedding = Embedding {
        input: input.to_vec(),
        model: model.into(),
        dimensions: 384,    // 1536
    };

    // Extract API Response
    let res = client
        .post(url)
        .json(&embedding)
        .send()
        .await;
    let res: EmbeddingResponse = res
        .map_err(|e| -> Box<dyn std::error::Error + Send> { Box::new(e) })?
        .json()
        .await
        .map_err(|e| -> Box<dyn std::error::Error + Send> { Box::new(e) })?;

    // Send Response
    Ok(res.data[0].embedding.clone())
}

async fn get_client() -> Result<Client, Box<dyn std::error::Error + Send>> {
    // Extract API Key information
    let api_key: String =
        env::var("OPEN_AI_KEY").expect("OPEN_AI_KEY not found in enviornment variables");
    //let api_org: String =
    //    env::var("OPEN_AI_ORG").expect("OPEN_AI_ORG not found in enviornment variables");

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
        .timeout(std::time::Duration::new(120, 0))
        //.gzip(true)
        .default_headers(headers)
        .build()
        .map_err(|e| -> Box<dyn std::error::Error + Send> { Box::new(e) })?;

    Ok(client)
}

pub async fn fetch_url(url: &str, file: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let response = reqwest::get(url).await?;
    let mut file = std::fs::File::create(file)?;
    let mut content =  std::io::Cursor::new(response.bytes().await?);
    std::io::copy(&mut content, &mut file)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_call_openai() {
        let messages: Vec<Message> = vec![Message { role: "user".into(), content: "What is the meaining of life?".into() }];
        match call_gpt(messages).await {
            Ok(answer) => { println!("{answer}"); assert!(true) },
            Err(e) => { println!("{e}"); assert!(false) },
        }
    }
}
