import 'dart:io';

import 'package:path/path.dart' as p;
import 'package:path_provider/path_provider.dart';
import 'package:sqflite/sqflite.dart';

import '../models.dart';

class LibraryDb {
  static const _dbVersion = 3;

  Database? _db;
  Directory? _mediaDir;

  Future<void> open() async {
    final docs = await getApplicationDocumentsDirectory();
    _mediaDir = Directory(p.join(docs.path, 'media'));
    await _mediaDir!.create(recursive: true);
    final dbPath = p.join(docs.path, 'vynl.db');
    _db = await openDatabase(
      dbPath,
      version: _dbVersion,
      onCreate: (db, version) async {
        await db.execute('''
          CREATE TABLE tracks (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            artist TEXT NOT NULL,
            album TEXT NOT NULL,
            year INTEGER,
            duration REAL NOT NULL,
            track_number INTEGER,
            lyrics TEXT,
            ext TEXT NOT NULL,
            mtime INTEGER,
            has_cover INTEGER NOT NULL,
            local_audio_path TEXT,
            local_cover_path TEXT,
            size INTEGER DEFAULT 0
          )
        ''');
        await db.execute('''
          CREATE TABLE playlists (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            cover TEXT,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
          )
        ''');
        await db.execute('''
          CREATE TABLE playlist_tracks (
            playlist_id TEXT NOT NULL,
            track_id TEXT NOT NULL,
            position INTEGER NOT NULL,
            PRIMARY KEY (playlist_id, position)
          )
        ''');
        await db.execute('''
          CREATE TABLE sync_meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
          )
        ''');
        await db.execute('''
          CREATE TABLE excluded_tracks (
            id TEXT PRIMARY KEY
          )
        ''');
      },
      onUpgrade: (db, from, to) async {
        if (from < 2) {
          await db.execute('ALTER TABLE playlists ADD COLUMN cover TEXT');
        }
        if (from < 3) {
          await db.execute('''
            CREATE TABLE excluded_tracks (
              id TEXT PRIMARY KEY
            )
          ''');
        }
      },
    );
  }

  Database get db {
    final d = _db;
    if (d == null) throw StateError('Database not open');
    return d;
  }

  Directory get mediaDir {
    final d = _mediaDir;
    if (d == null) throw StateError('Media dir not ready');
    return d;
  }

  File audioFileFor(String id, String ext) =>
      File(p.join(mediaDir.path, 'audio', '$id.$ext'));

  File coverFileFor(String id) =>
      File(p.join(mediaDir.path, 'covers', '$id.jpg'));

  File coverFileForPlaylist(String id) =>
      File(p.join(mediaDir.path, 'covers', 'pl-$id.jpg'));

  Future<List<CatalogTrack>> allTracks() async {
    final rows = await db.rawQuery(
      'SELECT * FROM tracks '
      'WHERE id NOT IN (SELECT id FROM excluded_tracks) '
      'ORDER BY title COLLATE NOCASE ASC',
    );
    return rows.map(_trackFromRow).toList();
  }

  Future<CatalogTrack?> getTrack(String id) async {
    final rows = await db.query('tracks', where: 'id = ?', whereArgs: [id]);
    if (rows.isEmpty) return null;
    return _trackFromRow(rows.first);
  }

  Future<void> upsertTracks(List<CatalogTrack> tracks, {Map<String, int>? sizes}) async {
    final batch = db.batch();
    for (final t in tracks) {
      batch.insert(
        'tracks',
        {
          'id': t.id,
          'title': t.title,
          'artist': t.artist,
          'album': t.album,
          'year': t.year,
          'duration': t.duration,
          'track_number': t.trackNumber,
          'lyrics': t.lyrics,
          'ext': t.ext,
          'mtime': t.mtime,
          'has_cover': t.hasCover ? 1 : 0,
          'local_audio_path': t.localAudioPath,
          'local_cover_path': t.localCoverPath,
          'size': sizes?[t.id] ?? 0,
        },
        conflictAlgorithm: ConflictAlgorithm.replace,
      );
    }
    await batch.commit(noResult: true);
  }

