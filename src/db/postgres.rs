use tokio_postgres::{Client, Row};
use pgvector::Vector;
use fastembed::{FlagEmbedding, EmbeddingBase};
use chrono::{DateTime, Utc, TimeZone, Duration};
use std::error::Error;
use std::env::var;

pub fn connect_string() -> Result<String, Box<dyn Error>> {
    let pg_host: String = var("PG_HOST").or(Err("Set PG_HOST"))?;
    let pg_db: String = var("PG_DB").or(Err("Set PG_DB"))?;
    let pg_user: String = var("PG_USER").or(Err("Set PG_USER"))?;
    let pg_pass: String = var("PG_PASS").or(Err("Set PG_PASS"))?;

    Ok(format!("host={} dbname={} user={} password={}", pg_host, pg_db, pg_user, pg_pass))
}

#[derive(Debug, Clone, PartialEq)]
pub struct DbNewsItem<Utc: TimeZone> {
    pub url: String,
    pub prompt: String,
    pub source: String,
    pub title: String,
    pub summary: Option<String>,
    pub queried: bool,
    pub dt: DateTime<Utc>,
    pub sentiment: f32,
    pub embedding: Option<Vec<f32>>,
}

impl DbNewsItem<Utc> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(url: &str, prompt: &str, source: &str, title: &str, dt: DateTime<Utc>, summary: &Option<String>, queried: bool, sentiment: f32, embedding: Option<Vec<f32>>) -> Self {
        DbNewsItem {
            url: url.into(),
            prompt: prompt.into(),
            source: source.into(),
            title: title.into(),
            summary: summary.clone(),
            queried,
            dt,
            sentiment,
            embedding
        }
    }
}

pub async fn get_rows(rows: &Vec<Row>) -> Vec<DbNewsItem<Utc>> {
    let mut rs = Vec::new();
    for row in rows {
        let url: &str = row.get(0);
        let prompt: &str = row.get(1);
        let source: &str = row.get(2);
        let title: &str = row.get(3);
        let summary: Option<&str> = row.get(4);
        let summary: Option<String> = summary.map(|s| s.into());
        let queried: bool = row.get(5);
        let dt: DateTime<Utc> = row.get::<usize, DateTime<Utc>>(6);
        let sentiment: f32 = row.get(7);
        let pgvec: Option<Vector> = row.get(8);
        let embedding = pgvec.map(|pgvec| pgvec.into());

        rs.push(DbNewsItem {url: url.into(), prompt: prompt.into(), source: source.into(), title: title.into(), summary, queried, dt, sentiment, embedding});
    }

    rs
}

pub async fn add_news(client: &mut Client, input: &[DbNewsItem<Utc>]) -> Result<(), Box<dyn Error>> {
    for (count, i) in input.iter().enumerate() {
        add_news_item(client, i, count as u32).await?;
    }
    
    Ok(())
}

pub async fn add_news_item(client: &mut Client, i: &DbNewsItem<Utc>, count: u32) -> Result<(), Box<dyn Error>> {
    let embedding = i.embedding.as_ref().map(|e| Vector::from(e.clone()));

    client.execute("INSERT INTO news_items (url, prompt, source, title, summary, queried, dt, seq, sentiment, embedding) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) ON CONFLICT ON CONSTRAINT news_pk DO UPDATE SET title = $4, summary = $5, queried = $6, dt = $7, seq = $8, sentiment = $9", &[&i.url, &i.prompt, &i.source, &i.title, &i.summary, &i.queried, &i.dt, &count, &i.sentiment, &embedding]).await?;
    
    Ok(())
}

pub async fn get_embed_model() -> Result<FlagEmbedding, Box<dyn Error>> {
    Ok(FlagEmbedding::try_new(Default::default())?)
}

pub async fn add_prompt_embed(client: &mut Client, model: &Option<FlagEmbedding>, prompt: &str, format: &str) -> Result<(), Box<dyn Error>> {
    let embedding = match model {
        None => None,
        Some(model) => {
            let embedding = model.query_embed(prompt)?;
            Some(Vector::from(embedding))
        },
    };
    client.execute("INSERT INTO prompt_embed (prompt, format, embedding) VALUES ($1, $2, $3) ON CONFLICT ON CONSTRAINT prompt_embed_pkey DO UPDATE set format = $2", &[&prompt, &format, &embedding]).await?;

    Ok(())
}

/*
pub async fn get_prompt_embed(client: &mut Client, prompt: &str) -> Result<String, Box<dyn Error>> {
    let rows: Vec<Row> = client.query("SELECT format FROM prompt_embed WHERE prompt = $1", &[&prompt]).await?;
    
    Ok(if rows.is_empty() { "headlines".into() } else { rows[0].get(0) })
}
*/

