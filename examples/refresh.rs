use env_logger::Env;
use regex::Regex;
use std::fs;
use std::env;
use news::refresh::refresh;
use news::aps::news::how_long_since_created;

const RETENTION_DAYS: u32 = 3;

#[tokio::main]
async fn main() {
    env_logger::init_from_env(Env::default().default_filter_or("info"));

    let args: Vec<String> = env::args().collect();
    if args.len() == 1 { panic!("format: refresh <prompt> [<target>]"); };
    let prompt = if args.len() >= 2 { &args[1] } else { "news" };
    let target = if args.len() >= 3 { &args[2] } else { "*" };

    // Remove old thumbnails
    let re = Regex::new(r#"\d{18}.png"#).unwrap();
    let old = (RETENTION_DAYS * 24 * 60 * 60) as u64;

    if let Ok(entries) = fs::read_dir("gen") {
        for entry in entries {
            if let Ok(entry) = entry {
                let file = entry.path();
                let file = file.to_str().unwrap();

                if re.is_match(&file) && how_long_since_created(&file) > old {
                    fs::remove_file(&file).unwrap();
                }
            }
        }
    }

    refresh(prompt, target).await;
}
