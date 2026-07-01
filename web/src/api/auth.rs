use axum::extract::Query;
use axum::response::{IntoResponse, Redirect};
use axum::Json;
use oauth2::basic::BasicClient;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope,
    TokenResponse, TokenUrl,
};
use serde::{Deserialize, Serialize};
use tower_sessions::Session;

use crate::session::AppSession;

#[derive(Serialize)]
pub struct AuthStatus {
    logged_in: bool,
    name: Option<String>,
    email: Option<String>,
}

pub async fn google_login(_session: Session) -> impl IntoResponse {
    let client_id =
        std::env::var("GOOGLE_CLIENT_ID").unwrap_or_else(|_| "dummy-client-id".into());
    let client_secret = std::env::var("GOOGLE_CLIENT_SECRET")
        .unwrap_or_else(|_| "dummy-client-secret".into());
    let redirect_url =
        std::env::var("REDIRECT_URL").unwrap_or_else(|_| "http://localhost:8900/api/auth/google/callback".into());

    let auth_url =
        AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".into()).expect("Invalid auth URL");
    let token_url =
        TokenUrl::new("https://oauth2.googleapis.com/token".into()).expect("Invalid token URL");

    let client = BasicClient::new(ClientId::new(client_id))
        .set_client_secret(ClientSecret::new(client_secret))
        .set_auth_uri(auth_url)
        .set_token_uri(token_url)
        .set_redirect_uri(
            RedirectUrl::new(redirect_url)
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

#[derive(Deserialize)]
pub struct CallbackParams {
    code: Option<String>,
    #[allow(dead_code)]
    state: Option<String>,
    error: Option<String>,
}

#[derive(Deserialize)]
struct PeopleResponse {
    names: Option<Vec<NameEntry>>,
    email_addresses: Option<Vec<EmailEntry>>,
}

#[derive(Deserialize)]
struct NameEntry {
    display_name: Option<String>,
}

#[derive(Deserialize)]
struct EmailEntry {
    value: Option<String>,
}

pub async fn google_callback(
    session: Session,
    Query(params): Query<CallbackParams>,
) -> impl IntoResponse {
    if let Some(error) = params.error {
        eprintln!("OAuth error: {}", error);
        return Redirect::to("/?error=oauth_denied").into_response();
    }

    let code = match params.code {
        Some(c) => c,
        None => return Redirect::to("/?error=no_code").into_response(),
    };

    let client_id =
        std::env::var("GOOGLE_CLIENT_ID").unwrap_or_else(|_| "dummy-client-id".into());
    let client_secret = std::env::var("GOOGLE_CLIENT_SECRET")
        .unwrap_or_else(|_| "dummy-client-secret".into());
    let redirect_url =
        std::env::var("REDIRECT_URL").unwrap_or_else(|_| "http://localhost:8900/api/auth/google/callback".into());

    let auth_url =
        AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".into()).expect("Invalid auth URL");
    let token_url =
        TokenUrl::new("https://oauth2.googleapis.com/token".into()).expect("Invalid token URL");

    let client = BasicClient::new(ClientId::new(client_id))
        .set_client_secret(ClientSecret::new(client_secret))
        .set_auth_uri(auth_url)
        .set_token_uri(token_url)
        .set_redirect_uri(
            RedirectUrl::new(redirect_url)
                .expect("Invalid redirect URL"),
        );

    let http_client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Client should build");

    let token_result = client
        .exchange_code(AuthorizationCode::new(code))
        .request_async(&http_client)
        .await;

    let token = match token_result {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Token exchange failed: {}", e);
            return Redirect::to("/?error=token_exchange_failed").into_response();
        }
    };

    let access_token = token.access_token().secret();

    let people_url = "https://people.googleapis.com/v1/people/me?personFields=names,email_addresses";
    let people_response = http_client
        .get(people_url)
        .bearer_auth(access_token)
        .send()
        .await;

    match people_response {
        Ok(resp) => {
            if let Ok(people) = resp.json::<PeopleResponse>().await {
                let name = people
                    .names
                    .and_then(|n| n.into_iter().next())
                    .and_then(|n| n.display_name);
                let email = people
                    .email_addresses
                    .and_then(|e| e.into_iter().next())
                    .and_then(|e| e.value);

                let mut app = AppSession::from_session(&session).await;
                app.google_user = Some(crate::session::GoogleUser {
                    name,
                    email,
                });
                app.save_to_session(&session).await;
            }
        }
        Err(e) => {
            eprintln!("Failed to fetch user info: {}", e);
        }
    }

    Redirect::to("/").into_response()
}

pub async fn status(session: Session) -> impl IntoResponse {
    let app = AppSession::from_session(&session).await;
    Json(AuthStatus {
        logged_in: app.google_user.is_some(),
        name: app.google_user.as_ref().and_then(|u| u.name.clone()),
        email: app.google_user.as_ref().and_then(|u| u.email.clone()),
    })
}
