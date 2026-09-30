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

    final localMtimes = await db.localMtimes();
    final localAudio = await db.localAudioPaths();

    final needDownload = <CatalogTrack>[];
    for (final entry in manifest) {
      final track = catalogById[entry.id];
      if (track == null) continue;
      final existingPath = localAudio[entry.id];
      final localMtime = localMtimes[entry.id] ?? 0;
      final remoteMtime = mtimeById[entry.id] ?? 0;
      final missing = existingPath == null || existingPath.isEmpty;
      final changed = remoteMtime != 0 && remoteMtime != localMtime;
      if (missing || changed) {
        needDownload.add(track);
      }
    }

    final merged = <CatalogTrack>[];
    for (final t in catalog) {
      final existing = await db.getTrack(t.id);
      merged.add(
        t.copyWith(
          localAudioPath: existing?.localAudioPath,
          localCoverPath: existing?.localCoverPath,
        ),
      );
    }
    await db.upsertTracks(merged, sizes: sizeById);
    await db.deleteTracksNotIn(catalogById.keys.toSet());

    final total = needDownload.length;
    var done = 0;
    final errors = <String>[];

    for (final track in needDownload) {
      if (_cancelled) break;
      onProgress(
        SyncProgress(
          phase: 'Downloading',
          done: done,
          total: total,
          currentTitle: track.title,
        ),
      );
      await notifications.showProgress(
        done: done,
        total: total <= 0 ? 1 : total,
        detail: track.title,
      );

      try {
        final audioDest = db.audioFileFor(track.id, track.ext);
        await api.downloadAudio(
          trackId: track.id,
          dest: audioDest,
          onProgress: (received, totalBytes) {
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
        await db.setLocalAudio(
          track.id,
          audioDest.path,
          size: sizeById[track.id],
        );

        if (track.hasCover) {
          try {
            final coverDest = db.coverFileFor(track.id);
            await api.downloadCover(trackId: track.id, dest: coverDest);
            await db.setLocalCover(track.id, coverDest.path);
          } catch (_) {
          }
        }
      } catch (e) {
        errors.add('${track.title}: $e');
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
}
