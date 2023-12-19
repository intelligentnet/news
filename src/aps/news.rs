use std::env;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs::File;
use std::io::{BufWriter, Write};
//use std::io::BufWriter;
use std::fmt::Write as fmt_write;
//use rand::Rng;
use reqwest::Client;
use chrono::{DateTime, Duration, Utc};
use tokio_postgres;
use whichlang::{detect_language, Lang};
use crate::image::render::{mk_image, use_image};
use crate::apis::{newsapi::News, call_builder::make_call};
use crate::llm::gpt::{GPTItem, llm_news, llm_image, llm_tale, llm_code, llm_tale_detail, truncate, truncate_sentence, SUMMARIZE_ERROR, clean_html};
use crate::image::render::{PAGE_TOTAL, mk_filename};
use crate::db::postgres::*;
use itertools::Itertools;
use is_html::is_html;
use fastembed::{FlagEmbedding, EmbeddingBase};

pub const TIMEOUT_PERIOD: u32 = 3;
pub const CLEAR_TIMEOUT_PERIOD: u32 = 24;

pub async fn news(prompt: &str, fmt: &str, initial: bool) -> Result<String, Box<dyn Error>> {
    // Determine whether we want embeddings
    let model = None; //Some(get_embed_model().await?);
    let (mut pg_client, connection) = tokio_postgres::connect(&connect_string()?, tokio_postgres::NoTls).await?;

    // The connection performs communicate with DB so spawn off
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    if initial {
        add_prompt_embed(&mut pg_client, &model, prompt, &fmt).await?;
    }

    // HTTP client
    let client = Client::builder()
        .user_agent("TargetR")
        .gzip(true)
        .timeout(std::time::Duration::new(10, 0))
        .build()?;

    let mut news = get_initial_news(&mut pg_client, prompt, &model).await?;

//println!("{prompt}: {initial} {} {}", news.is_empty(), news[0].dt);
    if news.is_empty() || news[0].dt + Duration::hours(TIMEOUT_PERIOD.into()) > Utc::now() {
        Ok(mk_filename(prompt))
    } else {
        match fmt.as_ref() {
            "picture" =>
                picture_news(&mut pg_client, prompt, &mut news).await,
            _ =>
                text_news(&mut pg_client, &client, prompt, &mut news).await,
        }
    }
}

pub async fn tale(prompt: &str, system: &str) -> Result<String, Box<dyn Error>> {
    let res: Result<String, Box<dyn std::error::Error + Send>> = llm_tale(system, prompt).await;
   
    let file_name =
        match res {
            Ok(content) => {
                let file_name = if is_html(&content) {
                    format!("gen/{}.html", truncate(prompt, 100).replace(' ', "_").to_lowercase())
                } else {
                    format!("gen/{}.txt", truncate(prompt, 100).replace(' ', "_").to_lowercase())
                };
                write_file(&file_name, &content)?;

                file_name
            },
            Err(_) => "gen/not_available.png".to_string()
        };

    Ok(file_name)
}

pub async fn language(lang: &str, prompt: &str, system: &str) -> Result<String, Box<dyn Error>> {
    let res: Result<String, Box<dyn std::error::Error + Send>> = llm_code(system, prompt, lang).await;
   
    let file_name =
        match res {
            Ok(content) => {
                let content = content.replace("\\n", "\n")
                       .replace("\\\"", "\"")
                       .replace("```{lang}", &format!("// {}", prompt.replace('_', " ")))
                       .replace("```", "");
                let file_name = format!("gen/{}.rs", truncate(prompt, 100).replace(' ', "_").to_lowercase());

                write_file(&file_name, &content)?;

                file_name
            },
            Err(_) => "gen/not_available.png".to_string()
        };

    Ok(file_name)
}

pub async fn detail_tale(title: &str, items: &Vec<(String, String, String)>) -> Result<String, Box<dyn Error>> {
    let mut first = true;
    let mut buf = String::new();
    write!(buf, "{{")?;
    for (n, _, s) in items {
        if ! first { write!(buf, ", ")?; }
        first = false;
        write!(buf, "\"{n}\": \"{s}\"")?;
    }
    write!(buf, "}}")?;
//println!(">>> {buf}");

    let file_name = 
        match llm_tale_detail(&buf).await {
            Ok(content) => {
//println!("<<< {content}");
                let file_name = format!("gen/{}_detail.html", truncate(title, 100).replace(' ', "_").to_lowercase());

                write_file(&file_name, &content)?;

                file_name
            },
            Err(e) => {
                println!("Error is: {}", e);
                "gen/not_available.png".into()
            }
        };

    Ok(file_name)
}

