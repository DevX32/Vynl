use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::OnceLock;
use std::time::Duration;

use regex::Regex;
use serde::Serialize;
use tauri::AppHandle;

use crate::services::{plugins, process, tools, util};

const ESBUILD_LATEST_URL: &str = "https://api.github.com/repos/evanw/esbuild/releases/latest";
const GH_UA: &str = "Vynl/1.0";
const SDK_MODULE: &str = "@vynl/plugin-sdk";

const MAX_SOURCE_BYTES: u64 = 8 * 1024 * 1024;
const MAX_SCAN_FILES: usize = 2000;
const COMPILE_TIMEOUT: Duration = Duration::from_secs(60);

const SOURCE_EXTS: [&str; 6] = ["ts", "tsx", "js", "jsx", "mjs", "cjs"];

const SDK_SHIM: &str = concat!(
    "const sdk = globalThis.__VYNL_PLUGIN_SDK__ || {};\n",
    "export default sdk;\n",
    "export const PLUGIN_API_VERSION = sdk.PLUGIN_API_VERSION || 0;\n"
);

#[derive(Serialize)]
pub struct CompiledPlugin {
    pub code: String,
    pub deps: Vec<String>,
}

fn compile_lock() -> &'static tokio::sync::Mutex<()> {
    static LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
}

fn build_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = plugins::app_dir(app)?.join("plugin-build");
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create build dir: {e}"))?;
    Ok(dir)
}

fn exe_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "esbuild.exe"
    } else {
        "esbuild"
    }
}

fn esbuild_target() -> Option<(&'static str, &'static str)> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => Some(("windows-64", "zip")),
        ("linux", "x86_64") => Some(("linux-64", "tar.gz")),
        ("linux", "aarch64") => Some(("linux-arm64", "tar.gz")),
        ("macos", "aarch64") => Some(("darwin-arm64", "tar.gz")),
        ("macos", "x86_64") => Some(("darwin-64", "tar.gz")),
        _ => None,
    }
}

fn esbuild_runs(bin: &str) -> bool {
    let mut command = process::hidden_std(std::process::Command::new(bin));
    command
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    command.output().map(|o| o.status.success()).unwrap_or(false)
}

fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(GH_UA)
        .redirect(reqwest::redirect::Policy::limited(10))
        .connect_timeout(Duration::from_secs(15))
        .read_timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))
}

pub async fn ensure_esbuild(app: &AppHandle) -> Result<String, String> {
    let user_data = plugins::app_dir(app)?;

    if let Some(bin) = tools::get_tool_path("esbuild", &user_data)
        && esbuild_runs(&bin)
    {
        return Ok(bin);
    }

    let _guard = compile_lock()
        .lock()
        .await;

    if let Some(bin) = tools::get_tool_path("esbuild", &user_data)
        && esbuild_runs(&bin)
    {
        return Ok(bin);
    }

    download_esbuild(app, &user_data).await
}

async fn download_esbuild(app: &AppHandle, user_data: &Path) -> Result<String, String> {
    let Some((token, ext)) = esbuild_target() else {
        return Err(format!(
            "esbuild is not available for {}-{}",
            std::env::consts::OS,
            std::env::consts::ARCH
        ));
    };

    let client = http_client()?;
    let release = client
        .get(ESBUILD_LATEST_URL)
        .send()
        .await
        .map_err(|e| format!("Failed to reach GitHub: {e}"))?
        .text()
        .await
        .map_err(|e| format!("Failed to read release info: {e}"))?;
    let release: serde_json::Value =
        serde_json::from_str(&release).map_err(|e| format!("Invalid release JSON: {e}"))?;

    let assets = release["assets"]
        .as_array()
        .ok_or("esbuild release has no assets")?;

    let url = assets
        .iter()
        .filter_map(|a| {
            let name = a["name"].as_str()?;
            let starts = name.starts_with(&format!("esbuild-{token}"));
            let ends = name.ends_with(&format!(".{ext}"));
            if starts && ends {
                a["browser_download_url"].as_str()
            } else {
                None
            }
        })
        .next()
        .ok_or_else(|| format!("No esbuild build for {token}"))?;

    let bytes = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Failed to download esbuild: {e}"))?
        .bytes()
        .await
        .map_err(|e| format!("Failed to read esbuild download: {e}"))?;

    let scratch = build_dir(app)?;
    let archive = scratch.join(format!("esbuild-archive.{ext}"));
    fs::write(&archive, &bytes).map_err(|e| format!("Failed to write esbuild archive: {e}"))?;

    let extract_dir = scratch.join("esbuild-extract");
    let _ = fs::remove_dir_all(&extract_dir);
    fs::create_dir_all(&extract_dir)
        .map_err(|e| format!("Failed to create extract dir: {e}"))?;

    let extracted = if ext == "zip" {
        extract_esbuild_zip(&archive, &extract_dir)?
    } else {
        extract_esbuild_tar(&archive, &extract_dir).await?
    };

    let bin_dir = user_data.join("bin");
    fs::create_dir_all(&bin_dir).map_err(|e| format!("Failed to create bin dir: {e}"))?;
    let dest = bin_dir.join(exe_name());
    fs::copy(&extracted, &dest).map_err(|e| format!("Failed to install esbuild: {e}"))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&dest, fs::Permissions::from_mode(0o755));
    }

    let _ = fs::remove_dir_all(&extract_dir);
    let _ = fs::remove_file(&archive);

    if !esbuild_runs(&dest.to_string_lossy()) {
        return Err("esbuild was installed but failed to run".into());
    }
    Ok(dest.to_string_lossy().into_owned())
}

