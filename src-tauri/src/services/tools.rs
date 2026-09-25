use crate::commands::types::ToolStatus;
use crate::services::process;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;

const GH_UA: &str = "Vynl/1.0";

fn bin_dir(user_data_dir: &Path) -> PathBuf {
    user_data_dir.join("bin")
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

fn user_data_search_roots(user_data_dir: &Path) -> Vec<PathBuf> {
    std::iter::once(user_data_dir.to_path_buf())
        .chain(legacy_user_data_dirs(user_data_dir))
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

fn version_of(bin: &str, args: &[&str]) -> Option<String> {
    let mut command = process::hidden_std(Command::new(bin));
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output = command.output().ok()?;
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

    if let Some(resource_dir) = dirs::data_dir() {
        let bundled = resource_dir.join("bin").join(&exe);
        if bundled.exists() {
            return Some(bundled.to_string_lossy().to_string());
        }
    }

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

fn version_nums(v: &str) -> Vec<u32> {
    v.split('.').filter_map(|s| s.parse::<u32>().ok()).collect()
}

fn is_newer(latest: &str, current: &str) -> bool {
    let a = version_nums(latest);
    let b = version_nums(current);
    let len = a.len().max(b.len());
    for i in 0..len {
        let x = a.get(i).copied().unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        if x > y {
            return true;
        }
        if x < y {
            return false;
        }
    }
    false
}

async fn download_file(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
    app_handle: &AppHandle,
    tool_name: &str,
) -> Result<(), String> {
    let resp = client
        .get(url)
        .header("User-Agent", GH_UA)
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

    let total = resp.content_length().unwrap_or(0);
    let mut received: u64 = 0;

    let mut stream = resp;
    let mut file = tokio::fs::File::create(dest)
        .await
        .map_err(|e| format!("Failed to create file: {e}"))?;
    emit_status(
        app_handle,
        &ToolStatus {
            name: if tool_name == "yt-dlp" {
                crate::commands::types::ToolNameEnum::YtDlp
            } else {
                crate::commands::types::ToolNameEnum::Ffmpeg
            },
            installed: false,
            path: None,
            version: None,
            state: crate::commands::types::ToolState::Downloading,
            progress: Some(0.0),
            error: None,
            update_available: None,
        },
    );

    while let Some(chunk) = stream.chunk().await.map_err(|e| {
        let _ = std::fs::remove_file(dest);
        format!("Stream read failed: {e}")
    })? {
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("Write failed: {e}"))?;
        received += chunk.len() as u64;
        let progress = if total > 0 {
            (received as f64 / total as f64 * 100.0).min(100.0)
        } else {
            (received as f64 / (1024.0 * 1024.0 * 20.0) * 95.0).min(95.0)
        };
        let status = ToolStatus {
            name: if tool_name == "yt-dlp" {
                crate::commands::types::ToolNameEnum::YtDlp
            } else {
                crate::commands::types::ToolNameEnum::Ffmpeg
            },
            installed: false,
            path: None,
            version: None,
            state: crate::commands::types::ToolState::Downloading,
            progress: Some(progress),
            error: None,
            update_available: None,
        };
        emit_status(app_handle, &status);
    }

    file.flush()
        .await
        .map_err(|e| format!("Flush failed: {e}"))?;
    Ok(())
}

fn emit_status(app_handle: &AppHandle, status: &ToolStatus) {
    let _ = app_handle.emit("vynl:tools:update", status);
}

fn check_single_tool(name: &str, user_data_dir: &Path) -> ToolStatus {
    let tool_name_enum = match name {
        "yt-dlp" => crate::commands::types::ToolNameEnum::YtDlp,
        "ffmpeg" => crate::commands::types::ToolNameEnum::Ffmpeg,
        _ => crate::commands::types::ToolNameEnum::YtDlp,
    };
    let bin = resolve_binary(name, user_data_dir);
    match bin {
        Some(path) => {
            let args = if name == "yt-dlp" {
                vec!["--version"]
            } else {
                vec!["-version"]
            };
            match version_of(&path, &args) {
                Some(version) => ToolStatus {
                    name: tool_name_enum,
                    installed: true,
                    path: Some(path),
                    version: Some(version),
                    state: crate::commands::types::ToolState::Ok,
                    progress: None,
                    error: None,
                    update_available: None,
                },
                None => ToolStatus {
                    name: tool_name_enum,
                    installed: true,
                    path: Some(path),
                    version: None,
                    state: crate::commands::types::ToolState::Error,
                    progress: None,
                    error: Some("binary found but failed to run".into()),
                    update_available: None,
                },
            }
        }
        None => ToolStatus {
            name: tool_name_enum,
            installed: false,
            path: None,
            version: None,
            state: crate::commands::types::ToolState::Missing,
            progress: None,
            error: None,
            update_available: None,
        },
    }
}

pub async fn check_tools(user_data_dir: &Path, app_handle: &AppHandle) -> Vec<ToolStatus> {
    let mut statuses = Vec::new();

    for name in &["yt-dlp", "ffmpeg"] {
        let status = check_single_tool(name, user_data_dir);
        emit_status(app_handle, &status);
        statuses.push(status);
    }

    statuses
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

pub fn covers_search_dirs(user_data_dir: &Path) -> Vec<PathBuf> {
    user_data_search_roots(user_data_dir)
        .into_iter()
        .map(|dir| dir.join("covers"))
        .collect()
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
    let tmp_path = td.join("yt-dlp.part");
    download_file(client, url, &tmp_path, app_handle, "yt-dlp").await?;

    let dest = bd.join(exe_name("yt-dlp"));
    let tmp = tmp_path.clone();
    let dest_for_task = dest.clone();
    tokio::task::spawn_blocking(move || {
        fs::rename(&tmp, &dest_for_task).or_else(|_| {
            fs::copy(&tmp, &dest_for_task)?;
            fs::remove_file(&tmp)?;
            Ok(())
        })
    })
    .await
    .map_err(|e| format!("Task join error: {e}"))?
    .map_err(|e: std::io::Error| format!("Failed to move binary: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&dest, fs::Permissions::from_mode(0o755));
    }
    Ok(())
}

async fn install_ffmpeg_windows(
    client: &reqwest::Client,
    bd: &Path,
    td: &Path,
    app_handle: &AppHandle,
) -> Result<(), String> {
    let release_url = "https://api.github.com/repos/GyanD/codexffmpeg/releases/latest";
    let release_body = client
        .get(release_url)
        .header("User-Agent", GH_UA)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch release info: {e}"))?
        .text()
        .await
        .map_err(|e| format!("Failed to read release body: {e}"))?;

    let release: serde_json::Value =
        serde_json::from_str(&release_body).map_err(|e| format!("Invalid release JSON: {e}"))?;

    let assets = release["assets"].as_array().ok_or("No assets in release")?;

    let asset = assets
        .iter()
        .find(|a| {
            let name = a["name"].as_str().unwrap_or("");
            name.ends_with("-essentials_build.zip") && !name.contains("-x64_shared-")
        })
        .ok_or("No ffmpeg essentials build found")?;

    let download_url = asset["browser_download_url"]
        .as_str()
        .ok_or("Missing download URL")?;

    let zip_path = td.join("ffmpeg.zip");
    download_file(client, download_url, &zip_path, app_handle, "ffmpeg").await?;

    extract_ffmpeg_from_zip(bd, &zip_path)
}

fn extract_ffmpeg_from_zip(bd: &Path, zip_path: &Path) -> Result<(), String> {
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
            let out_name = if is_ffmpeg {
                found_ffmpeg = true;
                exe_name("ffmpeg")
            } else {
                found_ffprobe = true;
                exe_name("ffprobe")
            };

            let out_path = bd.join(&out_name);
            let mut out_file = fs::File::create(&out_path)
                .map_err(|e| format!("Failed to create {out_name}: {e}"))?;
            std::io::copy(&mut entry, &mut out_file)
                .map_err(|e| format!("Failed to extract {out_name}: {e}"))?;
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
    let release_url = "https://api.github.com/repos/BtbN/FFmpeg-Builds/releases/latest";
    let release_body = client
        .get(release_url)
        .header("User-Agent", GH_UA)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch release info: {e}"))?
        .text()
        .await
        .map_err(|e| format!("Failed to read release body: {e}"))?;

    let release: serde_json::Value =
        serde_json::from_str(&release_body).map_err(|e| format!("Invalid release JSON: {e}"))?;

    let assets = release["assets"].as_array().ok_or("No assets in release")?;

    let asset = assets
        .iter()
        .find(|a| {
            let name = a["name"].as_str().unwrap_or("");
            name.contains("linux64-gpl")
                && name.ends_with(".tar.xz")
                && !name.contains("shared")
                && !name.contains("arm64")
        })
        .ok_or("No ffmpeg linux build found")?;

    let download_url = asset["browser_download_url"]
        .as_str()
        .ok_or("Missing download URL")?;

    let tar_path = td.join("ffmpeg.tar.xz");
    download_file(client, download_url, &tar_path, app_handle, "ffmpeg").await?;

    extract_ffmpeg_from_tar(bd, td, &tar_path).await
}

async fn extract_ffmpeg_from_tar(bd: &Path, td: &Path, tar_path: &Path) -> Result<(), String> {
    let bd = bd.to_path_buf();
    let td = td.to_path_buf();
    let tar_path = tar_path.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let output = Command::new("tar")
            .args(["xf", tar_path.to_string_lossy().as_ref(), "-C", td.to_string_lossy().as_ref()])
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

        if let Ok(entries) = fs::read_dir(&td) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let bin_dir = path.join("bin");
                    if bin_dir.exists() {
                        let ffmpeg_bin = bin_dir.join("ffmpeg");
                        let ffprobe_bin = bin_dir.join("ffprobe");
                        if ffmpeg_bin.exists() {
                            let dest = bd.join("ffmpeg");
                            fs::copy(&ffmpeg_bin, &dest)
                                .map_err(|e| format!("Failed to copy ffmpeg: {e}"))?;
                            found_ffmpeg = true;
                        }
                        if ffprobe_bin.exists() {
                            let dest = bd.join("ffprobe");
                            fs::copy(&ffprobe_bin, &dest)
                                .map_err(|e| format!("Failed to copy ffprobe: {e}"))?;
                            found_ffprobe = true;
                        }
                        break;
                    }
                }
            }
        }

        if !found_ffmpeg {
            return Err("ffmpeg not found in archive".into());
        }
        if !found_ffprobe {
            return Err("ffprobe not found in archive".into());
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(bd.join("ffmpeg"), fs::Permissions::from_mode(0o755));
            let _ = fs::set_permissions(bd.join("ffprobe"), fs::Permissions::from_mode(0o755));
        }
        Ok(())
    })
    .await
    .map_err(|e| format!("Task join error: {e}"))?
}

