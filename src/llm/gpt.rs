use std::error::Error;
use std::path::Path;
use std::fs::File;
use std::io::{stdin, BufRead, BufReader};
use std::collections::HashMap;
use std::fmt::Write;
use regex::Regex;
use stringreader::StringReader;
use crate::apis::openai::{Message, call_gpt, call_gpt_model, call_gpt_image_model, call_embedding_model, fetch_url};
use fastembed::{FlagEmbedding, EmbeddingBase};
use crate::apis::mistral::{call_mistral, call_mistral_model};
//use crate::apis::gemini::{call_gemini, call_gemini_model, message_to_content};
use crate::apis::bard::{call_bard, call_bard_model, message_to_content};
use crate::image::render::{PAGE_TOTAL, mk_filename};
use crate::template::Template;
use serde_derive::{Serialize, Deserialize};
//use serde_json::Value;
//use shannon_entropy::shannon_entropy;
use is_html::is_html;
//use strfmt::strfmt;
use log::{warn, info};

type LlmValue = (String, String, f32, bool);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GPTItem {
    pub title: String,
    pub body: String,
    pub indb: bool,
}

impl GPTItem {
    pub fn new(title: &str, body: &str, indb: bool) -> Self {
        GPTItem { title: title.to_string(), body: body.to_string(), indb }
    }

    pub fn size(&self) -> usize {
        self.title.len() + self.body.len()
    }
}

pub const SUMMARIZE_ERROR: &str = "CANNOT SUMMARIZE";

pub fn truncate(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        None => s,
        Some((idx, _)) => &s[..idx],
    }
}

pub fn truncate_sentence(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        None => s,
        Some((idx, _)) => {
            match s[..idx].rfind('.') {
                None => &s[..idx],
                Some(pos) => &s[..pos+1]
            }
        }
    }
}

pub fn initcap(word: &str) -> String {
    word.char_indices()
        .fold(String::new(), |mut acc, (i, c)| {
            if i == 0 {
                acc.push(c.to_ascii_uppercase());
            } else {
                acc.push(c);
            }
            acc
        })
}

pub async fn llm_code(system: &str, req: &str, language: &str) -> Result<String, Box<dyn std::error::Error + Send>> {
    let mut messages: Vec<Message> = Vec::new();
    //let file = &format!("instructions/{language}_pre.txt");
    let file = "instructions/code_pre.txt";

    let h = HashMap::from([("lang".to_string(), initcap(language))]);
    if Path::new(file).exists() {
        let pre = parse_instructions(file).into_iter().map(|s| Template::new(&s).render_strings(&h)).collect();

        add_messages("system", &pre, &mut messages);
    }

    add_message("system", system, &mut messages);

    add_message("user", req, &mut messages);

    //call_gpt_model("gpt-4-1106-preview", messages, false).await
    call_model(messages, true, false).await
}

pub async fn llm_embedding(req: &str) -> Result<Vec<f32>, Box<dyn std::error::Error + Send>> {
    let model: String =
        std::env::var("GPT_EMBEDDING_VERSION").expect("GPT_EMBEDDING_VERSION not found in enviornment variables");

    call_embedding_model(&model, &[req.to_string()]).await
}

pub async fn get_embed_model() -> Result<FlagEmbedding, Box<dyn Error>> {
    Ok(FlagEmbedding::try_new(Default::default())?)
}

pub async fn get_an_embedding(prompt: &str, model: &Option<FlagEmbedding>) -> Result<Vec<f32>, Box<dyn std::error::Error + Send>> {
    match model {
        None => {
            llm_embedding(prompt).await
        },
        Some(model) => {
            Ok(model.query_embed(prompt)?)
        },
    }
}

pub async fn llm_image(prompt: &str, system: &str) -> Result<String, Box<dyn std::error::Error + Send>> {
    let url = call_gpt_image_model("dall-e-3", &format!("{prompt}\n{system}"), "1792x1024", 1).await?;
    let file = &mk_filename(prompt);

    match fetch_url(&url, file).await {
        Ok(()) => Ok(file.into()),
        Err(e) => Err(anyhow::Error::msg(e.to_string()).into())
    }
}