fn extract_esbuild_zip(archive: &Path, dest: &Path) -> Result<PathBuf, String> {
    let file = fs::File::open(archive).map_err(|e| format!("Failed to open zip: {e}"))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("Failed to read zip: {e}"))?;

    for i in 0..zip.len() {
        let mut entry = zip
            .by_index(i)
            .map_err(|e| format!("Failed to read zip entry: {e}"))?;
        let name = entry.name().to_lowercase();
        if name != "esbuild.exe" && !name.ends_with("/esbuild.exe") {
            continue;
        }
        let out = dest.join("esbuild.exe");
        let mut out_file =
            fs::File::create(&out).map_err(|e| format!("Failed to create esbuild: {e}"))?;
        std::io::copy(&mut entry, &mut out_file)
            .map_err(|e| format!("Failed to extract esbuild: {e}"))?;
        return Ok(out);
    }
    Err("esbuild.exe not found in archive".into())
}

async fn extract_esbuild_tar(archive: &Path, dest: &Path) -> Result<PathBuf, String> {
    let archive = archive.to_path_buf();
    let dest = dest.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let output = std::process::Command::new("tar")
            .args([
                "xf",
                archive.to_string_lossy().as_ref(),
                "-C",
                dest.to_string_lossy().as_ref(),
            ])
            .output()
            .map_err(|e| format!("Failed to run 'tar' (is it installed?): {e}"))?;
        if !output.status.success() {
            return Err(format!(
                "Failed to extract esbuild: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let mut found: Option<PathBuf> = None;
        let mut stack = vec![dest.clone()];
        while let Some(dir) = stack.pop() {
            let Ok(entries) = fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.file_name().is_some_and(|n| n == "esbuild") {
                    found = Some(path);
                }
            }
        }
        found.ok_or("esbuild not found in archive".to_string())
    })
    .await
    .map_err(|e| format!("Task join error: {e}"))?
}

fn resolve_relative(importer_rel: &str, spec: &str) -> Result<String, String> {
    let mut segments: Vec<&str> = Vec::new();
    if !importer_rel.is_empty() && importer_rel != "<stdin>" {
        let parts: Vec<&str> = importer_rel.split('/').collect();
        segments = parts[..parts.len().saturating_sub(1)]
            .iter()
            .copied()
            .filter(|s| !s.is_empty())
            .collect();
    }

    for seg in spec.split('/') {
        if seg.is_empty() || seg == "." {
            continue;
        }
        if seg == ".." {
            if segments.pop().is_none() {
                return Err(format!("Import escapes the plugin directory: {spec}"));
            }
        } else {
            segments.push(seg);
        }
    }

    let out = segments.join("/");
    if out.is_empty() {
        return Err(format!("Invalid import path: {spec}"));
    }
    Ok(out)
}

fn is_absolute_specifier(spec: &str) -> bool {
    if spec.starts_with('/') || spec.starts_with('\\') {
        return true;
    }
    let bytes = spec.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'/' || bytes[2] == b'\\')
}

