import { invoke, type InvokeArgs } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { open as openShell } from "@tauri-apps/plugin-shell";
import type {
  Collection,
  DownloadEvent,
  DownloadOpts,
  DownloadSummary,
  LibraryTrack,
  LyricsLookup,
  LyricsResult,
  NowPlayingState,
  Playlist,
  PlaylistMeta,
  RpcPresence,
  SearchCandidate,
  Settings,
  SearchResult,
  ToolName,
  ToolStatus,
  UpdateStatus,
} from "../lib/types";
import type {
  PluginHttpInit,
  PluginHttpResponse,
  PluginStoreResult,
} from "./plugins/types";

async function cmd<T>(command: string, args?: InvokeArgs): Promise<T> {
  return invoke<T>(command, args);
}

const APP_UPDATE_CACHE_MS = 5 * 60 * 1000;
let appUpdateCache: { status: UpdateStatus; at: number } | null = null;
let appUpdateCheck: Promise<UpdateStatus> | null = null;
let appUpdateInstall: Promise<void> | null = null;

function rememberUpdateStatus(status: UpdateStatus): void {
  appUpdateCache = { status, at: Date.now() };
}

function checkAppUpdate(force = false): Promise<UpdateStatus> {
  if (
    !force &&
    appUpdateCache &&
    Date.now() - appUpdateCache.at < APP_UPDATE_CACHE_MS
  ) {
    return Promise.resolve(appUpdateCache.status);
  }
  if (!appUpdateCheck) {
    appUpdateCheck = cmd<UpdateStatus>("check_app_update")
      .then((status) => {
        rememberUpdateStatus(status);
        return status;
      })
      .finally(() => {
        appUpdateCheck = null;
      });
  }
  return appUpdateCheck;
}

function installAppUpdate(): Promise<void> {
  if (!appUpdateInstall) {
    appUpdateInstall = cmd<void>("install_app_update").finally(() => {
      appUpdateInstall = null;
    });
  }
  return appUpdateInstall;
}

