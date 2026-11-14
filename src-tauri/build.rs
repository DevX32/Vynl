use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let properties = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("supabase.properties");
    let mut url = env::var("SUPABASE_URL").unwrap_or_default();
    let mut anon_key = env::var("SUPABASE_ANON_KEY").unwrap_or_default();

    if let Ok(contents) = fs::read_to_string(&properties) {
        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((name, value)) = line.split_once('=') else {
                continue;
            };
            match name.trim() {
                "url" if url.is_empty() => url = value.trim().to_string(),
                "anonKey" if anon_key.is_empty() => anon_key = value.trim().to_string(),
                _ => {}
            }
        }
    }

    let sanitize = |value: String| value.replace(['\r', '\n'], "");
    println!("cargo:rustc-env=SUPABASE_URL={}", sanitize(url));
    println!("cargo:rustc-env=SUPABASE_ANON_KEY={}", sanitize(anon_key));
    println!("cargo:rerun-if-changed={}", properties.display());

    tauri_build::build()
}
