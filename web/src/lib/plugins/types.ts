import type { ArtistInfo, Collection } from "@lib/types";

export const PLUGIN_API_VERSION = 1;

export type PluginPermission = "network" | "shell" | "player";

export const PLUGIN_PERMISSIONS: PluginPermission[] = [
  "network",
  "shell",
  "player",
];

type PluginInstallationMethod = "store" | "dev" | "sideload";

export interface PluginManifest {
  name: string;
  version: string;
  description?: string;
  author?: string;
  main?: string;
  license?: string;
  vynl?: {
    displayName?: string;
    categories?: string[];
    permissions?: string[];
  };
}

export interface PluginEntry {
  id: string;
  version: string;
  path: string;
  installationMethod: PluginInstallationMethod;
  originalPath: string | null;
  enabled: boolean;
  installedAt: number;
  lastUpdatedAt: number;
  displayName?: string;
  description?: string;
  author?: string;
  categories: string[];
  sourceRepo?: string;
}

export interface StorePlugin {
  id: string;
  name: string;
  description: string;
  author: string;
  repo: string | null;
  categories: string[];
  tags: string[];
  version: string | null;
  downloadUrl: string | null;
  homepage: string | null;
  addedAt: string | null;
}

export interface PluginStoreResult {
  entries: StorePlugin[];
  sourceError: string | null;
}

declare global {
  // eslint-disable-next-line no-var
  var __VYNL_PLUGIN_SDK__:
    | { readonly PLUGIN_API_VERSION: number }
    | undefined;
}


type PluginSettingFieldKind = "boolean" | "number" | "text" | "select";

export interface PluginSettingField {
  key: string;
  title: string;
  description?: string;
  kind: PluginSettingFieldKind;
  default?: boolean | number | string;
  options?: { value: string; label: string }[];
  min?: number;
  max?: number;
  step?: number;
}

export interface PluginSettingsSection {
  id: string;
  title: string;
  fields: PluginSettingField[];
  pluginId?: string;
  pluginName?: string;
}


interface PluginHomeItem {
  id: string;
  title: string;
  subtitle?: string;
  url?: string;
  trackId?: string;
}

export interface PluginHomeSection {
  id: string;
  title: string;
  items: PluginHomeItem[];
  pluginId?: string;
  pluginName?: string;
}


export interface PluginLyricsLookup {
  title: string;
  artist: string;
  album: string;
  duration: number;
}

export interface PluginLyricsResult {
  kind: "lrc" | "txt";
  text: string;
}

export interface PluginLyricsProvider {
  id: string;
  kind: "lyrics";
  name: string;
  fetch: (lookup: PluginLyricsLookup) => Promise<PluginLyricsResult | null>;
}

export interface PluginMetadataProvider {
  id: string;
  kind: "metadata";
  name: string;
  artistInfo?: (artist: string) => Promise<ArtistInfo | null>;
}

export interface PluginResolverProvider {
  id: string;
  kind: "resolver";
  name: string;
  canHandle: (url: string) => boolean | Promise<boolean>;
  resolve: (url: string) => Promise<Collection>;
}

export type PluginProvider =
  | PluginLyricsProvider
  | PluginMetadataProvider
  | PluginResolverProvider;

export type PluginProviderKind = PluginProvider["kind"];


export interface PluginHttpInit {
  method?: string;
  headers?: Record<string, string>;
  body?: string;
}

export interface PluginHttpResponse {
  status: number;
  ok: boolean;
  headers: Record<string, string>;
  text: string;
}


export interface PluginPlayerState {
  playing: boolean;
  position: number;
  shared: boolean;
  lyrics: { text: string; kind: "lrc" | "txt" } | null;
  track: {
    id: string;
    title: string;
    artist: string;
    album: string;
    duration: number;
    path?: string;
  } | null;
}

export interface PluginSharedState {
  sourceUrl?: string | null;
  cacheKey?: string | null;
  title: string;
  artist: string;
  album?: string;
  duration?: number;
  coverUrl?: string | null;
  lyrics?: string | null;
  time: number;
  playing: boolean;
}


export interface PluginPage {
  title: string;
  pluginId?: string;
  pluginName?: string;
}

export type PluginPageMount = (container: HTMLElement) => void | (() => void);

export interface PluginPageRecord extends PluginPage {
  pluginId: string;
  pluginName: string;
  mount: PluginPageMount;
}


export interface VynlPluginAPI {
  readonly pluginId: string;
  readonly permissions: readonly string[];
  readonly apiVersion: number;

  Settings: {
    register(section: PluginSettingsSection): void;
    get(key: string): Promise<unknown>;
    set(key: string, value: unknown): Promise<void>;
    subscribe(key: string, cb: (value: unknown) => void): () => void;
  };

  Providers: {
    register<T extends PluginProvider>(provider: T): string;
    unregister(id: string): boolean;
    list(kind?: PluginProviderKind): PluginProvider[];
    get<T extends PluginProvider>(id: string, kind: PluginProviderKind): T | undefined;
    subscribe(cb: () => void): () => void;
  };

  UI: {
    registerSettingsSection(section: PluginSettingsSection): string;
    registerHomeSection(section: PluginHomeSection): string;
    unregisterSettingsSection(id: string): void;
    unregisterHomeSection(id: string): void;
    registerPage(page: PluginPage, mount: PluginPageMount): string;
    unregisterPage(): void;
  };

  Player: {
    getState(): PluginPlayerState;
    isShared(): boolean;
    play(): void;
    pause(): void;
    seek(seconds: number): void;
    playShared(state: PluginSharedState): Promise<void>;
    stopShared(): void;
    getLocalFileUrl(kind: "audio" | "cover"): Promise<string | null>;
    clearAudioCache(): Promise<void>;
  };

  Http: {
    fetch(url: string, init?: PluginHttpInit): Promise<PluginHttpResponse>;
  };

  Shell: {
    openExternal(url: string): Promise<void>;
  };

  Events: {
    on(event: string, cb: (payload: unknown) => void): () => void;
  };

  Logger: {
    debug(message: string): void;
    info(message: string): void;
    warn(message: string): void;
    error(message: string): void;
  };
}

export const PLUGIN_EVENTS = [
  "track-changed",
  "playback-changed",
  "download-finished",
  "library-updated",
] as const;

export interface VynlPlugin {
  onLoad?(api: VynlPluginAPI): void | Promise<void>;
  onEnable?(api: VynlPluginAPI): void | Promise<void>;
  onDisable?(api: VynlPluginAPI): void | Promise<void>;
  onUnload?(api: VynlPluginAPI): void | Promise<void>;
}
