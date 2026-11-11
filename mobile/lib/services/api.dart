import 'dart:convert';
import 'dart:io';

import 'package:http/http.dart' as http;

import '../models.dart';

class RemoteLyrics {
  const RemoteLyrics({
    required this.kind,
    required this.text,
    required this.source,
  });

  final String kind;
  final String text;
  final String source;
}

enum SyncedLyricsStatus { found, missing, failed }

class SyncedLyricsResponse {
  const SyncedLyricsResponse._({required this.status, this.lyrics});

  const SyncedLyricsResponse.found(RemoteLyrics lyrics)
      : this._(status: SyncedLyricsStatus.found, lyrics: lyrics);

  const SyncedLyricsResponse.missing()
      : this._(status: SyncedLyricsStatus.missing);

  const SyncedLyricsResponse.failed()
      : this._(status: SyncedLyricsStatus.failed);

  final SyncedLyricsStatus status;
  final RemoteLyrics? lyrics;

  bool get isFound => status == SyncedLyricsStatus.found;
}

class ApiException implements Exception {
  ApiException(this.message, {this.statusCode});
  final String message;
  final int? statusCode;

  @override
  String toString() => message;
}

class VynlApi {
  VynlApi({required this.baseUrl, required this.token});

  String baseUrl;
  String token;

  final http.Client _client = http.Client();

  Uri _uri(String path) {
    final root = baseUrl.endsWith('/')
        ? baseUrl.substring(0, baseUrl.length - 1)
        : baseUrl;
    return Uri.parse('$root$path');
  }

  Map<String, String> get _headers => {
        'Authorization': 'Bearer $token',
        'Accept': 'application/json',
      };

  static Future<PairingCredentials> pair({
    required String baseUrl,
    required String pin,
  }) async {
    final root = baseUrl.endsWith('/')
        ? baseUrl.substring(0, baseUrl.length - 1)
        : baseUrl;
    final res = await http
        .post(
          Uri.parse('$root/pair'),
          headers: {'Content-Type': 'application/json'},
          body: jsonEncode({'pin': pin.trim()}),
        )
        .timeout(const Duration(seconds: 12));
    if (res.statusCode == 429) {
      throw ApiException(
        'Too many attempts. Wait a moment and try again.',
        statusCode: 429,
      );
    }
    if (res.statusCode == 403) {
      throw ApiException('Wrong PIN', statusCode: 403);
    }
    if (res.statusCode != 200) {
      throw ApiException(
        'Pairing failed (${res.statusCode})',
        statusCode: res.statusCode,
      );
    }
    final body = jsonDecode(res.body) as Map<String, dynamic>;
    final token = body['token'] as String?;
    if (token == null || token.isEmpty) {
      throw ApiException('No token returned');
    }
    return PairingCredentials(baseUrl: root, token: token);
  }

  Future<List<SyncManifestEntry>> syncManifest() async {
    final res = await _client
        .get(_uri('/v1/sync-manifest'), headers: _headers)
        .timeout(const Duration(seconds: 30));
    _ensureOk(res);
    final list = jsonDecode(res.body) as List<dynamic>;
    return list
        .map((e) => SyncManifestEntry.fromJson(e as Map<String, dynamic>))
        .toList();
  }

  Future<List<CatalogTrack>> catalog() async {
    final res = await _client
        .get(_uri('/v1/catalog'), headers: _headers)
        .timeout(const Duration(seconds: 60));
    _ensureOk(res);
    final list = jsonDecode(res.body) as List<dynamic>;
    return list
        .map((e) => CatalogTrack.fromJson(e as Map<String, dynamic>))
        .toList();
  }

  Future<List<PlaylistDto>> playlists() async {
    final res = await _client
        .get(_uri('/v1/playlists'), headers: _headers)
        .timeout(const Duration(seconds: 30));
    _ensureOk(res);
    final list = jsonDecode(res.body) as List<dynamic>;
    return list
        .map((e) => PlaylistDto.fromJson(e as Map<String, dynamic>))
        .toList();
  }

  Future<CatalogueSearchResults> catalogueSearch(String query) async {
    final res = await _client
        .get(
          _uri('/v1/catalogue/search').replace(
            queryParameters: {'q': query},
          ),
          headers: _headers,
        )
        .timeout(const Duration(seconds: 45));
    _ensureOk(res);
    return CatalogueSearchResults.fromJson(
      jsonDecode(res.body) as Map<String, dynamic>,
    );
  }

  Future<CatalogueAlbumDetail> catalogueAlbum(String id) async {
    final res = await _client
        .get(_uri('/v1/catalogue/album/$id'), headers: _headers)
        .timeout(const Duration(seconds: 45));
    _ensureOk(res);
    return CatalogueAlbumDetail.fromJson(
      jsonDecode(res.body) as Map<String, dynamic>,
    );
  }

  Future<CatalogueArtist> catalogueArtist(String id) async {
    final res = await _client
        .get(_uri('/v1/catalogue/artist/$id'), headers: _headers)
        .timeout(const Duration(seconds: 45));
    _ensureOk(res);
    return CatalogueArtist.fromJson(
      jsonDecode(res.body) as Map<String, dynamic>,
    );
  }

