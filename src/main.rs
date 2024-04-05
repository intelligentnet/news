use news::aps::news::{news, tale, detail_tale, language, image};
use serde_derive::Deserialize;
use actix_web::{get, middleware, web, Responder, HttpServer, HttpRequest, HttpResponse, App};
use actix_files::NamedFile;
//use actix_identity::Identity;
use actix_ip_filter::IPFilter;
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use std::time::Duration;
use news::aps::news::TIMEOUT_PERIOD;
use stemplate::Template;
use std::collections::BTreeMap;
use env_logger::Env;
use log::{warn, info};

#[derive(Deserialize, Debug)]
struct FormData {
    prompt: String,
    callback: Option<String>,
    domain: Option<String>,
    system: String,
    chapters: Option<String>,
}

#[get("/news.json")]
async fn service_builder_news() -> impl Responder {
    NamedFile::open_async("news.json").await
}

#[get("/image.json")]
async fn service_builder_image() -> impl Responder {
    NamedFile::open_async("image.json").await
}

#[get("/picture.json")]
async fn service_builder_picture() -> impl Responder {
    NamedFile::open_async("picture.json").await
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
async fn news_index(info: web::Query<std::collections::HashMap<String, String>>, req: HttpRequest) -> impl Responder {
    warn!("news: {:?} {:?}", req.headers().get("referer"), req.headers().get("host"));
    match std::fs::read_to_string("news.html") {
        Ok(form) => {
            HttpResponse::Ok().body(index_common(form, info))
        },
        Err(_) => 
            HttpResponse::build(actix_web::http::StatusCode::NOT_FOUND).into()
    }
}

#[get("/picture")]
async fn picture_index(info: web::Query<std::collections::HashMap<String, String>>) -> impl Responder {
    match std::fs::read_to_string("picture.html") {
        Ok(form) => {
            HttpResponse::Ok().body(index_common(form, info))
        },
        Err(_) => 
            HttpResponse::build(actix_web::http::StatusCode::NOT_FOUND).into()
    }
}

fn index_common(form: String, info: web::Query<std::collections::HashMap<String, String>>) -> String {
    Template::new(&form).render_strings(&info.into_inner())
}

#[get("/ind")]
//#[cfg(allow)]
async fn index() -> impl Responder {
    let form = std::fs::read_to_string("index.html");

    if let Ok(form) = form {
        HttpResponse::Ok().body(form)
    } else {
        HttpResponse::build(actix_web::http::StatusCode::NOT_FOUND).into()
    }
}

/*
#[get("/ind")]
#[cfg(not(allow))]
async fn index() -> impl Responder {
    HttpResponse::build(actix_web::http::StatusCode::NOT_FOUND)
}
*/

#[get("/image")]
async fn image_index(info: web::Query<std::collections::HashMap<String, String>>) -> impl Responder {
    let form = std::fs::read_to_string("image.html");
    match form {
        Ok(form) => {
            HttpResponse::Ok().body(index_common(form, info))
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
            HttpResponse::Ok().body(index_common(form, info))
        },
        Err(_) => 
            HttpResponse::build(actix_web::http::StatusCode::NOT_FOUND).into()
    }
}

#[get("/code")]
async fn code_index(info: web::Query<std::collections::HashMap<String, String>>) -> impl Responder {
    let form = std::fs::read_to_string("code.html");
    match form {
        Ok(form) => {
            HttpResponse::Ok().body(index_common(form, info))
        },
        Err(_) => 
            HttpResponse::build(actix_web::http::StatusCode::NOT_FOUND).into()
    }
}

#[get("/chat")]
async fn chat_index(info: web::Query<std::collections::HashMap<String, String>>) -> impl Responder {
warn!("chat_index");
    let form = std::fs::read_to_string("chat.html");
    match form {
        Ok(form) => {
            HttpResponse::Ok().body(index_common(form, info))
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
//async fn gen(info: web::Path<String>, req: HttpRequest) -> impl Responder {
async fn gen(info: web::Path<String>) -> impl Responder {
    //warn!("gen: {:?} {:?}", req.headers().get("referer"), req.headers().get("host"));
//    #[cfg(allow)]
    let file = gen_pic_common("news", info).await;
//    #[cfg(not(allow))]
//    let file = &format!("gen/{}", info.into_inner());

    NamedFile::open_async(file).await
}

#[get("/pic/{file}")]
async fn pic(info: web::Path<String>) -> impl Responder {
//    #[cfg(allow)]
    let file = gen_pic_common("picture", info).await;
//    #[cfg(not(allow))]
//    let file = &format!("gen/{}", info.into_inner());

    NamedFile::open_async(file).await
}

async fn gen_pic_common(format: &str, info: web::Path<String>) -> String {
    let file = info.into_inner();

    if file == "not_available.png" {
        return format!("gen/{file}");
    }

    let parts: Vec<&str> = file.split('.').collect();

    if parts.len() == 2 {
        if parts[1] == "png" {
            let call = process_news(&parts[0].replace('_', " ").to_lowercase(), &None, &None, format, false).await;

            if call == "Not Found" {
                format!("gen/{file}")
            } else {
                call
            }
        } else {
            format!("gen/{file}")
        }
    } else {
        let file = urlencoding::decode(&file).unwrap().replace(' ', "_");

        format!("gen/{file}")
    }
}

#[get("/image/{file}")]
async fn image_get(info: web::Path<String>) -> impl Responder {
    let file = format!("gen/{}", info.into_inner());

    if std::path::Path::new(&file).exists() {
        NamedFile::open_async(file).await
    } else {
        NamedFile::open_async("gen/not_available.png").await
    }
}

async fn proc_news(form: web::Form<FormData>, req: HttpRequest) -> impl Responder {
    warn!("proc_news: {:?} {:?}", req.headers().get("referer"), req.headers().get("host"));
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
            Some(callback) => {
                format!("/news?callback={}&message=Nothing found for {}, try again.",
                        urlencoding::encode(callback),
                        form.prompt)
            },
            None => cb.to_string(),
        }
    } else {
        cb.to_string()
    }
}

async fn process_news(prompt: &str, callback: &Option<String>, domain: &Option<String>, fmt: &str, initial: bool) -> String {
    let start = std::time::Instant::now();
    // Process the form data here
    let (call, file) =
        match news(&prompt.to_lowercase(), fmt, initial).await {
            Ok(file) => {
                info!("{} took {:?}", prompt, start.elapsed());
                match &callback {
                    Some(cb) if !cb.is_empty() => {
                        match &domain {
                            Some(dom) => {
                                let file_url = format!("{dom}/{}", file); 

                                (format!("{cb}{}url={}",
                                    if cb.contains('?') { "&" } else { "?" },
                                    urlencoding::encode(&file_url))
                                 , file
                                )
                            },
                            None => (file.clone(), file),
                        }
                    },
                    Some(_) | None => (file.clone(), file),
                }
            },
            Err(e) => {
                warn!("{prompt} redirect error: {e}");
                ("".into(), "Not Found".into())
            }
        };

    if std::path::Path::new(&file.replace("pic/", "gen/")).exists() {
        if call.starts_with("http") {
            info!("calling back: {file}");
            call
        } else {
            info!("{file} for {prompt}: found");
            file
        }
    } else {
        warn!("{file}: file does not exist");
        "Not Found".into()
    }
}

/*
async fn process_image(form: web::Form<FormData>) -> impl Responder {
    let start = std::time::Instant::now();
    let prompt = if form.prompt.ends_with('.') {
        form.prompt.strip_suffix('.').unwrap()
    } else {
        &form.prompt
    };

    let file =
        match image(&prompt.to_lowercase(), &form.system).await {
            Ok(file) => {
                info!("{} took {:?}", prompt, start.elapsed());
                file
            },
            Err(e) => {
                warn!("{prompt} image gen failed:: {e}");
                "image/not_available.png".into()
            }
        };


    info!("Image {file} for {prompt}: found");
    web::Redirect::to(file.replace("gen/", "image/")).see_other()
}
*/
async fn process_image(form: web::Form<FormData>) -> impl Responder {
    let start = std::time::Instant::now();
    let prompt = if form.prompt.ends_with('.') {
        form.prompt.strip_suffix('.').unwrap()
    } else {
        &form.prompt
    };
    let callback = &form.callback;
    let domain = &form.domain;

    let (call, file) =
        match image(&prompt.to_lowercase(), &form.system).await {
            Ok(file) => {
                info!("{} took {:?}", prompt, start.elapsed());
                match &callback {
                    Some(cb) if !cb.is_empty() => {
                        match &domain {
                            Some(dom) => {
                                let file_url = format!("{dom}/{file}").replace("/gen/", "/image/"); 

                                (format!("{cb}{}url={}",
                                    if cb.contains('?') { "&" } else { "?" },
                                    urlencoding::encode(&file_url))
                                 , file
                                )
                            },
                            None => (file.clone(), file),
                        }
                    },
                    Some(_) | None => (file.clone(), file),
                }
            },
            Err(e) => {
                warn!("{prompt} image gen failed:: {e}");
                ("".into(), "/image/not_available.png".into())
            }
        };


    if std::path::Path::new(&file.replace("/gen/", "gen/")).exists() {
        if call.starts_with("http") {
            info!("calling back image: {}", file.replace("/gen/", "/image/"));
            web::Redirect::to(call).see_other()
        } else {
            info!("Image {file} for {prompt}: found");
            web::Redirect::to(file.replace("gen/", "image/")).see_other()
        }
    } else {
        /*
        warn!("Image {}: file does not exist", file);
        let cb = match callback {
            Some(callback) => {
                format!("/image?callback={}&message=Nothing found for {}, try again.",
                    urlencoding::encode(callback),
                    prompt)
            },
            None => {
                format!("/image?message=Nothing found for {}, try again.",
                        prompt)
            }
        };
        */
        web::Redirect::to(file).see_other()
    }
}


async fn process_tale(form: web::Form<FormData>) -> impl Responder {
    let prompt = if form.prompt.ends_with('.') {
        form.prompt.strip_suffix('.').unwrap()
    } else {
        &form.prompt
    };
    let chapters: usize = match &form.chapters {
        //Some(n) => match n.parse::<usize>() { Ok(n) => n, Err(_) => 5 },
        Some(n) => n.parse::<usize>().unwrap_or(5),
        None => 5
    };
    let call =
        match tale(&prompt.to_lowercase(), &form.system, chapters).await {
            Ok(file) => file,
            Err(e) => {
                warn!("{e}");
                "Not Found".into()
            }
        };

    NamedFile::open_async(call).await
}

async fn process_chat(form: web::Form<FormData>) -> impl Responder {
warn!("process_chat");
    NamedFile::open_async("chat.html").await
}

async fn process_code(form: web::Form<FormData>) -> impl Responder {
    let call = match form.domain.clone() {
        Some(dom) => process_lang(&dom, form).await,
        None => process_lang("rust", form).await,
    };

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
            warn!("{e}");
            "Not Found".into()
        }
    }
}

