use crate::commands::types::{ALL_TOOLS, ToolNameEnum, ToolState, ToolStatus};
use crate::services::{process, util};
use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::str::FromStr;
use std::sync::OnceLock;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;

const PROBE_TIMEOUT: Duration = Duration::from_secs(10);
const PROBE_CACHE_TTL: Duration = Duration::from_secs(10);
const PROGRESS_EMIT_STEP: f64 = 1.0;

fn bin_dir(user_data_dir: &Path) -> PathBuf {
    user_data_dir.join("bin")
}

pub fn user_data_search_roots(user_data_dir: &Path) -> Vec<PathBuf> {
    std::iter::once(user_data_dir.to_path_buf())
        .chain(legacy_user_data_dirs(user_data_dir))
        .collect()
}

fn legacy_user_data_dirs(current: &Path) -> Vec<PathBuf> {
    const LEGACY_IDS: &[&str] = &["vynl.app"];

    let Some(parent) = current.parent() else {
        return Vec::new();
    };

    LEGACY_IDS
        .iter()
        .map(|id| parent.join(id))
        .filter(|legacy| legacy != current && legacy.exists())
        .collect()
}

fn tmp_dir(user_data_dir: &Path) -> PathBuf {
    user_data_dir.join("tmp")
}

fn exe_name(name: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("{}.exe", name)
    } else {
        name.to_string()
    }
}

fn parse_tool(name: &str) -> Result<ToolNameEnum, String> {
    ToolNameEnum::from_str(name)
}

fn tool_exe(tool: ToolNameEnum) -> String {
    exe_name(tool.as_str())
}

fn run_which(shell_cmd: Command) -> Option<String> {
    let mut command = process::hidden_std(shell_cmd);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    let output = command.output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.lines().next()?.trim();
    if line.is_empty() {
        None
    } else {
        Some(line.to_string())
    }
}

#[cfg(not(target_os = "windows"))]
const FALLBACK_BIN_DIRS: &[&str] = &[
    "/usr/local/bin",
    "/usr/bin",
    "/bin",
    "/opt/homebrew/bin",
    "/home/linuxbrew/.linuxbrew/bin",
    "/snap/bin",
    "/var/lib/flatpak/exports/bin",
];

#[cfg(not(target_os = "windows"))]
fn which_in_login_shell(name: &str) -> Option<String> {
    let shell = "/bin/sh";
    let mut cmd = Command::new(shell);
    cmd.arg("-c").arg("command -v $1").arg("--").arg(name);
    run_which(cmd)
}

#[cfg(not(target_os = "windows"))]
fn which_in_common_dirs(name: &str) -> Option<String> {
    FALLBACK_BIN_DIRS
        .iter()
        .map(|dir| Path::new(dir).join(name))
        .find(|path| path.is_file())
        .map(|path| path.to_string_lossy().to_string())
}

fn which(name: &str) -> Option<String> {
    if cfg!(target_os = "windows") {
        let mut cmd = Command::new("where");
        cmd.arg(name);
        return run_which(cmd);
    }

    let mut cmd = Command::new("which");
    cmd.arg(name);
    if let Some(found) = run_which(cmd) {
        return Some(found);
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Some(found) = which_in_login_shell(name) {
            return Some(found);
        }
        if let Some(found) = which_in_common_dirs(name) {
            return Some(found);
        }
    }

    None
}

fn parse_version(tool: ToolNameEnum, raw: &str) -> Option<String> {
    let line = raw.trim();
    if line.is_empty() {
        return None;
    }

    let candidate = match tool {
        ToolNameEnum::Ffmpeg => {
            let rest = line.strip_prefix("ffmpeg version ")?;
            rest.split_whitespace().next()?
        }
        ToolNameEnum::YtDlp => line,
    };

    let token = candidate.trim_start_matches(['v', 'V']);
    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}

async fn version_of(bin: &str, args: &[&str]) -> Option<String> {
    let mut command = process::hidden_tokio(tokio::process::Command::new(bin));
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    let output = tokio::time::timeout(PROBE_TIMEOUT, command.output())
        .await
        .ok()?
        .ok()?;
    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let line = combined.lines().next()?.trim();
    if line.is_empty() {
        None
    } else {
        Some(line.to_string())
    }
}