pub async fn install_tool(
    user_data_dir: &Path,
    name: &str,
    app_handle: AppHandle,
) -> Result<(), String> {
    let status = ToolStatus {
        name: match name {
            "yt-dlp" => crate::commands::types::ToolNameEnum::YtDlp,
            "ffmpeg" => crate::commands::types::ToolNameEnum::Ffmpeg,
            _ => crate::commands::types::ToolNameEnum::YtDlp,
        },
        installed: false,
        path: None,
        version: None,
        state: crate::commands::types::ToolState::Downloading,
        progress: Some(0.0),
        error: None,
        update_available: None,
    };
    emit_status(&app_handle, &status);

    let bd = bin_dir(user_data_dir);
    let td = tmp_dir(user_data_dir);
    fs::create_dir_all(&bd).map_err(|e| format!("Failed to create bin dir: {e}"))?;
    fs::create_dir_all(&td).map_err(|e| format!("Failed to create tmp dir: {e}"))?;

    let client = reqwest::Client::builder()
        .user_agent(GH_UA)
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))?;

    match name {
        "yt-dlp" => {
            install_ytdlp(&client, &bd, &td, &app_handle).await?;
        }
        "ffmpeg" => {
            if cfg!(target_os = "windows") {
                install_ffmpeg_windows(&client, &bd, &td, &app_handle).await?;
            } else {
                install_ffmpeg_linux(&client, &bd, &td, &app_handle).await?;
            }
        }
        _ => {
            return Err(format!("Unknown tool: {name}"));
        }
    }

    let final_statuses = check_tools(user_data_dir, &app_handle).await;
    final_statuses
        .iter()
        .find(|s| match &s.name {
            crate::commands::types::ToolNameEnum::YtDlp => name == "yt-dlp",
            crate::commands::types::ToolNameEnum::Ffmpeg => name == "ffmpeg",
        })
        .cloned()
        .ok_or_else(|| format!("Tool {name} not found after install"))
        .map(|_| ())
}

