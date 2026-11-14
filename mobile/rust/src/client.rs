use std::collections::HashSet;
use std::sync::Arc;

use ytmusic::{Album, AlbumKind, Playlist as YtPlaylist, Track, YtMusic};

use crate::types::{
    CatalogueAlbum, CatalogueAlbumDetail, CatalogueAlbumRef, CatalogueArtist, CatalogueArtistRef,
    CataloguePlaylist, CataloguePlaylistDetail, CataloguePlaylistPage, CatalogueSearchResults,
    CatalogueTrack,
};

pub const SEARCH_LIMIT: usize = 20;
pub const TASTE_LIMIT: usize = 60;
pub const TASTE_SEEDS: usize = 6;

#[derive(Debug, Clone)]
pub struct CatalogueStream {
    pub url: String,
    pub mime: String,
}

pub fn catalogue_error(context: &str, error: impl std::fmt::Display) -> String {
    let detail = error.to_string();
    let detail = detail
        .rsplit_once(" -> ")
        .map(|(_, tail)| tail.trim().to_string())
        .unwrap_or(detail);
    if detail.is_empty() {
        return format!("{context} failed");
    }
    format!("{context} failed: {detail}")
}

pub struct CatalogueClient {
    ytmusic: Arc<YtMusic>,
}

impl CatalogueClient {
    pub fn new() -> Self {
        Self {
            ytmusic: Arc::new(YtMusic::anonymous()),
        }
    }

    pub fn with_client(ytmusic: Arc<YtMusic>) -> Self {
        Self { ytmusic }
    }

    pub async fn search(&self, query: &str) -> Result<CatalogueSearchResults, String> {
        let query = query.trim();
        if query.is_empty() {
            return Ok(CatalogueSearchResults {
                songs: Vec::new(),
                albums: Vec::new(),
                playlists: Vec::new(),
            });
        }

        let songs = self.ytmusic.search_songs(query).await;

        Ok(CatalogueSearchResults {
            songs: songs
                .map_err(|e| catalogue_error("song search", e))?
                .iter()
                .filter_map(track_dto)
                .take(SEARCH_LIMIT)
                .collect(),
            albums: Vec::new(),
            playlists: Vec::new(),
        })
    }

    pub async fn album(&self, browse_id: &str) -> Result<CatalogueAlbumDetail, String> {
        let detail = self
            .ytmusic
            .album(browse_id)
            .await
            .map_err(|e| catalogue_error("album lookup", e))?;
        Ok(CatalogueAlbumDetail {
            album: album_dto(&detail.album),
            description: detail.description,
            duration_text: detail.duration_text,
            tracks: detail.tracks.iter().filter_map(track_dto).collect(),
        })
    }

    pub async fn artist(&self, browse_id: &str) -> Result<CatalogueArtist, String> {
        let artist = self
            .ytmusic
            .artist(browse_id)
            .await
            .map_err(|e| catalogue_error("artist lookup", e))?;
        Ok(CatalogueArtist {
            id: artist.browse_id,
            name: artist.name,
            description: artist.description,
            subscribers: artist.subscribers,
            cover: cover_of(&artist.thumbnails),
            top_tracks: artist.top_tracks.iter().filter_map(track_dto).collect(),
            albums: artist.albums.iter().map(album_dto).collect(),
            singles: artist.singles.iter().map(album_dto).collect(),
        })
    }

    pub async fn playlist(&self, playlist_id: &str) -> Result<CataloguePlaylistDetail, String> {
        let detail = self
            .ytmusic
            .playlist_page(playlist_id)
            .await
            .map_err(|e| catalogue_error("playlist lookup", e))?;
        Ok(CataloguePlaylistDetail {
            playlist: playlist_dto(&detail.playlist),
            tracks: detail.tracks.iter().filter_map(track_dto).collect(),
            continuation: detail.continuation,
        })
    }

    pub async fn playlist_page(&self, token: &str) -> Result<CataloguePlaylistPage, String> {
        let page = self
            .ytmusic
            .playlist_continuation(token)
            .await
            .map_err(|e| catalogue_error("playlist page", e))?;
        Ok(CataloguePlaylistPage {
            tracks: page.tracks.iter().filter_map(track_dto).collect(),
            continuation: page.continuation,
        })
    }

    pub async fn stream(&self, video_id: &str) -> Result<CatalogueStream, String> {
        let format = self
            .ytmusic
            .best_audio(video_id)
            .await
            .map_err(|e| catalogue_error("stream lookup", e))?;
        Ok(CatalogueStream {
            url: format.url,
            mime: format.mime,
        })
    }