fn resolve_binary(name: &str, user_data_dir: &Path) -> Option<String> {
    let exe = exe_name(name);

    for dir in user_data_search_roots(user_data_dir) {
        let local = bin_dir(&dir).join(&exe);
        if local.exists() {
            return Some(local.to_string_lossy().to_string());
        }
    }

    if let Some(system_path) = which(name) {
        return Some(system_path);
    }

    None
}

fn numeric_version(v: &str) -> Option<Vec<u32>> {
    let token = v
        .split(|c: char| !c.is_ascii_digit() && c != '.')
        .find(|t| !t.is_empty())?;
    let parts: Vec<u32> = token
        .split('.')
        .map(str::parse)
        .collect::<Result<_, _>>()
        .ok()?;
    (parts.len() >= 2).then_some(parts)
}

fn compare_versions(a: &[u32], b: &[u32]) -> Ordering {
    for i in 0..a.len().max(b.len()) {
        match a
            .get(i)
            .copied()
            .unwrap_or(0)
            .cmp(&b.get(i).copied().unwrap_or(0))
        {
            Ordering::Equal => continue,
            other => return other,
        }
    }
    Ordering::Equal
}

fn has_update(installed: &str, latest_tag: &str) -> Option<bool> {
    let latest = numeric_version(latest_tag)?;
    let current = numeric_version(installed)?;
    Some(compare_versions(&latest, &current) == Ordering::Greater)
}

fn update_repo(tool: ToolNameEnum) -> &'static str {
    match tool {
        ToolNameEnum::YtDlp => "yt-dlp/yt-dlp",
        ToolNameEnum::Ffmpeg => {
            if cfg!(target_os = "windows") {
                "GyanD/codexffmpeg"
            } else {
                "BtbN/FFmpeg-Builds"
            }
        }
    }
}

async fn download_file(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
    app_handle: &AppHandle,
    tool: ToolNameEnum,
) -> Result<(), String> {
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        let _ = tokio::fs::remove_file(dest).await;
        return Err(format!(
            "HTTP {status}: {}",
            body.chars().take(300).collect::<String>()
        ));
    }

    let total = resp.content_length().filter(|n| *n > 0);
    let mut received: u64 = 0;
    let mut last_emitted = f64::NEG_INFINITY;

    let mut stream = resp;
    let mut file = tokio::fs::File::create(dest)
        .await
        .map_err(|e| format!("Failed to create file: {e}"))?;
    emit_status(
        app_handle,
        &ToolStatus::downloading(tool, total.map(|_| 0.0)),
    );

    while let Some(chunk) = stream.chunk().await.map_err(|e| {
        let _ = std::fs::remove_file(dest);
        format!("Stream read failed: {e}")
    })? {
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("Write failed: {e}"))?;
        received += chunk.len() as u64;

        let Some(total) = total else { continue };
        let progress = (received as f64 / total as f64 * 100.0).min(100.0);
        if progress - last_emitted < PROGRESS_EMIT_STEP {
            continue;
        }
        last_emitted = progress;

        emit_status(app_handle, &ToolStatus::downloading(tool, Some(progress)));
    }

    file.flush()
        .await
        .map_err(|e| format!("Flush failed: {e}"))?;
    Ok(())
}

fn emit_status(app_handle: &AppHandle, status: &ToolStatus) {
    let _ = app_handle.emit("vynl:tools:update", status);
}

async fn check_single_tool(tool: ToolNameEnum, user_data_dir: PathBuf) -> ToolStatus {
    let name = tool.as_str();

    let bin = match tokio::task::spawn_blocking(move || resolve_binary(name, &user_data_dir)).await
    {
        Ok(bin) => bin,
        Err(_) => return ToolStatus::failed(tool, "probe failed"),
    };

    let Some(path) = bin else {
        return ToolStatus::new(tool, ToolState::Missing);
    };

    let version = version_of(&path, tool.version_args())
        .await
        .and_then(|raw| parse_version(tool, &raw));

    ToolStatus::new(tool, ToolState::Ok).found(path, version)
}

struct ProbeCache {
    user_data_dir: PathBuf,
    at: std::time::Instant,
    statuses: Vec<ToolStatus>,
}

fn probe_cache() -> &'static tokio::sync::Mutex<Option<ProbeCache>> {
    static CACHE: OnceLock<tokio::sync::Mutex<Option<ProbeCache>>> = OnceLock::new();
    CACHE.get_or_init(|| tokio::sync::Mutex::new(None))
}

