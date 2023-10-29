use std::error::Error;
use reqwest::Client;
// https://stacks.targetr.net/builder_callback?configName=stacks.targetr.net&itemType=libraryitem&itemId=109FE03EF12448&pendingId=d-1064537285760736&url=http%3A%2F%2Fwww.example.com%2Fgenerated-content-123456.jpg
//
async fn callback(callback: &str, url: &str) -> Result<String, Box<dyn Error>> {
    // Create a new reqwest client
    let call = 

    let client = Client::builder()
        .user_agent("TargetR/News")
        .timeout(std::time::Duration::new(10, 0))
        .build()?;

    // Make the secure GET request to the news server
    let response = client
        .get(call)
        .send()
        .await?;
//println!("{:?}", response);

    // Read the response body as a string
    let body = response.text().await?;
//println!("{:?}", body);

    Ok(body)
}