fn specifier_regexes() -> &'static Vec<Regex> {
    static RES: OnceLock<Vec<Regex>> = OnceLock::new();
    RES.get_or_init(|| {
        [
            r#"(?:\bfrom|\bimport)\s*["']([^"'\n]+)["']"#,
            r#"\bimport\s*\(\s*["']([^"'\n]+)["']\s*\)"#,
            r#"\brequire\s*\(\s*["']([^"'\n]+)["']\s*\)"#,
        ]
        .iter()
        .filter_map(|p| Regex::new(p).ok())
        .collect()
    })
}

fn collect_source_files(dir: &Path, root: &Path, out: &mut Vec<(String, PathBuf)>) -> bool {
    let Ok(entries) = fs::read_dir(dir) else {
        return true;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        if name == "node_modules" || name.to_string_lossy().starts_with('.') {
            continue;
        }
        if path.is_dir() {
            if !collect_source_files(&path, root, out) {
                return false;
            }
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| SOURCE_EXTS.contains(&e.to_lowercase().as_str()))
            && let Ok(rel) = path.strip_prefix(root)
        {
            out.push((rel.to_string_lossy().replace('\\', "/"), path));
            if out.len() > MAX_SCAN_FILES {
                return false;
            }
        }
    }
    true
}

fn validate_imports(root: &Path) -> Result<(), String> {
    let mut files = Vec::new();
    if !collect_source_files(root, root, &mut files) {
        return Err("Plugin has too many source files to compile safely".into());
    }

    for (rel, path) in files {
        let Ok(meta) = fs::metadata(&path) else {
            continue;
        };
        if meta.len() > MAX_SOURCE_BYTES {
            return Err(format!("'{rel}' is too large to compile"));
        }
        let Ok(source) = fs::read_to_string(&path) else {
            continue;
        };

        for re in specifier_regexes() {
            for caps in re.captures_iter(&source) {
                let Some(spec) = caps.get(1) else { continue };
                let spec = spec.as_str();

                if spec == SDK_MODULE {
                    continue;
                }
                if is_absolute_specifier(spec) {
                    return Err(format!(
                        "'{rel}' imports an absolute path, which is not allowed: {spec}"
                    ));
                }
                if spec.starts_with('.') {
                    resolve_relative(&rel, spec)?;
                }
            }
        }
    }
    Ok(())
}

fn write_sdk_shim(app: &AppHandle) -> Result<PathBuf, String> {
    let path = build_dir(app)?.join("plugin-sdk-shim.js");
    fs::write(&path, SDK_SHIM).map_err(|e| format!("Failed to write SDK shim: {e}"))?;
    Ok(path)
}

