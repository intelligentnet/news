use std::env;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use reqwest::Client;
use chrono::{DateTime, Duration, Utc};
use tokio_postgres::NoTls;
//use whichlang::{detect_language, Lang};
use crate::image::render::mk_image;
use crate::apis::{newsapi::News, call_builder::make_call};
use crate::llm::gpt::{GPTItem, llm_news, truncate_sentence, SUMMARIZE_ERROR};
use crate::image::render::{PAGE_TOTAL, mk_filename};
use crate::db::postgres::*;
use itertools::Itertools;
use fastembed::{FlagEmbedding, EmbeddingBase};

pub const TIMEOUT_PERIOD: u32 = 3;
pub const CLEAR_TIMEOUT_PERIOD: u32 = 24;

pub async fn news(prompt: &str) -> Result<String, Box<dyn Error>> {
    let (mut pg_client, connection) = tokio_postgres::connect(&connect_string()?, NoTls).await?;

    // The connection performs communicate with DB so spawn off
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    let model = None; //Some(get_embed_model().await?);

    let mut first = true;
    let mut db_news = get_saved_news(&mut pg_client, prompt, CLEAR_TIMEOUT_PERIOD).await?;
    let mut new_news: Vec<DbNewsItem<Utc>> = vec![];
    let min_rows = (PAGE_TOTAL as usize + 1) / 2;

    // REFRESH logic...
    if db_news.is_empty() {
        new_news = get_new_news(prompt, &model).await?;
        if new_news.is_empty() {
            // Nothing found anywhere!
            Err("Not Found")?
        }
        // Ah, we have some new ones...
        // Possibly the first occurence of this prompt!
        add_prompt_embed(&mut pg_client, &model, prompt).await?;
        db_news = new_news.clone();
    } else if db_news[0].dt + Duration::hours(TIMEOUT_PERIOD.into()) > Utc::now() {
        // Still in time window and file must exist
        return Ok(mk_filename(prompt));
    } else if db_news.len() > min_rows {
        // Do we have enough news to fill page
        // clear out 'used' news
        clear_news(&mut pg_client, prompt, CLEAR_TIMEOUT_PERIOD).await?;
        // and get hopefully new unused news
        //db_news = get_saved_news(&mut pg_client, prompt, CLEAR_TIMEOUT_PERIOD).await?;
    }

    let news =
        if db_news.len() > min_rows || (new_news.is_empty() && !db_news.is_empty()) {
            // We still have some news
            //first = false;
            db_news
        } else {
            // Get more news
            reset_news_item_seq(&mut pg_client, prompt).await?;

            if new_news.is_empty() {
                if db_news.len() > min_rows {
                    first = false;
                    // We still have enough news
                    db_news
                } else {
                    // Hum, just return what we can find
                    get_new_news(prompt, &model).await?
                }
            } else if db_news != new_news {
                [db_news, new_news].concat()
            } else {
                new_news
            }
        };

    if news.is_empty() {
        // No news found!
        return Ok(mk_filename(prompt));
    }

    // Core processing with given news
    let mut count = 0;
    let mut articles: Vec<(String, String, String)> = vec![];

    let client = Client::builder()
        .user_agent("TargetR")
        .timeout(std::time::Duration::new(10, 0))
        .build()?;
    let mut titles = HashSet::new();
    let mut fetched_news: Vec<DbNewsItem<Utc>> = Vec::new();
//let mut ptitle = "".to_string();
//let mut pbody = "".to_string();

    // seq must monotonically increase, but can have gaps
    for (seq, n) in news.into_iter().enumerate() {
        let title = n.title;

        if count < PAGE_TOTAL {
            if titles.contains(&title) {
                // Remove duplicates
                // Should not happen after deduping???
                eprintln!("Duplicate title: {title}");
                del_news_item(&mut pg_client, &prompt, &n.url).await?;
                continue
            }

            titles.insert(title.clone());

            let res: Result<Vec<(String, String)>, Box<dyn Error + Send>> = 
                match n.summary {
                    None => {
                        // Make the GET request to the source news url
                        let resp = client.get(&n.url).send().await;
                        let body =
                            match resp {
                                Ok(resp) => resp.text().await?,
                                Err(e) => {
                                    eprintln!("connection error: {}", e);
                                    continue
                                }
                            };

                        // Make the POST request to the LLM
                        /*
                        let a =
                        if !ptitle.is_empty() {
//println!("HERE {} ### {}", title, ptitle);
                            llm_news(prompt, &vec![GPTItem::new(&title, &body), GPTItem::new(&ptitle, &pbody)], PAGE_TOTAL).await

                        } else {
                        llm_news(prompt, &vec![GPTItem::new(&title, &body)], PAGE_TOTAL).await
                        };
pbody = body.clone();
a
                        */
                        llm_news(prompt, &vec![GPTItem::new(&title, &body)], PAGE_TOTAL).await
                    },
                    Some(summary) => {
                        Ok(vec![(title.clone(), summary)])
                    },
                };

            match res {
                Ok(res) if bad_translations(&res[0].0, true) || bad_translations(&res[0].1, false) => {
                    if bad_translations(&res[0].0, true) {
                        eprintln!("Skipping: Title {}", &res[0].0);
                    } else if res[0].1.len() > 700 {
//                        eprintln!("Skipping: Body {}", &res[0].1);
//                    } else {
                        eprintln!("Skipping: Long Body {} with title: {}", res[0].1.len(), &res[0].0);
                        //eprintln!("Skipping: Long Body {} {}", res[0].1.len(), crate::llm::gpt::truncate(&res[0].1, 1000));
                    }
                    del_news_item(&mut pg_client, &prompt, &n.url).await?;
                    continue
                }
                Ok(res) => {
                    let title = &res[0].0;
                    let res_str = &res[0].1;
                    let title: String = truncate_sentence(&title, 150).into();
                    articles.push((n.source.clone(), title.clone(), res_str.clone()));
                    let news_item = 
                        DbNewsItem{url: n.url, prompt: prompt.into(), source: n.source, title: title, summary: Some(res_str.into()), queried: true, dt: Utc::now(), embedding: n.embedding};
                    add_news_item(&mut pg_client, &news_item, seq as u32).await?;
                    fetched_news.push(news_item);
                    count += 1;
                }
                Err(e) => {
                    if e.to_string().starts_with("expected value") {
                        eprintln!("LLM cannot handle request: {}", e);
                    } else if e.to_string().contains("operation timed out") {
                        eprintln!("LLM timed out: {}", e);
                    } else if e.to_string().contains("missing field") {
                        eprintln!("Missing field for title: {}", title);
                    } else {
                        eprintln!("*Unexpected Error: {}", e);
                    }
                    continue
                }
            }
        } else if first {
            let news_item = 
                DbNewsItem{url: n.url, prompt: prompt.into(), source: n.source, title: title.clone(), summary: None, queried: false, dt: Utc::now(), embedding: n.embedding};
            add_news_item(&mut pg_client, &news_item, seq as u32).await?;
            fetched_news.push(news_item);
        }
//ptitle = title.clone();
    }

    if count == 0 {
        eprintln!("Count is zero");
        Err("Not Found")?
    } else {
        mk_image(prompt, &articles, PAGE_TOTAL, false).map_err(|e| e.into())
    }
}

