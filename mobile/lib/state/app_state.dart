import 'package:flutter/foundation.dart';

import '../models.dart';
import '../services/api.dart';
import '../services/auth_store.dart';
import '../services/library_db.dart';
import '../services/notifications.dart';
import '../services/sync_service.dart';

class AppState extends ChangeNotifier {
  AppState({
    required this.authStore,
    required this.db,
    required this.notifications,
  }) : syncService = SyncService(db: db, notifications: notifications);

  final AuthStore authStore;
  final LibraryDb db;
  final SyncNotifications notifications;
  final SyncService syncService;

  PairingCredentials? credentials;
  VynlApi? api;
  List<CatalogTrack> tracks = [];
  List<PlaylistDto> playlists = [];
  bool loading = true;
  bool syncing = false;
  SyncProgress? syncProgress;
  String? lastError;
  int cacheBytes = 0;

  Future<void> bootstrap() async {
    loading = true;
    notifyListeners();
    try {
      await db.open();
      await notifications.init();
      credentials = await authStore.load();
      if (credentials != null) {
        api = VynlApi(baseUrl: credentials!.baseUrl, token: credentials!.token);
      }
      await refreshLocal();
    } catch (e, st) {
      lastError = 'Startup failed: $e';
      debugPrint('bootstrap failed: $e\n$st');
    } finally {
      loading = false;
      notifyListeners();
    }
  }

  Future<void> refreshLocal() async {
    tracks = await db.allTracks();
    playlists = await db.allPlaylists();
    cacheBytes = await db.cacheBytes();
    notifyListeners();
  }

  bool get isPaired => credentials != null;

  List<CatalogTrack> get downloadedTracks =>
      tracks.where((t) => t.isDownloaded).toList();

  Future<void> pair(String host, String pin) async {
    lastError = null;
    notifyListeners();
    var base = host.trim();
    if (!base.startsWith('http://') && !base.startsWith('https://')) {
      base = 'http://$base';
    }
    final uri = Uri.parse(base);
    final hostOnly = uri.host.isEmpty ? base.replaceFirst(RegExp(r'^https?://'), '') : uri.host;
    final hasExplicitPort = RegExp(r':\d+(/|$)').hasMatch(base) ||
        (uri.hasPort && uri.port != 80 && uri.port != 443);
    if (hasExplicitPort) {
      base = uri.hasPort
          ? '${uri.scheme}://${uri.host}:${uri.port}'
          : base.split('/').take(3).join('/');
    } else {
      base = '${uri.scheme.isEmpty ? 'http' : uri.scheme}://$hostOnly:17865';
    }
    try {
      final creds = await VynlApi.pair(baseUrl: base, pin: pin);
      await authStore.save(creds);
      credentials = creds;
      api = VynlApi(baseUrl: creds.baseUrl, token: creds.token);
      notifyListeners();
      await runSync();
    } catch (e) {
      lastError = e.toString();
      notifyListeners();
      rethrow;
    }
  }

  Future<void> unpair() async {
    await authStore.clear();
    credentials = null;
    api = null;
    notifyListeners();
  }

  Future<void> runSync() async {
    final client = api;
    if (client == null || syncing) return;
    syncing = true;
    lastError = null;
    syncProgress = SyncProgress(phase: 'Starting', done: 0, total: 1);
    notifyListeners();
    try {
      await syncService.sync(
        api: client,
        onProgress: (p) {
          syncProgress = p;
          notifyListeners();
        },
      );
      await refreshLocal();
    } catch (e) {
      lastError = e.toString();
      syncProgress = SyncProgress(
        phase: 'Error',
        done: 0,
        total: 1,
        error: lastError,
      );
    } finally {
      syncing = false;
      notifyListeners();
    }
  }

  Future<void> clearCache() async {
    await db.clearCache();
    await refreshLocal();
  }

  CatalogTrack? trackById(String id) {
    for (final t in tracks) {
      if (t.id == id) return t;
    }
    return null;
  }

  List<CatalogTrack> tracksForPlaylist(PlaylistDto pl) {
    return pl.trackIds
        .map(trackById)
        .whereType<CatalogTrack>()
        .where((t) => t.isDownloaded)
        .toList();
  }
}
