use std::fs::File;
use std::io::Write;
use std::process::Command;
use std::env::var;
use is_html::is_html;
use tempfile::tempfile;
use filepath::FilePath;
use std::error::Error;

pub fn new(from: &str, to: &str, reply_to: &str, subject: &str, body: &str) -> Result<String, Box<dyn Error>> {
    let from = format!("From:{from}");
    let to = format!("To:{to}");
    let reply_to = format!("Reply-To:{reply_to}");
    let subject = format!("Subject:{subject}");
    let content_type = if is_html(body) {
        "Content-Type: text/html"
    } else {
        "Content-Type: text/plain"
    };
    let header = format!("{from}\n{to}\n{reply_to}\n{subject}\n{content_type}\n");

    Ok(format!("{header}\n\n{body}\n"))
}

pub fn send(content: &str) -> Result<bool, Box<dyn Error>> {
    let to: String = var("EMAIL_USER").or(Err("Set EMAIL_USER"))?;
    let file = tempfile()?;

    let path = file.path()?.display().to_string();

    let mut file = File::create(&path)?;

    // Write the 'text' to file
    file.write_all(content.as_bytes())?;

    // Send Email using sendmail 
    let sendmail_sh = format!("sendmail {to} < '{path}'");
    let output = Command::new("sh")
                    .arg("-c")
                    .arg(sendmail_sh)
                    .output()?;

    std::fs::remove_file(&path)?;

    Ok(output.status.success())
}
