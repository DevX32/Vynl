use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use reqwest::header::{ACCEPT, RANGE, USER_AGENT};
use symphonia::core::io::MediaSource;
use ytmusic::AudioFormat as YtAudioFormat;

use super::cache::StreamCache;
use super::resolver::UrlResolver;

pub(super) const FETCH_CHUNK: u64 = 256 * 1024;
pub(super) const READAHEAD: u64 = 1024 * 1024;
pub(super) const FETCH_STALL: Duration = Duration::from_secs(15);
pub(super) const MAX_FETCH_ATTEMPTS: u32 = 4;
pub(super) const MAX_REFRESH_ATTEMPTS: u32 = 3;
pub(super) const MAX_REDIRECT_HOPS: usize = 6;


pub(super) struct SessionControl {
    pub(super) cancelled: AtomicBool,
    pub(super) complete: AtomicBool,
    pub(super) fetched: AtomicU64,
}

impl SessionControl {
    pub(super) fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub(super) fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }
}

pub(super) struct RangeState {
    pub(super) coverage: Vec<(u64, u64)>,
    pub(super) total: u64,
    pub(super) demand_start: u64,
    pub(super) demand_end: u64,
    pub(super) demand_gen: u64,
    pub(super) error: Option<String>,
}

pub struct RangeStore {
    pub(super) state: Mutex<RangeState>,
    pub(super) cv: Condvar,
    pub(super) control: Arc<SessionControl>,
    pub(super) sink: Option<(Arc<StreamCache>, String, String)>,
}

impl RangeStore {
    pub(super) fn new(
        total: u64,
        control: Arc<SessionControl>,
        coverage: Vec<(u64, u64)>,
        sink: Option<(Arc<StreamCache>, String, String)>,
    ) -> Self {
        let mut merged = Vec::new();
        let mut sorted = coverage;
        sorted.sort_unstable();
        for (start, end) in sorted {
            mark_range(&mut merged, start, end);
        }
        Self {
            state: Mutex::new(RangeState {
                coverage: merged,
                total,
                demand_start: 0,
                demand_end: 0,
                demand_gen: 0,
                error: None,
            }),
            cv: Condvar::new(),
            control,
            sink,
        }
    }

    pub(super) fn covered(state: &RangeState, start: u64, end: u64) -> bool {
        state.coverage.iter().any(|&(a, b)| a <= start && end <= b)
    }

    pub(super) fn mark(&self, start: u64, end: u64) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        mark_range(&mut state.coverage, start, end);
        if !self.control.complete.load(Ordering::Relaxed) && Self::covered(&state, 0, state.total) {
            self.control.complete.store(true, Ordering::SeqCst);
        }
        if let Some((cache, video_id, file_name)) = &self.sink {
            cache.record_coverage(video_id, file_name, &state.coverage);
        }
        self.cv.notify_all();
    }

    pub(super) fn ensure(&self, start: u64, end: u64) -> Result<(), String> {
        let end = end.min(self.state.lock().unwrap_or_else(|e| e.into_inner()).total);
        if start >= end {
            return Ok(());
        }
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if Self::covered(&state, start, end) {
            return Ok(());
        }
        state.demand_start = start;
        state.demand_end = end;
        state.demand_gen = state.demand_gen.wrapping_add(1);
        self.cv.notify_all();

        loop {
            if Self::covered(&state, start, end) {
                return Ok(());
            }
            if let Some(error) = &state.error {
                return Err(error.clone());
            }
            if self.control.is_cancelled() {
                return Err("stream cancelled".into());
            }
            let (guard, _) = self
                .cv
                .wait_timeout(state, Duration::from_millis(100))
                .unwrap_or_else(|e| e.into_inner());
            state = guard;
        }
    }

    pub(super) fn wait_demand(&self, last_gen: u64) -> Option<(u64, u64, u64)> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        loop {
            if self.control.is_cancelled() || state.error.is_some() {
                return None;
            }
            if state.demand_gen != last_gen
                && !Self::covered(&state, state.demand_start, state.demand_end)
            {
                return Some((state.demand_start, state.demand_end, state.demand_gen));
            }
            let (guard, _) = self
                .cv
                .wait_timeout(state, Duration::from_millis(200))
                .unwrap_or_else(|e| e.into_inner());
            state = guard;
        }
    }

    pub(super) fn fail(&self, message: String) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if state.error.is_none() {
            state.error = Some(message);
        }
        self.cv.notify_all();
    }

    pub(super) fn is_complete(&self) -> bool {
        self.control.complete.load(Ordering::Relaxed)
    }
}

