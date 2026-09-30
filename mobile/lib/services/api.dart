import 'dart:convert';
import 'dart:io';

import 'package:http/http.dart' as http;

import '../models.dart';

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
    final res = await http
        .get(_uri('/v1/sync-manifest'), headers: _headers)
        .timeout(const Duration(seconds: 30));
    _ensureOk(res);
    final list = jsonDecode(res.body) as List<dynamic>;
    return list
        .map((e) => SyncManifestEntry.fromJson(e as Map<String, dynamic>))
        .toList();
  }

  Future<List<CatalogTrack>> catalog() async {
    final res = await http
        .get(_uri('/v1/catalog'), headers: _headers)
        .timeout(const Duration(seconds: 60));
    _ensureOk(res);
    final list = jsonDecode(res.body) as List<dynamic>;
    return list
        .map((e) => CatalogTrack.fromJson(e as Map<String, dynamic>))
        .toList();
  }

  Future<List<PlaylistDto>> playlists() async {
    final res = await http
        .get(_uri('/v1/playlists'), headers: _headers)
        .timeout(const Duration(seconds: 30));
    _ensureOk(res);
    final list = jsonDecode(res.body) as List<dynamic>;
    return list
        .map((e) => PlaylistDto.fromJson(e as Map<String, dynamic>))
        .toList();
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

    final client = http.Client();
    try {
      final req = http.Request('GET', _uri(path));
      req.headers.addAll(headers);
      final streamed = await client.send(req).timeout(const Duration(minutes: 5));

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
    } finally {
      client.close();
    }
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