pub async fn image(prompt: &str, system: &str) -> Result<String, Box<dyn Error>> {
    let _: Result<String, Box<dyn Error>> = match llm_image(prompt, system).await {
        Ok(f) => Ok(f),
        Err(e) => Err(anyhow::Error::msg(e.to_string()).into())
    };

    //use_image(prompt, prompt).map_err(|e| e.into())
    Ok(mk_filename(prompt))
}

async fn picture_news(pg_client: &mut tokio_postgres::Client, prompt: &str, news: &mut Vec<DbNewsItem<Utc>>) -> Result<String, Box<dyn Error>> {
    /*
    let mxi = news.iter().enumerate()
        .filter(|(_, n)| detect_language(&n.title) == Lang::Eng)
        .max_by(|(_, a), (_, b)| a.sentiment.total_cmp(&b.sentiment))
        .map(|(i, _)| i).unwrap();
    */
    let mut ni: DbNewsItem<Utc> = news[0].clone();

    for n in news.iter() {
        if detect_language(&n.title) == Lang::Eng { ni = n.clone(); break; }
    };
//println!("{prompt} news len = {} {} {} {}", mxi, news[mxi].sentiment, news[mxi].title, news[mxi].url);
    let url = ni.url.clone();
    del_news_item(pg_client, prompt, &url).await?;
    news.retain(|n| n.url != url);
    
    let _: Result<String, Box<dyn Error>> = match llm_image(prompt, &ni.title).await {
        Ok(f) => Ok(f),
        Err(e) => Err(anyhow::Error::msg(e.to_string()).into())
    };

    news.iter_mut().for_each(|n| n.dt = Utc::now());
    add_news(pg_client, news).await?;

    use_image(prompt, &ni.title).map_err(|e| e.into())
//println!("### {prompt} {fmt} news len = {}", news.len());
}

async fn text_news(pg_client: &mut tokio_postgres::Client, client: &Client, prompt: &str, news: &mut Vec<DbNewsItem<Utc>>) -> Result<String, Box<dyn Error>> {
    let mut urls: HashSet<String> = HashSet::new();
    let mut used: HashSet<String> = HashSet::new();
    let mut unprocessed_news: Vec<DbNewsItem<Utc>> = Vec::new();
    let mut processed_news: Vec<DbNewsItem<Utc>> = Vec::new();
    // Core processing with given news
    let mut size = 0;

    for n in news.iter() {
        used.insert(n.url.clone());

        // Filter duplicates
        if urls.contains(&n.url) {
            // Remove duplicate urls. Still some after deduping
            eprintln!("Duplicate title: {}", n.title);
            if n.summary.is_some() {
                del_news_item(pg_client, prompt, &n.url).await?;
            }
            continue
        }
        urls.insert(n.url.clone());

        match &n.summary {
            None => {
                // Make the GET request to the source news url
                let resp = client.get(&n.url).send().await;
                let summary =
                    match resp {
                        Ok(resp) => { 
                            let body = resp.text().await?;

                            clean_html(&body)
                        },
                        Err(e) => {
                            eprintln!("connection error: {}", e);
                            SUMMARIZE_ERROR.to_owned()
                        }
                    };

                if summary == SUMMARIZE_ERROR {
                    continue
                }

                let nn = DbNewsItem::new(&n.url, prompt, &n.source, &n.title, Utc::now(), &Some(summary.clone()), true, n.sentiment, n.embedding.clone());

//println!("UnProcessed : {}", unprocessed_news.len());
                unprocessed_news.push(nn);

                let sz = n.title.len() + summary.len();

                size += sz;

//                if count >= PAGE_TOTAL as usize - 1 || size > get_context_size() - sz {
                if size > get_context_size()? - sz {
                    let unproc: Vec<GPTItem> = unprocessed_news.iter().map(|n| GPTItem::new(&n.title, n.summary.as_ref().unwrap())).collect();
                    let proc_news = llm_news(&unproc, PAGE_TOTAL).await;

                    match proc_news {
                        Err(e) => {
                            if e.to_string().starts_with("expected value") {
                                eprintln!("LLM cannot handle request: {}", e);
                            } else if e.to_string().contains("operation timed out") {
                                eprintln!("LLM timed out: {}", e);
                            } else if e.to_string().contains("missing field") {
                                eprintln!("Missing field: {}", e);
                            } else {
                                eprintln!("*Unexpected Error: {}", e);
                            }
                            for n in unproc {
                                del_news_title(pg_client, prompt, &n.title).await?;
                            }
                            //continue;
                        },
                        Ok(gpt_items) => {
                            for (title, _, _, good) in &gpt_items {
                                if !good {
                                    del_news_title(pg_client, prompt, title).await?;
                                }
                            }
                            let gpts: Vec<DbNewsItem<Utc>> = gpt_items.into_iter().zip(unprocessed_news.into_iter())
                                .filter(|((title, body, _, good), _)| {
                                    *good && !bad(title, body)
                                })
                                .map(|((title, body, sentiment, _), mut n)| {
                                    n.title = truncate_sentence(&title, 150).into();
                                    n.summary = Some(body);
                                    n.sentiment = sentiment;
                                    n
                                })
                                .collect();

                            processed_news = [processed_news, gpts].concat();
                        },
                    };
                    unprocessed_news = Vec::new();
                    //count = 0;
                    size = 0;
                }
            },
            Some(_summary) => {
                processed_news.push(n.clone());
            },
        }

        if processed_news.len() >= PAGE_TOTAL {
            break;
        }
    }

    let mut articles: Vec<(String, String, String)> = Vec::new();

    // Put stuff in the DB if processed and enumerate for the image
    for (seq, n) in processed_news.iter().enumerate() {
        if seq < PAGE_TOTAL {
            articles.push((n.source.clone(), n.title.clone(), n.summary.clone().unwrap()));
            del_news_item(pg_client, prompt, &n.url).await?;
        }
    }

    news.retain(|n| !used.contains(&n.url));

    news.iter_mut().for_each(|n| n.dt = Utc::now());
    add_news(pg_client, news).await?;

    if processed_news.is_empty() {
        eprintln!("Count is zero");
        Err("Not Found")?
    } else {
        mk_image(prompt, &articles, PAGE_TOTAL, false).map_err(|e| e.into())
    }
}