pub(super) fn mark_range(ranges: &mut Vec<(u64, u64)>, start: u64, end: u64) {
    if start >= end {
        return;
    }
    ranges.push((start, end));
    ranges.sort_unstable();
    let mut merged: Vec<(u64, u64)> = Vec::with_capacity(ranges.len());
    for (a, b) in ranges.drain(..) {
        if let Some(last) = merged.last_mut()
            && a <= last.1
        {
            last.1 = last.1.max(b);
            continue;
        }
        merged.push((a, b));
    }
    *ranges = merged;
}

pub(super) fn first_gap(ranges: &[(u64, u64)], start: u64, end: u64) -> Option<u64> {
    let mut at = start;
    for &(a, b) in ranges {
        if b <= at {
            continue;
        }
        if a > at {
            return Some(at);
        }
        if b >= end {
            return None;
        }
        at = b;
    }
    if at < end { Some(at) } else { None }
}

pub(super) fn fetch_loop(
    store: Arc<RangeStore>,
    resolver: Arc<UrlResolver>,
    video_id: String,
    mut format: YtAudioFormat,
    mut file: File,
    refresh_count: Arc<std::sync::atomic::AtomicU32>,
) {
    let mut last_gen = 0u64;
    let mut attempts = 0u32;
    let control = store.control.clone();

    loop {
        let Some((demand_start, demand_end, generation)) = store.wait_demand(last_gen) else {
            return;
        };
        last_gen = generation;

        'demand: loop {
            if control.is_cancelled() {
                return;
            }
            let total = {
                let state = store.state.lock().unwrap_or_else(|e| e.into_inner());
                state.total
            };
            let snapshot = {
                let state = store.state.lock().unwrap_or_else(|e| e.into_inner());
                state.coverage.clone()
            };
            let Some(gap_start) = first_gap(&snapshot, demand_start, total) else {
                break 'demand;
            };
            if gap_start >= demand_end {
                break 'demand;
            }
            let fetch_end = (gap_start + FETCH_CHUNK.max(READAHEAD.min(total)))
                .min(total)
                .max(demand_end.min(total));
            let result = run_fetch(
                &store,
                &resolver,
                &mut format,
                &mut file,
                gap_start,
                fetch_end,
            );

            match result {
                Ok(()) => {
                    attempts = 0;
                }
                Err(FetchError::Cancelled) => return,
                Err(FetchError::Auth)
                    if refresh_count.load(Ordering::Relaxed) < MAX_REFRESH_ATTEMPTS =>
                {
                    refresh_count.fetch_add(1, Ordering::Relaxed);
                    match resolver
                        .resolve_blocking(&video_id, (format.itag != 0).then_some(format.itag))
                    {
                        Ok(next) => {
                            if next.content_length.is_some_and(|length| length != total) {
                                store.fail(
                                    "the renewed stream has a different size than the cached one"
                                        .into(),
                                );
                                return;
                            }
                            format = next;
                        }
                        Err(error) => {
                            store.fail(format!("the stream url expired: {error}"));
                            return;
                        }
                    }
                }
                Err(FetchError::Auth) => {
                    store.fail("the stream url expired and could not be renewed".into());
                    return;
                }
                Err(error) => {
                    attempts += 1;
                    if attempts >= MAX_FETCH_ATTEMPTS {
                        store.fail(format!("stream download failed: {error}"));
                        return;
                    }
                    std::thread::sleep(Duration::from_millis(250 * attempts as u64));
                }
            }
        }
    }
}

pub(super) enum FetchError {
    Auth,
    Cancelled,
    Other(String),
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FetchError::Auth => f.write_str("the stream url was refused"),
            FetchError::Cancelled => f.write_str("cancelled"),
            FetchError::Other(message) => f.write_str(message),
        }
    }
}

