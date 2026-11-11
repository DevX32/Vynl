pub mod client;
pub mod types;

pub use client::{
    CatalogueClient, SEARCH_LIMIT, TASTE_LIMIT, TASTE_SEEDS, album_dto, album_kind, artist_names,
    artist_refs, catalogue_error, cover_of, playlist_dto, track_dto,
};
pub use types::{
    CatalogueAlbum, CatalogueAlbumDetail, CatalogueAlbumRef, CatalogueArtist, CatalogueArtistRef,
    CataloguePlaylist, CataloguePlaylistDetail, CataloguePlaylistPage, CatalogueSearchResults,
    CatalogueTrack,
};
