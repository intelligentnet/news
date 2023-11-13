use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::Client;
//use dotenv::dotenv;
use std::env;
use serde_derive::{Deserialize, Serialize};
//use crate::llm::gpt::GPTITEM_SCHEMA;

// Input structures
#[derive(Debug, Serialize, Clone)]
pub struct ChatCompletion {
    pub model: String,
    pub messages: Vec<Message>,
//    pub functions: Vec<Function>,
//    pub function_call: FunctionCall,
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
//    #[serde(rename = "type")]
    pub r#type: String,
}

/*
#[derive(Debug, Serialize, Clone)]
pub struct Function {
    pub name: String,
    pub description: String,
    pub parameters: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct FunctionCall {
    pub name: String,
}
*/

// Output structures
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

// Call Large Language Model (i.e. GPT-4)
pub async fn call_gpt(messages: Vec<Message>) -> Result<String, Box<dyn std::error::Error + Send>> {
    let gpt_version: String = std::env::var("GPT_VERSION").map_err(|e| anyhow::Error::new(e))?;
    call_gpt_model(&gpt_version, messages).await
    //call_gpt_model("gpt-3.5-turbo-1106", messages).await
    //call_gpt_model("gpt-4-1106-preview", messages).await
}

pub async fn call_gpt_model(model: &str, messages: Vec<Message>) -> Result<String, Box<dyn std::error::Error + Send>> {
    // Extract API Key information
    let api_key: String =
        env::var("OPEN_AI_KEY").expect("OPEN_AI_KEY not found in enviornment variables");
    //let api_org: String =
    //    env::var("OPEN_AI_ORG").expect("OPEN_AI_ORG not found in enviornment variables");

    // Confirm endpoint
    let url: &str = "https://api.openai.com/v1/chat/completions";

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
        .timeout(std::time::Duration::new(60, 0))
        .default_headers(headers)
        .build()
        .map_err(|e| -> Box<dyn std::error::Error + Send> { Box::new(e) })?;

    // Create chat completion
    let chat_completion: ChatCompletion = ChatCompletion {
        model: model.into(),
        messages,
        temperature: 0.2,
        response_format: ResponseFormat { r#type: "json_object".to_string() },
//        functions: vec![Function { name: "news".to_string(), description: "Json".to_string(), parameters: GPTITEM_SCHEMA.to_string() }],
//        function_call: FunctionCall { name: "news".to_string() },
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
//println!("# of choices {}", choices.len());
            Ok(choices[0].message.content.clone())
        },
        None => {
            Err(anyhow::Error::msg("No Choice found").into())
        }
    }
}