async fn range_request(
    url: &str,
    start: u64,
    end: u64,
    user_agent: &str,
) -> Result<reqwest::Response, FetchError> {
    let mut current =
        reqwest::Url::parse(url).map_err(|_| FetchError::Other("invalid stream URL".into()))?;
    for _ in 0..=MAX_REDIRECT_HOPS {
        let http = crate::services::util::bound_public_client(current.as_str())
            .map_err(FetchError::Other)?;
        let response = http
            .get(current.clone())
            .header(RANGE, format!("bytes={start}-{}", end - 1))
            .header(USER_AGENT, user_agent)
            .header(ACCEPT, "*/*")
            .send()
            .await
            .map_err(|e| FetchError::Other(format!("range request: {e}")))?;
        if !response.status().is_redirection() {
            return Ok(response);
        }
        let location = response
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| FetchError::Other("stream redirect had no target".into()))?;
        current = current
            .join(location)
            .map_err(|_| FetchError::Other("bad stream redirect target".into()))?;
        if current.scheme() != "https" {
            return Err(FetchError::Other("stream redirect left https".into()));
        }
    }
    Err(FetchError::Other("too many stream redirects".into()))
}

pub(super) fn run_fetch(
    store: &Arc<RangeStore>,
    resolver: &Arc<UrlResolver>,
    format: &mut YtAudioFormat,
    file: &mut File,
    start: u64,
    end: u64,
) -> Result<(), FetchError> {
    let url = format.url.clone();
    let agent = format.user_agent;
    let runtime = resolver.runtime.clone();
    let control = store.control.clone();
    let store_task = store.clone();
    let request = async move {
        let response = tokio::time::timeout(FETCH_STALL, range_request(&url, start, end, agent))
            .await
            .map_err(|_| FetchError::Other("the stream did not answer".into()))??;
        let status = response.status();
        if status == reqwest::StatusCode::FORBIDDEN
            || status == reqwest::StatusCode::UNAUTHORIZED
            || status == reqwest::StatusCode::GONE
            || status == reqwest::StatusCode::NOT_FOUND
        {
            return Err(FetchError::Auth);
        }
        if status != reqwest::StatusCode::PARTIAL_CONTENT {
            return Err(FetchError::Other(format!(
                "range request returned {status}"
            )));
        }

        let mut response = response;
        let mut at = start;
        while at < end {
            if control.is_cancelled() {
                return Err(FetchError::Cancelled);
            }
            let chunk = tokio::time::timeout(FETCH_STALL, response.chunk())
                .await
                .map_err(|_| FetchError::Other("the stream stalled".into()))?
                .map_err(|e| FetchError::Other(format!("range body: {e}")))?;
            let Some(bytes) = chunk else {
                break;
            };
            if bytes.is_empty() {
                continue;
            }
            let take = bytes.len().min((end - at) as usize);
            file.seek(SeekFrom::Start(at))
                .map_err(|e| FetchError::Other(format!("cache seek: {e}")))?;
            file.write_all(&bytes[..take])
                .map_err(|e| FetchError::Other(format!("cache write: {e}")))?;
            at += take as u64;
            store_task.mark(at - take as u64, at);
            control.fetched.fetch_add(take as u64, Ordering::Relaxed);
            if at < end {
                continue;
            }
        }
        if at < end {
            return Err(FetchError::Other("the stream ended early".into()));
        }
        Ok(())
    };

    match runtime.block_on(request) {
        Ok(()) => Ok(()),
        Err(error) => Err(error),
    }
}

pub(super) struct RangeReader {
    pub(super) store: Arc<RangeStore>,
    pub(super) file: File,
    pub(super) pos: u64,
    pub(super) total: u64,
}

impl Read for RangeReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.pos >= self.total || buf.is_empty() {
            return Ok(0);
        }
        let want = (buf.len() as u64).min(self.total - self.pos);
        self.store
            .ensure(self.pos, self.pos + want)
            .map_err(io::Error::other)?;
        self.file.seek(SeekFrom::Start(self.pos))?;
        self.file.read_exact(&mut buf[..want as usize])?;
        self.pos += want;
        Ok(want as usize)
    }
}

impl Seek for RangeReader {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let next = match pos {
            SeekFrom::Start(offset) => offset as i128,
            SeekFrom::End(offset) => self.total as i128 + offset as i128,
            SeekFrom::Current(offset) => self.pos as i128 + offset as i128,
        };
        if next < 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "negative seek"));
        }
        self.pos = (next as u64).min(self.total);
        Ok(self.pos)
    }
}

impl MediaSource for RangeReader {
    fn is_seekable(&self) -> bool {
        true
    }

    fn byte_len(&self) -> Option<u64> {
        Some(self.total)
    }
}

