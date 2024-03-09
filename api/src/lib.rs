use std::env;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::{get_service, post};
use axum::Form;
use axum::{routing::get, Router};
use entity::pastes;
use serde::{Deserialize, Serialize};
use service::sea_orm::{Database, DatabaseConnection, DbErr, SqlErr};
use service::{Mutation, Query};
use tera::Tera;
use tower_http::services::ServeDir;

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

    let templates = Tera::new(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/**/*"))
        .expect("tera initialization failed");

    let state: AppState = AppState { templates, conn };

    let app = Router::new()
        .route("/", get(root))
        .route("/", post(create_paste))
        .route("/:paste_id", get(show_paste))
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
            }),
        )
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

#[derive(Clone, Serialize, Deserialize)]
struct Flash {
    pub info: Option<String>,
    pub warn: Option<String>,
}

// basic handler that responds with a static string
async fn root(state: State<AppState>) -> Result<Html<String>, (StatusCode, &'static str)> {
    let ctx = tera::Context::new();

    let body = state
        .templates
        .render("index.html.tera", &ctx)
        .map_err(|e| {
            tracing::error!("Error rendering template {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "error rendering template",
            );
        })?;

    Ok(Html(body))
}

async fn create_paste(state: State<AppState>, form: Form<pastes::Model>) -> Response {
    let form = form.0;

    let create_result = Mutation::create_paste(&state.conn, &form).await;
    if let Err(error) = create_result {
        match error.sql_err() {
            Some(sql_err) => match sql_err {
                SqlErr::UniqueConstraintViolation(_) => {
                    // this key already exists
                    // re-render the index page with a flash
                    let mut ctx = tera::Context::new();
                    if form.custom_url.is_some() {
                        ctx.insert(
                            "flash",
                            &Flash {
                                info: None,
                                warn: Some(String::from("This custom URL has already been taken.")),
                            },
                        );
                    }

                    let body = state
                        .templates
                        .render("index.html.tera", &ctx)
                        .map_err(|e| {
                            tracing::error!("Error rendering template {}", e);
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "error rendering template",
                            )
                        });
                    if let Err(e) = body {
                        return e.into_response();
                    }

                    return Html(body.unwrap()).into_response();
                }
                SqlErr::ForeignKeyConstraintViolation(_) => todo!(),
                _ => {
                    return (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong!")
                        .into_response()
                }
            },
            None => {
                return (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong!").into_response()
            }
        }
    }

    let redirect_url = format!("/{}", create_result.unwrap().id);
    Redirect::to(&redirect_url).into_response()
}

async fn show_paste(
    state: State<AppState>,
    Path(paste_id): Path<String>,
) -> Result<Html<String>, (StatusCode, &'static str)> {
    // if paste_id contains a ".", split on it
    let split_paste: Vec<_> = paste_id.split(".").collect();
    let mut extension = "";
    if split_paste.len() == 2 {
        extension = split_paste[1];
    }

    let paste = Query::get_paste_by_id(&state.conn, split_paste[0])
        .await
        .map_err(|err| match err {
            DbErr::RecordNotFound(_) => (StatusCode::NOT_FOUND, "Not found"),
            _ => (StatusCode::NOT_FOUND, "Not found"),
        })?;

    let mut ctx = tera::Context::new();
    ctx.insert("paste", &paste);
    ctx.insert("extension", extension);

    let body = state
        .templates
        .render("show.html.tera", &ctx)
        .map_err(|e| {
            tracing::error!("Error rendering template {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "error rendering template",
            );
        })?;

    Ok(Html(body))
}

pub fn main() {
    let result = start();

    if let Some(err) = result.err() {
        println!("Error: {err}");
    }
}
