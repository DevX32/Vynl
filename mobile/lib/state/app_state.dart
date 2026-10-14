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
import 'nickname.dart';

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
  static const _nicknameKey = 'vynl.nickname';
  static const _maxDisplayName = 32;

  final AuthStore authStore;
  final LibraryDb db;
  final SyncNotifications notifications;
  final SyncService syncService;

  final Map<String, CatalogTrack> _trackIndex = {};

  Future<void>? _syncInFlight;
  bool _syncQueued = false;

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
  String nickname = randomNickname();

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
      nickname = prefs.getString(_nicknameKey) ?? '';
      if (nickname.isEmpty) {
        nickname = randomNickname();
        await prefs.setString(_nicknameKey, nickname);
      }
    } catch (e) {
      debugPrint('display name load failed: $e');
      if (nickname.isEmpty) nickname = randomNickname();
    }
  }

  String get greetingName =>
      displayName.trim().isEmpty ? nickname : displayName.trim();

  void rollNickname() {
    nickname = randomNickname();
    notifyListeners();
    _saveNickname();
  }

  Future<void> _saveNickname() async {
    try {
      final prefs = await SharedPreferences.getInstance();
      await prefs.setString(_nicknameKey, nickname);
    } catch (e) {
      debugPrint('nickname save failed: $e');
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
    _trackIndex
      ..clear()
      ..addEntries(tracks.map((t) => MapEntry(t.id, t)));
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

    final bases = _candidateBases(host, alts);
    Object? failure;
    for (final base in bases) {
      try {
        final creds = await VynlApi.pair(baseUrl: base, pin: pin);
        await _adopt(
          creds.copyWith(
            alternates: bases.where((b) => b != base).toList(),
          ),
        );
        unawaited(runSync());
        return;
      } catch (e) {
        failure = e;
      }
    }

    lastError = describePairFailure(failure ?? StateError('pairing failed'));
    notifyListeners();
    throw StateError(lastError!);
  }

  Future<void> _adopt(PairingCredentials creds) async {
    api?.dispose();
    await authStore.save(creds);
    credentials = creds;
    api = VynlApi(baseUrl: creds.baseUrl, token: creds.token);
    notifyListeners();
  }

  static String _normalizeBase(String candidate) {
    var base = candidate.trim();
    if (!base.startsWith('http://') && !base.startsWith('https://')) {
      base = 'http://$base';
    }
    final uri = Uri.parse(base);
    final hostOnly = uri.host.isEmpty
        ? base.replaceFirst(RegExp(r'^https?://'), '')
        : uri.host;
    final hasExplicitPort = RegExp(r':\d+(/|$)').hasMatch(base) ||
        (uri.hasPort && uri.port != 80 && uri.port != 443);
    if (hasExplicitPort) {
      return uri.hasPort
          ? '${uri.scheme}://${uri.host}:${uri.port}'
          : base.split('/').take(3).join('/');
    }
    return '${uri.scheme.isEmpty ? 'http' : uri.scheme}://$hostOnly:17865';
  }

  static List<String> _candidateBases(String host, List<String> alts) {
    final out = <String>[];
    for (final candidate in [host, ...alts]) {
      final base = _normalizeBase(candidate);
      if (base.isNotEmpty && !out.contains(base)) out.add(base);
    }
    return out;
  }

  Future<VynlApi?> _reconnect(VynlApi stale) async {
    final creds = credentials;
    if (creds == null) return null;

    for (final base in [creds.baseUrl, ...creds.alternates]) {
      if (base == stale.baseUrl) continue;
      final probe = VynlApi(baseUrl: base, token: creds.token);
      if (await probe.ping()) {
        final remaining = [
          creds.baseUrl,
          ...creds.alternates.where((a) => a != base),
        ].where((a) => a != base).toList();
        await _adopt(creds.copyWith(baseUrl: base, alternates: remaining));
        probe.dispose();
        debugPrint('desktop moved to $base');
        return api;
      }
      probe.dispose();
    }
    return null;
  }

  Future<void> unpair() async {
    syncService.cancel();
    _syncQueued = false;
    api?.dispose();
    await authStore.clear();
    credentials = null;
    api = null;
    notifyListeners();
  }

  Future<void> runSync() async {
    if (api == null) return;

    final running = _syncInFlight;
    if (running != null) {
      _syncQueued = true;
      await running;
      return;
    }

    final pass = _runSyncPass();
    _syncInFlight = pass;
    try {
      await pass;
    } finally {
      _syncInFlight = null;
      if (_syncQueued) {
        _syncQueued = false;
        unawaited(runSync());
      }
    }
  }

  Future<void> _runSyncPass() async {
    var client = api;
    if (client == null) return;
    syncing = true;
    lastError = null;
    syncProgress = SyncProgress(phase: 'Starting', done: 0, total: 1);
    notifyListeners();
    try {
      if (!await client.ping()) {
        syncProgress = SyncProgress(
          phase: 'Looking for your desktop',
          done: 0,
          total: 1,
        );
        notifyListeners();
        client = await _reconnect(client) ?? client;
      }
      await syncService.sync(
        api: client,
        onProgress: (p) {
          syncProgress = p;
          notifyListeners();
        },
      );
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
    }
    await refreshLocal();
  }

  Future<void> clearCache() async {
    await db.clearCache();
    await refreshLocal();
  }

  CatalogTrack? trackById(String id) => _trackIndex[id];

  List<CatalogTrack> tracksForPlaylist(PlaylistDto pl) {
    return pl.trackIds
        .map(trackById)
        .whereType<CatalogTrack>()
        .where((t) => t.isDownloaded)
        .toList();
  }

  ({int total, int downloaded}) playlistStats(PlaylistDto pl) {
    var downloaded = 0;
    for (final id in pl.trackIds) {
      final t = _trackIndex[id];
      if (t != null && t.isDownloaded) downloaded += 1;
    }
    return (total: pl.trackIds.length, downloaded: downloaded);
  }
}
