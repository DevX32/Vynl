export type AudioFormat = "mp3";

type TrackStatus =
  | "queued"
  | "searching"
  | "downloading"
  | "processing"
  | "done"
  | "skipped"
  | "error"
  | "cancelled";

export interface TrackMeta {
  id: string;
  title: string;
  artist: string;
  album: string;
  year: number | null;
  duration: number | null;
  cover: string | null;
  trackNumber: number | null;
  url: string;
}

export interface Collection {
  kind: "playlist" | "album" | "track";
  id: string;
  title: string;
  owner?: string;
  cover: string | null;
  tracks: TrackMeta[];
}

export interface Settings {
  outputDir: string;
  format: AudioFormat;
  bitrate: number | null;
  filenamePattern: string;
  overwrite: boolean;
  discordRpc: boolean;
  confirmMatches: boolean;
  minimizeToTray: boolean;
  accentColor: string;
  dynamicAccent: boolean;
  hardwareAcceleration: boolean;
  launchAtStartup: boolean;
  displayName: string;
  eqEnabled: boolean;
  eqBands: number[];
  pluginsAutoUpdate: boolean;
}

export interface EqDesign {
  freqs: number[];
  qs: number[];
  lowShelf: number;
  highShelf: number;
  shape: number;
  flatDb: number;
  preampTrim: number;
  minDb: number;
  maxDb: number;
}

export type MatchSource = "youtube" | "youtubeMusic";

export interface SearchCandidate {
  url: string;
  title: string;
  duration: number | null;
  channel: string | null;
  source: MatchSource;
  viewCount?: number;
  channelVerified?: boolean;
  uploadDate?: string;
}

export interface SearchResult {
  id: string;
  title: string;
  artist: string;
  album: string;
  duration: number | null;
  cover: string | null;
  url: string;
}

export interface CatalogueArtistRef {
  name: string;
  id: string | null;
}

export interface CatalogueAlbumRef {
  name: string;
  id: string | null;
}

export interface CatalogueTrack {
  id: string;
  title: string;
  artist: string;
  artists: CatalogueArtistRef[];
  album: CatalogueAlbumRef | null;
  duration: number | null;
  cover: string | null;
  explicit: boolean;
}

export interface CatalogueAlbum {
  id: string;
  playlistId: string | null;
  title: string;
  artist: string;
  artists: CatalogueArtistRef[];
  kind: string;
  year: number | null;
  trackCount: number | null;
  cover: string | null;
}

export interface CatalogueAlbumDetail {
  album: CatalogueAlbum;
  description: string | null;
  durationText: string | null;
  tracks: CatalogueTrack[];
}

export interface CatalogueArtist {
  id: string;
  name: string;
  description: string | null;
  subscribers: string | null;
  cover: string | null;
  topTracks: CatalogueTrack[];
  albums: CatalogueAlbum[];
  singles: CatalogueAlbum[];
}

export interface CataloguePlaylist {
  id: string;
  title: string;
  author: string | null;
  trackCount: number | null;
  cover: string | null;
}

export interface CataloguePlaylistDetail {
  playlist: CataloguePlaylist;
  tracks: CatalogueTrack[];
  continuation: string | null;
}

export interface CataloguePlaylistPage {
  tracks: CatalogueTrack[];
  continuation: string | null;
}

export interface CatalogueSearchResults {
  songs: CatalogueTrack[];
  albums: CatalogueAlbum[];
  playlists: CataloguePlaylist[];
}

export interface CatalogueStreamInfo {
  videoId: string;
  duration: number | null;
  cached: boolean;
}

export interface CatalogueAddResult {
  path: string;
  downloaded: boolean;
  playlist: Playlist | null;
}

export type CataloguePageRoute =
  | { view: "search" }
  | { view: "liked" }
  | { view: "album" | "artist" | "playlist"; id: string };

export function catalogueRoute(page: Page): CataloguePageRoute | null {
  if (page === "catalogue") return { view: "search" };
  if (page === "catalogue:liked") return { view: "liked" };
  const match = /^catalogue:(album|artist|playlist):(.+)$/.exec(page);
  if (!match) return null;
  return { view: match[1] as "album" | "artist" | "playlist", id: match[2] };
}

export interface RpcPresence {
  title: string;
  artist: string;
  album: string;
  duration: number;
  time: number;
  playing: boolean;
  cover: string | null;
}