export const vynl = {
  resolve: (url: string): Promise<Collection> => cmd("resolve", { url }),

  searchSpotify: (query: string, limit?: number, offset?: number): Promise<SearchResult[]> =>
    cmd("search_spotify", { query, limit: limit ?? 20, offset: offset ?? 0 }),

  downloadedPaths: (collection: Collection): Promise<string[]> =>
    cmd("downloaded_paths", { collection }),

  startDownload: (collection: Collection, opts?: DownloadOpts): Promise<void> =>
    cmd("start_download", { collection, opts: opts ?? null }),

  cancelDownload: (): Promise<void> => cmd("cancel_download"),

  matches: (
    collection: Collection,
  ): Promise<Record<string, SearchCandidate[]>> =>
    cmd("matches", { collection }),

  syncDone: (collection: Collection): Promise<string[]> =>
    cmd("sync_done", { collection }),

  getSettings: (): Promise<Settings> => cmd("get_settings"),

  setSettings: (patch: Partial<Settings>): Promise<Settings> =>
    cmd("set_settings", { patch }),

  getTools: (): Promise<ToolStatus[]> => cmd("get_tools"),

  checkToolUpdates: (): Promise<ToolStatus[]> => cmd("check_tool_updates"),

  installTool: (name: ToolName): Promise<void> => cmd("install_tool", { name }),

  updateTool: (name: ToolName): Promise<void> => cmd("update_tool", { name }),

  getLibrary: (): Promise<LibraryTrack[]> => cmd("get_library"),

  deleteLibraryTrack: (path: string): Promise<void> =>
    cmd("delete_library_track", { path }),

  listPlaylists: (): Promise<PlaylistMeta[]> => cmd("list_playlists"),

  getPlaylist: (id: string): Promise<Playlist | null> =>
    cmd("get_playlist", { id }),

  createPlaylist: (name: string): Promise<Playlist> =>
    cmd("create_playlist", { name }),

  renamePlaylist: (id: string, name: string): Promise<Playlist> =>
    cmd("rename_playlist", { id, name }),

  deletePlaylist: (id: string): Promise<void> => cmd("delete_playlist", { id }),

  addToPlaylist: (id: string, paths: string[]): Promise<Playlist> =>
    cmd("add_to_playlist", { id, paths }),

  removeFromPlaylist: (
    id: string,
    trackPath: string,
  ): Promise<Playlist> => cmd("remove_from_playlist", { id, trackPath }),

  moveInPlaylist: (
    id: string,
    from: number,
    to: number,
  ): Promise<Playlist> => cmd("move_in_playlist", { id, from, to }),

  setPlaylistCover: (
    id: string,
    cover: string | null,
  ): Promise<Playlist> => cmd("set_playlist_cover", { id, cover }),

  savePlaylistCover: (
    id: string,
    ext: string,
    data: number[],
  ): Promise<string> => cmd("save_playlist_cover", { id, ext, data }),

  lyricsLocal: (file: string): Promise<LyricsResult | null> =>
    cmd("lyrics_local", { file }),

  lyricsFetch: (lookup: LyricsLookup): Promise<LyricsResult | null> =>
    cmd("lyrics_fetch", { lookup }),

  lyricsEmbed: (payload: { file: string; text: string }): Promise<void> =>
    cmd("lyrics_embed", payload),

  lyricsSearch: (opts: {
    query: string;
    trackName?: string;
    artistName?: string;
  }): Promise<import("../lib/types").LrcSearchResult[]> =>
    cmd("lyrics_search", opts),

  lyricsExport: (opts: {
    content: string;
    defaultName: string;
  }): Promise<string | null> => cmd("lyrics_export", opts),

  rpcUpdate: (state: RpcPresence | null): Promise<void> =>
    cmd("rpc_update", { state }),

  copyText: (text: string): void => {
    writeText(text).catch((e) => {
      console.warn("clipboard write failed:", e);
    });
  },

  openExternal: (url: string): Promise<void> => openShell(url),

  onToolsUpdate: (cb: (status: ToolStatus) => void): (() => void) =>
    onEvent<ToolStatus>("vynl:tools:update", cb),

  onDownloadEvent: (cb: (e: DownloadEvent) => void): (() => void) =>
    onEvent<DownloadEvent>("vynl:dl", cb),

  saveNowPlaying: (state: NowPlayingState): Promise<void> =>
    cmd("save_now_playing", { state }),

  loadNowPlaying: (): Promise<NowPlayingState | null> =>
    cmd("load_now_playing"),

  clearNowPlaying: (): Promise<void> => cmd("clear_now_playing"),

  checkAppUpdate,

  installAppUpdate,

  restartApp: (): Promise<void> => cmd("restart_app"),

  onUpdateStatus: (cb: (status: UpdateStatus) => void): (() => void) =>
    onEvent<UpdateStatus>("vynl:update:status", (status) => {
      rememberUpdateStatus(status);
      cb(status);
    }),

  onToast: (cb: (summary: DownloadSummary) => void): (() => void) =>
    onEvent<DownloadSummary>("vynl:toast", cb),

  onLibraryUpdated: (
    cb: (tracks: LibraryTrack[]) => void,
    onReady?: () => void,
  ): (() => void) => onEvent<LibraryTrack[]>("library-updated", cb, onReady),

  playerPlay: (
    path: string,
    seekTo?: number,
    generation?: number,
  ): Promise<void> =>
    cmd("player_play", {
      path,
      seekTo: seekTo ?? null,
      generation: generation ?? 0,
    }),

  playerStop: (generation?: number): Promise<void> =>
    cmd("player_stop", { generation: generation ?? 0 }),

  playerPause: (generation?: number): Promise<void> =>
    cmd("player_pause", { generation: generation ?? 0 }),

  playerResume: (generation?: number): Promise<boolean> =>
    cmd("player_resume", { generation: generation ?? 0 }),

  playerSeek: (time: number, generation?: number): Promise<void> =>
    cmd("player_seek", { time, generation: generation ?? 0 }),

  playerSetVolume: (vol: number): Promise<void> =>
    cmd("player_set_volume", { vol }),

  playerGetPosition: (generation?: number): Promise<number> =>
    cmd("player_get_position", { generation: generation ?? 0 }),

  playerIsPlaying: (generation?: number): Promise<boolean> =>
    cmd("player_is_playing", { generation: generation ?? 0 }),

  playerCheckFinished: (generation?: number): Promise<boolean> =>
    cmd("player_check_finished", { generation: generation ?? 0 }),

  cacheRemoteAudio: (url: string, key: string): Promise<string> =>
    cmd("cache_remote_audio", { url, key }),

  clearAudioCache: (): Promise<void> => cmd("clear_audio_cache"),

  fetchArtistInfo: (artist: string): Promise<import("../lib/types").ArtistInfo> =>
    cmd("fetch_artist_info", { artist }),

  pluginsList: (): Promise<import("./plugins/types").PluginEntry[]> =>
    cmd("plugins_list"),

  pluginsSetEnabled: (
    id: string,
    enabled: boolean,
  ): Promise<import("./plugins/types").PluginEntry> =>
    cmd("plugins_set_enabled", { id, enabled }),

  pluginsRemove: (id: string): Promise<void> => cmd("plugins_remove", { id }),

  pluginsInstallFromUrl: (
    url: string,
    repo: string | null,
  ): Promise<import("./plugins/types").PluginEntry> =>
    cmd("plugins_install_from_url", { url, repo }),

  pluginsPickFolder: (): Promise<string | null> => cmd("plugins_pick_folder"),

  pluginsInstallFromFolder: (
    path: string,
  ): Promise<import("./plugins/types").PluginEntry> =>
    cmd("plugins_install_from_folder", { path }),

  pluginsPickZip: (): Promise<string | null> => cmd("plugins_pick_zip"),

  pluginsInstallFromZip: (
    path: string,
  ): Promise<import("./plugins/types").PluginEntry> =>
    cmd("plugins_install_from_zip", { path }),

  pluginsReloadDev: (
    id: string,
  ): Promise<import("./plugins/types").PluginEntry> =>
    cmd("plugins_reload_dev", { id }),

  pluginsReadFile: (id: string, path: string): Promise<string> =>
    cmd("plugins_read_file", { id, path }),

  pluginGetConfig: (id: string): Promise<Record<string, unknown>> =>
    cmd("plugin_get_config", { id }),

  pluginSetConfig: (id: string, config: Record<string, unknown>): Promise<void> =>
    cmd("plugin_set_config", { id, config }),

  pluginHttpFetch: (
    url: string,
    opts: PluginHttpInit | null,
  ): Promise<PluginHttpResponse> =>
    cmd("plugin_http_fetch", { url, opts }),

  pluginValidateLocalMedia: (path: string, kind: string): Promise<string> =>
    cmd("plugin_validate_local_media", { path, kind }),

  pluginStoreFetch: (sources: string[]): Promise<PluginStoreResult> =>
    cmd("plugin_store_fetch", { sources }),
};

function onEvent<T>(
  event: string,
  cb: (payload: T) => void,
  onReady?: () => void,
): () => void {
  let unlisten: UnlistenFn | null = null;
  let disposed = false;
  let unlistenCalled = false;
  const p = listen<T>(event, (e) => cb(e.payload));
  p.then(
    (fn) => {
      onReady?.();
      if (disposed) {
        if (!unlistenCalled) {
          unlistenCalled = true;
          fn();
        }
        return;
      }
      unlisten = fn;
    },
    () => onReady?.(),
  );
  return () => {
    if (disposed) return;
    disposed = true;
    if (unlisten) {
      unlistenCalled = true;
      unlisten();
    } else if (!unlistenCalled) {
      unlistenCalled = true;
      void p.then(
        (fn) => fn(),
        () => {},
      );
    }
  };
}