use std::env;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use reqwest::Client;
use chrono::{DateTime, Duration, Utc};
use tokio_postgres::NoTls;
use whichlang::{detect_language, Lang};
use crate::image::render::mk_image;
use crate::apis::{newsapi::News, call_builder::make_call};
use crate::llm::gpt::{llm_news_items, llm_title, truncate, SUMMARIZE_ERROR};
use crate::image::render::{PAGE_TOTAL, mk_filename};
use crate::db::postgres::*;
use itertools::Itertools;

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

    let mut first = true;
    let mut db_news = get_saved_news(&mut pg_client, prompt, CLEAR_TIMEOUT_PERIOD).await?;
    let mut new_news: Vec<DbNewsItem<Utc>> = vec![];
    let min_rows = (PAGE_TOTAL as usize + 1) / 2;

    // REFRESH logic...
    if db_news.is_empty() {
        new_news = get_new_news(prompt).await?;
        if new_news.is_empty() {
            // Nothing found anywhere!
            Err("Not Found")?
        }
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
                    get_new_news(prompt).await?
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
    let news_len = news.len();

    let client = Client::builder()
        .user_agent("TargetR")
        .timeout(std::time::Duration::new(10, 0))
        .build()?;
    let mut titles = HashSet::new();

    // seq must monotonically increase, but can have gaps
    for (seq, n) in news.into_iter().enumerate() {
        let mut title = n.title;

        if count < PAGE_TOTAL {
            if titles.contains(&title) {
                // Remove duplicates
                // Should not happen after deduping???
                eprintln!("Duplicate title: {title}");
                del_news_item(&mut pg_client, &prompt, &n.url).await?;
                continue
            } else if detect_language(&title) != Lang::Eng {
                if news_len - seq > 10 {
                    // Not our language and plenty more opportunities so skip
                    eprintln!("Skipping: title wrong language");
                    del_news_item(&mut pg_client, &prompt, &n.url).await?;
                    continue
                }
                match llm_title(&title).await {
                    Ok(trans_title) if bad_translations(&trans_title) => {
                        eprintln!("Skipping: title {}", &trans_title);
                        del_news_item(&mut pg_client, &prompt, &n.url).await?;
                        continue
                    },
                    Ok(trans_title) => {
                        title = trans_title
                    },
                    Err(e) => {
                        eprintln!("Translation error: {e}");
                        del_news_item(&mut pg_client, &prompt, &n.url).await?;
                        continue
                    }
                }
            }

            title = truncate(&title, 150).into();

            titles.insert(title.clone());

            let res: Result<String, Box<dyn Error + Send>> = 
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

                        // Make the GET request to the source news url
                        llm_news_items(prompt, &title, &body, PAGE_TOTAL).await
                    },
                    Some(summary) => {
                        Ok(summary)
                    },
                };

            match res {
                Ok(res_str) if bad_translations(&res_str) => {
                        eprintln!("Skipping: title {}", &res_str);
                        del_news_item(&mut pg_client, &prompt, &n.url).await?;
                        continue
                }
                Ok(res_str) => {
                    articles.push((n.source.clone(), title.clone(), res_str.clone()));
                    let news_item = 
                        DbNewsItem{url: n.url, prompt: prompt.into(), source: n.source, title: title, summary: Some(res_str), queried: true, dt: Utc::now()};
                    add_news_item(&mut pg_client, &news_item, seq as u32).await?;
                    count += 1;
                }
                Err(e) => {
                    eprintln!("*Unexpected Error: {}", e);
                    continue
                }
            }
        } else if first {
            let news_item = 
                DbNewsItem{url: n.url, prompt: prompt.into(), source: n.source, title: title, summary: None, queried: false, dt: Utc::now()};
            add_news_item(&mut pg_client, &news_item, seq as u32).await?;
        }
    }

    if count == 0 {
        eprintln!("Count is zero");
        Err("Not Found")?
    } else {
        mk_image(prompt, &articles, PAGE_TOTAL, false).map_err(|e| e.into())
    }
}

async fn get_new_news(prompt: &str) -> Result<Vec<DbNewsItem<Utc>>, Box<dyn Error>> {
    // find additional news and unpack it
    let mut nv: Vec<DbNewsItem<Utc>> = vec![];
    let dt: DateTime<Utc> = Utc::now();

    for n in get_news(prompt).await?.articles {
        nv.push(DbNewsItem::new(&n.url, prompt, &n.source.name, &n.title, dt));
    }

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

    let call = make_call("https://newsapi.org/v2/everything", paras);
    //let call = make_call("https://newsapi.org/v2/top-headlines", paras);
//println!("### {call}");

    let client = Client::builder()
        .user_agent("TargetR")
        .timeout(std::time::Duration::new(10, 0))
        .build()?;
//println!("1 {:?}", client);

    // Make the secure GET request to the news source
    let resp = client.get(call).send().await?;
//println!("2 {:?}", response);

    // Read the response body as a string
    let body = resp.text().await?;
//println!("{:?}", body);

    let mut news: News = serde_json::from_str(&body)?;
    let uniq_articles: Vec<_> = news.articles.into_iter().unique().collect();
    println!("{search}: {:?} results of which {:?} are unique", news.total_results, uniq_articles.len());
    news.articles = uniq_articles;

    Ok(news)
}

fn bad_translations(res: &str) -> bool {
    res.len() < 30 ||
    res.to_uppercase().starts_with(SUMMARIZE_ERROR) ||      // LLM working produces this
    res.contains("JavaScript and cookies") ||
    res.contains("HTML") ||
    res.starts_with("The text") ||
    res.starts_with("This text") ||
    //res.starts_with("The article") ||
    res.starts_with("I apologise") ||
    res.starts_with("I'm sorry") ||
    res.starts_with("Sorry") ||
    res.starts_with("This is a webpage") ||
    res.starts_with("WATCH:") ||
    res.starts_with("The website")
}
