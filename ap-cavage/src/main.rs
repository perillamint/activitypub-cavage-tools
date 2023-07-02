use reqwest::Request;
use sigh::alg::RsaSha256;
use sigh::{Key, PrivateKey, SigningConfig};
use url::Url;

use clap::Parser;

mod config;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[arg(short, long)]
    config: String,

    #[arg(short, long)]
    key_id: String,

    #[arg(long)]
    url: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let cfg = config::Config::from_file(&args.config);
    let key = cfg.key.iter().find(|k| k.id == args.key_id).unwrap();

    let private_key = PrivateKey::from_pem(key.pem.as_bytes()).unwrap();
    // Mastodon only supports rsa-sha256
    let signing_config = SigningConfig::new(RsaSha256, &private_key, &key.id, false);

    let url: Url = Url::parse(&args.url).unwrap();
    let host = url.host_str().unwrap();

    let client = reqwest::Client::new();
    let mut req: http::Request<reqwest::Body> = http::Request::builder()
        .method("GET")
        .uri(url.as_str())
        .header("Accept", "application/activity+json")
        .header(
            "Date",
            chrono::Utc::now()
                .format("%a, %d %b %Y %H:%M:%S GMT")
                .to_string(),
        )
        .header("Host", host)
        .body(reqwest::Body::from(""))
        .unwrap();

    println!("req: {:#?}", req);

    signing_config.sign(&mut req).unwrap();

    println!("req: {:#?}", req);

    let result = client.execute(req.try_into().unwrap()).await.unwrap();

    println!("result: {}", result.text().await.unwrap());
}
