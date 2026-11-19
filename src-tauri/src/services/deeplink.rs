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

fn percent_decode(input: &str) -> Option<String> {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' => {
                if i + 2 >= bytes.len() {
                    return None;
                }
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok()?;
                let value = u8::from_str_radix(hex, 16).ok()?;
                out.push(value);
                i += 3;
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8(out).ok()
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
            "token_hash" => token_hash = Some(percent_decode(value)?),
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
