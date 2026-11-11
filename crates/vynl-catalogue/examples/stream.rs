use std::time::Duration;

use vynl_catalogue::CatalogueClient;

#[tokio::main]
async fn main() {
    let client = CatalogueClient::new();
    let id = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "khnokW3Mw24".to_string());

    match client.stream(&id).await {
        Ok(stream) => {
            println!("mime: {}", stream.mime);
            println!("url: {}", &stream.url[..stream.url.len().min(110)]);

            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(20))
                .build()
                .unwrap();
            let head = client
                .get(&stream.url)
                .header("Range", "bytes=0-1023")
                .send()
                .await;
            match head {
                Ok(res) => {
                    let status = res.status();
                    let len = res.content_length().unwrap_or(0);
                    println!("fetch: status={status} bytes={len}");
                }
                Err(err) => println!("fetch failed: {err}"),
            }
        }
        Err(err) => println!("resolve failed: {err}"),
    }
}