pub async fn compile(
    app: &AppHandle,
    id: &str,
    entry_file: &str,
) -> Result<CompiledPlugin, String> {
    let base = plugins::plugin_dir(app, id)?;
    let entry_path = plugins::resolve_in_plugin(&base, entry_file)?;
    if !entry_path.is_file() {
        return Err(format!("Plugin entry '{entry_file}' does not exist"));
    }

    validate_imports(&base)?;

    let esbuild = ensure_esbuild(app).await?;
    let shim = write_sdk_shim(app)?;

    let scratch = build_dir(app)?.join(format!("run-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&scratch).map_err(|e| format!("Failed to create scratch dir: {e}"))?;

    let outfile = scratch.join("bundle.js");
    let metafile = scratch.join("meta.json");

    let result = run_esbuild(
        &esbuild,
        &base,
        &entry_path,
        &shim,
        &outfile,
        &metafile,
    )
    .await;

    let outcome = match result {
        Ok(()) => read_outputs(&outfile, &metafile),
        Err(e) => Err(e),
    };

    let _ = fs::remove_dir_all(&scratch);
    outcome
}

async fn run_esbuild(
    bin: &str,
    base: &Path,
    entry: &Path,
    shim: &Path,
    outfile: &Path,
    metafile: &Path,
) -> Result<(), String> {
    let mut command = process::hidden_tokio(tokio::process::Command::new(bin));
    command
        .current_dir(base)
        .arg(entry)
        .arg("--bundle")
        .arg("--format=esm")
        .arg("--platform=browser")
        .arg("--target=es2020")
        .arg("--log-level=warning")
        .arg(format!("--alias:{SDK_MODULE}={}", shim.to_string_lossy()))
        .arg(format!("--outfile={}", outfile.to_string_lossy()))
        .arg(format!("--metafile={}", metafile.to_string_lossy()))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output = tokio::time::timeout(COMPILE_TIMEOUT, command.kill_on_drop(true).output())
        .await
        .map_err(|_| "esbuild timed out after 60s".to_string())?
        .map_err(|e| format!("Failed to run esbuild: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let trimmed: String = stderr
            .lines()
            .filter(|l| !l.contains("esbuild-wasm") && l.trim() != "✘")
            .take(24)
            .collect::<Vec<_>>()
            .join("\n");
        return Err(format!(
            "Plugin failed to compile: {}",
            if trimmed.trim().is_empty() {
                format!("esbuild exited with {}", output.status)
            } else {
                trimmed
            }
        ));
    }
    Ok(())
}

fn read_outputs(outfile: &Path, metafile: &Path) -> Result<CompiledPlugin, String> {
    let code = fs::read_to_string(outfile)
        .map_err(|e| format!("Failed to read esbuild output: {e}"))?;

    let mut deps: Vec<String> = util::read_json::<serde_json::Value>(metafile)
        .and_then(|meta| {
            meta.get("inputs")
                .and_then(|i| i.as_object())
                .map(|inputs| inputs.keys().cloned().collect())
        })
        .unwrap_or_default();

    deps.retain(|d| !d.starts_with("..") && !d.contains(':'));
    deps.sort();
    deps.dedup();

    if deps.is_empty() {
        deps.push("package.json".to_string());
    }

    Ok(CompiledPlugin { code, deps })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_sibling_and_nested_imports() {
        assert_eq!(resolve_relative("index.ts", "./helper").unwrap(), "helper");
        assert_eq!(
            resolve_relative("src/index.ts", "./util/x").unwrap(),
            "src/util/x"
        );
        assert_eq!(
            resolve_relative("src/lib/a.ts", "../b").unwrap(),
            "src/b"
        );
        assert_eq!(resolve_relative("index.ts", "./a/../b").unwrap(), "b");
    }

    #[test]
    fn rejects_imports_that_escape_the_plugin() {
        assert!(resolve_relative("index.ts", "../outside").is_err());
        assert!(resolve_relative("a/b.ts", "../../../etc/passwd").is_err());
        assert!(resolve_relative("index.ts", "..").is_err());
    }

    #[test]
    fn detects_absolute_specifiers() {
        assert!(is_absolute_specifier("/etc/passwd"));
        assert!(is_absolute_specifier("C:/secrets"));
        assert!(is_absolute_specifier("c:\\secrets"));
        assert!(is_absolute_specifier("\\\\server\\share"));
        assert!(!is_absolute_specifier("./relative"));
        assert!(!is_absolute_specifier("../relative"));
        assert!(!is_absolute_specifier("some-package"));
        assert!(!is_absolute_specifier("@scope/pkg"));
    }

    fn check_specifiers(source: &str, rel: &str) -> Result<(), String> {
        for re in specifier_regexes() {
            for caps in re.captures_iter(source) {
                let Some(spec) = caps.get(1) else { continue };
                let spec = spec.as_str();
                if spec == SDK_MODULE {
                    continue;
                }
                if is_absolute_specifier(spec) {
                    return Err(format!("'{rel}' imports an absolute path: {spec}"));
                }
                if spec.starts_with('.') {
                    resolve_relative(rel, spec)?;
                }
            }
        }
        Ok(())
    }

    #[test]
    fn allows_the_sdk_module_and_ordinary_imports() {
        assert!(check_specifiers(
            r#"import sdk from "@vynl/plugin-sdk"; import "./helper";"#,
            "index.ts"
        )
        .is_ok());
        assert!(check_specifiers(r#"import { x } from "some-pkg";"#, "index.ts").is_ok());
        assert!(check_specifiers(
            r#"const m = await import("./lazy");"#,
            "index.ts"
        )
        .is_ok());
    }

    #[test]
    fn blocks_absolute_and_escaping_imports_in_source() {
        assert!(check_specifiers(r#"import p from "/etc/passwd";"#, "index.ts").is_err());
        assert!(check_specifiers(r#"import p from "C:/win";"#, "index.ts").is_err());
        assert!(check_specifiers(r#"export { a } from "../../secret";"#, "a.ts").is_err());
        assert!(check_specifiers(r#"import("/abs/path");"#, "index.ts").is_err());
    }

    #[test]
    fn ignores_absolute_paths_that_are_not_imports() {
        assert!(check_specifiers(
            r#"const hint = "install to /usr/local/bin";"#,
            "index.ts"
        )
        .is_ok());
        assert!(
            check_specifiers("// see /etc/passwd for details", "index.ts").is_ok()
        );
    }
}
