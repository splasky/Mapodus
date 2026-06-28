use anyhow::{anyhow, Result};
use super::auth::CookieAuth;

pub async fn proxy_login(umap_url: &str, username: &str, password: &str) -> Result<CookieAuth> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .cookie_store(true)
        .build()?;

    let base_url = umap_url.trim_end_matches('/').to_string();
    let login_url = format!("{}/login/", base_url);

    // Step 1: GET login page to obtain CSRF token
    let resp = client.get(&login_url).send().await?;
    let body: String = resp.text().await?;

    let csrf_token = extract_csrf_token(&body)
        .ok_or_else(|| anyhow!("Could not find CSRF token in login page"))?;

    // Step 2: POST credentials with CSRF token
    let params = [
        ("username", username),
        ("password", password),
        ("csrfmiddlewaretoken", &csrf_token),
        ("next", "/"),
    ];

    let resp = client
        .post(&login_url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Referer", &login_url)
        .form(&params)
        .send()
        .await?;

    // Step 3: Follow redirect and capture cookies
    let cookies: Vec<String> = resp
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .map(|s| {
            s.split(';')
                .next()
                .unwrap_or("")
                .to_string()
        })
        .collect();

    let mut session_id = String::new();
    for cookie in &cookies {
        if let Some((key, value)) = cookie.split_once('=') {
            if key == "sessionid" {
                session_id = value.to_string();
            }
        }
    }

    if session_id.is_empty() {
        return Err(anyhow!("Login failed: no sessionid cookie received. Check username/password."));
    }

    // Step 4: Verify auth by visiting the home page
    let verify = client
        .get(&base_url)
        .send()
        .await?;

    if !verify.status().is_success() {
        return Err(anyhow!("Login verification failed (HTTP {})", verify.status()));
    }

    let auth = CookieAuth {
        session_id,
        csrf_token,
    };

    Ok(auth)
}

fn extract_csrf_token(html: &str) -> Option<String> {
    // Try name="csrfmiddlewaretoken" value="..."
    if let Some(start) = html.find("csrfmiddlewaretoken") {
        let after_name = &html[start..];
        if let Some(val_start) = after_name.find("value=\"") {
            let val_begin = val_start + 7;
            if let Some(val_end) = after_name[val_begin..].find('\"') {
                return Some(after_name[val_begin..val_begin + val_end].to_string());
            }
        }
    }
    None
}
