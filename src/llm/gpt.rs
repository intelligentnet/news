use html2text;
use regex::Regex;
use std::io::BufReader;
use std::fmt;
use stringreader::StringReader;
use crate::apis::openai::{Message, call_gpt, call_gpt_model};
use crate::image::render::PAGE_TOTAL;
use serde_derive::{Serialize, Deserialize};
use shannon_entropy::shannon_entropy;
use is_html::is_html;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GPTItems {
    pub pairs: Vec<GPTItem>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GPTItem {
    pub title: String,
    pub body: String,
}

/*
pub const GPTITEM_SCHEMA: &str =
r#"{"type": "object",
  "properties": {
    "title": { "type": "string", "description": "Title" },
    "body": { "type": "string", "description": "Body" }
  },
  "required": ["title", "body"]
}"#;
*/

impl GPTItems {
    pub fn new(title: &str, body: &str) -> Self {
        GPTItems { pairs: vec![GPTItem { title: title.to_string(), body: body.to_string() }]}
    }
}

impl GPTItem {
    pub fn new(title: &str, body: &str) -> Self {
        GPTItem { title: title.to_string(), body: body.to_string() }
    }
}

impl fmt::Display for GPTItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TITLE: {}\nBODY: {}\n", self.title, self.body)
    }
}

/*
impl ToString for GPTItem {
    fn to_string(&self) -> String {
        format!("TITLE: {}\n\nBODY: {}\n\n", self.title, self.body)
    }
}
*/

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

/*
pub async fn llm_plain(req: &str) -> Result<String, Box<dyn std::error::Error + Send>> {
    let system = r#"
# MAIN PURPOSE
The goal is to enable the chatbot to communicate in the persona of a Scottish barkeep in an alternate history setting of 1834, where Vikings still exist. The dialect should be Anglicized Scottish.

# DIALECT
- Utilize Scottish words and expressions that were prevalent in the early 19th century.
- Keep the language Anglicized to ensure accessibility.
- Sprinkle in some localized slang or idioms for authenticity, but make sure they are comprehensible to a modern audience.

# PERSONALITY
- Portray a warm, approachable barkeep character.
- Express opinions and attitudes typical of a Scottish tavern owner.
- Use colloquial language and be prepared to share stories, insights, or rumors about the local area, particularly involving Vikings.

# INTERACTIONS
- Respond to inquiries about drinks, food, and lodging as would be expected of a barkeep.
- Engage in light banter or serious conversation as the situation demands.
- Share tales or legends about Vikings if prompted.
- Show familiarity with local events, customs, and the political climate of the time.

# LIMITATIONS
- Stay consistent with the time period and cultural context.
- Avoid using modern slang or references that would break immersion.
- Be mindful of the user's comprehension level, balancing authenticity with understandability.
"#;
    llm_news_items_with_context(&mut [system], &mut [], req).await
}

pub async fn llm_context(req: &str) -> Result<String, Box<dyn std::error::Error + Send>> {
    llm_news_items_with_context(&mut ["The town is called Bogvile", "People in Bogvile are bogs", "Bogs like climbing freetums", "Bogs pets are cows called Mabel", "I am 16 years old"], &mut [], req).await
    //llm_news_items_with_context(&mut ["The Los Angeles Friends won the Boris Cup in 2020."], &mut ["Who won the Boris Cup in 2020?"], req).await
}
*/

pub async fn llm_code(req: &str) -> Result<String, Box<dyn std::error::Error + Send>> {
    let mut messages: Vec<Message> = Vec::new();

    let system: Message = Message {
            role: "system".to_string(),
            content: "Show Rust code without explanation".to_string(),
        };

    let user: Message = Message {
            role: "user".to_string(),
            content: req.to_string(),
        };

    messages.push(system);
    messages.push(user);

    call_gpt_model("gpt-4-1106-preview", messages).await
}

