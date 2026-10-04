import '../models.dart';
import 'api.dart';
import 'library_db.dart';
import 'notifications.dart';

class SyncProgress {
  SyncProgress({
    required this.phase,
    required this.done,
    required this.total,
    this.currentTitle,
    this.bytesReceived,
    this.error,
  });

  final String phase;
  final int done;
  final int total;
  final String? currentTitle;
  final int? bytesReceived;
  final String? error;

  double get fraction => total <= 0 ? 0 : done / total;
}

/// How many tracks download at once. LAN transfers are latency-bound, so a
/// small pool of parallel requests far beats the old one-at-a-time loop while
/// still leaving the phone's radio and disk headroom.
const int _downloadConcurrency = 6;

/// Minimum gap between system notification updates. Without this, six workers
/// finishing together would post a burst of notifications per track.
const Duration _notifyInterval = Duration(milliseconds: 400);

/// Minimum gap between in-app progress updates driven by download chunks.
const Duration _uiInterval = Duration(milliseconds: 120);

class SyncService {
  SyncService({
    required this.db,
    required this.notifications,
  });

  final LibraryDb db;
  final SyncNotifications notifications;

  bool _cancelled = false;

  void cancel() => _cancelled = true;

  Future<void> sync({
    required VynlApi api,
    required void Function(SyncProgress) onProgress,
  }) async {
    _cancelled = false;
    onProgress(SyncProgress(phase: 'Fetching manifest', done: 0, total: 1));

    final manifest = await api.syncManifest();
    if (_cancelled) return;

    final catalog = await api.catalog();
    if (_cancelled) return;

    final sizeById = {for (final m in manifest) m.id: m.size};
    final mtimeById = {for (final m in manifest) m.id: m.mtime ?? 0};
    final catalogById = {for (final t in catalog) t.id: t};

    // One query for mtimes and local paths, replacing the per-track `getTrack`
    // lookup that previously ran once for every track in the catalog.
    final local = await db.localSyncState();

    final needDownload = <CatalogTrack>[];
    for (final entry in manifest) {
      final track = catalogById[entry.id];
      if (track == null) continue;
      final existingPath = local[entry.id]?.audio;
      final localMtime = local[entry.id]?.mtime ?? 0;
      final remoteMtime = mtimeById[entry.id] ?? 0;
      final missing = existingPath == null || existingPath.isEmpty;
      final changed = remoteMtime != 0 && remoteMtime != localMtime;
      if (missing || changed) {
        needDownload.add(track);
      }
    }

    final merged = <CatalogTrack>[];
    for (final t in catalog) {
      final existing = local[t.id];
      merged.add(
        t.copyWith(
          localAudioPath: existing?.audio,
          localCoverPath: existing?.cover,
        ),
      );
    }
    await db.upsertTracks(merged, sizes: sizeById);
    await db.deleteTracksNotIn(catalogById.keys.toSet());

    final total = needDownload.length;
    var done = 0;
    final errors = <String>[];

    if (total > 0) {
      onProgress(SyncProgress(phase: 'Downloading', done: 0, total: total));

      // Shared cursor drained by a fixed pool of workers. Dart runs this on one
      // isolate, so plain reads/writes here need no extra synchronisation.
      var next = 0;
      var lastNotify = DateTime.fromMillisecondsSinceEpoch(0);
      var lastUi = DateTime.fromMillisecondsSinceEpoch(0);

      Future<void> worker() async {
        while (!_cancelled) {
          final index = next;
          if (index >= needDownload.length) return;
          next += 1;

          final track = needDownload[index];
          final error = await _downloadOne(
            api: api,
            track: track,
            size: sizeById[track.id],
            onBytes: (received) {
              final now = DateTime.now();
              if (now.difference(lastUi) < _uiInterval) return;
              lastUi = now;
              onProgress(
                SyncProgress(
                  phase: 'Downloading',
                  done: done,
                  total: total,
                  currentTitle: track.title,
                  bytesReceived: received,
                ),
              );
            },
          );
          if (error != null) {
            errors.add('${track.title}: $error');
          }

          done += 1;
          onProgress(
            SyncProgress(
              phase: 'Downloading',
              done: done,
              total: total,
              currentTitle: track.title,
            ),
          );

          final now = DateTime.now();
          if (now.difference(lastNotify) >= _notifyInterval) {
            lastNotify = now;
            await notifications.showProgress(
              done: done,
              total: total,
              detail: track.title,
            );
          }
        }
      }

      final workers = <Future<void>>[
        for (
          var i = 0;
          i < _downloadConcurrency && i < needDownload.length;
          i++
        )
          worker(),
      ];
      await Future.wait(workers);
    }

    if (_cancelled) return;

    onProgress(SyncProgress(phase: 'Playlists', done: total, total: total));
    final playlists = await api.playlists();
    await db.replacePlaylists(playlists);

    final errSummary = errors.isEmpty
        ? null
        : '${errors.length} failed — ${errors.first}';
    await notifications.showDone(count: done, error: errSummary);
    onProgress(
      SyncProgress(
        phase: 'Done',
        done: done,
        total: total,
        error: errSummary,
      ),
    );
  }

  Future<Object?> _downloadOne({
    required VynlApi api,
    required CatalogTrack track,
    required int? size,
    required void Function(int received) onBytes,
  }) async {
    try {
      final audioDest = db.audioFileFor(track.id, track.ext);
      await api.downloadAudio(
        trackId: track.id,
        dest: audioDest,
        onProgress: (received, _) => onBytes(received),
      );
      await db.setLocalAudio(track.id, audioDest.path, size: size);

      if (track.hasCover) {
        try {
          final coverDest = db.coverFileFor(track.id);
          await api.downloadCover(trackId: track.id, dest: coverDest);
          await db.setLocalCover(track.id, coverDest.path);
        } catch (_) {}
      }

      return null;
    } catch (e) {
      return e;
    }
  }
}
