use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use tauri::Emitter;
use tokio::sync::Mutex;

use crate::commands::types::*;

use super::fs::{cleanup_tmp_dir, deduplicate_tracks, existing_songs};
use super::finalize::emit_final_summary;
use super::media::start_cover_prefetch;
use super::paths::tmp_dir;
use super::process::{resolve_tool, CANCELLED_DOWNLOAD};
use super::{run_pass, BUSY, GENERATION};

pub async fn download_collection(
    collection: Collection,
    settings: Settings,
    user_data_dir: &Path,
    app_handle: tauri::AppHandle,
    opts: DownloadOpts,
) -> Result<(), String> {
    if BUSY.swap(true, Ordering::SeqCst) {
        return Err("A download is already running.".into());
    }

    struct BusyGuard;
    impl Drop for BusyGuard {
        fn drop(&mut self) {
            BUSY.store(false, Ordering::SeqCst);
        }
    }
    let _guard = BusyGuard;

    download_collection_inner(collection, settings, user_data_dir, &app_handle, opts).await
}

async fn download_collection_inner(
    collection: Collection,
    settings: Settings,
    user_data_dir: &Path,
    app_handle: &tauri::AppHandle,
    opts: DownloadOpts,
) -> Result<(), String> {
    CANCELLED_DOWNLOAD.store(false, Ordering::SeqCst);
    let generation = GENERATION.fetch_add(1, Ordering::SeqCst) + 1;

    let tmp = tmp_dir(user_data_dir);

    cleanup_tmp_dir(&tmp);

    let summary = Arc::new(Mutex::new(DownloadSummary {
        total: 0,
        done: 0,
        skipped: 0,
        failed: 0,
        cancelled: false,
    }));

    let (deduped_tracks, dup_skipped) = {
        let existing: HashMap<String, Vec<f64>> = if settings.overwrite {
            HashMap::new()
        } else {
            let od = settings.output_dir.clone();
            let ud = user_data_dir.to_path_buf();
            tokio::task::spawn_blocking(move || existing_songs(Path::new(&od), &ud))
                .await
                .unwrap_or_default()
        };
        deduplicate_tracks(&collection.tracks, app_handle, &existing)
    };
    {
        let mut s = summary.lock().await;
        s.skipped += dup_skipped as i64;
        s.total = deduped_tracks.len() as i64 + s.skipped;
    }
    let collection = Collection {
        tracks: deduped_tracks,
        ..collection
    };

    let ytdlp = resolve_tool("yt-dlp", user_data_dir)
        .ok_or_else(|| "yt-dlp is required to download.".to_string())?;

    let (prefetched_covers, covers_task) = start_cover_prefetch(&collection, &tmp);

    let retry_indices: Arc<Mutex<Vec<usize>>> = Arc::new(Mutex::new(Vec::new()));

    let skip_ids_set: HashSet<String> = opts
        .skip_ids
        .as_ref()
        .map(|ids| ids.iter().cloned().collect())
        .unwrap_or_default();

    let first_queue: VecDeque<usize> = (0..collection.tracks.len()).collect();
    let first_queue = Arc::new(Mutex::new(first_queue));

    run_pass(
        first_queue,
        &collection,
        &settings,
        user_data_dir,
        &tmp,
        opts.matches.as_ref(),
        opts.picks.as_ref(),
        &skip_ids_set,
        &ytdlp,
        app_handle,
        &prefetched_covers,
        &summary,
        generation,
        false,
        &retry_indices,
    )
    .await;

    if !CANCELLED_DOWNLOAD.load(Ordering::Relaxed) {
        let retries: VecDeque<usize> = {
            let mut r = retry_indices.lock().await;
            std::mem::take(&mut *r).into_iter().collect()
        };
        if !retries.is_empty() {
            let retry_queue = Arc::new(Mutex::new(retries));
            run_pass(
                retry_queue,
                &collection,
                &settings,
                user_data_dir,
                &tmp,
                opts.matches.as_ref(),
                opts.picks.as_ref(),
                &skip_ids_set,
                &ytdlp,
                app_handle,
                &prefetched_covers,
                &summary,
                generation,
                true,
                &retry_indices,
            )
            .await;
        }
    }

    let _ = covers_task.await;

    cleanup_tmp_dir(&tmp);

    if CANCELLED_DOWNLOAD.load(Ordering::SeqCst) {
        let mut s = summary.lock().await;
        s.cancelled = true;
    }

    emit_final_summary(app_handle, &summary).await;

    let _ = app_handle.emit("vynl:dl", DownloadEvent::Finished);
    Ok(())
}



