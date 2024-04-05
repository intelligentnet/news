use std::fs;
use std::collections::HashSet;
use std::error::Error;
//use std::fs::File;
//use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{BufWriter, Write};
use reqwest::Client;
use chrono::{DateTime, Duration, Utc};
use tokio_postgres;
//use fastembed::{TextEmbedding};
use whichlang::{detect_language, Lang};
use crate::image::render::{mk_image_with_thumbnails, use_image};
use crate::llm::gpt::{GPTItem, llm_news, llm_image, llm_tale, llm_code, llm_tale_detail, llm_embedding, llm_embedding_many, truncate, truncate_sentence, SUMMARIZE_ERROR, clean_html};
use crate::image::render::{PAGE_TOTAL, mk_filename};
use crate::db::postgres::*;
use crate::apis::newsapi::get_news;
//use crate::apis::distance::{cosine_dist, euclidian_dist};
use is_html::is_html;
use log::warn;

pub const TIMEOUT_PERIOD: u32 = 4;
pub const CLEAR_TIMEOUT_PERIOD: u32 = 24;
pub const PURGE_TIMEOUT_PERIOD: u32 = 24 * 7;

pub fn get_http_client(timeout: u64) -> Result<Client, Box<dyn Error>> {
    Ok(Client::builder()
        .user_agent("TargetR")
        .gzip(true)
        .timeout(std::time::Duration::new(timeout, 0))
        .build()?)
}

pub async fn news(prompt: &str, fmt: &str, initial: bool) -> Result<String, Box<dyn Error>> {
    let prompt = if fmt == "picture" && !prompt.contains(fmt) {
        format!("{prompt} {fmt}")
    } else {
        prompt.into()
    };
    let prompt = &prompt;
    let mut pg_client = pg_connect().await;

    // Only need to do this initially or if does not exist
    if initial || how_long_since_created(&mk_filename(prompt).replace("/gen/", "gen/")) > (TIMEOUT_PERIOD * 60 * 60) as u64 {

        // Same here
        if initial || !has_prompt_embed(&mut pg_client, prompt).await {
            let eprompt = llm_embedding(prompt).await.map_err(|e| e.to_string())?;

            add_prompt_embed(&mut pg_client, eprompt[0].clone(), prompt, fmt).await?;
        }

        let mut news = get_initial_news(&mut pg_client, prompt).await?;

        if news.is_empty() || news[0].dt + Duration::try_hours(TIMEOUT_PERIOD.into()).unwrap() > Utc::now() {
            Ok(mk_filename(prompt))
        } else {
            match fmt {
                "picture" =>
                    picture_news(&mut pg_client, prompt, &mut news).await,
                _ => {
                    let client = get_http_client(10)?;

                    text_news(&mut pg_client, &client, prompt, &mut news).await
                },
            }
        }
    } else {
        Ok(mk_filename(prompt))
    }
}

