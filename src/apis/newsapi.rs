use serde_derive::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct News {
    pub status: String,
    pub total_results: u32,
    pub articles: Vec<NewsItem>,
}

impl News {
    pub fn new(articles: &[NewsItem]) -> Self {
        News { status: "FromDB".into(), total_results: articles.len() as u32, articles: articles.to_vec() }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewsItem {
    #[serde(deserialize_with="no_title")]
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    pub source: NewsSource,
    pub published_at: String,
    pub url: String,
}

fn no_title<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    match serde::de::Deserialize::deserialize(deserializer) {
        Ok(v) => Ok(v),
        Err(_) => Ok("No Title".to_string())
    }
}

impl NewsItem {
    pub fn new(title: &str, url: &str, source: &str) -> Self {
        let source = NewsSource { id: None, name: source.into() };

        NewsItem { title: title.into(), author: None, source, published_at: "".into(), url: url.into() }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewsSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub name: String,
}