async fn probe_all(
    user_data_dir: &Path,
    app_handle: &AppHandle,
    use_cache: bool,
) -> Vec<ToolStatus> {
    if use_cache {
        let guard = probe_cache().lock().await;
        if let Some(cache) = guard.as_ref()
            && cache.user_data_dir == user_data_dir
            && cache.at.elapsed() < PROBE_CACHE_TTL
        {
            return cache.statuses.clone();
        }
    }

    let dir = user_data_dir.to_path_buf();
    let mut set = tokio::task::JoinSet::new();
    for tool in ALL_TOOLS {
        set.spawn(check_single_tool(tool, dir.clone()));
    }

    let mut statuses = Vec::with_capacity(ALL_TOOLS.len());
    while let Some(joined) = set.join_next().await {
        let Ok(status) = joined else { continue };
        emit_status(app_handle, &status);
        statuses.push(status);
    }

    *probe_cache().lock().await = Some(ProbeCache {
        user_data_dir: user_data_dir.to_path_buf(),
        at: std::time::Instant::now(),
        statuses: statuses.clone(),
    });

    statuses
}

pub async fn check_tools(user_data_dir: &Path, app_handle: &AppHandle) -> Vec<ToolStatus> {
    probe_all(user_data_dir, app_handle, true).await
}

pub fn get_tool_path(name: &str, user_data_dir: &Path) -> Option<String> {
    resolve_binary(name, user_data_dir)
}

pub fn get_ffprobe_path(user_data_dir: &Path) -> Option<String> {
    let ffprobe_exe = exe_name("ffprobe");

    for dir in user_data_search_roots(user_data_dir) {
        let local = bin_dir(&dir).join(&ffprobe_exe);
        if local.exists() {
            return Some(local.to_string_lossy().to_string());
        }
    }

    if let Some(ffmpeg_path) = resolve_binary("ffmpeg", user_data_dir) {
        let ffmpeg_dir = Path::new(&ffmpeg_path).parent()?;
        let side = ffmpeg_dir.join(&ffprobe_exe);
        if side.exists() {
            return Some(side.to_string_lossy().to_string());
        }
    }

    if let Some(system_path) = which("ffprobe") {
        return Some(system_path);
    }

    None
}

struct ScratchDir(PathBuf);

impl ScratchDir {
    fn create(parent: &Path) -> Result<Self, String> {
        let unique = format!(
            "install-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or_default()
        );
        let path = parent.join(unique);
        fs::create_dir_all(&path).map_err(|e| format!("Failed to create temp dir: {e}"))?;
        Ok(Self(path))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

async fn promote(staged: &Path, dest: &Path) -> Result<(), String> {
    let (staged, dest) = (staged.to_path_buf(), dest.to_path_buf());
    tokio::task::spawn_blocking(move || -> std::io::Result<()> {
        fs::rename(&staged, &dest).or_else(|_| -> std::io::Result<()> {
            fs::copy(&staged, &dest)?;
            fs::remove_file(&staged)?;
            Ok(())
        })?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&dest, fs::Permissions::from_mode(0o755));
        }
        Ok(())
    })
    .await
    .map_err(|e| format!("Task join error: {e}"))?
    .map_err(|e| format!("Failed to install binary: {e}"))?;

    Ok(())
}

async fn install_ytdlp(
    client: &reqwest::Client,
    bd: &Path,
    td: &Path,
    app_handle: &AppHandle,
) -> Result<(), String> {
    let url = if cfg!(target_os = "windows") {
        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe"
    } else {
        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp"
    };
    let scratch = ScratchDir::create(td)?;
    let staged = scratch.path().join(tool_exe(ToolNameEnum::YtDlp));
    download_file(client, url, &staged, app_handle, ToolNameEnum::YtDlp).await?;
    promote(&staged, &bd.join(tool_exe(ToolNameEnum::YtDlp))).await
}

async fn latest_release(client: &reqwest::Client, repo: &str) -> Result<serde_json::Value, String> {
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch release info: {e}"))?;

    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| format!("Failed to read release info: {e}"))?;

    if !status.is_success() {
        return Err(format!(
            "GitHub returned {status} for {repo}: {}",
            body.chars().take(200).collect::<String>()
        ));
    }

    serde_json::from_str(&body).map_err(|e| format!("Invalid release JSON: {e}"))
}

fn pick_asset(release: &serde_json::Value, matches: impl Fn(&str) -> bool) -> Option<String> {
    release["assets"]
        .as_array()?
        .iter()
        .find(|a| matches(a["name"].as_str().unwrap_or("")))?
        .get("browser_download_url")?
        .as_str()
        .map(str::to_string)
}

