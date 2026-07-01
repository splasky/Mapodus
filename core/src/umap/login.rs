use anyhow::{Context, Result, anyhow};
use reqwest::header::{COOKIE, HeaderMap, REFERER, SET_COOKIE};

use super::auth::CookieAuth;

fn extract_cookie(headers: &HeaderMap, cookie_name: &str) -> Option<String> {
    headers.get_all(SET_COOKIE).iter().find_map(|value| {
        let cookie = value.to_str().ok()?;
        cookie
            .split(';')
            .find_map(|segment| segment.trim().split_once('='))
            .and_then(|(name, value)| (name == cookie_name).then(|| value.to_string()))
    })
}

fn extract_hidden_input(body: &str, input_name: &str) -> Option<String> {
    let needle = format!("name=\"{input_name}\"");
    let start = body.find(&needle)?;
    let slice = &body[start..];
    let value_marker = "value=\"";
    let value_start = slice.find(value_marker)? + value_marker.len();
    let rest = &slice[value_start..];
    let value_end = rest.find('"')?;
    Some(rest[..value_end].to_string())
}

pub async fn proxy_login(base_url: &str, username: &str, password: &str) -> Result<CookieAuth> {
    let login_url = format!("{}/login/", base_url.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .context("failed to create login client")?;

    let login_page = client
        .get(&login_url)
        .send()
        .await
        .context("failed to fetch login page")?;
    let login_headers = login_page.headers().clone();
    let login_body = login_page
        .text()
        .await
        .context("failed to read login page")?;

    let csrf_token = extract_hidden_input(&login_body, "csrfmiddlewaretoken")
        .or_else(|| extract_cookie(&login_headers, "csrftoken"))
        .ok_or_else(|| anyhow!("Failed to extract CSRF token from login page"))?;

    let response = client
        .post(&login_url)
        .header(COOKIE, format!("csrftoken={csrf_token}"))
        .header(REFERER, &login_url)
        .form(&[
            ("username", username),
            ("password", password),
            ("csrfmiddlewaretoken", csrf_token.as_str()),
            ("next", "/"),
        ])
        .send()
        .await
        .context("failed to submit login form")?;

    let status = response.status();
    let headers = response.headers().clone();
    let body = response.text().await.unwrap_or_default();

    if !status.is_success() && !status.is_redirection() {
        return Err(anyhow!("Login failed ({status}): {body}"));
    }

    let session_id = extract_cookie(&headers, "sessionid")
        .ok_or_else(|| anyhow!("Login succeeded but sessionid cookie was missing"))?;
    let csrf_token = extract_cookie(&headers, "csrftoken").unwrap_or(csrf_token);

    let auth = CookieAuth {
        session_id,
        csrf_token,
    };

    let verify_url = format!("{}/", base_url.trim_end_matches('/'));
    client
        .get(&verify_url)
        .header(COOKIE, auth.to_cookie_header())
        .send()
        .await
        .context("login verification request failed")?;

    Ok(auth)
}