  Future<CataloguePlaylistDetail> cataloguePlaylist(String id) async {
    final res = await _client
        .get(_uri('/v1/catalogue/playlist/$id'), headers: _headers)
        .timeout(const Duration(seconds: 45));
    _ensureOk(res);
    return CataloguePlaylistDetail.fromJson(
      jsonDecode(res.body) as Map<String, dynamic>,
    );
  }

  Uri streamUri(String videoId) => _uri('/v1/stream/$videoId/audio');

  Map<String, String> get streamHeaders => {
        'Authorization': 'Bearer $token',
      };

  CatalogTrack streamTrack(StreamTrack track) {
    return track.toCatalogTrack(
      streamUrl: streamUri(track.id),
      streamHeaders: streamHeaders,
    );
  }

  Future<MobilePlayerState> playerState() async {
    final res = await _client
        .get(_uri('/v1/player'), headers: _headers)
        .timeout(const Duration(seconds: 15));
    _ensureOk(res);
    return MobilePlayerState.fromJson(
      jsonDecode(res.body) as Map<String, dynamic>,
    );
  }

  Future<void> playerAction(
    String action, {
    double? time,
    int? volume,
  }) async {
    final body = <String, dynamic>{'action': action};
    if (time != null) body['time'] = time;
    if (volume != null) body['volume'] = volume;
    final res = await _client
        .post(
          _uri('/v1/player/action'),
          headers: {..._headers, 'Content-Type': 'application/json'},
          body: jsonEncode(body),
        )
        .timeout(const Duration(seconds: 15));
    _ensureOk(res);
  }

  Future<MobileTaste> taste() async {
    final res = await _client
        .get(_uri('/v1/taste'), headers: _headers)
        .timeout(const Duration(seconds: 20));
    _ensureOk(res);
    return MobileTaste.fromJson(jsonDecode(res.body) as Map<String, dynamic>);
  }

  void dispose() => _client.close();

  Future<bool> ping() async {
    try {
      final res = await _client
          .get(_uri('/health'))
          .timeout(const Duration(seconds: 4));
      return res.statusCode == 200;
    } catch (_) {
      return false;
    }
  }

  Future<void> downloadAudio({
    required String trackId,
    required File dest,
    void Function(int received, int? total)? onProgress,
  }) async {
    await _downloadFile(
      path: '/v1/tracks/$trackId/audio',
      dest: dest,
      onProgress: onProgress,
    );
  }

  Future<void> downloadCover({
    required String trackId,
    required File dest,
  }) async {
    await _downloadFile(path: '/v1/tracks/$trackId/cover', dest: dest);
  }

  Future<void> downloadPlaylistCover({
    required String playlistId,
    required File dest,
  }) async {
    await _downloadFile(path: '/v1/playlists/$playlistId/cover', dest: dest);
  }

  Future<SyncedLyricsResponse> syncedLyrics(String trackId) async {
    final http.Response res;
    try {
      res = await _client
          .get(_uri('/v1/tracks/$trackId/synced-lyrics'), headers: _headers)
          .timeout(const Duration(seconds: 60));
    } catch (_) {
      return const SyncedLyricsResponse.failed();
    }
    if (res.statusCode == 404) return const SyncedLyricsResponse.missing();
    if (res.statusCode != 200) return const SyncedLyricsResponse.failed();
    try {
      final body = jsonDecode(res.body) as Map<String, dynamic>;
      final text = body['text'] as String? ?? '';
      if (text.trim().isEmpty) return const SyncedLyricsResponse.missing();
      return SyncedLyricsResponse.found(
        RemoteLyrics(
          kind: body['kind'] as String? ?? 'lrc',
          text: text,
          source: body['source'] as String? ?? 'remote',
        ),
      );
    } catch (_) {
      return const SyncedLyricsResponse.failed();
    }
  }

  Future<void> _downloadFile({
    required String path,
    required File dest,
    void Function(int received, int? total)? onProgress,
  }) async {
    await dest.parent.create(recursive: true);
    final partial = File('${dest.path}.part');
    int existing = 0;
    if (await partial.exists()) {
      existing = await partial.length();
    }

    final headers = Map<String, String>.from(_headers);
    if (existing > 0) {
      headers['Range'] = 'bytes=$existing-';
    }

    final req = http.Request('GET', _uri(path));
    req.headers.addAll(headers);
    final streamed = await _client
        .send(req)
        .timeout(const Duration(minutes: 5));

    if (streamed.statusCode == 404) {
      throw ApiException('File not found', statusCode: 404);
    }
    if (streamed.statusCode != 200 && streamed.statusCode != 206) {
      throw ApiException(
        'Download failed (${streamed.statusCode})',
        statusCode: streamed.statusCode,
      );
    }

    final totalHeader = streamed.contentLength;
    final sink = partial.openWrite(mode: FileMode.append);
    var received = existing;
    try {
      await for (final chunk in streamed.stream) {
        sink.add(chunk);
        received += chunk.length;
        onProgress?.call(
          received,
          totalHeader == null ? null : existing + totalHeader,
        );
      }
    } finally {
      await sink.close();
    }

    if (await dest.exists()) {
      await dest.delete();
    }
    await partial.rename(dest.path);
  }

  void _ensureOk(http.Response res) {
    if (res.statusCode == 401) {
      throw ApiException('Unauthorized — re-pair with desktop', statusCode: 401);
    }
    if (res.statusCode < 200 || res.statusCode >= 300) {
      throw ApiException('Request failed (${res.statusCode})', statusCode: res.statusCode);
    }
  }
}
