use crate::llm::gpt::clean_html;
use crate::aps::news::{get_new_news, process_news, bad_translations, get_context_size};
//use news::apis::distance::{cosine_dist, euclidian_dist};
use std::collections::HashSet;
use crate::llm::gpt::llm_embedding;
use chrono::{Utc};
use crate::db::postgres::{DbNewsItem, pg_connect, add_news_item, url_exists, tidy_news, has_prompt_embed, add_prompt_embed};
use log::{error, warn, info};

pub async fn refresh(prompt: &str, target: &str) {
    let prompt: Vec<&str> = prompt.split(',').collect();

    let prompt = prompt.into_iter()
        .filter(|s| !s.trim().is_empty())
        .map(|s| {
            let start = s.chars().next().unwrap();
            let end = s.chars().nth(s.len()-1).unwrap();

            if start == '\'' && end == '\'' || start == '"' && end == '"' {
                &s[1..s.len()-1]
            } else {
                s
            }.trim()
        }
        )
        .collect::<Vec<_>>()
        .join(",");

    refresh_search(&prompt, target).await;
}

async fn refresh_search(prompt: &str, target: &str) {
    let target = if target.is_empty() { "*" } else { target };

    let mut pg = pg_connect().await;

    let mut urls: HashSet<String> = HashSet::new();
    let mut db: HashSet<DbNewsItem<Utc>> = HashSet::new();
    let mut dups = 0;
    let mut totals = 0;
    for prompt in prompt.split(',') {
        if !has_prompt_embed(&mut pg, prompt).await {
            let eprompt = llm_embedding(prompt).await.unwrap();

            add_prompt_embed(&mut pg, eprompt[0].clone(), prompt, "news").await.unwrap();
        }

        let res: Result<Vec<DbNewsItem<Utc>>, Box<dyn std::error::Error>> = get_new_news(prompt).await;

        match res {
            Ok(news) => {
                let mut i = 0;
                let mut sum = 0.0;
                news.iter().for_each(|n| {
                    if ! urls.contains(&n.url) {
                        if n.sentiment < 0.25 && ! bad_translations(&n.title, true) {
                           // info!("True {}", n.title);
                            totals += 1;
                        } else {
                            i += 1;
                            sum += n.sentiment;
                        }
                        db.insert(n.clone());
                    } else {
                        dups += 1;
                    }
                    urls.insert(n.url.clone());
                });
                warn!("{i} fails out of {} average {} with dups {} ", news.len(), sum / news.len() as f32, dups);
            },
            Err(e) => {
                error!("{}", e);
            }
        }
    }

    info!("New news: {} records of which {totals} match {target}", db.len());

    let _ = tidy_news(&mut pg, 3).await;

    let mut unproc: Vec<DbNewsItem<Utc>> = Vec::new();
    let mut size = 0;
    let mut proc_so_far = 0;
    let db_len = db.len();
    for mut i in db.into_iter() {
        if i.summary.is_none() {
            //println!("{}", i.url);
            match reqwest::blocking::get(&i.url) {
                Ok(resp) => {
                    match resp.text() {
                        Ok(resp) => { 
                            if resp.len() < 1000 {
                                warn!("Found data too short: {} - {}", i.url, resp.len())
                            } else {
                                let summary = clean_html(&resp);

                                let s_len = summary.len();
                                if s_len < 1000 {
                                    warn!("Clean data too short: {} - {}", i.url, s_len)

                                } else if url_exists(&mut pg, &i.url).await {
                                    warn!("Url already exists: {} - {}", i.url, s_len)

                                } else {
                                    let sz = i.title.len() + s_len;

                                    size += sz;
                                    i.queried = true;
                                    i.summary = Some(summary);

                                    unproc.push(i.clone());

                                    if size > get_context_size() - sz || proc_so_far + unproc.len() >= db_len {
                                        proc_so_far += unproc.len();

                                        let news = process_news(&mut pg, prompt, unproc, &Vec::new()).await;
                                        for i in news.iter() {
                                            let _ = add_news_item(&mut pg, i, 0).await;
                                        }

                                        unproc = Vec::new();
                                        size = 0;
                                    }
//println!("{} - {} / {}", i.url, resp.len(), s_len);
                                }
                            }
                        },
                        Err(e) => error!("{e}")
                    }
                },
                Err(e) => error!("{} # {}", i.url, e)
            }
        }
    }
}
