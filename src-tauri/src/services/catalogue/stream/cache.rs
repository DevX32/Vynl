use std::collections::HashMap;
use std::fs::OpenOptions;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::store::mark_range;

const CACHE_LIMIT: u64 = 2 * 1024 * 1024 * 1024;

static FILE_SERIAL: AtomicU64 = AtomicU64::new(0);

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub(super) struct BeginResult {
    pub(super) path: PathBuf,
    pub(super) file_name: String,
    pub(super) coverage: Vec<(u64, u64)>,
}

#[derive(Clone, Serialize, Deserialize)]
struct CacheEntry {
    file: String,
    #[serde(default)]
    size: u64,
    #[serde(default)]
    complete: bool,
    #[serde(default)]
    last_used: u64,
    #[serde(default)]
    itag: Option<u32>,
    #[serde(default)]
    coverage: Vec<(u64, u64)>,
}

#[derive(Default, Serialize, Deserialize)]
struct CacheIndex {
    #[serde(default = "index_version")]
    version: u32,
    #[serde(default)]
    entries: HashMap<String, CacheEntry>,
}

pub(super) fn index_version() -> u32 {
    1
}

pub(super) struct CacheState {
    index: CacheIndex,
    active: HashMap<String, Vec<String>>,
    last_coverage_save: HashMap<String, std::time::Instant>,
    dirty: bool,
}

pub struct StreamCache {
    dir: PathBuf,
    state: Mutex<CacheState>,
}

impl StreamCache {
    pub fn open(dir: PathBuf) -> Arc<Self> {
        let _ = std::fs::create_dir_all(&dir);
        let index_path = dir.join("index.json");
        let index: CacheIndex = std::fs::read(&index_path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .filter(|index: &CacheIndex| index.version == index_version())
            .unwrap_or_else(|| CacheIndex {
                version: index_version(),
                entries: HashMap::new(),
            });

        let cache = Arc::new(Self {
            dir,
            state: Mutex::new(CacheState {
                index,
                active: HashMap::new(),
                last_coverage_save: HashMap::new(),
                dirty: false,
            }),
        });
        cache.cleanup();
        cache
    }

    pub(super) fn cleanup(&self) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let mut keep: HashMap<String, CacheEntry> = HashMap::new();
        for (video_id, entry) in state.index.entries.drain() {
            let path = self.dir.join(&entry.file);
            if path.is_file() {
                keep.insert(video_id, entry);
            } else {
                let _ = std::fs::remove_file(&path);
            }
        }
        state.index.entries = keep;

        if let Ok(read) = std::fs::read_dir(&self.dir) {
            for entry in read.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name == "index.json" || !entry.path().is_file() {
                    continue;
                }
                if !state.index.entries.values().any(|e| e.file == name) {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
        state.dirty = true;
        self.save_locked(&mut state);
    }

    pub fn clear(&self) -> u64 {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let mut freed = 0u64;

        let to_remove: Vec<(String, String)> = state
            .index
            .entries
            .iter()
            .filter(|(id, _)| !state.active.contains_key(id.as_str()))
            .map(|(id, entry)| (id.clone(), entry.file.clone()))
            .collect();
        for (id, file) in to_remove {
            let path = self.dir.join(&file);
            let size = path.metadata().map(|m| m.len()).unwrap_or(0);
            let deleted = if path.exists() {
                std::fs::remove_file(&path).is_ok()
            } else {
                true
            };
            if deleted {
                freed += size;
                state.index.entries.remove(&id);
            }
        }

        if let Ok(read) = std::fs::read_dir(&self.dir) {
            for entry in read.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name == "index.json" || !entry.path().is_file() {
                    continue;
                }
                if state.index.entries.values().any(|e| e.file == name) {
                    continue;
                }
                if state
                    .active
                    .values()
                    .any(|files| files.iter().any(|f| f == &name))
                {
                    continue;
                }
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                if std::fs::remove_file(entry.path()).is_ok() {
                    freed += size;
                }
            }
        }

        state.dirty = true;
        self.save_locked(&mut state);
        freed
    }

    pub(super) fn index_path(&self) -> PathBuf {
        self.dir.join("index.json")
    }

    pub(super) fn save_locked(&self, state: &mut CacheState) {
        if !state.dirty {
            return;
        }
        if let Ok(bytes) = serde_json::to_vec_pretty(&state.index) {
            let tmp = self.index_path().with_extension("json.tmp");
            let target = self.index_path();
            if std::fs::write(&tmp, &bytes).is_ok() && std::fs::rename(&tmp, &target).is_ok() {
                state.dirty = false;
            }
        }
    }

    pub fn lookup_complete(&self, video_id: &str) -> Option<PathBuf> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let entry = state.index.entries.get(video_id)?.clone();
        if !entry.complete {
            return None;
        }
        let path = self.dir.join(&entry.file);
        if !path.is_file() {
            state.index.entries.remove(video_id);
            state.dirty = true;
            self.save_locked(&mut state);
            return None;
        }
        if let Some(entry) = state.index.entries.get_mut(video_id) {
            entry.last_used = now_secs();
        }
        state.dirty = true;
        self.save_locked(&mut state);
        Some(path)
    }

