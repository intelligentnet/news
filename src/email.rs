use std::env::var;
use std::error::Error;
use is_html::is_html;
use log::error;
use lettre::{message::header::ContentType, transport::smtp::authentication::Credentials, Message, SmtpTransport, Transport};

pub fn new(from: &str, to: &str, reply_to: &str, subject: &str, body: &str) -> Result<Message, Box<dyn Error>> {
    let content = Message::builder()
        .from(from.parse()?)
        .to(to.parse()?)
        .reply_to(reply_to.parse()?)
        .subject(subject)
        .header(if is_html(body) {
            ContentType::TEXT_HTML
        } else {
            ContentType::TEXT_PLAIN
        })
        .body(String::from(body))?;

    Ok(content)
}

pub fn send(content: &Message) -> Result<bool, Box<dyn Error>> {
    let pg_host: String = var("EMAIL_HOST").or(Err("Set EMAIL_HOST"))?;
    let pg_user: String = var("EMAIL_USER").or(Err("Set EMAIL_USER"))?;
    let pg_pass: String = var("EMAIL_PASS").or(Err("Set EMAIL_PASS"))?;


    let creds = Credentials::new(pg_user.to_owned(), pg_pass.to_owned());

    // Open a remote connection
    let mailer = SmtpTransport::relay(&pg_host)?
        .credentials(creds)
        .build();

    // Send the email
    match mailer.send(content) {
        Ok(_) => Ok(true),
        Err(e) => {
            error!("Could not send email: {e:?}");

            Ok(false)
        }
    }
}