fn windows_ffmpeg_asset(name: &str) -> bool {
    name.ends_with("-essentials_build.zip") && !name.contains("-x64_shared-")
}

fn linux_ffmpeg_asset(name: &str) -> bool {
    name.contains("linux64-gpl")
        && name.ends_with(".tar.xz")
        && !name.contains("shared")
        && !name.contains("arm64")
}

async fn install_ffmpeg_windows(
    client: &reqwest::Client,
    bd: &Path,
    td: &Path,
    app_handle: &AppHandle,
) -> Result<(), String> {
    let release = latest_release(client, "GyanD/codexffmpeg").await?;
    let download_url =
        pick_asset(&release, windows_ffmpeg_asset).ok_or("No ffmpeg essentials build found")?;

    let scratch = ScratchDir::create(td)?;
    let zip_path = scratch.path().join("ffmpeg.zip");
    download_file(
        client,
        &download_url,
        &zip_path,
        app_handle,
        ToolNameEnum::Ffmpeg,
    )
    .await?;

    let ffmpeg = scratch.path().join(exe_name("ffmpeg"));
    let ffprobe = scratch.path().join(exe_name("ffprobe"));
    extract_ffmpeg_from_zip(&ffmpeg, &ffprobe, &zip_path)?;

    promote(&ffmpeg, &bd.join(exe_name("ffmpeg"))).await?;
    promote(&ffprobe, &bd.join(exe_name("ffprobe"))).await
}

fn extract_ffmpeg_from_zip(ffmpeg: &Path, ffprobe: &Path, zip_path: &Path) -> Result<(), String> {
    let zip_file = fs::File::open(zip_path).map_err(|e| format!("Failed to open zip: {e}"))?;
    let mut archive =
        zip::ZipArchive::new(zip_file).map_err(|e| format!("Failed to read zip: {e}"))?;

    let mut found_ffmpeg = false;
    let mut found_ffprobe = false;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("Failed to read zip entry: {e}"))?;
        let entry_name = entry.name().to_string();

        let is_ffmpeg = entry_name.to_lowercase().ends_with("/bin/ffmpeg.exe");
        let is_ffprobe = entry_name.to_lowercase().ends_with("/bin/ffprobe.exe");

        if is_ffmpeg || is_ffprobe {
            let out_path = if is_ffmpeg {
                found_ffmpeg = true;
                ffmpeg
            } else {
                found_ffprobe = true;
                ffprobe
            };

            let mut out_file = fs::File::create(out_path)
                .map_err(|e| format!("Failed to create {}: {e}", out_path.display()))?;
            std::io::copy(&mut entry, &mut out_file)
                .map_err(|e| format!("Failed to extract {}: {e}", out_path.display()))?;
        }
    }

    if !found_ffmpeg {
        return Err("ffmpeg not found in archive".into());
    }
    if !found_ffprobe {
        return Err("ffprobe not found in archive".into());
    }
    Ok(())
}

async fn install_ffmpeg_linux(
    client: &reqwest::Client,
    bd: &Path,
    td: &Path,
    app_handle: &AppHandle,
) -> Result<(), String> {
    let release = latest_release(client, "BtbN/FFmpeg-Builds").await?;
    let download_url =
        pick_asset(&release, linux_ffmpeg_asset).ok_or("No ffmpeg linux build found")?;

    let scratch = ScratchDir::create(td)?;
    let tar_path = scratch.path().join("ffmpeg.tar.xz");
    download_file(
        client,
        &download_url,
        &tar_path,
        app_handle,
        ToolNameEnum::Ffmpeg,
    )
    .await?;

    let ffmpeg = scratch.path().join(exe_name("ffmpeg"));
    let ffprobe = scratch.path().join(exe_name("ffprobe"));
    extract_ffmpeg_from_tar(&ffmpeg, &ffprobe, &tar_path).await?;

    promote(&ffmpeg, &bd.join(exe_name("ffmpeg"))).await?;
    promote(&ffprobe, &bd.join(exe_name("ffprobe"))).await
}