    pub(super) fn begin(
        &self,
        video_id: &str,
        ext: &str,
        total: u64,
        itag: u32,
        resume: bool,
    ) -> Result<BeginResult, String> {
        if resume && itag != 0 {
            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(entry) = state.index.entries.get(video_id) {
                let usable = !entry.complete
                    && entry.itag == Some(itag)
                    && entry.size == total
                    && self.dir.join(&entry.file).is_file();
                if usable {
                    let file_name = entry.file.clone();
                    let coverage = entry.coverage.clone();
                    if let Some(entry) = state.index.entries.get_mut(video_id) {
                        entry.last_used = now_secs();
                    }
                    state
                        .active
                        .entry(video_id.to_string())
                        .or_default()
                        .push(file_name.clone());
                    state.dirty = true;
                    self.save_locked(&mut state);
                    return Ok(BeginResult {
                        path: self.dir.join(&file_name),
                        file_name,
                        coverage,
                    });
                }
            }
        }
        let serial = FILE_SERIAL.fetch_add(1, Ordering::Relaxed);
        let safe_id: String = video_id
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
            .take(64)
            .collect();
        let file_name = format!("{}-{}-{}.{}", safe_id, now_secs(), serial, ext);
        let path = self.dir.join(&file_name);
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());

        let has_active = state
            .active
            .get(video_id)
            .is_some_and(|files| !files.is_empty());
        let stale_file = match state.index.entries.get(video_id) {
            Some(entry) if !entry.complete && !has_active => Some(entry.file.clone()),
            _ => None,
        };
        let file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|e| format!("could not create the stream cache: {e}"))?;
        drop(file);
        if !state
            .index
            .entries
            .get(video_id)
            .is_some_and(|entry| entry.complete)
        {
            if let Some(stale) = stale_file {
                let _ = std::fs::remove_file(self.dir.join(stale));
            }
            if !has_active {
                state.index.entries.insert(
                    video_id.to_string(),
                    CacheEntry {
                        file: file_name.clone(),
                        size: total,
                        complete: false,
                        last_used: now_secs(),
                        itag: (itag != 0).then_some(itag),
                        coverage: Vec::new(),
                    },
                );
            }
        }
        state
            .active
            .entry(video_id.to_string())
            .or_default()
            .push(file_name.clone());
        state.dirty = true;
        self.save_locked(&mut state);
        Ok(BeginResult {
            path,
            file_name,
            coverage: Vec::new(),
        })
    }

    pub(super) fn remove_one_active(
        state: &mut CacheState,
        video_id: &str,
        file_name: &str,
    ) -> bool {
        if let Some(active) = state.active.get_mut(video_id) {
            if let Some(pos) = active.iter().position(|f| f == file_name) {
                active.remove(pos);
            }
            if active.is_empty() {
                state.active.remove(video_id);
            }
            return state
                .active
                .get(video_id)
                .is_some_and(|active| active.iter().any(|f| f == file_name));
        }
        false
    }

    pub fn record_coverage(&self, video_id: &str, file_name: &str, coverage: &[(u64, u64)]) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let due = state
            .last_coverage_save
            .get(video_id)
            .map(|saved| saved.elapsed() >= std::time::Duration::from_secs(1))
            .unwrap_or(true);
        let Some(entry) = state.index.entries.get_mut(video_id) else {
            return;
        };
        if entry.file != file_name {
            return;
        }
        let mut merged = std::mem::take(&mut entry.coverage);
        for &(start, end) in coverage {
            mark_range(&mut merged, start, end);
        }
        entry.coverage = merged;
        state.dirty = true;
        if due {
            state
                .last_coverage_save
                .insert(video_id.to_string(), std::time::Instant::now());
            self.save_locked(&mut state);
        }
    }

    pub fn mark_complete(&self, video_id: &str, file_name: &str, total: u64) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let entry = state
            .index
            .entries
            .entry(video_id.to_string())
            .or_insert(CacheEntry {
                file: file_name.to_string(),
                size: total,
                complete: false,
                last_used: now_secs(),
                itag: None,
                coverage: Vec::new(),
            });
        let previous = if entry.file != file_name {
            if entry.complete {
                return;
            }
            let previous = std::mem::replace(&mut entry.file, file_name.to_string());
            entry.coverage.clear();
            Some(previous)
        } else {
            None
        };
        if let Some(entry) = state.index.entries.get_mut(video_id) {
            entry.complete = true;
            entry.size = total;
            entry.last_used = now_secs();
        }
        if let Some(previous) = previous {
            let previous_active = state
                .active
                .get(video_id)
                .is_some_and(|files| files.contains(&previous));
            if !previous_active {
                let _ = std::fs::remove_file(self.dir.join(&previous));
            }
        }
        state.dirty = true;
        self.save_locked(&mut state);
    }

    pub fn discard_entry(&self, video_id: &str, file_name: &str) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let still_active = Self::remove_one_active(&mut state, video_id, file_name);
        if still_active {
            return;
        }
        let matches = state
            .index
            .entries
            .get(video_id)
            .is_some_and(|entry| entry.file == file_name);
        if matches {
            state.index.entries.remove(video_id);
            state.dirty = true;
            self.save_locked(&mut state);
        }
        let _ = std::fs::remove_file(self.dir.join(file_name));
    }

    pub fn finish(&self, video_id: &str, file_name: &str, complete: bool) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let still_active = Self::remove_one_active(&mut state, video_id, file_name);

        if state
            .index
            .entries
            .get(video_id)
            .is_some_and(|entry| entry.file == file_name)
        {
            let keep = complete
                || state
                    .index
                    .entries
                    .get(video_id)
                    .is_some_and(|entry| entry.complete);
            if keep {
                if let Some(entry) = state.index.entries.get_mut(video_id) {
                    entry.complete = true;
                    entry.last_used = now_secs();
                }
            } else if !still_active {
                let resumable = state
                    .index
                    .entries
                    .get(video_id)
                    .is_some_and(|entry| !entry.coverage.is_empty());
                if resumable {
                    if let Some(entry) = state.index.entries.get_mut(video_id) {
                        entry.last_used = now_secs();
                    }
                } else {
                    state.index.entries.remove(video_id);
                }
            }
        }

        let keep = state
            .index
            .entries
            .get(video_id)
            .is_some_and(|entry| entry.file == file_name);
        if !keep && !still_active {
            let _ = std::fs::remove_file(self.dir.join(file_name));
        }
        state.dirty = true;
        self.evict_locked(&mut state);
        self.save_locked(&mut state);
    }

    pub(super) fn evict_locked(&self, state: &mut CacheState) {
        let mut total: u64 = state.index.entries.values().map(|e| e.size).sum();
        if total <= CACHE_LIMIT {
            return;
        }
        let mut order: Vec<(String, u64)> = state
            .index
            .entries
            .iter()
            .map(|(id, entry)| (id.clone(), entry.last_used))
            .collect();
        order.sort_by_key(|(_, used)| *used);
        for (video_id, _) in order {
            if total <= CACHE_LIMIT {
                break;
            }
            if state.active.contains_key(&video_id) {
                continue;
            }
            if let Some(entry) = state.index.entries.remove(&video_id) {
                let _ = std::fs::remove_file(self.dir.join(&entry.file));
                total = total.saturating_sub(entry.size);
                state.dirty = true;
            }
        }
    }
}

pub(super) fn abandon_cache(
    store: &super::store::RangeStore,
    cache: &StreamCache,
    video_id: &str,
    file_name: &str,
) {
    store.fail("the stream could not be decoded".to_string());
    store.control.cancel();
    cache.discard_entry(video_id, file_name);
}