export type ToolName = "yt-dlp";

export interface LibraryTrack {
  id: string;
  path: string;
  title: string;
  artist: string;
  album: string;
  year: number | null;
  duration: number;
  cover: string | null;
  trackNumber: number | null;
  lyrics: string | null;
  ext: string;
  mtime?: number | null;
  addedAt?: number | null;
}

export type Page =
  | "home"
  | "library"
  | "playlist"
  | "player"
  | "vault"
  | "settings"
  | "catalogue"
  | "catalogue:liked"
  | `catalogue:album:${string}`
  | `catalogue:artist:${string}`
  | `catalogue:playlist:${string}`
  | `plugin:${string}`;

export function isPluginPage(page: Page): page is `plugin:${string}` {
  return page.startsWith("plugin:");
}

export interface PlaylistMeta {
  id: string;
  name: string;
  trackCount: number;
  createdAt: number;
  updatedAt: number;
}

export interface Playlist {
  id: string;
  name: string;
  paths: string[];
  streams: CatalogueTrack[];
  cover: string | null;
  createdAt: number;
  updatedAt: number;
}

export interface ToolStatus {
  name: ToolName;
  installed: boolean;
  path?: string;
  version?: string;
  state: "ok" | "missing" | "downloading" | "error";
  progress?: number;
  error?: string;
  updateAvailable?: boolean;
}

export interface MobileSyncStatus {
  enabled: boolean;
  running: boolean;
  port: number;
  pairingPin: string;
  lanAddresses: string[];
  pairUrls: string[];
  hostname: string | null;
  qrSvg: string | null;
}

export interface TrackProgress {
  trackId: string;
  status: TrackStatus;
  percent: number;
  message?: string;
}

export interface DownloadSummary {
  total: number;
  done: number;
  skipped: number;
  failed: number;
  cancelled: boolean;
}

type LyricsKind = "lrc" | "txt" | "ttml";
type LyricsSource = "local" | "remote";

export interface LyricsResult {
  kind: LyricsKind;
  text: string;
  source: LyricsSource;
  file?: string;
}

export interface LyricsLookup {
  title: string;
  artist: string;
  album: string;
  duration: number;
}

export interface LrcSearchResult {
  id: string;
  trackName: string;
  artistName: string;
  albumName: string;
  duration: number;
  instrumental: boolean;
  plainLyrics: string | null;
  syncedLyrics: string | null;
}

export type DownloadEvent =
  | { type: "track"; payload: TrackProgress }
  | { type: "summary"; payload: DownloadSummary }
  | { type: "finished" };

export interface DownloadOpts {
  matches?: Record<string, SearchCandidate[]>;
  picks?: Record<string, number>;
  skipIds?: string[];
}

export interface NowPlaying {
  id: string;
  path: string;
  title: string;
  artist: string;
  album: string;
  year: number | null;
  duration: number;
  cover: string | null;
  trackNumber: number | null;
  playing: boolean;
  time: number;
  lyrics: string | null;
}

export interface NowPlayingState {
  id: string;
  path: string;
  time: number;
  queue: string[];
  userQueue?: string[];
  contextQueue?: string[];
  contextIndex?: number;
  timestamp: number;
}

export interface PlaybackTick {
  generation: number;
  position: number;
  playing: boolean;
  finished: boolean;
  duration: number | null;
}

export interface UpdateStatus {
  available: boolean;
  version?: string;
  currentVersion: string;
  downloading?: boolean;
  progress?: number;
  ready?: boolean;
  error?: string;
}

export interface ExportSummary {
  path: string;
  format: string;
  trackCount: number;
  playlistCount: number;
  bytes: number;
}

export interface RestoredBackup {
  collection: Collection;
  matches: Record<string, SearchCandidate[]>;
  picks: Record<string, number>;
  total: number;
  knownSources: number;
}

export interface MediaUpdate {
  title: string;
  artist: string;
  album: string;
  duration: number;
  cover: string | null;
  playing: boolean;
  position: number;
}

export type MediaControlEvent =
  | {
      action:
        | "play"
        | "pause"
        | "toggle"
        | "next"
        | "previous"
        | "stop";
    }
  | { action: "seek"; position: number }
  | { action: "seekBy"; offset: number }
  | { action: "setVolume"; volume: number };
