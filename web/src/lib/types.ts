export type AudioFormat = "mp3" | "m4a" | "opus" | "flac" | "wav";

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

export interface RpcPresence {
  title: string;
  artist: string;
  album: string;
  duration: number;
  time: number;
  playing: boolean;
  cover: string | null;
}

export type ToolName = "yt-dlp" | "ffmpeg";

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

type LyricsKind = "lrc" | "txt";
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
  id: number;
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

export interface UpdateStatus {
  available: boolean;
  version?: string;
  currentVersion: string;
  downloading?: boolean;
  progress?: number;
  ready?: boolean;
  error?: string;
}

export interface ArtistInfo {
  name: string;
  disambiguation: string | null;
  country: string | null;
  beginArea: string | null;
  lifeSpan: {
    ended: boolean;
    begin: string | null;
    end: string | null;
  } | null;
  genres: string[];
  tags: string[];
  bio: string | null;
  artistType: string | null;
  score: number | null;
}
