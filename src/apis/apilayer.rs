use serde_derive::Deserialize;
use serde_derive::Serialize;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct News {
    pub offset: u32,
    pub number: u32,
    pub available: u32,
    pub news: Vec<NewsItem>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewsItem {
    pub id: u64,
    pub title: String,
    pub text: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    pub publish_date: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    pub language: String,
    pub source_country: String,
    pub sentiment: f32,
}