    pub async fn taste(&self, seeds: &[String]) -> Result<Vec<CatalogueTrack>, String> {
        let mut seen_seeds = HashSet::new();
        let seeds: Vec<String> = seeds
            .iter()
            .map(|seed| seed.trim().to_string())
            .filter(|seed| !seed.is_empty() && seen_seeds.insert(seed.clone()))
            .take(TASTE_SEEDS)
            .collect();
        if seeds.is_empty() {
            return Ok(Vec::new());
        }

        let mut tasks = Vec::new();
        for seed in &seeds {
            let client = self.ytmusic.clone();
            let seed = seed.clone();
            tasks.push(tokio::spawn(async move { client.track_radio(&seed).await }));
        }

        let mut lists: Vec<Vec<Track>> = Vec::new();
        for task in tasks {
            match task.await {
                Ok(Ok(tracks)) => lists.push(tracks),
                Ok(Err(error)) => eprintln!("taste radio failed: {error:#}"),
                Err(error) => eprintln!("taste radio task failed: {error}"),
            }
        }
        if lists.is_empty() {
            return Err("could not build your mix".into());
        }

        let seed_ids: HashSet<&str> = seeds.iter().map(String::as_str).collect();
        let mut seen = HashSet::new();
        let mut songs = Vec::new();
        let longest = lists.iter().map(Vec::len).max().unwrap_or(0);
        'outer: for index in 0..longest {
            for list in &lists {
                let Some(track) = list.get(index) else {
                    continue;
                };
                let Some(video_id) = track.video_id.as_deref() else {
                    continue;
                };
                if seed_ids.contains(video_id) || !seen.insert(video_id.to_string()) {
                    continue;
                }
                if let Some(dto) = track_dto(track) {
                    songs.push(dto);
                    if songs.len() >= TASTE_LIMIT {
                        break 'outer;
                    }
                }
            }
        }

        if songs.is_empty() {
            return Err("could not build your mix".into());
        }
        Ok(songs)
    }
}

impl Default for CatalogueClient {
    fn default() -> Self {
        Self::new()
    }
}

pub fn cover_of(thumbnails: &[ytmusic::Thumbnail]) -> Option<String> {
    ytmusic::best_thumbnail(thumbnails).map(|thumbnail| full_quality_cover(&thumbnail.url))
}

fn full_quality_cover(url: &str) -> String {
    let Some(start) = url.find("=w") else {
        return url.to_string();
    };
    let digits_start = start + 2;
    let mut i = digits_start;
    while url.as_bytes().get(i).is_some_and(|b| b.is_ascii_digit()) {
        i += 1;
    }
    if i == digits_start || !url[i..].starts_with("-h") {
        return url.to_string();
    }
    let mut j = i + 2;
    let height_start = j;
    while url.as_bytes().get(j).is_some_and(|b| b.is_ascii_digit()) {
        j += 1;
    }
    if j == height_start {
        return url.to_string();
    }
    format!("{}=w512-h512{}", &url[..start], &url[j..])
}

pub fn artist_names(artists: &[ytmusic::ArtistRef]) -> String {
    artists
        .iter()
        .map(|artist| artist.name.trim())
        .filter(|name| !name.is_empty())
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn artist_refs(artists: &[ytmusic::ArtistRef]) -> Vec<CatalogueArtistRef> {
    artists
        .iter()
        .map(|artist| CatalogueArtistRef {
            name: artist.name.clone(),
            id: artist.id.clone(),
        })
        .collect()
}

pub fn track_dto(track: &Track) -> Option<CatalogueTrack> {
    let id = track.video_id.clone()?;
    if id.is_empty() {
        return None;
    }
    Some(CatalogueTrack {
        id,
        title: track.title.clone(),
        artist: artist_names(&track.artists),
        artists: artist_refs(&track.artists),
        album: track.album.as_ref().map(|album| CatalogueAlbumRef {
            name: album.name.clone(),
            id: album.id.clone(),
        }),
        duration: track.duration.map(|d| d.as_secs_f64()),
        cover: cover_of(&track.thumbnails),
        explicit: track.explicit,
    })
}

pub fn album_kind(kind: AlbumKind) -> &'static str {
    match kind {
        AlbumKind::Album => "album",
        AlbumKind::Single => "single",
        AlbumKind::Ep => "ep",
        AlbumKind::Compilation => "compilation",
    }
}

pub fn album_dto(album: &Album) -> CatalogueAlbum {
    CatalogueAlbum {
        id: album.browse_id.clone(),
        playlist_id: album.playlist_id.clone(),
        title: album.title.clone(),
        artist: artist_names(&album.artists),
        artists: artist_refs(&album.artists),
        kind: album_kind(album.kind).to_string(),
        year: album.year,
        track_count: album.track_count,
        cover: cover_of(&album.thumbnails),
    }
}

pub fn playlist_dto(playlist: &YtPlaylist) -> CataloguePlaylist {
    CataloguePlaylist {
        id: playlist.id.clone(),
        title: playlist.title.clone(),
        author: playlist.author.clone(),
        track_count: playlist.track_count,
        cover: cover_of(&playlist.thumbnails),
    }
}
