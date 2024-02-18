use std::env;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs::File;
use std::io::{BufWriter, Write};
//use std::io::BufWriter;
//use std::fmt::Write as fmt_write;
//use rand::Rng;
use reqwest::Client;
use chrono::{DateTime, Duration, Utc};
use tokio_postgres;
use fastembed::{FlagEmbedding, EmbeddingBase};
use whichlang::{detect_language, Lang};
use crate::image::render::{mk_image, use_image};
use crate::apis::{newsapi::News, call_builder::make_call};
use crate::llm::gpt::{GPTItem, llm_news, llm_image, llm_tale, llm_code, llm_tale_detail, truncate, truncate_sentence, SUMMARIZE_ERROR, clean_html, get_embed_model, get_an_embedding};
use crate::image::render::{PAGE_TOTAL, mk_filename};
use crate::db::postgres::*;
use crate::apis::distance::{cosine_dist, euclidian_dist};
//use crate::apis::distance::cosine_dist;
use itertools::Itertools;
use is_html::is_html;
use log::{error, warn, info};

pub const TIMEOUT_PERIOD: u32 = 4;
pub const CLEAR_TIMEOUT_PERIOD: u32 = 24;
pub const PURGE_TIMEOUT_PERIOD: u32 = 24 * 7;

pub async fn get_embedding_model() -> Option<FlagEmbedding> {
    // Determine whether we want embeddings
    let embed: &str = &match std::env::var("EMBEDDING") {
        Ok(v) => v,
        Err(_) => "embed".to_string()
    };

    match embed {
        "openai" => None,
        _ => Some(get_embed_model().await.ok()?)
    }
}

pub fn get_http_client(timeout: u64) -> Result<Client, Box<dyn Error>> {
    Ok(Client::builder()
        .user_agent("TargetR")
        .gzip(true)
        .timeout(std::time::Duration::new(timeout, 0))
        .build()?)
}

