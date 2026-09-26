use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde_json::Value;
use tokio::sync::Mutex as AsyncMutex;

use crate::services::{process, tools};

use super::search::CANCELLED_FIND;
use super::{GENERATION, LAST_ERROR_RE, PROGRESS_RE};

pub(super) static CANCELLED_DOWNLOAD: AtomicBool = AtomicBool::new(false);

pub(super) static CHILDREN: once_cell::sync::Lazy<AsyncMutex<Vec<(u32, u64)>>> =
    once_cell::sync::Lazy::new(|| AsyncMutex::new(Vec::new()));

pub(super) const DOWNLOAD_STALL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(180);

pub(super) fn fmt_dur(sec: f64) -> String {
    let m = (sec / 60.0).floor() as u64;
    let s = (sec % 60.0).round() as u64;
    format!("{}:{:02}", m, s)
}

pub(super) fn progress_from_line(line: &str) -> Option<f64> {
    PROGRESS_RE
        .captures(line)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse::<f64>().ok())
        .map(|p| p.min(100.0))
}

pub(super) fn last_error(stderr: &str) -> String {
    let lines: Vec<&str> = stderr.lines().collect();
    let error_lines: Vec<&str> = lines
        .iter()
        .filter(|l| l.starts_with("ERROR:") || l.to_lowercase().contains("error"))
        .copied()
        .collect();
    if !error_lines.is_empty() {
        let last = error_lines.last().unwrap();
        if let Some(caps) = LAST_ERROR_RE.captures(last) {
            return caps.get(1).unwrap().as_str().trim().to_string();
        }
        return last.trim().to_string();
    }
    lines
        .iter()
        .rev()
        .find(|l| !l.trim().is_empty())
        .map(|l| l.trim().to_string())
        .unwrap_or_else(|| "Unknown error".to_string())
}

fn output_with_timeout(
    mut command: std::process::Command,
    timeout: std::time::Duration,
) -> Option<std::process::Output> {
    use std::io::Read;

    let mut child = command.spawn().ok()?;
    let stdout_pipe = child.stdout.take();
    let stderr_pipe = child.stderr.take();

    let out_reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut pipe) = stdout_pipe {
            let _ = pipe.read_to_end(&mut buf);
        }
        buf
    });
    let err_reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut pipe) = stderr_pipe {
            let _ = pipe.read_to_end(&mut buf);
        }
        buf
    });

    let start = std::time::Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if start.elapsed() < timeout => {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Ok(None) | Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = out_reader.join();
                let _ = err_reader.join();
                return None;
            }
        }
    };

    Some(std::process::Output {
        status,
        stdout: out_reader.join().unwrap_or_default(),
        stderr: err_reader.join().unwrap_or_default(),
    })
}

pub(super) fn run_yt_json(
    cmd: &str,
    args: &[String],
    timeout: std::time::Duration,
) -> Option<Value> {
    let mut command = process::hidden_std(std::process::Command::new(cmd));
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let output = output_with_timeout(command, timeout)?;
    if output.status.code() == Some(0) {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let trimmed = stdout.trim();
        if !trimmed.is_empty() {
            serde_json::from_str(trimmed).ok()
        } else {
            None
        }
    } else {
        None
    }
}

pub fn resolve_tool(name: &str, user_data_dir: &Path) -> Option<String> {
    tools::get_tool_path(name, user_data_dir)
}

pub(super) fn terminate_process(id: u32) {
    if id == 0 {
        return;
    }
    #[cfg(windows)]
    {
        unsafe {
            use std::os::windows::raw::HANDLE;
            const PROCESS_TERMINATE: u32 = 0x0001;
            extern "system" {
                fn OpenProcess(
                    dwDesiredAccess: u32,
                    bInheritHandle: i32,
                    dwProcessId: u32,
                ) -> HANDLE;
                fn TerminateProcess(hProcess: HANDLE, uExitCode: u32) -> i32;
                fn CloseHandle(hObject: HANDLE) -> i32;
            }
            let h = OpenProcess(PROCESS_TERMINATE, 0, id);
            if !h.is_null() {
                let _ = TerminateProcess(h, 1);
                let _ = CloseHandle(h);
            }
        }
    }
    #[cfg(not(windows))]
    {
        unsafe {
            libc::kill(id as i32, libc::SIGTERM);
        }
    }
}

pub async fn cancel_download() {
    CANCELLED_FIND.store(true, Ordering::SeqCst);
    CANCELLED_DOWNLOAD.store(true, Ordering::SeqCst);
    let gen = GENERATION.load(Ordering::SeqCst);
    let mut children = CHILDREN.lock().await;
    for &(id, child_gen) in children.iter() {
        if child_gen != gen {
            continue;
        }
        terminate_process(id);
    }
    children.retain(|&(_, g)| g != gen);
}

pub(super) async fn watch_for_stall(
    id: u32,
    last_activity: Arc<std::sync::Mutex<std::time::Instant>>,
    stalled: Arc<AtomicBool>,
    done: Arc<AtomicBool>,
) {
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        if done.load(Ordering::Relaxed) {
            return;
        }
        let idle = last_activity
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .elapsed();
        if idle >= DOWNLOAD_STALL_TIMEOUT {
            if done.load(Ordering::Relaxed) {
                return;
            }
            eprintln!(
                "[downloader] yt-dlp pid {id} stalled for {}s, terminating",
                idle.as_secs()
            );
            stalled.store(true, Ordering::Relaxed);
            terminate_process(id);
            return;
        }
    }
}
