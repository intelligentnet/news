use html2text;
use std::io::BufReader;
use stringreader::StringReader;
use crate::apis::openai::{Message, call_gpt};
use crate::image::render::PAGE_TOTAL;

pub const SUMMARIZE_ERROR: &str = "CANNOT SUMMARIZE";

pub fn truncate(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        None => s,
        Some((idx, _)) => &s[..idx],
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
    llm_news_items_with_context(&mut [], &mut ["Show Rust code without explanation"], req).await
}

pub async fn llm_title(req: &str) -> Result<String, Box<dyn std::error::Error + Send>> {
    llm_news_items_with_context(&mut [], &mut ["Summarize in English", "Summarize in less then 15 words", "Summarize in 1 paragraph.", &format!("If cannot summarize say '{SUMMARIZE_ERROR}' only")], req).await
}

pub async fn llm_news_items(prompt: &str, title: &str, req: &str, its: u32) -> Result<String, Box<dyn std::error::Error + Send>> {
    let sum_len = (50 * PAGE_TOTAL) / its;
    let word_len = sum_len / 10;
    llm_news_items_with_context(&mut [], &mut [&format!("Summarize with less than {sum_len} words"), "Explain as simply as possible", "Only process if written in English", &format!("Summary must explain {prompt}"), &format!("Summary must be relevant to: {title}"), &format!("No sentences longer than {word_len} words"), &format!("If cannot summarize say '{SUMMARIZE_ERROR}' only")], req).await
}

pub async fn llm_news_items_with_context(prior: &mut [&str], context: &mut [&str], req: &str) -> Result<String, Box<dyn std::error::Error + Send>> {
    let req = html2text::from_read(BufReader::new(StringReader::new(req)), 1000);
    let message: Message = Message {
            role: "user".to_string(),
            content: truncate(&req, 5000).to_string(),
        };
    let mut messages: Vec<Message> = Vec::new();

    prior.iter_mut().for_each(|m| messages.push(Message {role: "system".to_string(), content:  m.to_string()}));
    //prior.iter_mut().for_each(|m| messages.push(Message {role: "assistant".to_string(), content:  m.to_string()}));
    context.iter_mut().for_each(|m| messages.push(Message {role: "user".to_string(), content:  m.to_string()}));

    messages.push(message);

    call_gpt(messages).await
}