  Future<void> setLocalAudio(String id, String path, {int? size}) async {
    await db.update(
      'tracks',
      {
        'local_audio_path': path,
        if (size != null) 'size': size,
      },
      where: 'id = ?',
      whereArgs: [id],
    );
  }

  Future<void> setLocalCover(String id, String path) async {
    await db.update(
      'tracks',
      {'local_cover_path': path, 'has_cover': 1},
      where: 'id = ?',
      whereArgs: [id],
    );
  }

  Future<Set<String>> excludedIds() async {
    final rows = await db.query('excluded_tracks', columns: ['id']);
    return {for (final row in rows) row['id'] as String};
  }

  Future<void> setExcluded(String id, bool excluded) async {
    if (excluded) {
      await db.insert(
        'excluded_tracks',
        {'id': id},
        conflictAlgorithm: ConflictAlgorithm.replace,
      );
    } else {
      await db.delete('excluded_tracks', where: 'id = ?', whereArgs: [id]);
    }
  }

  Future<void> removeDownload(String id) async {
    final rows = await db.query(
      'tracks',
      columns: ['local_audio_path', 'local_cover_path', 'ext'],
      where: 'id = ?',
      whereArgs: [id],
    );
    if (rows.isNotEmpty) {
      final audio = rows.first['local_audio_path'] as String?;
      final cover = rows.first['local_cover_path'] as String?;
      final ext = rows.first['ext'] as String?;
      if (audio != null && audio.isNotEmpty) {
        final f = File(audio);
        if (await f.exists()) await f.delete();
      }
      if (cover != null && cover.isNotEmpty) {
        final f = File(cover);
        if (await f.exists()) await f.delete();
      }
      if (ext != null && ext.isNotEmpty) {
        final part = File('${audioFileFor(id, ext).path}.part');
        if (await part.exists()) await part.delete();
      }
      final coverPart = File('${coverFileFor(id).path}.part');
      if (await coverPart.exists()) await coverPart.delete();
    }

    await db.transaction((txn) async {
      await txn.update(
        'tracks',
        {'local_audio_path': null, 'local_cover_path': null, 'size': 0},
        where: 'id = ?',
        whereArgs: [id],
      );
      await txn.insert(
        'excluded_tracks',
        {'id': id},
        conflictAlgorithm: ConflictAlgorithm.replace,
      );
    });
  }

  Future<void> deleteTracksNotIn(Set<String> keepIds) async {
    final rows = await db.query('tracks', columns: ['id', 'local_audio_path', 'local_cover_path']);
    for (final row in rows) {
      final id = row['id'] as String;
      if (keepIds.contains(id)) continue;
      final audio = row['local_audio_path'] as String?;
      final cover = row['local_cover_path'] as String?;
      if (audio != null) {
        final f = File(audio);
        if (await f.exists()) await f.delete();
      }
      if (cover != null) {
        final f = File(cover);
        if (await f.exists()) await f.delete();
      }
      await db.delete('tracks', where: 'id = ?', whereArgs: [id]);
    }

    final excluded = await db.query('excluded_tracks', columns: ['id']);
    for (final row in excluded) {
      final id = row['id'] as String;
      if (keepIds.contains(id)) continue;
      await db.delete('excluded_tracks', where: 'id = ?', whereArgs: [id]);
    }
  }

  Future<void> replacePlaylists(List<PlaylistDto> playlists) async {
    final covers = await localPlaylistCovers();
    await db.transaction((txn) async {
      await txn.delete('playlist_tracks');
      await txn.delete('playlists');
      final batch = txn.batch();
      for (final pl in playlists) {
        batch.insert(
          'playlists',
          {
            'id': pl.id,
            'name': pl.name,
            'cover': covers[pl.id],
            'created_at': pl.createdAt,
            'updated_at': pl.updatedAt,
          },
          conflictAlgorithm: ConflictAlgorithm.replace,
        );
        for (var i = 0; i < pl.trackIds.length; i++) {
          batch.insert(
            'playlist_tracks',
            {
              'playlist_id': pl.id,
              'track_id': pl.trackIds[i],
              'position': i,
            },
            conflictAlgorithm: ConflictAlgorithm.replace,
          );
        }
      }
      await batch.commit(noResult: true);
    });
  }

