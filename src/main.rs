use news::aps::news::news;
use serde_derive::Deserialize;
use actix_web::{get, middleware, web, Responder, HttpServer, HttpResponse, App};
use actix_files::NamedFile;
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use std::time::Duration;
use env_logger::Env;
use news::aps::news::TIMEOUT_PERIOD;

#[derive(Deserialize)]
struct FormData {
    prompt: String,
    callback: Option<String>,
    domain: Option<String>,
}

#[get("/favicon.ico")]
async fn favicon() -> impl Responder {
    NamedFile::open_async("favicon.ico").await
}

#[get("/spinner.gif")]
async fn spinner() -> impl Responder {
    NamedFile::open_async("spinner.gif").await
}

#[get("/news")]
async fn index(info: web::Query<std::collections::HashMap<String, String>>) -> impl Responder {
    let form = std::fs::read_to_string("prompt.html");
    match form {
        Ok(form) => {
            let params = info.into_inner();
            let cb = params.get("callback").map_or("", String::as_str);
            let ms = params.get("message").map_or("", String::as_str);
            // Simple templating O(N*n)
            let form = form.replace("${callback}", cb)
                .replace("${message}", ms);

            HttpResponse::Ok().body(form)
        },
        Err(_) => 
            HttpResponse::build(actix_web::http::StatusCode::NOT_FOUND).into()
    }
}

#[get("/gen/{file}")]
async fn gen(info: web::Path<String>) -> impl Responder {
    let file = info.into_inner();
    let parts: Vec<&str> = file.split(['.']).collect();
    if parts.len() == 2 {
        let call = &process_news(&parts[0].replace("_", " ").to_lowercase(), &None, &None).await;

        if call == "Not Found" {
            NamedFile::open_async(&format!("gen/{file}")).await
        } else {
            NamedFile::open_async(call).await
        }
    } else {
        let file = urlencoding::decode(&file).unwrap().replace(" ", "_");

        NamedFile::open_async(&format!("gen/{file}")).await
    }
}

async fn process_form(form: web::Form<FormData>) -> impl Responder {
    let cb = process_news(&form.prompt.to_lowercase(), &form.callback, &form.domain).await;

    let cb = 
        if cb == "Not Found" {
            match &form.callback {
                Some(callback) => format!("/news?callback={}&message=Nothing found for {}, try again.", callback, form.prompt),
                None => cb,
            }
        } else {
            cb
        };

    web::Redirect::to(cb).see_other()
}

async fn process_news(prompt: &str, callback: &Option<String>, domain: &Option<String>) -> String {
    let start = std::time::Instant::now();
    // Process the form data here
    match news(prompt).await {
        Ok(file) => {
            println!("{} took {:?}", prompt, start.elapsed());
            match &callback {
                Some(cb) if !cb.is_empty() => {
                    match &domain {
                        Some(dom) => {
                            let file_url = format!("{dom}/{}", file); 
                            let cb = format!("{cb}{}url={}",
                                if cb.contains("?") { "&" } else { "?" },
                                urlencoding::encode(&file_url));

                            cb
                        },
                        None => file,
                    }
                },
                Some(_) | None => file,
            }
        },
        Err(e) => {
            eprintln!("{e}");
            "Not Found".into()
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(Env::default().default_filter_or("info"));

    let mut ssl_builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
    ssl_builder.set_private_key_file("key.pem", SslFiletype::PEM).unwrap();
    ssl_builder.set_certificate_chain_file("cert.pem").unwrap();

    HttpServer::new(|| {
        App::new()
            .wrap(middleware::DefaultHeaders::new().add(("Cache-Control", "public, max-age=10800")))
            .wrap(middleware::Logger::default())
            .wrap(middleware::Logger::new("%a %{User-Agent}i"))
            .service(favicon)
            .service(spinner)
            .service(index)
            .route("/submit", web::post().to(process_form))
            .service(gen)
    })
    .keep_alive(Duration::from_secs(60 * TIMEOUT_PERIOD as u64))
    .bind("0.0.0.0:8080")?
    //.bind_openssl("0.0.0.0:443", ssl_builder)?
    //.bind("0.0.0.0:80")?
    .run()
    .await
}