pub fn how_long_since_created(file: &str) -> u64 {
    fn file_modified_time_in_seconds(path: &str) -> u64 {
        fs::metadata(path)
            .unwrap()
            .modified()
            .unwrap()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    if std::path::Path::new(file).exists() {
        let ts: u64 = file_modified_time_in_seconds(file);
        let now: u64 = Utc::now().timestamp() as u64;

        now - ts
    } else {
        u64::MAX
    }
}

async fn picture_news(pg_client: &mut tokio_postgres::Client, prompt: &str, news: &mut Vec<DbNewsItem<Utc>>) -> Result<String, Box<dyn Error>> {
    /*
    let mxi = news.iter().enumerate()
        .filter(|(_, n)| detect_language(&n.title) == Lang::Eng)
        .max_by(|(_, a), (_, b)| a.sentiment.total_cmp(&b.sentiment))
        .map(|(i, _)| i).unwrap();
    */
    let mut ni: DbNewsItem<Utc> = news[0].clone();

    // Find first one in English
    for n in news.iter() {
        if detect_language(&n.title) == Lang::Eng { ni = n.clone(); break; }
    };
    let url = ni.url.clone();
    if ni.indb {
        let _ = del_news_item(pg_client, &url).await;
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

async fn text_news(pg_client: &mut tokio_postgres::Client, client: &Client, prompt: &str, news: &mut Vec<DbNewsItem<Utc>>) -> Result<String, Box<dyn Error>> {
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
            warn!("Duplicate url: {}", n.url);
            if n.summary.is_some() && n.indb {
                let _ = del_news_item(pg_client, &n.url).await;
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
                    let _ = del_news_item(pg_client, &n.url).await;
                    continue
                }

                let nn = DbNewsItem::new(&n.url, prompt, &n.source, &n.title, Utc::now(), &Some(summary.clone()), true, n.sentiment, n.embedding.clone(), n.url_to_image.clone(), n.indb);

                unprocessed_news.push(nn);

                let sz = n.title.len() + summary.len();

                size += sz;

                if (size > get_context_size() - sz || proc_so_far + unprocessed_news.len() >= PAGE_TOTAL) && !unprocessed_news.is_empty() {
                    //proc_so_far += unprocessed_news.len();
                    processed_news = process_news(pg_client, prompt, unprocessed_news, &processed_news).await;
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
            //articles.push((n.source.clone(), n.title.clone(), n.summary.clone().unwrap()));
            let thumbnail =
                match n.url_to_image {
                    Some(ref url) => {
                        url.to_string()
                    },
                    None => {
                        warn!("Thumbnail: Not Found");

                        "no_thumbnail".into()
                    }
                };
            articles.push((thumbnail, n.title.clone(), n.summary.clone().unwrap()));
            if n.indb {
                let _ = del_news_item(pg_client, &n.url).await;
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
        //mk_image(prompt, &articles, PAGE_TOTAL, false).map_err(|e| e.into())
        mk_image_with_thumbnails(prompt, &articles, PAGE_TOTAL, false).map_err(|e| e.into())
    }
}

pub async fn process_news(pg_client: &mut tokio_postgres::Client, prompt: &str, unprocessed_news: Vec<DbNewsItem<Utc>>, news: &Vec<DbNewsItem<Utc>>) -> Vec<DbNewsItem<Utc>> {
    let mut processed_news: Vec<DbNewsItem<Utc>> = news.to_owned();
    let unproc: Vec<GPTItem> = unprocessed_news.iter().map(|n| GPTItem::new(&n.title, n.summary.as_ref().unwrap(), n.indb)).collect();
    let proc_news = llm_news(&unproc, prompt, PAGE_TOTAL).await;

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
        },
        Ok(gpt_items) => {
            /*
            // Too expensive
            let eprompt = llm_embedding(prompt).await;
            if eprompt.is_ok() {
                let eprompt = eprompt.unwrap()[0].clone();
                for (title, body, score, indb) in &mut gpt_items {
                    if *score > 0.0 {
                        let tscore = *score;
                        let ebody = llm_embedding(body).await;

                        if ebody.is_ok() {
                            *score = get_embedding(prompt, body, &eprompt, &ebody.unwrap()[0]);

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
            }
            */
            let mut gpts: Vec<DbNewsItem<Utc>> = Vec::new();
            for (i, ((title, mut body, score, indb), mut n)) in gpt_items.into_iter().zip(unprocessed_news.into_iter()).enumerate() {
            // Too expensive
                if score >= 0.3 || bad(i, &title, &body) || !body.contains(' ') || detect_language(&body) != Lang::Eng {
                    if indb {
                        warn!("Poor data removed {}", n.url);
                        let _ = del_news_item(pg_client, &n.url).await;
                    }
                } else {
                    n.title = truncate_sentence(&title, 150).into();
                    body.retain(|c| c != '\n' && c != '\r');
                    body = truncate_sentence(&body, 600).into();
                    n.summary = Some(body);

                    gpts.push(n);
                }
            }

            processed_news = [processed_news, gpts].concat();
        },
    };

    processed_news
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

fn bad(i: usize, title: &str, body: &str) -> bool {
    if bad_translations(title, true) {
        warn!("Skipping: Title {i} {}", title);
        true
    } else if bad_translations(body, false) {
        warn!("Skipping: Body {i} {}", body.len());
        true
    //} else if body.len() > 700 {
    //    warn!("Skipping: Long Body {} with title: {}", body.len(), title);
    //    false
    } else {
        false
    }
}

pub async fn get_new_news(prompt: &str) -> Result<Vec<DbNewsItem<Utc>>, Box<dyn Error>> {
    // find additional news and unpack it
    let dt: DateTime<Utc> = Utc::now() - Duration::try_hours(TIMEOUT_PERIOD.into()).unwrap();
    let articles = get_news(prompt).await?.articles;

    let mut nv: Vec<DbNewsItem<Utc>> =
        articles.iter()
            .filter(|n| n.title.len() >= 30)
            .map(|n| DbNewsItem::new(&n.url, prompt, &n.source_name, &n.title, dt, &None, false, 0.0, None, n.url_to_image.clone(), false))
            //.map(|n| DbNewsItem::new(&n.url, prompt, &n.source_name, &n.title, dt, &Some(n.description.clone()), false, 0.0, None, n.url_to_image.clone(), false))
            .collect();
//articles.iter().for_each(|n| println!("{}: {}", n.description, n.description.len()));

    let titles: Vec<_> = nv.iter().map(|n| n.title.clone()).collect();
    let embeddings = llm_embedding_many(&titles[..]).await.map_err(|e| e.to_string())?;
    
    nv.iter_mut().zip(embeddings.into_iter()).for_each(|(n, e)| n.embedding = Some(e));
//println!("{:?}", nv[0]);

    Ok(nv)
}

/*
fn match_embedding(prompt: &Vec<f32>, title: &Vec<f32>) -> bool {
    let cd = cosine_dist(&prompt, &title);
    let ed = crate::apis::distance::euclidian_dist(&prompt, &title);

    cd < 0.25 || ed < 0.7
}

pub fn get_embedding(prompt: &str, text: &str, eprompt: &[f32], etext: &[f32]) -> f32 {
    let cd = cosine_dist(eprompt, etext);

    if prompt.is_empty() || prompt == "*" {
        -2.0 + cd
    } else if text.to_lowercase().contains(prompt) {
        -1.0 + cd
    } else if cd < 0.25 {
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
*/

pub fn bad_translations(res: &str, is_title: bool) -> bool {
    let res = res.to_lowercase();

    (is_title && res.len() < 30 || !is_title && res.len() < 100) ||
    res.to_uppercase().starts_with(SUMMARIZE_ERROR) ||      // LLM working produces this
    res.contains("javascript") ||
    res.contains("iframe") ||
    res.contains(" html ") ||
    (res.contains("access") && res.contains("denied")) ||
    res.contains("subscription") ||
    res.contains("does not contain relevant") ||
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

async fn get_initial_news(pg_client: &mut tokio_postgres::Client, prompt: &str) -> Result<Vec<DbNewsItem<Utc>>, Box<dyn Error>> {
    // Clear any old news
    let _ = clear_news(pg_client, prompt, CLEAR_TIMEOUT_PERIOD, PURGE_TIMEOUT_PERIOD).await;

    // Query valid saved news
    let mut db_news = get_saved_news(pg_client, prompt).await?;
    let db_news_len = db_news.len();
    //let _ = reset_news_item_seq(pg_client, prompt).await;

    let min_rows = (PAGE_TOTAL + 1) / 2;

    // REFRESH logic...
    if db_news_len <= min_rows {
        let dt_purge: DateTime<Utc> = Utc::now() - Duration::try_hours(TIMEOUT_PERIOD.into()).unwrap();
        let current_news: Vec<_> = db_news.into_iter().filter(|n| n.dt < dt_purge).collect();

        if current_news.len() <= min_rows {
//println!("refresh: {} {} {}", db_news_len, current_news.len(), min_rows);
            let new_news = get_new_news(prompt).await?;

            db_news = [current_news, new_news].concat();
        } else {
            db_news = current_news;
        }
    }

    Ok(db_news)
}

/*
pub fn thumbnail(size: u32, url: &str) -> Result<i64, String> {
    fn hashname(url: &str) -> i64 {
        let mut hasher = DefaultHasher::new();

        url.hash(&mut hasher);

        (hasher.finish() >> 1) as i64
    }

    let file_no = hashname(url);
    let file_name = format!("gen/{file_no}.png");

    let img_bytes = reqwest::blocking::get(url)
        .map_err(|e| format!("{}: {}", url, e))?
        .bytes()
        .map_err(|e| format!("{}: {}", url, e))?;
    let img = image::load_from_memory(&img_bytes)
        .map_err(|e| format!("{}: {}", url, e))?;

    let scaled = img.thumbnail(size, size);
    let mut output = File::create(file_name)
        .map_err(|e| format!("{}: {}", url, e))?;

    scaled.write_to(&mut output, image::ImageFormat::Png)
        .map_err(|e| format!("{}: {}", url, e))?;

    Ok(file_no)
}
*/

fn write_file(file_name: &str, content: &str) -> Result<(), Box<dyn Error>> {
    let file = fs::File::create(file_name)?;
    let mut writer = BufWriter::new(file);

    writer.write_all(content.as_bytes())?;

    writer.flush()?;

    Ok(())
}
