use std::env;

use axum::{routing::get, Router};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Html;
use axum::routing::get_service;
use tera::Tera;
use tower_http::services::ServeDir;
use service::sea_orm::{Database, DatabaseConnection};

#[tokio::main]
async fn start() -> anyhow::Result<()> {
    env::set_var("RUST_LOG", "debug");
    tracing_subscriber::fmt::init();

    // load env variables

    dotenvy::dotenv().ok();
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL not found in environment");
    let port = env::var("PORT").expect("PORT not found in environment");

    // make db connection
    let conn = Database::connect(db_url)
        .await
        .expect("database connection failed");

    let templates = Tera::new(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/**/*")).expect("tera initialization failed");

    let state: AppState = AppState { templates, conn };

    let app = Router::new()
        .route("/", get(root))
        .nest_service(
            "/static",
            get_service(ServeDir::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/static"
        )))
        .handle_error(|error| async move {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Unhandled internal error: {error}"),
            )
        }))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port))
        .await
        .unwrap();

    tracing::info!("listening on http://{}", listener.local_addr().unwrap());
    axum::serve(listener, app).await?;

    Ok(())
}

#[derive(Clone)]
struct AppState {
    templates: Tera,
    conn: DatabaseConnection,
}

// basic handler that responds with a static string
async fn root(
    state: State<AppState>
) -> Result<Html<String>, (StatusCode, &'static str)> {
    let mut ctx = tera::Context::new();

    let body = state.templates.render("index.html.tera", &ctx)
        .map_err(|e| {
            tracing::error!("Error rendering template {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "error rendering template")
        })?;

    Ok(Html(body))
}

pub fn main() {
    let result = start();

    if let Some(err) = result.err() {
        println!("Error: {err}");
    }
}
