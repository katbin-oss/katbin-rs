use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use service::Query;
use tower_cookies::Cookies;

use crate::{AppState, COOKIE_NAME, KEY};

pub async fn current_user_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let cookies = request
        .extensions_mut()
        .get::<Cookies>()
        .expect("expected cookie middleware to be added before auth");
    let signed_cookies = cookies.signed(KEY.get().unwrap());

    // fetch current user and insert into req extensions if present
    let current_user = signed_cookies
        .get(COOKIE_NAME)
        .map(|c| c.value().to_owned())
        .unwrap_or_default();
    println!("{}", current_user);
    if !current_user.is_empty() {
        let user = Query::get_user_by_email(&state.conn, &current_user).await;
        if let Ok(user) = user {
            request.extensions_mut().insert(user);
        }
    }

    let response = next.run(request).await;
    response
}
