import 'package:flutter_test/flutter_test.dart';
import 'package:vynl_mobile/models.dart';

void main() {
  group('CatalogTrack.fromJson', () {
    test('parses a full payload', () {
      final t = CatalogTrack.fromJson({
        'id': 'a1',
        'title': 'Title',
        'artist': 'Artist',
        'album': 'Album',
        'year': 2024,
        'duration': 213.5,
        'trackNumber': 3,
        'lyrics': 'la la',
        'ext': 'flac',
        'mtime': 1700000000,
        'hasCover': true,
      });
      expect(t.id, 'a1');
      expect(t.duration, 213.5);
      expect(t.trackNumber, 3);
      expect(t.ext, 'flac');
      expect(t.hasCover, isTrue);
    });

    test('falls back when fields are missing', () {
      final t = CatalogTrack.fromJson({'id': 'a1'});
      expect(t.title, 'Unknown');
      expect(t.artist, 'Unknown');
      expect(t.ext, 'mp3');
      expect(t.duration, 0);
      expect(t.hasCover, isFalse);
    });

    test('treats a non-true hasCover as false', () {
      expect(
        CatalogTrack.fromJson({'id': 'a', 'hasCover': 'yes'}).hasCover,
        isFalse,
      );
    });
  });

  group('isDownloaded', () {
    test('is false when no local path', () {
      expect(CatalogTrack.fromJson({'id': 'a'}).isDownloaded, isFalse);
    });

    test('is false for an empty local path', () {
      final t = CatalogTrack.fromJson({'id': 'a'})
          .copyWith(localAudioPath: '');
      expect(t.isDownloaded, isFalse);
    });
  });

  group('copyWith', () {
    CatalogTrack base() => CatalogTrack.fromJson({
          'id': 'a1',
          'title': 'T',
          'artist': 'Ar',
          'album': 'Al',
          'ext': 'mp3',
        });

    test('sets the local path', () {
      final t = base().copyWith(localAudioPath: '/tmp/a.mp3');
      expect(t.localAudioPath, '/tmp/a.mp3');
      expect(t.isDownloaded, isTrue);
    });

    test('clearAudio wins over a provided path', () {
      final t = base().copyWith(localAudioPath: '/tmp/a.mp3').copyWith(clearAudio: true);
      expect(t.localAudioPath, isNull);
    });

    test('omitting a path preserves the existing one', () {
      final t = base().copyWith(localAudioPath: '/tmp/a.mp3').copyWith(localCoverPath: '/c.jpg');
      expect(t.localAudioPath, '/tmp/a.mp3');
      expect(t.localCoverPath, '/c.jpg');
    });

    test('preserves identity fields', () {
      final t = base().copyWith(localAudioPath: '/tmp/a.mp3');
      expect(t.id, 'a1');
      expect(t.title, 'T');
      expect(t.ext, 'mp3');
    });
  });

  group('PlaylistDto.fromJson', () {
    test('coerces trackIds to strings', () {
      final p = PlaylistDto.fromJson({
        'id': 'p1',
        'name': 'Mix',
        'trackIds': ['a', 2, null],
        'createdAt': 1,
        'updatedAt': 2,
      });
      expect(p.trackIds, ['a', '2', 'null']);
    });

    test('defaults to an empty track list', () {
      expect(PlaylistDto.fromJson({'id': 'p'}).trackIds, isEmpty);
    });
  });

  group('SyncManifestEntry.fromJson', () {
    test('defaults size to zero and mtime to null', () {
      final m = SyncManifestEntry.fromJson({'id': 'a'});
      expect(m.size, 0);
      expect(m.mtime, isNull);
    });
  });
}
