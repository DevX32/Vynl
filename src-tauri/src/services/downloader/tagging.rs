use std::path::Path;

use crate::commands::types::TrackMeta;

pub(super) fn tag_type_for(file: &str) -> lofty::tag::TagType {
    match Path::new(file)
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("m4a") | Some("mp4") | Some("m4b") => lofty::tag::TagType::Mp4Ilst,
        Some("opus") | Some("ogg") | Some("flac") => lofty::tag::TagType::VorbisComments,
        _ => lofty::tag::TagType::Id3v2,
    }
}

pub(super) fn try_tag_with_lofty(
    audio_path: &str,
    track: &TrackMeta,
    cover: Option<&str>,
    lyrics: Option<&str>,
) -> Result<(), String> {
    use lofty::picture::{Picture, PictureType};
    use lofty::prelude::*;
    use lofty::probe::Probe;
    use lofty::tag::ItemKey;
    use std::fs;

    let path = Path::new(audio_path);
    let mut tagged_file = Probe::open(path)
        .map_err(|e| format!("lofty probe failed: {e}"))?
        .read()
        .map_err(|e| format!("lofty read failed: {e}"))?;

    let tag_type = tag_type_for(audio_path);
    let tag = if let Some(primary_type) = tagged_file.primary_tag().map(|t| t.tag_type()) {
        if primary_type != tag_type {
            if tagged_file.tag(tag_type).is_none() {
                tagged_file.insert_tag(lofty::tag::Tag::new(tag_type));
            }
            tagged_file
                .tag_mut(tag_type)
                .ok_or_else(|| "could not update the audio tags".to_string())?
        } else {
            tagged_file
                .primary_tag_mut()
                .ok_or_else(|| "could not update the audio tags".to_string())?
        }
    } else {
        let new_tag = lofty::tag::Tag::new(tag_type);
        tagged_file.insert_tag(new_tag);
        tagged_file
            .primary_tag_mut()
            .ok_or_else(|| "could not create the audio tags".to_string())?
    };

    tag.set_title(track.title.clone());
    tag.set_artist(track.artist.clone());
    tag.set_album(track.album.clone());

    if let Some(year) = track.year {
        tag.insert_text(ItemKey::Year, year.to_string());
        tag.insert_text(ItemKey::RecordingDate, year.to_string());
    }
    if let Some(num) = track.track_number {
        tag.set_track(num as u32);
    }
    if let Some(l) = lyrics
        && !l.trim().is_empty()
    {
        tag.insert_text(ItemKey::Lyrics, l.to_string());
        tag.insert_text(ItemKey::UnsyncLyrics, l.to_string());
    }
    if let Some(cover_path) = cover
        && let Ok(bytes) = fs::read(cover_path)
        && !bytes.is_empty()
    {
        let mime = image_mime(&bytes);
        let pic = Picture::unchecked(bytes)
            .mime_type(mime)
            .pic_type(PictureType::CoverFront)
            .build();
        tag.remove_picture_type(PictureType::CoverFront);
        tag.push_picture(pic);
    }

    tagged_file
        .save_to_path(path, lofty::config::WriteOptions::new().use_id3v23(true))
        .map_err(|e| format!("lofty save failed: {e}"))?;
    Ok(())
}

pub(super) fn image_mime(bytes: &[u8]) -> lofty::picture::MimeType {
    use lofty::picture::MimeType;
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        MimeType::Png
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        MimeType::Jpeg
    } else if bytes.starts_with(b"GIF8") {
        MimeType::Gif
    } else if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        MimeType::Unknown(String::from("image/webp"))
    } else if bytes.starts_with(b"BM") {
        MimeType::Bmp
    } else {
        MimeType::Jpeg
    }
}

pub(super) async fn tag(
    audio_in: &str,
    audio_out: &str,
    track: &TrackMeta,
    cover: Option<&str>,
    lyrics: Option<&str>,
) -> Result<(), String> {
    if audio_in == audio_out {
        return Err("source and destination are the same file".into());
    }

    if let Err(e) = tokio::fs::copy(audio_in, audio_out).await {
        return Err(format!("Failed to copy audio: {e}"));
    }

    let out = audio_out.to_string();
    let track_c = track.clone();
    let cover_c = cover.map(|s| s.to_string());
    let lyrics_c = lyrics.map(|s| s.to_string());

    let result = tokio::task::spawn_blocking(move || {
        try_tag_with_lofty(&out, &track_c, cover_c.as_deref(), lyrics_c.as_deref())
    })
    .await
    .map_err(|e| format!("Tagging task failed: {e}"))?;

    match result {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = tokio::fs::remove_file(audio_out).await;
            Err(e)
        }
    }
}

