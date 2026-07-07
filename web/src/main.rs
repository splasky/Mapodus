use clap::Parser;

#[derive(Parser)]
#[command(name = "web", about = "google-maps-to-umap web server")]
struct Args {
    #[arg(
        long,
        help = "Google Maps cookies for dev mode (e.g. 'SAPISID=xxx; SID=yyy; HSID=zzz'). Imports and prints saved lists on startup."
    )]
    google_cookies: Option<String>,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let args = Args::parse();

    if let Some(ref cookies) = args.google_cookies {
        web::run_dev_import(cookies).await;
    }

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8900")
        .await
        .expect("Failed to bind to port 8900");

    println!("Server running on http://localhost:8900");
    web::serve_listener(listener).await.unwrap();
}