pub async fn llm_brainstorm(file: &str, req: &str) -> Result<String, Box<dyn std::error::Error + Send>> {
    let mut messages: Vec<Message> = Vec::new();
    let bits: Vec<&str> = file.split('.').collect();

    if bits.len() == 2 && Path::new(&format!("{}_pre.{}", bits[0], bits[1])).exists() {
        let pre = parse_instructions(&format!("{}_pre.{}", bits[0], bits[1]));

        add_messages("system", &pre, &mut messages);
    }

    let sys = parse_instructions(file);

    add_messages("system", &sys, &mut messages);

    if bits.len() == 2 && Path::new(&format!("{}_post.{}", bits[0], bits[1])).exists() {
        let post = parse_instructions(&format!("{}_post.{}", bits[0], bits[1]));

        add_messages("system", &post, &mut messages);
    }

    add_message("user", req, &mut messages);

    //call_gpt_model("gpt-4-1106-preview", messages, false).await
    call_model(messages, true, false).await
}

pub async fn llm_tale(text: &str, req: &str, n: usize) -> Result<String, Box<dyn std::error::Error + Send>> {
    let mut messages: Vec<Message> = Vec::new();

    if Path::new("instructions/tale_pre.txt").exists() {
        let pre = parse_instructions("instructions/tale_pre.txt");

        add_messages("system", &pre, &mut messages);
    }

    let sys = parse_text_instructions(text);

    add_messages("system", &sys, &mut messages);

    if Path::new("instructions/tale_post.txt").exists() {
        let h = HashMap::from([("chapters".to_string(), n.to_string())]);
        let post = parse_instructions("instructions/tale_post.txt").into_iter().map(|s| Template::new(&s).render_strings(&h)).collect();

        add_messages("system", &post, &mut messages);
    }

    add_message("user", req, &mut messages);

    //call_gpt_model("gpt-4-1106-preview", messages, false).await
    //call_gpt_model("gpt-3.5-turbo-1106", messages, false).await
    call_model(messages, true, false).await
}

pub async fn llm_tale_detail(req: &str) -> Result<String, Box<dyn std::error::Error + Send>> {
    let mut messages: Vec<Message> = Vec::new();

    if Path::new("instructions/tale_detail.txt").exists() {
        let pre = parse_instructions("instructions/tale_detail.txt");

        add_messages("system", &pre, &mut messages);
    }

    add_message("user", req, &mut messages);

    //call_gpt_model("gpt-4-1106-preview", messages, true).await
    //call_gpt_model("gpt-3.5-turbo-1106", messages, false).await
    call_model(messages, true, false).await
}

async fn call_model(messages: Vec<Message>, high: bool, is_json: bool) -> Result<String, Box<dyn std::error::Error + Send>> {
    let llm: &str = &std::env::var("LLM_TO_USE").map_err(anyhow::Error::new)?;

    match llm {
        "mistral" => {
            let model: &str = &std::env::var(if high { "MISTRAL_HIGH_VERSION" } else { "MISTRAL_VERSION" }).map_err(anyhow::Error::new)?;
            call_mistral_model(model, messages).await
        },
//        "google" | "gemini" => call_gemini_model(message_to_content(&messages)).await,
        "bard" => call_bard_model(message_to_content(&messages)).await,
        _ => {
            let model: &str = &std::env::var(if high { "GPT_HIGH_VERSION" } else { "GPT_VERSION" }).map_err(anyhow::Error::new)?;
            call_gpt_model(model, messages, is_json).await
        },
    }
}

