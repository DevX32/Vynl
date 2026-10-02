import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../models.dart';
import '../services/api.dart';
import '../services/auth_store.dart';
import '../services/library_db.dart';
import '../services/notifications.dart';
import '../services/sync_service.dart';
import '../theme.dart';

class AppState extends ChangeNotifier {
  AppState({
    required this.authStore,
    required this.db,
    required this.notifications,
  }) : syncService = SyncService(db: db, notifications: notifications);

  static const tabHome = 0;
  static const tabLibrary = 1;
  static const tabPlaylists = 2;

  static const _displayNameKey = 'vynl.display_name';
  static const _maxDisplayName = 32;

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
  int tabIndex = tabHome;
  String displayName = '';

  void setTab(int index) {
    if (tabIndex == index) return;
    tabIndex = index;
    notifyListeners();
  }

  Future<void> setDisplayName(String value) async {
    final next = value.trim();
    final clipped = next.length > _maxDisplayName
        ? next.substring(0, _maxDisplayName)
        : next;
    if (clipped == displayName) return;
    displayName = clipped;
    notifyListeners();
    try {
      final prefs = await SharedPreferences.getInstance();
      await prefs.setString(_displayNameKey, clipped);
    } catch (e) {
      debugPrint('display name save failed: $e');
    }
  }

  Future<void> _loadDisplayName() async {
    try {
      final prefs = await SharedPreferences.getInstance();
      displayName = prefs.getString(_displayNameKey) ?? '';
    } catch (e) {
      debugPrint('display name load failed: $e');
    }
  }

  static String describePairFailure(Object error) {
    final text = error.toString();
    const networkHints = [
      'SocketException',
      'Failed host lookup',
      'Connection refused',
      'Connection timed out',
      'Network is unreachable',
      'No address associated with hostname',
      'TimeoutException',
      'HandshakeException',
    ];
    if (networkHints.any(text.contains)) {
      return "Couldn't reach the desktop. Vynl pairs over the local "
          'network, so both devices need to be on the same Wi-Fi.';
    }
    return text;
  }

  static final _minSplashMs = VynlMotion.splashEntrance.inMilliseconds;

  Future<void> bootstrap() async {
    final started = DateTime.now();
    loading = true;
    notifyListeners();
    try {
      await db.open();
      await notifications.init();
      await _loadDisplayName();
      credentials = await authStore.load();
      if (credentials != null) {
        api = VynlApi(baseUrl: credentials!.baseUrl, token: credentials!.token);
      }
      await refreshLocal();
    } catch (e, st) {
      lastError = 'Startup failed: $e';
      debugPrint('bootstrap failed: $e\n$st');
    } finally {
      final elapsed = DateTime.now().difference(started).inMilliseconds;
      if (elapsed < _minSplashMs) {
        await Future<void>.delayed(
          Duration(milliseconds: _minSplashMs - elapsed),
        );
      }
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

  Future<void> pair(String host, String pin, {List<String> alts = const []}) async {
    lastError = null;
    notifyListeners();

    Object? failure;
    for (final candidate in [host, ...alts]) {
      try {
        await _pairOne(candidate, pin);
        return;
      } catch (e) {
        failure = e;
      }
    }

    lastError = describePairFailure(failure ?? StateError('pairing failed'));
    notifyListeners();
    throw StateError(lastError!);
  }

  Future<void> _pairOne(String candidate, String pin) async {
    var base = candidate.trim();
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
    final creds = await VynlApi.pair(baseUrl: base, pin: pin);
    await authStore.save(creds);
    credentials = creds;
    api = VynlApi(baseUrl: creds.baseUrl, token: creds.token);
    notifyListeners();
    unawaited(runSync());
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
