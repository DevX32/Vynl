# vynl-ytmusic

YouTube Music InnerTube client in Rust.

## What's in here

- `search_songs` / `search_albums` / `search_playlists` with YouTube Music
  search filters (`EgWKAQIIAQ%3D%3D` and friends)
- `TrackKind::{Song, Video}` — authoritative song-vs-video discrimination,
  straight from YouTube Music
- `player.rs` / `stream.rs` / `AudioStream` — audio stream resolution
- `potoken.rs` and `deobf.rs` — PO token generation and signature
  deobfuscation, using a bundled QuickJS runtime (`rquickjs`)
- `browse.rs` / `library.rs` / `mutate.rs` — albums, playlists, artists,
  and authenticated library actions

## Status

Unreleased, v0.1.0. Treat the API as unstable.

