use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmResult {
    pub ok: bool,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct ConfirmRequest {
    pub token_hash: String,
    pub email_type: String,
}

pub fn parse_confirm_url(url: &str) -> Option<ConfirmRequest> {
    let prefix = "vynl://confirm";
    if !url.starts_with(prefix) {
        return None;
    }
    let rest = url.trim_start_matches(prefix).trim_start_matches('?');
    if rest.is_empty() {
        return None;
    }
    let mut token_hash: Option<String> = None;
    let mut email_type: Option<String> = None;
    for part in rest.split('&') {
        let Some((k, value)) = part.split_once('=') else {
            continue;
        };
        let key = k.trim().to_lowercase();
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        match key.as_str() {
            "token_hash" => token_hash = Some(value.to_string()),
            "type" => email_type = Some(value.to_string()),
            _ => {}
        }
    }
    let token_hash = token_hash?;
    if token_hash.is_empty() {
        return None;
    }
    Some(ConfirmRequest {
        token_hash,
        email_type: email_type.unwrap_or_else(|| "signup".to_string()),
    })
}
