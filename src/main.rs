use news::aps::news::{news, tale, detail_tale, language, image};
use serde_derive::Deserialize;
use actix_web::{get, middleware, web, Responder, HttpServer, HttpResponse, App};
use actix_files::NamedFile;
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use std::time::Duration;
use env_logger::Env;
use news::aps::news::TIMEOUT_PERIOD;

#[derive(Deserialize, Debug)]
struct FormData {
    prompt: String,
    callback: Option<String>,
    domain: Option<String>,
    system: String,
}

#[get("/news.json")]
async fn service_builder_news() -> impl Responder {
    NamedFile::open_async("news.json").await
}

#[get("/image.json")]
async fn service_builder_image() -> impl Responder {
    NamedFile::open_async("image.json").await
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
async fn news_index(info: web::Query<std::collections::HashMap<String, String>>) -> impl Responder {
    match std::fs::read_to_string("news.html") {
        Ok(form) => {
            HttpResponse::Ok().body(index_common(form, info).await)
        },
        Err(_) => 
            HttpResponse::build(actix_web::http::StatusCode::NOT_FOUND).into()
    }
}

#[get("/picture")]
async fn picture_index(info: web::Query<std::collections::HashMap<String, String>>) -> impl Responder {
    match std::fs::read_to_string("picture.html") {
        Ok(form) => {
            HttpResponse::Ok().body(index_common(form, info).await)
        },
        Err(_) => 
            HttpResponse::build(actix_web::http::StatusCode::NOT_FOUND).into()
    }
}

async fn index_common(form: String, info: web::Query<std::collections::HashMap<String, String>>) -> String {
    let params = info.into_inner();
    let cb = params.get("callback").map_or("", String::as_str);
    let ms = params.get("message").map_or("", String::as_str);
 
    // Simple templating O(N*n)
    form.replace("${callback}", cb).replace("${message}", ms)
}

#[get("/ind")]
async fn index() -> impl Responder {
    let form = std::fs::read_to_string("index.html");

    if let Ok(form) = form {
        HttpResponse::Ok().body(form)
    } else {
        HttpResponse::build(actix_web::http::StatusCode::NOT_FOUND).into()
    }
}

#[get("/image")]
async fn image_index(info: web::Query<std::collections::HashMap<String, String>>) -> impl Responder {
    let form = std::fs::read_to_string("image.html");
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

#[get("/tale")]
async fn tale_index(info: web::Query<std::collections::HashMap<String, String>>) -> impl Responder {
    let form = std::fs::read_to_string("tale.html");
    match form {
        Ok(form) => {
            let params = info.into_inner();
            let ms = params.get("message").map_or("", String::as_str);
            // Simple templating O(N*n)
            let form = form.replace("${message}", ms);

            HttpResponse::Ok().body(form)
        },
        Err(_) => 
            HttpResponse::build(actix_web::http::StatusCode::NOT_FOUND).into()
    }
}

#[get("/rust")]
async fn rust_index(info: web::Query<std::collections::HashMap<String, String>>) -> impl Responder {
    let form = std::fs::read_to_string("rust.html");
    match form {
        Ok(form) => {
            let params = info.into_inner();
            let ms = params.get("message").map_or("", String::as_str);
            // Simple templating O(N*n)
            let form = form.replace("${message}", ms);

            HttpResponse::Ok().body(form)
        },
        Err(_) => 
            HttpResponse::build(actix_web::http::StatusCode::NOT_FOUND).into()
    }
}

#[get("/python")]
async fn python_index(info: web::Query<std::collections::HashMap<String, String>>) -> impl Responder {
    let form = std::fs::read_to_string("python.html");
    match form {
        Ok(form) => {
            let params = info.into_inner();
            let ms = params.get("message").map_or("", String::as_str);
            // Simple templating O(N*n)
            let form = form.replace("${message}", ms);

            HttpResponse::Ok().body(form)
        },
        Err(_) => 
            HttpResponse::build(actix_web::http::StatusCode::NOT_FOUND).into()
    }
}

#[get("/java")]
async fn java_index(info: web::Query<std::collections::HashMap<String, String>>) -> impl Responder {
    let form = std::fs::read_to_string("java.html");
    match form {
        Ok(form) => {
            let params = info.into_inner();
            let ms = params.get("message").map_or("", String::as_str);
            // Simple templating O(N*n)
            let form = form.replace("${message}", ms);

            HttpResponse::Ok().body(form)
        },
        Err(_) => 
            HttpResponse::build(actix_web::http::StatusCode::NOT_FOUND).into()
    }
}

#[get("/html")]
async fn html_index(info: web::Query<std::collections::HashMap<String, String>>) -> impl Responder {
    let form = std::fs::read_to_string("html.html");
    match form {
        Ok(form) => {
            let params = info.into_inner();
            let ms = params.get("message").map_or("", String::as_str);
            // Simple templating O(N*n)
            let form = form.replace("${message}", ms);

            HttpResponse::Ok().body(form)
        },
        Err(_) => 
            HttpResponse::build(actix_web::http::StatusCode::NOT_FOUND).into()
    }
}

/*
#[get("/bench")]
async fn bench(info: web::Query<std::collections::HashMap<String, u32>>) -> impl Responder {
    let params = info.into_inner();
    let seq = params.get("seq").map_or(0, |&i| i as u32);
    let secs = params.get("secs").map_or(10, |&i| i as u32);

    std::thread::sleep(std::time::Duration::from_secs(secs.into()));

    HttpResponse::Ok().body(format!("{seq} = {secs}"))
}

#[derive(Deserialize)]
struct BenchData {
    seq: u32,
    secs: u32,
}

// HttpResponse::BadRequest().body("Oh Bother")

async fn post_bench(form: web::Json<BenchData>) -> impl Responder {
    let seq = form.seq;
    let secs = form.secs;

    std::thread::sleep(std::time::Duration::from_secs(secs.into()));

    HttpResponse::Ok().body(format!("{seq} = {secs}"))
}
*/

#[get("/gen/{file}")]
async fn gen(info: web::Path<String>) -> impl Responder {
    let file = gen_pic_common("news", info).await;

    NamedFile::open_async(file).await
}

#[get("/pic/{file}")]
async fn pic(info: web::Path<String>) -> impl Responder {
    let file = gen_pic_common("picture", info).await;

    NamedFile::open_async(file).await
}

async fn gen_pic_common(format: &str, info: web::Path<String>) -> String {
    let dir = if format == "picture" { "pic" } else { "gen" };
    let file = info.into_inner();
    let parts: Vec<&str> = file.split('.').collect();

    if parts.len() == 2 {
        if parts[1] == "png" {
            let call = process_news(&parts[0].replace('_', " ").to_lowercase(), &None, &None, format, false).await;

            if call == "Not Found" {
                format!("{dir}/{file}")
            } else {
                call
            }
        } else {
            format!("{dir}/{file}")
        }
    } else {
        let file = urlencoding::decode(&file).unwrap().replace(' ', "_");

        format!("{dir}/{file}")
    }
}

async fn proc_news(form: web::Form<FormData>) -> impl Responder {
    let cb = process_news_pic("news", form).await;

    web::Redirect::to(cb).see_other()
}

async fn proc_pic(form: web::Form<FormData>) -> impl Responder {
    let cb = process_news_pic("picture", form).await;

    web::Redirect::to(cb).see_other()
}

async fn process_news_pic(fmt: &str, form: web::Form<FormData>) -> String {
    let prompt = (if form.prompt.ends_with('.') {
        form.prompt.strip_suffix('.').unwrap()
    } else {
        &form.prompt
    }).to_lowercase();
    let cb = process_news(&prompt, &form.callback, &form.domain, fmt, true).await;

    if cb == "Not Found" {
        match &form.callback {
            Some(callback) => format!("/news?callback={}&message=Nothing found for {}, try again.", callback, form.prompt),
            None => cb.to_string(),
        }
    } else {
        cb.to_string()
    }
}

async fn process_news(prompt: &str, callback: &Option<String>, domain: &Option<String>, fmt: &str, initial: bool) -> String {
    let start = std::time::Instant::now();
    // Process the form data here
    match news(&prompt.to_lowercase(), fmt, initial).await {
        Ok(file) => {
            println!("{} took {:?}", prompt, start.elapsed());
            match &callback {
                Some(cb) if !cb.is_empty() => {
                    match &domain {
                        Some(dom) => {
                            let file_url = format!("{dom}/{}", file); 

                            format!("{cb}{}url={}",
                                if cb.contains('?') { "&" } else { "?" },
                                urlencoding::encode(&file_url))
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

async fn process_image(form: web::Form<FormData>) -> impl Responder {
    let prompt = if form.prompt.ends_with('.') {
        form.prompt.strip_suffix('.').unwrap()
    } else {
        &form.prompt
    };
    let call =
        match image(&prompt.to_lowercase(), &form.system).await {
            Ok(file) => file,
            Err(e) => {
                eprintln!("{e}");
                "Not Found".into()
            }
        };

    NamedFile::open_async(call).await
}

async fn process_tale(form: web::Form<FormData>) -> impl Responder {
    let prompt = if form.prompt.ends_with('.') {
        form.prompt.strip_suffix('.').unwrap()
    } else {
        &form.prompt
    };
    let call =
        match tale(&prompt.to_lowercase(), &form.system).await {
            Ok(file) => file,
            Err(e) => {
                eprintln!("{e}");
                "Not Found".into()
            }
        };

    NamedFile::open_async(call).await
}

async fn process_rust(form: web::Form<FormData>) -> impl Responder {
    let call = process_lang("rust", form).await;

    NamedFile::open_async(call).await
}

async fn process_python(form: web::Form<FormData>) -> impl Responder {
    let call = process_lang("python", form).await;

    NamedFile::open_async(call).await
}

async fn process_java(form: web::Form<FormData>) -> impl Responder {
    let call = process_lang("java", form).await;

    NamedFile::open_async(call).await
}

async fn process_html(form: web::Form<FormData>) -> impl Responder {
    let call = process_lang("html", form).await;

    NamedFile::open_async(call).await
}

async fn process_lang(lang: &str, form: web::Form<FormData>) -> String {
    let prompt = if form.prompt.ends_with('.') {
        form.prompt.strip_suffix('.').unwrap()
    } else {
        &form.prompt
    };

    match language(lang, &prompt.to_lowercase(), &form.system).await {
        Ok(file) => file,
        Err(e) => {
            eprintln!("{e}");
            "Not Found".into()
        }
    }
}

async fn tale_detail(info: web::Form<std::collections::BTreeMap<String, String>>) -> impl Responder {
    let title = info.get("title").map_or("Story", String::as_str);
    println!("{title}");
    let items: Vec<(String, String, String)> = info.iter()
        //.map(|(_, v)| v.split([':', '\n']).collect::<Vec<&str>>())
        .filter(|(&ref t, _)| t != "title")
        .map(|(_, v)| v.split([':', '-', '\n']).collect::<Vec<&str>>())
        .map(|bits| {
            (bits[0].trim().to_string(), bits[1].trim().to_string(), bits[bits.len() - 1].trim().to_string())
        }).collect();

    let call =
        match detail_tale(title, &items).await {
            Ok(file) => file,
            Err(e) => {
                eprintln!("{e}");
                "Not Found".into()
            }
        };

    NamedFile::open_async(call).await
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(Env::default().default_filter_or("info"));

    let mut ssl_builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
    ssl_builder.set_private_key_file("key.pem", SslFiletype::PEM).unwrap();
    ssl_builder.set_certificate_chain_file("cert.pem").unwrap();

    let server = HttpServer::new(|| {
        App::new()
            .wrap(middleware::DefaultHeaders::new().add(("Cache-Control", "public, max-age=10800")))
            .wrap(middleware::Logger::default())
            .wrap(middleware::Logger::new("%a %{User-Agent}i"))
            .service(service_builder_news)
            .service(service_builder_image)
            .service(favicon)
            .service(spinner)
            .service(index)
            .service(news_index)
            .service(picture_index)
            .service(image_index)
            .service(tale_index)
            .service(rust_index)
            .service(python_index)
            .service(java_index)
            .service(html_index)
            .route("/submit_news", web::post().to(proc_news))
            .route("/submit_pic", web::post().to(proc_pic))
            .route("/image", web::post().to(process_image))
            .route("/tale", web::post().to(process_tale))
            .route("/taledetail", web::post().to(tale_detail))
            .route("/rust", web::post().to(process_rust))
            .route("/python", web::post().to(process_python))
            .route("/java", web::post().to(process_java))
            .route("/html", web::post().to(process_html))
            .service(gen)
            .service(pic)
            //.service(bench)
            //.route("/post", web::post().to(post_bench))
        })
        .keep_alive(Duration::from_secs(60 * TIMEOUT_PERIOD as u64));

    match std::env::var("LIVE") {
        Ok(val) if val == "true" =>  {
            server.bind_openssl("0.0.0.0:443", ssl_builder)?
                  .bind("0.0.0.0:80")?
        },
        Ok(_) | Err(_) =>
            server.bind("0.0.0.0:8080")?
    }
    .run()
    .await
}