pub async fn news(prompt: &str, fmt: &str, _initial: bool) -> Result<String, Box<dyn Error>> {
    let model = get_embedding_model().await;
    let mut pg_client = pg_connect().await;

    //if initial {
        let embedding = get_an_embedding(prompt, &model).await.map_err(|e| e.to_string())?;
        add_prompt_embed(&mut pg_client, embedding, prompt, fmt).await?;
    //}

    let client = get_http_client(10)?;
    let mut news = get_initial_news(&mut pg_client, prompt, &model).await?;

    if news.is_empty() || news[0].dt + Duration::hours(TIMEOUT_PERIOD.into()) > Utc::now() {
        Ok(mk_filename(prompt))
    } else {
        match fmt {
            "picture" =>
                picture_news(&mut pg_client, prompt, &mut news).await,
            _ =>
                text_news(&mut pg_client, &model, &client, prompt, &mut news).await,
        }
    }
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

pub async fn tale(prompt: &str, system: &str, n: usize) -> Result<String, Box<dyn Error>> {
    let res: Result<String, Box<dyn std::error::Error + Send>> = llm_tale(system, prompt, n).await;
   
    let file_name =
        match res {
            Ok(content) => {
                let file_name = if is_html(&content) {
                    format!("gen/{}.html", truncate(prompt, 100).replace(' ', "_").to_lowercase())
                } else {
                    format!("gen/{}.txt", truncate(prompt, 100).replace(' ', "_").to_lowercase())
                };
                let content = content
                    .replace("```html", "<!- HTML -->")
                    .replace("```", "");

                write_file(&file_name, &content)?;

                file_name
            },
            Err(_) => "gen/not_available.png".to_string()
        };

    Ok(file_name)
}

pub async fn detail_tale(title: &str, chapters: &Vec<(String, String)>) -> Result<String, Box<dyn Error>> {
    #[derive(serde_derive::Serialize)]
    struct Detail<'a> {
        title: String,
        chapters: &'a Vec<(String, String)>
    }
    let detail = Detail { title: title.into(), chapters };
    let buf: String = serde_json::to_string(&detail).unwrap();

    let file_name = 
        match llm_tale_detail(&buf).await {
            Ok(content) => {
//println!("<<< {content}");
                let file_name = format!("gen/{}_detail.html", truncate(title, 100).replace(' ', "_").to_lowercase());
                let content = content
                    .replace("```html", "<!- HTML -->")
                    .replace("```", "");

                write_file(&file_name, &content)?;

                file_name
            },
            Err(e) => {
                warn!("Error is: {}", e);
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
    if ni.indb {
        let _ = del_news_item(pg_client, prompt, &url).await;
    }
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

async fn text_news(pg_client: &mut tokio_postgres::Client, model: &Option<FlagEmbedding>, client: &Client, prompt: &str, news: &mut Vec<DbNewsItem<Utc>>) -> Result<String, Box<dyn Error>> {
    let mut urls: HashSet<String> = HashSet::new();
    let mut used: HashSet<String> = HashSet::new();
    let mut unprocessed_news: Vec<DbNewsItem<Utc>> = Vec::new();
    let mut processed_news: Vec<DbNewsItem<Utc>> = Vec::new();
    // Core processing with given news
    let mut proc_so_far = 0;
    let mut size = 0;

    for n in news.iter() {
        used.insert(n.url.clone());

        // Filter duplicates
        if urls.contains(&n.url) {
            // Remove duplicate urls. Still some after deduping
            warn!("Duplicate title: {}", n.title);
            if n.summary.is_some() && n.indb {
                let _ = del_news_item(pg_client, prompt, &n.url).await;
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
                            warn!("connection error: {}", e);
                            SUMMARIZE_ERROR.to_owned()
                        }
                    };

                if summary == SUMMARIZE_ERROR {
                    let _ = del_news_item(pg_client, prompt, &n.url).await;
                    continue
                }

                let nn = DbNewsItem::new(&n.url, prompt, &n.source, &n.title, Utc::now(), &Some(summary.clone()), true, n.sentiment, n.embedding.clone(), n.indb);

//println!("UnProcessed : {} {} {} {}", unprocessed_news.len(), n.prompt, n.queried, n.summary.is_some());
                unprocessed_news.push(nn);

                let sz = n.title.len() + summary.len();

                size += sz;

//                if count >= PAGE_TOTAL as usize - 1 || size > get_context_size() - sz {
//println!("---- LLM Batch {proc_so_far} {} >= {}", unprocessed_news.len(), news.len());
//                if (size > get_context_size() - sz || proc_so_far + unprocessed_news.len() >= news.len()) && !unprocessed_news.is_empty() {
                if (size > get_context_size() - sz || proc_so_far + unprocessed_news.len() >= PAGE_TOTAL) && !unprocessed_news.is_empty() {
//println!("LLM Batch {proc_so_far} {} >= {}", unprocessed_news.len(), news.len());
                    //proc_so_far += unprocessed_news.len();
                    processed_news = process_news(pg_client, model, prompt, unprocessed_news, &processed_news).await;
                    proc_so_far += processed_news.len();
                    unprocessed_news = Vec::new();
                    //unprocessed_news = processed_news.clone();
                    //count = 0;
                    size = 0;
                }
            },
            Some(_summary) => {
//println!("Pre Processed : {} {} {}", n.prompt, n.queried, n.summary.is_some());
                proc_so_far += 1;
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
            if n.indb {
                let _ = del_news_item(pg_client, &n.prompt, &n.url).await;
            }
        }
    }

    news.retain(|n| !used.contains(&n.url));

    news.iter_mut().for_each(|n| n.dt = Utc::now());
    add_news(pg_client, news).await?;

    if processed_news.is_empty() {
        warn!("Count is zero");
        Err("Not Found")?
    } else {
        mk_image(prompt, &articles, PAGE_TOTAL, false).map_err(|e| e.into())
    }
}

pub async fn process_news(pg_client: &mut tokio_postgres::Client, model: &Option<FlagEmbedding>, prompt: &str, unprocessed_news: Vec<DbNewsItem<Utc>>, news: &Vec<DbNewsItem<Utc>>) -> Vec<DbNewsItem<Utc>> {
    let mut processed_news: Vec<DbNewsItem<Utc>> = news.to_owned();
    let unproc: Vec<GPTItem> = unprocessed_news.iter().map(|n| GPTItem::new(&n.title, n.summary.as_ref().unwrap(), n.indb)).collect();
    let proc_news = llm_news(&unproc, PAGE_TOTAL).await;

    match proc_news {
        Err(e) => {
            if e.to_string().starts_with("expected value") {
                warn!("LLM cannot handle request: {}", e);
            } else if e.to_string().contains("operation timed out") {
                warn!("LLM timed out: {}", e);
            } else if e.to_string().contains("missing field") {
                warn!("Missing field: {}", e);
            } else {
                warn!("*Unexpected Error: {}", e);
            }
            //for n in unproc {
            //    let _ = del_news_title(pg_client, prompt, &n.title).await;
            //}
            // probably just coms problem or LLM glitch, if in DB can try again
            //continue;
        },
        Ok(mut gpt_items) => {
            let eprompt = get_an_embedding(prompt, model).await;
            for (title, body, score, indb) in &mut gpt_items {
                if *score > 0.0 {
                    let tscore = *score;
                    let ebody = get_an_embedding(body, model).await;

                    if eprompt.is_ok() && ebody.is_ok() {
                        *score = get_embedding(prompt, body, eprompt.as_ref().unwrap(), ebody.as_ref().unwrap());

                        if *score >= 0.3 {
                            warn!("Rejected {prompt} <=> {body} == {tscore}/{score} >= 0.3");
                        }
                    }
                }
                // Consume the items, delete if in DB
                if *indb {
                    let _ = del_news_title(pg_client, prompt, title).await;
                }
            }
//let un_len = unprocessed_news.len();
            let gpts: Vec<DbNewsItem<Utc>> = gpt_items.into_iter().zip(unprocessed_news.into_iter())
                .filter(|((title, body, score, _), _)| {
                    *score < 0.3 && !bad(title, body) && body.contains(' ') && detect_language(body) == Lang::Eng
                })
                .map(|((title, mut body, _sentiment, _), mut n)| {
                    n.title = truncate_sentence(&title, 150).into();
                    body.retain(|c| c != '\n' && c != '\r');
                    body = truncate_sentence(&body, 600).into();
                    n.summary = Some(body);
                    //n.sentiment = sentiment;

                    n
                })
                .collect();
//warn!("processed {} -> unprocessed {} ==> gpts {}", processed_news.len(), un_len, gpts.len());

            processed_news = [processed_news, gpts].concat();
        },
    };

    processed_news
}

fn bad(title: &str, body: &str) -> bool {
    if bad_translations(title, true) {
        warn!("Skipping: Title {}", title);
        true
    } else if bad_translations(body, false) {
        warn!("Skipping: Body {}", body.len());
        true
    //} else if body.len() > 700 {
    //    warn!("Skipping: Long Body {} with title: {}", body.len(), title);
    //    false
    } else {
        false
    }
}

pub async fn get_new_news(prompt: &str, model: &Option<FlagEmbedding>) -> Result<Vec<DbNewsItem<Utc>>, Box<dyn Error>> {
    get_new_generic_news(prompt, prompt, model).await
}

pub async fn get_new_generic_news(prompt: &str, filter: &str, model: &Option<FlagEmbedding>) -> Result<Vec<DbNewsItem<Utc>>, Box<dyn Error>> {
    // find additional news and unpack it
    let dt: DateTime<Utc> = Utc::now() - Duration::hours(TIMEOUT_PERIOD.into());
    let articles = get_news(prompt).await?.articles;

    let nv: Vec<DbNewsItem<Utc>> = match model {
        None => {
            articles.iter()
                .map(|n| DbNewsItem::new(&n.url, prompt, &n.source.name, &n.title, dt, &None, false, 0.0, None, false))
                .collect()
        },
        Some(model) => {
            let titles: Vec<&str> = articles.iter().map(|n| n.title.as_str()).collect();
            let pe = model.query_embed(filter)?;
            let embeddings = model.passage_embed(titles, None)?;

            let na: Vec<DbNewsItem<Utc>> = articles.iter()
                .zip(embeddings.iter())
                //.filter(|(n, e)| match_embedding(prompt, n.title, &pe, e))
                //.map(|(n, e)| DbNewsItem::new(&n.url, prompt, &n.source.name, &n.title, dt, &None, false, cosine_dist(&pe, e), Some(e.clone()), n.indb))
                .map(|(n, e)| DbNewsItem::new(&n.url, prompt, &n.source.name, &n.title, dt, &None, false, get_embedding(filter, &n.title, &pe, e), Some(e.clone()), false))
                //.map(|n| {println!("n.sentiment: {}", n.sentiment); n })
                .filter(|n| n.sentiment < 0.25 && !bad_translations(&n.title, true))
                .collect();

            info!("{prompt}: {:?} results of which {:?} are relevant", articles.len(), na.len());

            na
        },
    };

    Ok(nv)
}

/*
fn match_embedding(prompt: &Vec<f32>, title: &Vec<f32>) -> bool {
    let cd = cosine_dist(&prompt, &title);
    let ed = crate::apis::distance::euclidian_dist(&prompt, &title);

    cd < 0.25 || ed < 0.7
}
*/
fn get_embedding(prompt: &str, text: &str, eprompt: &[f32], etext: &[f32]) -> f32 {
    if prompt.is_empty() || prompt == "*" {
        -1.0
    } else if text.to_lowercase().contains(prompt) {
        0.0
    } else {
        let cd = cosine_dist(eprompt, etext);

        if cd < 0.25 {
            return cd
        } else {
            let ed = euclidian_dist(eprompt, etext);

            if ed < 0.7 {
                ed / 4.0
            } else {
                cd
            }
        }
    }
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
//println!("{:?}", res);
    let mut news: News = res?;
//println!("2 get_enough {search} {call}");
    let uniq_articles: Vec<_> = news.articles.into_iter().unique().collect();
    info!("{search}: {:?} results of which {:?} are unique", news.total_results, uniq_articles.len());
    news.articles = uniq_articles;

    Ok(news)
}

pub fn bad_translations(res: &str, is_title: bool) -> bool {
    let res = res.to_lowercase();
    (is_title && res.len() < 25 || !is_title && res.len() < 100) ||
    res.to_uppercase().starts_with(SUMMARIZE_ERROR) ||      // LLM working produces this
    res.contains("javascript") ||
    res.contains("iframe") ||
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

pub fn get_context_size() -> usize {
    match std::env::var("CONTEXT_SIZE") {
        Ok(val) => val.parse().unwrap_or(16000),
        Err(_) => 16000,
    }
}

async fn get_initial_news(pg_client: &mut tokio_postgres::Client, prompt: &str, model: &Option<FlagEmbedding>) -> Result<Vec<DbNewsItem<Utc>>, Box<dyn Error>> {
    // Clear any old news
    let _ = clear_news(pg_client, prompt, CLEAR_TIMEOUT_PERIOD, PURGE_TIMEOUT_PERIOD).await;

    // Query valid saved news
    let mut db_news = get_saved_news(pg_client, prompt).await?;
    let db_news_len = db_news.len();
    //let _ = reset_news_item_seq(pg_client, prompt).await;

    let min_rows = (PAGE_TOTAL + 1) / 2;

    // REFRESH logic...
    if db_news_len <= min_rows {
        let dt_purge: DateTime<Utc> = Utc::now() - Duration::hours(TIMEOUT_PERIOD.into());
        let current_news: Vec<_> = db_news.into_iter().filter(|n| n.dt < dt_purge).collect();

        if current_news.len() <= min_rows {
//println!("refresh: {} {} {}", db_news_len, current_news.len(), min_rows);
            let new_news = get_new_news(prompt, model).await?;

            db_news = [current_news, new_news].concat();
        } else {
            db_news = current_news;
        }
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
