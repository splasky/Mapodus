use axum::response::{IntoResponse, Redirect};
use axum::Json;
use oauth2::basic::BasicClient;
use oauth2::{AuthUrl, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope, TokenUrl};
use serde::Serialize;
use tower_sessions::Session;

use crate::session::AppSession;

#[derive(Serialize)]
pub struct AuthStatus {
    logged_in: bool,
    name: Option<String>,
    email: Option<String>,
    avatar_url: Option<String>,
}

pub async fn google_login(_session: Session) -> impl IntoResponse {
    let client_id = std::env::var("GOOGLE_CLIENT_ID")
        .unwrap_or_else(|_| "dummy-client-id".into());
    let client_secret = std::env::var("GOOGLE_CLIENT_SECRET")
        .unwrap_or_else(|_| "dummy-client-secret".into());

    let auth_url = AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".into())
        .expect("Invalid auth URL");
    let token_url = TokenUrl::new("https://oauth2.googleapis.com/token".into())
        .expect("Invalid token URL");

    let client = BasicClient::new(ClientId::new(client_id))
        .set_client_secret(ClientSecret::new(client_secret))
        .set_auth_uri(auth_url)
        .set_token_uri(token_url)
        .set_redirect_uri(
            RedirectUrl::new("http://localhost:3000/api/auth/google/callback".into())
                .expect("Invalid redirect URL"),
        );

    let (auth_url, _csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("openid".into()))
        .add_scope(Scope::new("email".into()))
        .add_scope(Scope::new("profile".into()))
        .url();

    Redirect::to(auth_url.as_str())
}

pub async fn google_callback(session: Session) -> impl IntoResponse {
    let mut app = AppSession::from_session(&session).await;
    app.google_user = Some(crate::session::GoogleUser {
        name: Some("Test User".into()),
        email: Some("test@example.com".into()),
        avatar_url: Some("https://example.com/avatar.png".into()),
    });
    app.save_to_session(&session).await;
    Redirect::to("/")
}

pub async fn status(session: Session) -> impl IntoResponse {
    let app = AppSession::from_session(&session).await;
    Json(AuthStatus {
        logged_in: app.google_user.is_some(),
        name: app.google_user.as_ref().and_then(|u| u.name.clone()),
        email: app.google_user.as_ref().and_then(|u| u.email.clone()),
        avatar_url: app.google_user.as_ref().and_then(|u| u.avatar_url.clone()),
    })
}