async fn get_new_news(prompt: &str, model: &Option<FlagEmbedding>) -> Result<Vec<DbNewsItem<Utc>>, Box<dyn Error>> {
    // find additional news and unpack it
    let dt: DateTime<Utc> = Utc::now();
    let articles = get_news(prompt).await?.articles;
    let titles: Vec<&str> = articles.iter().map(|n| n.title.as_str()).collect();
    let nv: Vec<DbNewsItem<Utc>> = match model {
        None => 
            articles.iter().map(|n| DbNewsItem::new(&n.url, prompt, &n.source.name, &n.title, dt, None)).collect(),
        Some(model) => {
            let embeddings = model.passage_embed(titles, None)?;

            articles.iter().zip(embeddings.iter()).map(|(n, e)| DbNewsItem::new(&n.url, prompt, &n.source.name, &n.title, dt, Some(e.clone()))).collect()
        },
    };

    Ok(nv)
}

async fn get_news(search: &str) -> Result<News, Box<dyn Error>> {
    // Create a new reqwest client
    let mut paras: HashMap<&str, &str> = HashMap::new();

    let utc = Utc::now() - Duration::days(1);
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
    paras.insert("q", search);

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
    let client = Client::builder()
        .user_agent("TargetR")
        .timeout(std::time::Duration::new(10, 0))
        .build()?;

    // Make the secure GET request to the news source
    let resp = client.get(call).send().await?;

    // Read the response body as a string
    let body = resp.text().await?;

    let mut news: News = serde_json::from_str(&body)?;
    let uniq_articles: Vec<_> = news.articles.into_iter().unique().collect();
    println!("{search}: {:?} results of which {:?} are unique", news.total_results, uniq_articles.len());
    news.articles = uniq_articles;

    Ok(news)
}

fn bad_translations(res: &str, is_title: bool) -> bool {
    let res = res.to_lowercase();
    (is_title && res.len() < 25 || !is_title && res.len() < 100) ||
    res.to_uppercase().starts_with(SUMMARIZE_ERROR) ||      // LLM working produces this
    res.contains("javascript") ||
    res.contains(" html ") ||
    (res.contains("access") && res.contains("denied")) ||
    res.contains("subscription") ||
    res.starts_with("the text") ||
    res.starts_with("this text") ||
    res.starts_with("i apologize") ||
    res.starts_with("i'm sorry") ||
    res.starts_with("sorry") ||
    res.starts_with("this is a webpage") ||
    res.starts_with("watch:") ||
    res.starts_with("the website")
}
