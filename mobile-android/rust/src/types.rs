use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CatalogueArtistRef {
    pub name: String,
    pub id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CatalogueAlbumRef {
    pub name: String,
    pub id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogueTrack {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub artists: Vec<CatalogueArtistRef>,
    pub album: Option<CatalogueAlbumRef>,
    pub duration: Option<f64>,
    pub cover: Option<String>,
    pub explicit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogueAlbum {
    pub id: String,
    pub playlist_id: Option<String>,
    pub title: String,
    pub artist: String,
    pub artists: Vec<CatalogueArtistRef>,
    pub kind: String,
    pub year: Option<i32>,
    pub track_count: Option<u32>,
    pub cover: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogueAlbumDetail {
    pub album: CatalogueAlbum,
    pub description: Option<String>,
    pub duration_text: Option<String>,
    pub tracks: Vec<CatalogueTrack>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogueArtist {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub subscribers: Option<String>,
    pub cover: Option<String>,
    pub top_tracks: Vec<CatalogueTrack>,
    pub albums: Vec<CatalogueAlbum>,
    pub singles: Vec<CatalogueAlbum>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CataloguePlaylist {
    pub id: String,
    pub title: String,
    pub author: Option<String>,
    pub track_count: Option<u32>,
    pub cover: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CataloguePlaylistDetail {
    pub playlist: CataloguePlaylist,
    pub tracks: Vec<CatalogueTrack>,
    pub continuation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CataloguePlaylistPage {
    pub tracks: Vec<CatalogueTrack>,
    pub continuation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogueSearchResults {
    pub songs: Vec<CatalogueTrack>,
    pub albums: Vec<CatalogueAlbum>,
    pub playlists: Vec<CataloguePlaylist>,
}
