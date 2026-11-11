import 'dart:async';
import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

import '../models.dart';

const MethodChannel _catalogueChannel = MethodChannel('vynl/catalogue');

/// The catalogue client that runs inside the app, built from the same Rust
/// code the desktop uses. It answers searches, albums, artists and playlists
/// without the desktop running.
class LocalCatalogue {
  LocalCatalogue();

  bool _available = false;
  bool _checked = false;

  bool get available => _available;

  Future<bool> ensureAvailable() async {
    if (_checked) return _available;
    _checked = true;
    if (kIsWeb) {
      _available = false;
      return false;
    }
    try {
      _available =
          await _catalogueChannel.invokeMethod<bool>('available') ?? false;
    } catch (e) {
      debugPrint('local catalogue unavailable: $e');
      _available = false;
    }
    return _available;
  }

  Future<Map<String, dynamic>> _call(
    String method,
    String argument,
  ) async {
    final raw = await _catalogueChannel.invokeMethod<String>(method, argument);
    if (raw == null || raw.isEmpty) {
      throw StateError('the catalogue returned nothing');
    }
    final body = jsonDecode(raw) as Map<String, dynamic>;
    final error = body['error'];
    if (error is String && error.isNotEmpty) {
      throw StateError(error);
    }
    return body;
  }

  Future<CatalogueSearchResults> search(String query) async {
    final body = await _call('search', query);
    return CatalogueSearchResults.fromJson(body);
  }

  Future<CatalogueAlbumDetail> album(String id) async {
    final body = await _call('album', id);
    return CatalogueAlbumDetail.fromJson(body);
  }

  Future<CatalogueArtist> artist(String id) async {
    final body = await _call('artist', id);
    return CatalogueArtist.fromJson(body);
  }

  Future<CataloguePlaylistDetail> playlist(String id) async {
    final body = await _call('playlist', id);
    return CataloguePlaylistDetail.fromJson(body);
  }
}