async fn extract_ffmpeg_from_tar(
    ffmpeg: &Path,
    ffprobe: &Path,
    tar_path: &Path,
) -> Result<(), String> {
    let ffmpeg = ffmpeg.to_path_buf();
    let ffprobe = ffprobe.to_path_buf();
    let tar_path = tar_path.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let extract_dir = tar_path.with_extension("extracted");
        fs::create_dir_all(&extract_dir)
            .map_err(|e| format!("Failed to create extraction dir: {e}"))?;

        let output = Command::new("tar")
            .args([
                "xf",
                tar_path.to_string_lossy().as_ref(),
                "-C",
                extract_dir.to_string_lossy().as_ref(),
            ])
            .output()
            .map_err(|e| format!("Failed to run 'tar' (is it installed?): {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("xz") || stderr.contains("unrecognized option") {
                return Err("Failed to extract ffmpeg archive: xz support not found. Install xz-utils (e.g. sudo apt install xz-utils)".into());
            }
            return Err(format!("Failed to extract ffmpeg archive: {stderr}"));
        }

        let mut found_ffmpeg = false;
        let mut found_ffprobe = false;

        if let Ok(entries) = fs::read_dir(&extract_dir) {
            for entry in entries.flatten() {
                let bin_dir = entry.path().join("bin");
                if !bin_dir.is_dir() {
                    continue;
                }
                let ffmpeg_bin = bin_dir.join("ffmpeg");
                let ffprobe_bin = bin_dir.join("ffprobe");
                if ffmpeg_bin.exists() {
                    fs::copy(&ffmpeg_bin, &ffmpeg)
                        .map_err(|e| format!("Failed to copy ffmpeg: {e}"))?;
                    found_ffmpeg = true;
                }
                if ffprobe_bin.exists() {
                    fs::copy(&ffprobe_bin, &ffprobe)
                        .map_err(|e| format!("Failed to copy ffprobe: {e}"))?;
                    found_ffprobe = true;
                }
                if found_ffmpeg && found_ffprobe {
                    break;
                }
            }
        }

        if !found_ffmpeg {
            return Err("ffmpeg not found in archive".into());
        }
        if !found_ffprobe {
            return Err("ffprobe not found in archive".into());
        }
        Ok(())
    })
    .await
    .map_err(|e| format!("Task join error: {e}"))?
}

fn install_lock() -> &'static tokio::sync::Mutex<()> {
    static LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
}

pub async fn install_tool(
    user_data_dir: &Path,
    name: &str,
    app_handle: AppHandle,
) -> Result<(), String> {
    let tool = parse_tool(name)?;
    let _guard = install_lock().lock().await;
    emit_status(&app_handle, &ToolStatus::downloading(tool, Some(0.0)));

    let bd = bin_dir(user_data_dir);
    let td = tmp_dir(user_data_dir);
    fs::create_dir_all(&bd).map_err(|e| format!("Failed to create bin dir: {e}"))?;
    fs::create_dir_all(&td).map_err(|e| format!("Failed to create tmp dir: {e}"))?;

    let client = util::github_client(Duration::from_secs(30))?;

    match tool {
        ToolNameEnum::YtDlp => {
            install_ytdlp(&client, &bd, &td, &app_handle).await?;
        }
        ToolNameEnum::Ffmpeg => {
            if cfg!(target_os = "windows") {
                install_ffmpeg_windows(&client, &bd, &td, &app_handle).await?;
            } else {
                install_ffmpeg_linux(&client, &bd, &td, &app_handle).await?;
            }
        }
    }

    let final_statuses = probe_all(user_data_dir, &app_handle, false).await;
    final_statuses
        .iter()
        .find(|s| s.name == tool)
        .ok_or_else(|| format!("Tool {name} not found after install"))?;

    Ok(())
}

pub async fn check_for_updates(user_data_dir: &Path, app_handle: &AppHandle) -> Vec<ToolStatus> {
    let mut statuses = check_tools(user_data_dir, app_handle).await;

    let Ok(client) = util::github_client(Duration::from_secs(30)) else {
        return statuses;
    };

    for tool in ALL_TOOLS {
        let Some(status) = statuses
            .iter_mut()
            .find(|s| s.name == tool && s.installed && s.version.is_some())
        else {
            continue;
        };

        let Some(current) = status.version.clone() else {
            continue;
        };
        if numeric_version(&current).is_none() {
            continue;
        }

        let Ok(release) = latest_release(&client, update_repo(tool)).await else {
            continue;
        };
        let Some(latest_tag) = release["tag_name"].as_str() else {
            continue;
        };
        let Some(available) = has_update(&current, latest_tag) else {
            continue;
        };

        status.update_available = Some(available);
        emit_status(app_handle, status);
    }

    statuses
}