fn bad(title: &str, body: &str) -> bool {
    if bad_translations(title, true) {
        eprintln!("Skipping: Title {}", title);
        true
    } else if bad_translations(body, false) {
        eprintln!("Skipping: Body {}", body.len());
        true
    //} else if body.len() > 700 {
    //    eprintln!("Skipping: Long Body {} with title: {}", body.len(), title);
    //    false
    } else {
        false
    }
}

async fn get_new_news(prompt: &str, model: &Option<FlagEmbedding>) -> Result<Vec<DbNewsItem<Utc>>, Box<dyn Error>> {
    // find additional news and unpack it
    let dt: DateTime<Utc> = Utc::now() - Duration::hours(TIMEOUT_PERIOD.into());
    let articles = get_news(prompt).await?.articles;
    let titles: Vec<&str> = articles.iter().map(|n| n.title.as_str()).collect();

    let nv: Vec<DbNewsItem<Utc>> = match model {
        None => 
            articles.iter().map(|n| DbNewsItem::new(&n.url, prompt, &n.source.name, &n.title, dt, &None, false, 0.0, None)).collect(),
        Some(model) => {
            let embeddings = model.passage_embed(titles, None)?;

            articles.iter().zip(embeddings.iter()).map(|(n, e)| DbNewsItem::new(&n.url, prompt, &n.source.name, &n.title, dt, &None, false, 0.0, Some(e.clone()))).collect()
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
        .gzip(true)
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

fn get_context_size() -> Result<usize, Box<dyn Error>> {
    match std::env::var("CONTEXT_SIZE") {
        Ok(val) => val.parse().map_err(|e| format!("Sentiment {val} {e}").into()),
        Err(_) => Ok(16000),
    }
}

async fn get_initial_news(pg_client: &mut tokio_postgres::Client, prompt: &str, model: &Option<FlagEmbedding>) -> Result<Vec<DbNewsItem<Utc>>, Box<dyn Error>> {
    // Clear any old news
    clear_news(pg_client, prompt, CLEAR_TIMEOUT_PERIOD).await?;

    // Query valid saved news
    let mut db_news = get_saved_news(pg_client, prompt, CLEAR_TIMEOUT_PERIOD).await?;

    let min_rows = (PAGE_TOTAL + 1) / 2;

    // REFRESH logic...
    if db_news.len() < min_rows {
        reset_news_item_seq(pg_client, prompt).await?;

        let new_news = get_new_news(prompt, model).await?;

        db_news = [db_news, new_news].concat();
    }

    Ok(db_news)
}

fn write_file(file_name: &str, content: &str) -> Result<(), Box<dyn Error>> {
    let file = File::create(file_name)?;
    let mut writer = BufWriter::new(file);

    writer.write_all(content.as_bytes())?;

    writer.flush()?;

    Ok(())
}
