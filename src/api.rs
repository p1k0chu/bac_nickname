use reqwest::StatusCode;
use serde_json::Value;
use std::time::Duration;

/// function makes post request to url with token and json body
///
/// if rate limited, sleeps and tries again
///
/// Returns status code
pub async fn post(url: String, token: String, json: Value) -> Option<StatusCode> {
    let client = reqwest::Client::new();

    loop {
        let res = match client
            .patch(&url)
            .header("Authorization", &token)
            .json(&json)
            .send()
            .await
        {
            Ok(x) => x,
            Err(e) => {
                eprintln!("{e}");
                return None;
            }
        };
        let status = res.status();

        match status.as_u16() {
            429 => {
                let headers = res.headers();

                let duration = if let Some(duration) = headers.get("Retry-After") {
                    let duration: &str = match duration.to_str().ok() {
                        Some(x) => x,
                        None => return Some(status),
                    };

                    let parsed: f64 = match duration.parse::<f64>() {
                        Ok(x) => x,
                        Err(_) => return Some(status),
                    };
                    Duration::from_secs_f64(parsed)
                } else if let Ok(Value::Object(duration)) = res.json::<Value>().await {
                    let duration = duration.get("retry-after").and_then(Value::as_f64);

                    match duration {
                        Some(x) => Duration::from_secs_f64(x),
                        None => {
                            eprintln!(
                                "discord api trolling, it sent either negative rate limit or 10 lifetimes"
                            );
                            return Some(status);
                        }
                    }
                } else {
                    eprintln!("discord is ghosting me");
                    return Some(status);
                };

                eprintln!("Rate limited, waiting {} seconds", duration.as_secs_f64());
                tokio::time::sleep(duration).await;
            }
            _ => return Some(status),
        }
    }
}