/*
pub async fn llm_title(req: &str) -> Result<String, Box<dyn std::error::Error + Send>> {
    llm_news_items_with_context(&mut [], &mut ["Summarize in English in less then 15 words.", &format!("If cannot summarize say '{SUMMARIZE_ERROR}' only")], req).await
}

pub async fn llm_news_items(prompt: &str, title: &str, req: &str, its: u32) -> Result<String, Box<dyn std::error::Error + Send>> {
    let sum_len = (50 * PAGE_TOTAL) / its;
    let word_len = sum_len / 10;
    llm_news_items_with_context(&mut [], &mut [&format!("Summarize with less than {sum_len} words"), "Explain as simply as possible", &format!("Summary must explain {prompt}"), &format!("Summary must be relevant to: {title}"), &format!("No sentences longer than {word_len} words"), &format!("If cannot summarize say '{SUMMARIZE_ERROR}' only")], req).await
}
*/

pub async fn llm_news(item: &[GPTItem], its: usize) -> Result<Vec<LlmValue>, Box<dyn std::error::Error + Send>> {
    llm_news_text(item, its).await
}

pub async fn llm_news_text(items: &[GPTItem], its: usize) -> Result<Vec<LlmValue>, Box<dyn std::error::Error + Send>> {
    let sum_len = (600 * PAGE_TOTAL) / its;
    let title_len = 30;
    //let title_len = sum_len / 7;
    let body_len = sum_len * 5;
    let mut vars: HashMap<String, String> = HashMap::new();

    vars.insert("title_len".to_string(), title_len.to_string());
    vars.insert("body_len".to_string(), body_len.to_string());

    let sys = parse_instructions("instructions/news_sys.txt");
    let sys: Vec<String> = sys.iter().map(|s| Template::new(s).render_strings(&vars)).collect();

    let res = llm_news_items_with_context(&vec![], &sys, items).await?;

    unpack_llm(&res.replace("\\_", "_"), items)
}

async fn llm_news_items_with_context(prior: &Vec<String>, context: &Vec<String>, item: &[GPTItem]) -> Result<String, Box<dyn std::error::Error + Send>> {
    let start = std::time::Instant::now();

    let req = pack_llm(item)?;
    let mut messages: Vec<Message> = Vec::new();

//println!("{:?}", context);
    add_messages("assistant", prior, &mut messages);
    add_messages("system", context, &mut messages);
    add_message("user", &req, &mut messages);

    let llm: &str = &std::env::var("LLM_TO_USE").map_err(anyhow::Error::new)?;
    let resp = match llm {
        "mistral" => call_mistral(messages).await,
//        "google" | "gemini" => call_gemini(message_to_content(&messages)).await,
        "bard" => call_bard(message_to_content(&messages)).await,
        _ => call_gpt(messages).await,
    };

    info!("{llm} LLM took {:?} for {} entries, net {:?}", start.elapsed(), item.len(), start.elapsed() / item.len() as u32);

    resp
}

fn pack_llm(item: &[GPTItem]) -> Result<String, Box<dyn std::error::Error + Send>> {
    let data: String = item.iter().enumerate()
        .map(|(i, n)| (i, &n.title, &n.body))
        .fold(String::new(), |mut o, (i, title, body)| {
            let _ = write!(o, "\"title_{i}\": \"{title}\", \"body_{i}\": \"{body}\", ");
            o
        });

//println!("{:?}", data.strip_suffix(", "));
    if data.is_empty() {
        Err(anyhow::Error::msg("Poor Quality Body").into())
    } else {
        // Note: not strictly correct json as theer is a trailing comma,
        // strangely LLM does nmot work without.
        Ok(format!("{{ {} }}", data))
    }
}

