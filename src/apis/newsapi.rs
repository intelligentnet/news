use serde_derive::{Deserialize, Serialize};
use crate::apis::call_builder::make_call;
use log::{error};
use std::collections::HashMap;
use std::env;
use std::error::Error;
use chrono::{Utc, Duration};
use crate::aps::news::get_http_client;
use itertools::Itertools;
use crate::llm::gpt::truncate_sentence;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FetchNews {
    pub status: String,
    pub total_results: u32,
    pub articles: Vec<FetchNewsItem>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FetchNewsItem {
    pub source: NewsSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    //#[serde(deserialize_with="no_title")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url_to_image: Option<String>,
    pub published_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>
}

/*
fn no_title<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    match serde::de::Deserialize::deserialize(deserializer) {
        Ok(v) => Ok(v),
        Err(_) => Ok("No Title".to_string())
    }
}
*/

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewsSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct News {
    pub articles: Vec<NewsItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NewsItem {
    pub title: String,
    pub description: String,
    pub url: String,
    pub url_to_image: Option<String>,
    pub published_at: String,
    pub source_name: String
}

pub async fn get_news(search: &str) -> Result<News, Box<dyn Error>> {
    // Create a new reqwest client
    let mut paras: HashMap<&str, &str> = HashMap::new();

    let utc = Utc::now() - Duration::try_days(1).unwrap();
    let dt = utc.format("%Y-%m-%d").to_string();

    paras.insert("from", &dt);
    let api_key = env::var("NEWSAPI_KEY")?;
    paras.insert("apiKey", &api_key);
    // sources or country: ae ar at au be bg br ca ch cn co cu cz de eg fr gb gr hk hu id ie il in it jp kr lt lv ma mx my ng nl no nz ph pl pt ro rs ru sa se sg si sk th tr tw ua us ve za
    //paras.insert("country", "gb");
    // category: business entertainment general health science sports technology
    //paras.insert("category", "science");
    //paras.insert("sources", "bbc");
    paras.insert("sortBy", "popularity");
    let news_search = &format!("{search} news");
    paras.insert("q", news_search);

    //let call = make_call("https://newsapi.org/v2/top-headlines", &paras);
    let call = make_call("https://newsapi.org/v2/everything", &paras);

    get_enough(search, &call).await

    /*
    if news.articles.len() > 10 {
        Ok(news)
    } else {
        let call = make_call("https://newsapi.org/v2/everything", &paras);

        get_enough(search, &call).await
    }
    */
}

async fn get_enough(search: &str, call: &str) -> Result<News, Box<dyn Error>> {
    let client = get_http_client(10)?;
    // Make the secure GET request to the news source
    let resp = client.get(call).send().await?;

    // Read the response body as a string
    let body = resp.text().await?;
//println!("get_enough {search} {call}");
    let res = serde_json::from_str(&body);
    if res.is_err() {
        error!("{:?}", res);
    }
    let news: FetchNews = res?;

//println!("2 get_enough {search} {call}");
    let articles: Vec<NewsItem> = news.articles.iter()
        .filter(|a| a.title.is_some() && a.url_to_image.is_some() && (a.description.is_some() || a.content.is_some()))
        .map(|a| {
            NewsItem { title: a.title.as_ref().unwrap().into(),
                       description: {
                           let desc =
                               if a.description.is_none() {
                                   a.content.as_ref().unwrap()
                               } else {
                                   a.description.as_ref().unwrap()
                               };

                           truncate_sentence(desc, desc.len()).into()
                       }, 
                       url: a.url.clone(),
                       url_to_image: a.url_to_image.clone(),
                       published_at: a.published_at.clone(),
                       source_name: a.source.name.clone()
            }
        })
        .collect();

    let uniq_articles: Vec<_> = articles.into_iter().unique().collect();
    //info!("{search}: {:?} results of which {:?} are unique", news.total_results, uniq_articles.len());
    println!("{search}: {:?} results of which {:?} are unique", news.total_results, uniq_articles.len());

    let news = News { articles: uniq_articles };

    Ok(news)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_call_newsapi() {
        let news = get_news("world news").await;
        match news {
            Ok(answer) => {
                println!("{answer:?} {}", answer.articles.len());
                assert!(true)
            },
            Err(e) => {
                println!("{e}");
                assert!(false)
            },
        }
    }
}
