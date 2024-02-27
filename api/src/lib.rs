use std::env;

use anyhow::Ok;
use axum::{routing::get, Router};
use service::sea_orm::Database;

#[tokio::main]
async fn start() -> anyhow::Result<()> {
    env::set_var("RUST_LOG", "debug");
    tracing_subscriber::fmt::init();

    // load env variables
    dotenvy::dotenv()?;
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL not found in environment");
    let port = env::var("PORT").expect("PORT not found in environment");

    // make db connection
    let conn = Database::connect(db_url)
        .await
        .expect("database connection failed");


    let app = Router::new().route("/", get(root));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    tracing::debug!("listening on http://{}", listener.local_addr().unwrap());
    axum::serve(listener, app).await?;

    Ok(())
}

// basic handler that responds with a static string
async fn root() -> &'static str {
    "Hello, World!"
}

pub fn main() {
    let result = start();

    if let Some(err) = result.err() {
        println!("Error: {err}");
    }
}