fn unpack_llm(result: &str, items: &[GPTItem]) -> Result<Vec<LlmValue>, Box<dyn std::error::Error + Send>> {
//println!("unpack_llm {}", res);
    // Treat sentiment as String and convert, simpler that dealing with Values
    //let re = Regex::new(r#"("sentiment_[0-9]+"): ([+-]?[0-9]+\.[0-9]+)"#).unwrap();
    //let result: &str = &re.replace_all(result, "$1: \"$2\"");
    let h: HashMap<String, String> = serde_json::from_str(result).map_err(anyhow::Error::new)?;
    let mut res: Vec<LlmValue> = vec![];

    //for i in 0 .. items.len() {
    for (i, item) in items.iter().enumerate() {
        let title = h.get(&format!("title_{}", i));
        let title =
            match title {
                Some(title) => title,
                None => {
                    warn!("Bad Title: {:?}", title);
                    res.push((item.title.clone(), "".into(), 0.0, false));
                    continue
                }
            };
        let body = h.get(&format!("body_{}", i));
        let body =
            match body {
                Some(body) => body,
                None => {
                    warn!("Bad Body: {:?}", body);
                    res.push((item.title.clone(), "".into(), 0.0, false));
                    continue
                }
            };
        /*
        let sentiment = h.get(&format!("sentiment_{}", i));
        let sentiment =
            match sentiment {
                Some(sentiment) => {
                    match sentiment.parse::<f32>() {
                        Ok(s) => s,
                        Err(e) => {
                            eprintln!("Bad Sentiment: parse {:?}", e);
                            res.push((items[i].title.clone(), "".into(), 0.0, false));
                            continue
                        }
                    }
                },
                None => {
                    eprintln!("Bad Sentiment: {:?}", sentiment);
                    res.push((items[i].title.clone(), "".into(), 0.0, false));
                    continue
                }
            };

//println!("{:?}", sentiment);
        */
        let sentiment = 0.0;
        res.push((title.into(), body.into(), sentiment, item.indb));
    }

    Ok(res)
}

// Expensive!
pub fn clean_html(body: &str) -> String {
    let body = html2text::from_read(BufReader::new(StringReader::new(body)), 10000);
    let body = Regex::new(r"\[.*?\]").unwrap().replace_all(&body, "");
    let body = Regex::new(r"http\S+").unwrap().replace_all(&body, "");
    let body = Regex::new(r"[^\w.,!?]+").unwrap().replace_all(&body, " ");
    let body = truncate_sentence(&body, 3000).to_owned();

    if body.len() < 500 || is_html(&body) {
        warn!("Poor Quality len: {}", body.len());

        SUMMARIZE_ERROR.to_owned()
    } else {
        //println!("Quality: {} Len: {}", shannon_entropy(&body), body.len());

        body
    }
}

pub fn parse_instructions(file: &str) -> Vec<String> {
    instructions(&read_to_vec(file))
}

pub fn parse_text_instructions(text: &str) -> Vec<String> {
    instructions(&read_to_vec_text(text))
}

fn instructions(lines: &Vec<String>) -> Vec<String> {
    let mut ins: Vec<String> = vec![];
    let mut line = "".to_string();

    for l in lines {
        let l = l.trim();

        if l.starts_with('#') {
            if !line.is_empty() {
                ins.push(line);
            }
            line = format!("{}: ", l.strip_prefix('#').unwrap().trim());
        } else if !l.trim().is_empty() {
            if l.starts_with('-') {
                line.push_str(&format!("{} ", l.strip_prefix('-').unwrap().trim()));
            } else {
                ins.push(l.into());
            };
        };
    }

    if !line.is_empty() { ins.push(line); };

    ins
}

pub fn read_to_vec_text(source: &str) -> Vec<String> {
    BufReader::new(StringReader::new(source)).lines().map(|l| l.unwrap()).collect()
}

pub fn read_to_vec(source: &str) -> Vec<String> {
    fn read_lines(filename: &str) -> Vec<String> {
        match File::open(filename) {
            Ok(lines) => BufReader::new(lines).lines().map(|l| l.unwrap()).collect(),
            Err(_) => vec![]
        }
    }

    fn read_stdio() -> Vec<String> {
        let lines = stdin().lock().lines();
        let mut all_lines: Vec<String> = vec![];

        for line in lines {
            all_lines.push(line.unwrap());
        }

        all_lines
    }

    if source != "-" {
        read_lines(source)
    } else {
        read_stdio()
    }
}

fn add_messages(typ: &str, sys: &Vec<String>, messages: &mut Vec<Message>) {
    for s in sys {
        add_message(typ, s, messages);
    }
}

fn add_message(typ: &str, s: &str, messages: &mut Vec<Message>) {
        let system: Message = Message {
                role: typ.to_string(),
                content: s.to_string(),
            };

        messages.push(system);
}