/*
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
*/
async fn tale_detail(info: web::Form<std::collections::BTreeMap<String, String>>) -> impl Responder {
    let title = info.get("title").map_or("Story", String::as_str);
    let ord: BTreeMap<String, String> = info.iter()
        .filter(|(n, _)| *n != "title")
        .map(|(n, v)| (n.into(), v.into()))
        .collect();
    let ns: Vec<(String, String)> = ord.iter()
        .filter(|(n, _)| n.starts_with("name"))
        .map(|(_, v)| v)
        .zip(ord.iter()
            .filter(|(n, _)| n.starts_with("summary"))
            .map(|(_, v)| v))
        .map(|(n, s)| (n.into(), s.into()))
        .collect();

    let call =
        match detail_tale(title, &ns).await {
            Ok(file) => file,
            Err(e) => {
                warn!("{e}");
                "Not Found".into()
            }
        };

    NamedFile::open_async(call).await
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(Env::default().default_filter_or("info"));

    let server = HttpServer::new(|| {
        App::new()
            .wrap(middleware::DefaultHeaders::new().add(("Cache-Control", "public, max-age=10800")))
            .wrap(middleware::Logger::default())
            .wrap(middleware::Logger::new("%a %{User-Agent}i"))
            .wrap(IPFilter::new().block(vec!["116.103.228.17,14.103.19.135,146.19.24.28","213.222.246.148","45.95.147.236","51.89.139.31","52.15.118.168","83.97.73.245"]))
            /*
            .wrap(IPFilter::new().allow(match std::env::var("IP_ALLOW") {
                Ok(allow) => allow,
                Err(_) => "127.0.0.1".into()
            }.split(",").collect()))
            */
            .service(service_builder_news)
            .service(service_builder_image)
            .service(service_builder_picture)
            .service(index)
            .service(news_index)
            .service(picture_index)
            .service(image_index)
            .service(tale_index)
            .service(code_index)
            .service(chat_index)
            .route("/submit_news", web::post().to(proc_news))
            .route("/submit_pic", web::post().to(proc_pic))
            .route("/image", web::post().to(process_image))
            .route("/tale", web::post().to(process_tale))
            .route("/taledetail", web::post().to(tale_detail))
            .route("/code", web::post().to(process_code))
            .route("/chat", web::post().to(process_chat))
            .service(favicon)
            .service(spinner)
            .service(gen)
            .service(pic)
            .service(image_get)
            //.service(bench)
            //.route("/post", web::post().to(post_bench))
        })
        .keep_alive(Duration::from_secs(60 * TIMEOUT_PERIOD as u64));

    //println!("{}", std::any::type_name_of_val(&server));
    match std::env::var("LIVE") {
        Ok(val) if val == "true" =>  {
            let mut ssl_builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
            ssl_builder.set_private_key_file("key.pem", SslFiletype::PEM).unwrap();
            ssl_builder.set_certificate_chain_file("cert.pem").unwrap();

            server.bind_openssl("0.0.0.0:443", ssl_builder)?
                  .bind("0.0.0.0:80")?
        },
        Ok(_) | Err(_) =>
            match std::env::var("TEST_PORT") {
                Ok(socket) => server.bind(&format!("0.0.0.0:{}", socket))?,
                Err(_) => server.bind("0.0.0.0:8080")?,
            }
    }
    .run()
    .await
}