pub async fn llm_brainstorm(req: &str) -> Result<String, Box<dyn std::error::Error + Send>> {
    let sys =
r#"# MISSION
You are a search query generator. You will be given a specific query or problem by the USER and you are to generate a JSON list of at most 5 questions that will be used to search the internet. Make sure you generate comprehensive and counterfactual search queries. Employ everything you know about information foraging and information literacy to generate the best possible questions.

# REFINE QUERIES
You might be given a first-pass information need, in which case you will do the best you can to generate "naive queries" (uninformed search queries). However the USER might also give you previous search queries or other background information such as accumulated notes. If these materials are present, you are to generate "informed queries" - more specific search queries that aim to zero in on the correct information domain. Do not duplicate previously asked questions. Use the notes and other information presented to create targeted queries and/or to cast a wider net.

# OUTPUT FORMAT
In all cases, your output must be a simple JSON list of strings.
"#;

    let mut messages: Vec<Message> = Vec::new();

    let system: Message = Message {
            role: "system".to_string(),
            content: sys.to_string(),
        };

    let user: Message = Message {
            role: "user".to_string(),
            content: req.to_string(),
        };

    messages.push(system);
    messages.push(user);

    call_gpt_model("gpt-4-1106-preview", messages).await
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

pub async fn llm_news(prompt: &str, item: &Vec<GPTItem>, its: u32) -> Result<Vec<(String, String)>, Box<dyn std::error::Error + Send>> {
    llm_news_text(prompt, item, its).await
}

pub async fn llm_news_text(prompt: &str, item: &Vec<GPTItem>, its: u32) -> Result<Vec<(String, String)>, Box<dyn std::error::Error + Send>> {
    let sum_len = (600 * PAGE_TOTAL) / its;
    let word_len = sum_len / 7;
    //let res = llm_news_items_with_context(&mut [], &mut ["Text is supplied as json with a list of pairs, each pair has a title and body, translate both title and body into English", "Summarize title in less that 15 words in a single sentence", &format!("Summarize body with less than {sum_len} wordsi into a new body"), "Explain as simply as possible", &format!("body summary must explain {prompt}"), &format!("body summary must be relevant to title"), &format!("No body sentences longer than {word_len} words"), &format!("If you cannot summarize or translate say '{SUMMARIZE_ERROR}' only"), "Output title and summarized body must be a json array of pairs only"], item).await?;
    //let res = llm_news_items_with_context(&mut [], &mut ["Text is supplied as TITLE: and BODY:, translate both into English", "Summarize TITLE: in less that 15 words in a single sentence", &format!("Summarize BODY: with less than {word_len} words"), "Explain as simply as possible", &format!("BODY: summary must explain {prompt}"), &format!("BODY: summary must be relevant to TITLE:"), &format!("No BODY: sentences longer than {word_len} words"), "More than one user message may contain a TITLE: and BODY: pairs, process each one into a response creating a json array of responses", "Output a json pair for each response where TITLE: is called title and BODY: is called body"], item).await?;
    //let mut res = llm_news_items_with_context(&mut [], &mut ["Message content is supplied as a json array with a list of pairs in each array element, each pair has a title and a body, translate both title and body separately into English", "Translate each pair in the json array separately", "Summarize each title in less that 15 words in a single sentence", &format!("Summarize each body in less than {word_len} words into a new body"), "Explain body as simply as possible", &format!("The body must be relevant to {prompt}"), &format!("The body must be relevant to title"), &format!("No body can be longer than {word_len} words"), "Output title and summarized body must be entries in a json object of pairs, each object of the array may come from more than one message, or from multiple elements of the original calling json array", &format!("If the title or body cannot be summarized or translated then say '{SUMMARIZE_ERROR}' only for that element"), "Always return a json array of pairs of title and body in a single json array"], item).await?;
    let mut res = llm_news_items_with_context(&mut [], &mut ["Message content is supplied as a json array with a list of pairs in each array element, each pair has a title and a body, translate both title and body separately into English", "Translate each pair in the json array separately", "Summarize each title in less that 15 words in a single sentence", &format!("Summarize each body in less than {word_len} words into a new body"), "Explain body as simply as possible", &format!("The body must be relevant to {prompt}"), &format!("The body must be relevant to the title"), &format!("No body can be longer than {word_len} words"), "Output title and summarized body must be entries in a json array of pairs, each object of the array may come from more than one message, or from multiple elements of the original calling json array", "Always return a json array of pairs of title and body in a single json array"], item).await?;
 
    //let res = res.replace("```json", "").replace("```", "");

    //let i: Vec<GPTItem> = serde_json::from_str(&res).map_err(|e| anyhow::Error::new(e))?;
    //let res: Vec<_> = i.iter().map(|i| (i.title.clone(), i.body.clone())).collect();
//println!("{res}");
    if res.contains(r#""title_summaries\""#) {
        res = res.replace(r#""title_summaries\""#, r#""pairs""#);
    }

    unpack_llm(res)
}

fn unpack_llm(res: String) -> Result<Vec<(String, String)>, Box<dyn std::error::Error + Send>> {
//println!("unpack_llm {}", res.len());
    let mut res = res;

    if res.contains(r#""title_summaries\""#) {
        res = res.replace(r#""title_summaries\""#, r#""pairs""#);
    }

    if res.contains(r#""pairs""#) {
println!("Array returned!");
        let res: GPTItems = serde_json::from_str(&res).map_err(|e| anyhow::Error::new(e))?;

        Ok(res.pairs.iter().map(|i| (i.title.clone(), i.body.clone())).collect::<Vec<(String, String)>>())
    } else {
        let res: GPTItem = serde_json::from_str(&res).map_err(|e| anyhow::Error::new(e))?;

        Ok(vec![(res.title, res.body)])
    }
}

// Expensive!
pub fn clean_html(body: &str) -> String {
    let body = html2text::from_read(BufReader::new(StringReader::new(body)), 10000);
    let body = Regex::new(r"\[.*?\]").unwrap().replace_all(&body, "");
    let body = Regex::new(r"http\S+").unwrap().replace_all(&body, "");
    let body = Regex::new(r"[^\w.,!?]+").unwrap().replace_all(&body, " ");
    let body = truncate_sentence(&body, 5000).to_owned();

    if body.len() < 500 || body.len() > 6000 || shannon_entropy(&body) < 4.0 ||  is_html(&body) {
        println!("Poor Quality: {} Len: {}", shannon_entropy(&body), body.len());

        SUMMARIZE_ERROR.to_owned()
    } else {
        //println!("Quality: {} Len: {}", shannon_entropy(&body), body.len());

        body
    }
}

fn pack_llm(item: &Vec<GPTItem>) -> Result<String, Box<dyn std::error::Error + Send>> {
//println!("pack_llm {} {}", item[0].title, item[0].body.len());
    let req: Vec<GPTItem> = item.iter()
        .map(|i| (&i.title, clean_html(&i.body)))
        .filter(|(_, body)| body != SUMMARIZE_ERROR)
        .map(|(title, body)| GPTItem::new(&title, &body))
        .collect();

    if req.is_empty() {
        Err(anyhow::Error::msg("Poor Quality Body").into())
    } else {
        Ok(serde_json::to_string(&req).map_err(|e| anyhow::Error::new(e))?)
    }
}

pub async fn llm_news_items_with_context(prior: &mut [&str], context: &mut [&str], item: &Vec<GPTItem>) -> Result<String, Box<dyn std::error::Error + Send>> {
    //let item = vec![item[0].clone(), item[0].clone()];
    let start = std::time::Instant::now();

    let req = pack_llm(item)?;
    //let req: String = req.iter().map(|i| i.to_string()).collect();
//println!("{req}");
    let message: Message = Message {
            role: "user".to_string(),
            content: req,
        };
    let mut messages: Vec<Message> = Vec::new();

//println!("{:?}", context);
    context.iter_mut().for_each(|m| messages.push(Message {role: "system".to_string(), content:  m.to_string()}));
    prior.iter_mut().for_each(|m| messages.push(Message {role: "assistant".to_string(), content:  m.to_string()}));

    //messages.push(message.clone());
    //messages.push(message.clone());
    messages.push(message);

    let resp = call_gpt(messages).await;
//println!("{:?}", resp);

    println!("LLM took {:?} for {} entries", start.elapsed(), item.len());

    resp
}
