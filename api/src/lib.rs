use std::env;
use std::sync::OnceLock;

use axum::extract::{Path, Request, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::{get_service, post};
use axum::{routing::get, Router};
use axum::{Extension, Form};
use entity::{pastes, schema, users};
use serde::{Deserialize, Serialize};
use service::sea_orm::{ConnectOptions, Database, DatabaseConnection, DbErr, SqlErr};
use service::{Mutation, Query};
use tera::Tera;
use tower_cookies::{Cookie, CookieManagerLayer, Cookies, Key};
use tower_http::services::ServeDir;

const COOKIE_NAME: &str = "current_user";
static KEY: OnceLock<Key> = OnceLock::new();

mod middleware;

#[tokio::main]
async fn start() -> anyhow::Result<()> {
    env::set_var("RUST_LOG", "debug");
    tracing_subscriber::fmt::init();

    // load env variables
    dotenvy::dotenv().ok();
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL not found in environment");
    let port = env::var("PORT").expect("PORT not found in environment");
    let key = env::var("SECRET_KEY").expect("SECRET_KEY not found in environment");

    KEY.set(Key::from(key.as_bytes())).unwrap();

    // make db connection
    let mut opt = ConnectOptions::new(db_url);
    opt.sqlx_logging(env::var("DB_LOG").is_ok());
    let conn = Database::connect(opt)
        .await
        .expect("database connection failed");

    let templates = Tera::new(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/**/*"))
        .expect("tera initialization failed");

    let state: AppState = AppState { templates, conn };

    let app = Router::new()
        .route("/", get(root))
        .route("/", post(create_paste))
        .route("/:paste_id", get(show_paste))
        .route("/:paste_id/edit", get(edit))
        .route("/:paste_id/edit", post(post_edit))
        .route("/v/:paste_id", get(show_paste))
        .route("/users/log_in", get(login))
        .route("/users/log_in", post(login_post))
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
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::current_user_middleware,
        ))
        .layer(CookieManagerLayer::new())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
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
async fn root(
    current_user: Option<Extension<users::Model>>,
    state: State<AppState>,
) -> Result<Html<String>, (StatusCode, &'static str)> {
    let mut ctx = tera::Context::new();
    if let Some(user) = current_user {
        ctx.insert("current_user", &user.0);
    }

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

async fn edit(
    current_user: Option<Extension<users::Model>>,
    state: State<AppState>,
    Path(paste_id): Path<String>,
) -> Response {
    // check if a paste exists for the given paste id
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
        });
    if paste.is_err() {
        return paste.unwrap_err().into_response();
    }
    let paste = paste.unwrap();

    // check if a user is logged in
    if current_user.is_none() {
        return Redirect::to(format!("/{}", paste_id).as_str()).into_response();
    }
    let current_user = current_user.unwrap();

    if paste.belongs_to != Some(current_user.id) {
        return Redirect::to(format!("/{}", paste_id).as_str()).into_response();
    }

    let mut ctx = tera::Context::new();
    ctx.insert("current_user", &current_user.0);
    ctx.insert("is_edit", &true);
    ctx.insert("content", &paste.content);

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

    Html(body.unwrap()).into_response()
}

async fn post_edit(
    current_user: Option<Extension<users::Model>>,
    state: State<AppState>,
    Path(paste_id): Path<String>,
    form: Form<pastes::Model>,
) -> Result<Redirect, (StatusCode, &'static str)> {
    let form = form.0;
    let user = current_user.map(|u| u.0);

    // if paste_id contains a ".", split on it
    let split_paste: Vec<_> = paste_id.split(".").collect();
    let mut extension = "";
    if split_paste.len() == 2 {
        extension = split_paste[1];
    }

    let paste = Mutation::update_paste_content(&state.conn, &form, user, split_paste[0])
        .await
        .map_err(|err| match err {
            DbErr::RecordNotFound(_) => (StatusCode::NOT_FOUND, "Not found"),
            _ => (StatusCode::NOT_FOUND, "Not found"),
        })?;

    let redirect_url = if paste.is_url {
        format!("/v/{}", paste.id)
    } else {
        format!("/{}", paste.id)
    };
    Ok(Redirect::to(&redirect_url))
}

async fn create_paste(
    current_user: Option<Extension<users::Model>>,
    state: State<AppState>,
    form: Form<pastes::Model>,
) -> Response {
    let form = form.0;
    let user = current_user.map(|u| u.0);

    let create_result = Mutation::create_paste(&state.conn, &form, user).await;
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

    let paste = create_result.unwrap();
    let redirect_url = if paste.is_url {
        format!("/v/{}", paste.id)
    } else {
        format!("/{}", paste.id)
    };
    Redirect::to(&redirect_url).into_response()
}

async fn show_paste(
    current_user: Option<Extension<users::Model>>,
    state: State<AppState>,
    Path(paste_id): Path<String>,
    request: Request,
) -> Response {
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
        });
    if paste.is_err() {
        return paste.unwrap_err().into_response();
    }

    let paste = paste.unwrap();

    if !request.uri().to_string().contains("/v/") && paste.is_url {
        tracing::debug!("Path is not for display, redirect to URL");
        return Redirect::temporary(&paste.content).into_response();
    }

    let show_edit = match (paste.belongs_to, current_user.as_ref()) {
        (Some(belongs_to), Some(current_user)) => current_user.id == belongs_to,
        _ => false,
    };

    let mut ctx = tera::Context::new();
    ctx.insert("paste", &paste);
    ctx.insert("extension", extension);
    ctx.insert("show_edit", &show_edit);
    if let Some(user) = current_user {
        ctx.insert("current_user", &user.0);
    }

    let body = state.templates.render("show.html.tera", &ctx).map_err(|e| {
        tracing::error!("Error rendering template {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "error rendering template",
        );
    });
    if body.is_err() {
        return body.unwrap_err().into_response();
    }

    Html(body.unwrap()).into_response()
}

async fn login(state: State<AppState>) -> Result<Html<String>, (StatusCode, &'static str)> {
    let ctx = tera::Context::new();
    let body = state
        .templates
        .render("login.html.tera", &ctx)
        .map_err(|err| {
            tracing::error!("error rendering template {}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "error rendering template",
            )
        })?;

    Ok(Html(body))
}

async fn login_post(
    cookies: Cookies,
    state: State<AppState>,
    form: Form<schema::LoginPost>,
) -> Response {
    let form = form.0;
    let signed_cookies = cookies.signed(KEY.get().unwrap());
    let user_res = Query::login(&state.conn, &form).await;

    if let Err(err) = user_res {
        match err {
            DbErr::RecordNotFound(msg) => {
                let mut ctx = tera::Context::new();
                ctx.insert(
                    "flash",
                    &Flash {
                        info: None,
                        warn: Some(msg),
                    },
                );

                let body_res = state
                    .templates
                    .render("login.html.tera", &ctx)
                    .map_err(|err| {
                        tracing::error!("error rendering template {}", err);
                        (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "error rendering template",
                        )
                    });
                if body_res.is_err() {
                    return body_res.unwrap_err().into_response();
                }
                return Html(body_res.unwrap()).into_response();
            }
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "something went wrong").into_response(),
        }
    } else {
        let mut cookie = Cookie::new(COOKIE_NAME, user_res.unwrap().email);
        cookie.set_path("/");
        signed_cookies.add(cookie);
        Redirect::to("/").into_response()
    }
}

pub fn main() {
    let result = start();

    if let Some(err) = result.err() {
        println!("Error: {err}");
    }
}