pub async fn upd_news_item(client: &mut Client, prompt: &str, url: &str, title: String, summary: Option<&str>) -> Result<(), Box<dyn Error>> {
    //let dt: DateTime<Utc> = Utc::now();
    client.execute("UPDATE news_items SET title = $3, summary = $4, queried = true WHERE url = $2 AND prompt = $1", &[&prompt, &url, &title, &summary]).await?;
    
    Ok(())
}

pub async fn reset_news_item_seq(client: &mut Client, prompt: &str) -> Result<(), Box<dyn Error>> {
    let dt: DateTime<Utc> = Utc::now();
    client.execute("UPDATE news_items SET seq = 0, dt = $2 WHERE prompt = $1", &[&prompt,&dt]).await?;
    
    Ok(())
}

pub async fn del_news_item(client: &mut Client, prompt: &str, url: &str) -> Result<(), Box<dyn Error>> {
    client.execute("DELETE FROM news_items WHERE url = $2 AND prompt = $1", &[&prompt, &url]).await?;
    
    Ok(())
}

pub async fn del_news_title(client: &mut Client, prompt: &str, title: &str) -> Result<(), Box<dyn Error>> {
    client.execute("DELETE FROM news_items WHERE title = $2 AND prompt = $1", &[&prompt, &title]).await?;
    
    Ok(())
}

pub async fn clear_news(client: &mut Client, prompt: &str, hours: u32) -> Result<(), Box<dyn Error>> {
    let dt: DateTime<Utc> = Utc::now() - Duration::hours(hours.into());

    //client.execute("DELETE FROM prompt_embed WHERE prompt = $1", &[&prompt]).await?;
    
    client.execute("DELETE FROM news_items WHERE prompt = $1 AND (summary IS NOT NULL OR dt < $2)", &[&prompt, &dt]).await?;
    
    Ok(())
}

pub async fn get_saved_news(client: &mut Client, prompt: &str, purge: u32) -> Result<Vec<DbNewsItem<Utc>>, Box<dyn Error>> {
    let dt_purge: DateTime<Utc> = Utc::now() - Duration::hours(purge.into());
    let rows: Vec<Row> = client.query("SELECT url, prompt, source, title, summary, queried, dt, sentiment, embedding FROM news_items WHERE prompt = $1 AND (((NOT queried OR summary IS NULL) AND dt > $2) OR summary is NOT NULL) ORDER BY summary, seq, dt", &[&prompt, &dt_purge]).await?;

    Ok(get_rows(&rows).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_postgres::NoTls;

    #[tokio::test]
    async fn it_works() -> Result<(), Box<dyn Error>> {
        let (mut client, connection) = tokio_postgres::connect(&connect_string()?, NoTls).await?;

        // The connection performs the actual communication with the database
        // so spawn it off to run on its own.
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });

        //client.execute("CREATE EXTENSION IF NOT EXISTS vector", &[])?;
        //client.execute("TRUNCATE TABLE news_items", &[])?;

        // Preallocate the strings
        let mut strs = Vec::new();
        for i in 0 .. 1000 {
            let s = (format!("http{i}"), format!("Animals{i}"), format!("title{i}"));
            strs.push(s.clone());
        }

        let mut input = Vec::new();
        for i in 0 .. 1000 {
            input.push(DbNewsItem{url: strs[i].0.clone(), prompt: strs[i].1.clone(), source: "bbc".into(), title: strs[i].2.clone(), summary: if i % 2 == 0 { Some("summary".into()) } else { None }, queried: false, dt: Utc::now(), embedding: vec![1.0, 2.0, 3.0]});
        }

//println!(">>>>");
        add_news(&mut client, &input).await?;
//println!("<<<<");

        let url = "http2";
        let rows: Vec<Row> = client.query("SELECT url, prompt, source, title, summary, queried, dt FROM news_items WHERE url = $1", &[&url]).await?;

        let _rs = get_rows(&rows);

        //println!("{:?}", _rs);

        upd_news_item(&mut client, "Animals2", 2, url, None).await?;

        let rows: Vec<Row> = client.query("SELECT url, prompt, source, title, summary, queried, dt FROM news_items WHERE url = $1", &[&url]).await?;

        let _rs = get_rows(&rows);

        //println!("{:?}", rs);

        upd_news_item(&mut client, "Animals2", 2, url, Some("New Summary")).await?;

        let rows: Vec<Row> = client.query("SELECT url, prompt, source, title, summary, queried, dt FROM news_items WHERE prompt = $1 AND url = $3", &[&"Animals2", &2_u32,  &url]).await?;

        let _rs = get_rows(&rows);

        //println!("{:?}", rs);

        //purge_news(&mut client, 1).await?;

        Ok(())
    }
}