  Future<void> setPlaylistCover(String id, String path) async {
    await db.update(
      'playlists',
      {'cover': path},
      where: 'id = ?',
      whereArgs: [id],
    );
  }

  Future<Map<String, String?>> localPlaylistCovers() async {
    final rows = await db.query('playlists', columns: ['id', 'cover']);
    return {
      for (final r in rows) r['id'] as String: r['cover'] as String?,
    };
  }

  Future<List<PlaylistDto>> allPlaylists() async {
    final rows = await db.query('playlists', orderBy: 'name COLLATE NOCASE ASC');
    final out = <PlaylistDto>[];
    for (final row in rows) {
      final id = row['id'] as String;
      final trackRows = await db.query(
        'playlist_tracks',
        columns: ['track_id'],
        where: 'playlist_id = ?',
        whereArgs: [id],
        orderBy: 'position ASC',
      );
      final cover = row['cover'] as String?;
      out.add(
        PlaylistDto(
          id: id,
          name: row['name'] as String,
          trackIds: trackRows.map((r) => r['track_id'] as String).toList(),
          hasCover: cover != null && cover.isNotEmpty,
          cover: cover,
          createdAt: row['created_at'] as int,
          updatedAt: row['updated_at'] as int,
        ),
      );
    }
    return out;
  }

  Future<Map<String, ({int mtime, String? audio, String? cover})>>
  localSyncState() async {
    final rows = await db.query(
      'tracks',
      columns: ['id', 'mtime', 'local_audio_path', 'local_cover_path'],
    );
    return {
      for (final r in rows)
        r['id'] as String: (
          mtime: (r['mtime'] as int?) ?? 0,
          audio: r['local_audio_path'] as String?,
          cover: r['local_cover_path'] as String?,
        ),
    };
  }

  Future<int> cacheBytes() async {
    var total = 0;
    if (!await mediaDir.exists()) return 0;
    await for (final entity in mediaDir.list(recursive: true, followLinks: false)) {
      if (entity is File) {
        total += await entity.length();
      }
    }
    return total;
  }

  Future<void> clearCache() async {
    final playlists = await allPlaylists();
    for (final pl in playlists) {
      final cover = pl.cover;
      if (cover == null) continue;
      final f = File(cover);
      if (await f.exists()) await f.delete();
    }
    final tracks = await allTracks();
    for (final t in tracks) {
      if (t.localAudioPath != null) {
        final f = File(t.localAudioPath!);
        if (await f.exists()) await f.delete();
      }
      if (t.localCoverPath != null) {
        final f = File(t.localCoverPath!);
        if (await f.exists()) await f.delete();
      }
    }
    await db.update('tracks', {
      'local_audio_path': null,
      'local_cover_path': null,
      'size': 0,
    });
    await db.update('playlists', {'cover': null});
    if (await mediaDir.exists()) {
      await mediaDir.delete(recursive: true);
      await mediaDir.create(recursive: true);
    }
  }

  CatalogTrack _trackFromRow(Map<String, Object?> row) {
    return CatalogTrack(
      id: row['id'] as String,
      title: row['title'] as String,
      artist: row['artist'] as String,
      album: row['album'] as String,
      year: row['year'] as int?,
      duration: (row['duration'] as num).toDouble(),
      trackNumber: row['track_number'] as int?,
      lyrics: row['lyrics'] as String?,
      ext: row['ext'] as String,
      mtime: row['mtime'] as int?,
      hasCover: (row['has_cover'] as int) == 1,
      localAudioPath: row['local_audio_path'] as String?,
      localCoverPath: row['local_cover_path'] as String?,
    );
  }
}