async fn fetch_latest_tag(client: &reqwest::Client, repo: &str) -> Option<String> {
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
    let body = client
        .get(&url)
        .header("User-Agent", GH_UA)
        .send()
        .await
        .ok()?
        .text()
        .await
        .ok()?;
    let json: serde_json::Value = serde_json::from_str(&body).ok()?;
    json["tag_name"].as_str().map(|s| s.to_string())
}

pub async fn check_for_updates(user_data_dir: &Path, app_handle: &AppHandle) -> Vec<ToolStatus> {
    let mut statuses = check_tools(user_data_dir, app_handle).await;

    let client = reqwest::Client::builder()
        .user_agent(GH_UA)
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .unwrap_or_default();

    let ffmpeg_repo = if cfg!(target_os = "windows") {
        "GyanD/codexffmpeg"
    } else {
        "BtbN/FFmpeg-Builds"
    };
    let repos = [("yt-dlp", "yt-dlp/yt-dlp"), ("ffmpeg", ffmpeg_repo)];

    for status in &mut statuses {
        if !status.installed || status.version.is_none() {
            continue;
        }

        if let Some(repo) = repos.iter().find(|(n, _)| match status.name {
            crate::commands::types::ToolNameEnum::YtDlp => *n == "yt-dlp",
            crate::commands::types::ToolNameEnum::Ffmpeg => *n == "ffmpeg",
        }) {
            if let Some(latest_tag) = fetch_latest_tag(&client, repo.1).await {
                let update_available = match status.name {
                    crate::commands::types::ToolNameEnum::Ffmpeg => None,
                    crate::commands::types::ToolNameEnum::YtDlp => Some(is_newer(
                        &latest_tag,
                        status.version.as_deref().unwrap_or(""),
                    )),
                };
                status.update_available = update_available;
            }
        }

        emit_status(app_handle, status);
    }

    statuses
}
