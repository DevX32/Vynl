import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:open_filex/open_filex.dart';
import 'package:package_info_plus/package_info_plus.dart';
import 'package:path_provider/path_provider.dart';
import 'package:shared_preferences/shared_preferences.dart';

const _manifestUrl =
    'https://github.com/DevX32/Vynl/releases/latest/download/latest.json';
const _dismissedKey = 'vynl.update.dismissed';

const _attempts = 3;
const _backoff = [Duration(seconds: 2), Duration(seconds: 6)];

Future<void> _pause(int attempt) {
  return Future<void>.delayed(
    attempt < _backoff.length ? _backoff[attempt] : const Duration(seconds: 10),
  );
}

String abiOfDevice() => AppUpdate.abiFor(Platform.version) ?? 'arm64-v8a';

class ReleaseNote {
  const ReleaseNote({required this.version, required this.notes});

  final String version;
  final String notes;
}

class AppUpdate extends ChangeNotifier {
  AppUpdate() {
    unawaited(_restore());
  }

  ReleaseNote? _available;
  bool _downloading = false;
  double _progress = 0;
  String? _error;
  String? _apkPath;
  bool _busy = false;
  String _current = '';
  bool _checked = false;
  String _dismissedVersion = '';

  bool get available => _available != null;
  bool get downloading => _downloading;
  double get progress => _progress;
  String? get error => _error;
  String? get apkPath => _apkPath;
  bool get readyToInstall => _apkPath != null && File(_apkPath!).existsSync();
  bool get checked => _checked;
  String get currentVersion => _current;

  ReleaseNote? get release => _available;

  Future<void> _restore() async {
    try {
      final prefs = await SharedPreferences.getInstance();
      _current = await _installedVersion();
      _dismissedVersion = prefs.getString(_dismissedKey) ?? '';
    } catch (e) {
      debugPrint('update restore failed: $e');
    }
    _checked = true;
    notifyListeners();
  }

  Future<String> _installedVersion() async {
    final info = await PackageInfo.fromPlatform();
    return info.version;
  }

  static String? abiFor(String raw) {
    if (raw.contains('arm64') || raw.contains('aarch64')) return 'arm64-v8a';
    if (raw.contains('x86_64') || raw.contains('amd64')) return 'x86_64';
    if (raw.contains('armeabi') || raw.contains('armv7')) {
      return 'armeabi-v7a';
    }
    return null;
  }

  static int compare(String a, String b) {
    final pa = a.split(RegExp(r'[.+-]'));
    final pb = b.split(RegExp(r'[.+-]'));
    for (var i = 0; i < 3; i++) {
      final na = i < pa.length ? int.tryParse(pa[i]) ?? 0 : 0;
      final nb = i < pb.length ? int.tryParse(pb[i]) ?? 0 : 0;
      if (na != nb) return na.compareTo(nb);
    }
    return 0;
  }

  HttpClient _client() {
    return HttpClient()
      ..connectionTimeout = const Duration(seconds: 12)
      ..autoUncompress = true;
  }

  Future<String> _fetchText(String url) async {
    Object? failure;
    for (var attempt = 0; attempt < _attempts; attempt++) {
      if (attempt > 0) {
        await _pause(attempt - 1);
      }
      final client = _client();
      try {
        final req = await client.getUrl(Uri.parse(url));
        req.followRedirects = true;
        req.headers.set(HttpHeaders.acceptHeader, 'application/json');
        final res = await req.close().timeout(const Duration(seconds: 20));
        if (res.statusCode != HttpStatus.ok) {
          throw HttpException('status ${res.statusCode}', uri: Uri.parse(url));
        }
        return await res
            .transform(const Utf8Decoder(allowMalformed: true))
            .join();
      } catch (e) {
        failure = e;
      } finally {
        client.close(force: true);
      }
    }
    throw failure ?? HttpException('unreachable', uri: Uri.parse(url));
  }

  Future<void> _download(String url, File dest) async {
    Object? failure;
    for (var attempt = 0; attempt < _attempts; attempt++) {
      if (attempt > 0) {
        await _pause(attempt - 1);
        _progress = 0;
        notifyListeners();
      }
      final client = _client();
      IOSink? sink;
      try {
        final req = await client.getUrl(Uri.parse(url));
        req.followRedirects = true;
        final res = await req.close().timeout(const Duration(seconds: 30));
        if (res.statusCode != HttpStatus.ok) {
          throw HttpException('status ${res.statusCode}', uri: Uri.parse(url));
        }
        final total = res.contentLength;
        var received = 0;
        sink = dest.openWrite();
        await for (final chunk in res) {
          received += chunk.length;
          sink.add(chunk);
          if (total > 0) {
            _progress = received / total;
            notifyListeners();
          }
        }
        await sink.flush();
        await sink.close();
        sink = null;
        return;
      } catch (e) {
        failure = e;
        try {
          if (await dest.exists()) await dest.delete();
        } catch (_) {}
      } finally {
        await sink?.close();
        client.close(force: true);
      }
    }
    throw failure ?? HttpException('unreachable', uri: Uri.parse(url));
  }

  Future<void> check({bool force = false}) async {
    if (_busy) return;
    _busy = true;
    _error = null;
    if (!force && _dismissedVersion.isNotEmpty) {
      _current = await _installedVersion();
      _busy = false;
      notifyListeners();
      return;
    }
    try {
      final text = await _fetchText(_manifestUrl);
      final body = jsonDecode(text) as Map<String, dynamic>;
      final latest = body['version'] as String? ?? '';
      if (latest.isEmpty) throw const FormatException('no version');
      final current = await _installedVersion();
      _current = current;
      if (compare(latest, current) <= 0) {
        _available = null;
      } else {
        _available = ReleaseNote(
          version: latest,
          notes: body['notes'] as String? ?? '',
        );
      }
      _error = null;
    } catch (e) {
      _error = 'Could not check for updates';
      debugPrint('update check failed: $e');
    }
    _busy = false;
    _checked = true;
    notifyListeners();
  }

  Future<void> dismiss() async {
    final version = _available?.version;
    _available = null;
    if (version != null) {
      try {
        final prefs = await SharedPreferences.getInstance();
        await prefs.setString(_dismissedKey, version);
        _dismissedVersion = version;
      } catch (e) {
        debugPrint('update dismiss failed: $e');
      }
    }
    notifyListeners();
  }

  Future<void> download(String abi) async {
    final version = _available?.version;
    if (version == null || _downloading) return;
    _downloading = true;
    _progress = 0;
    _error = null;
    notifyListeners();

    try {
      final url =
          'https://github.com/DevX32/Vynl/releases/download/v$version/'
          'Vynl-mobile-$version-$abi.apk';
      final dir = await getTemporaryDirectory();
      final dest = File('${dir.path}/Vynl-mobile-$version-$abi.apk');

      await _download(url, dest);
      _apkPath = dest.path;
      _progress = 1;
    } catch (e) {
      _error = 'Download failed';
      debugPrint('update download failed: $e');
    }
    _downloading = false;
    notifyListeners();
  }

  Future<void> install() async {
    final path = _apkPath;
    if (path == null) return;
    try {
      final result = await OpenFilex.open(
        path,
        type: 'application/vnd.android.package-archive',
      );
      if (result.type != ResultType.done) {
        _error = 'Could not open the installer';
        notifyListeners();
      }
    } catch (e) {
      _error = 'Could not open the installer';
      debugPrint('update install failed: $e');
      notifyListeners();
    }
  }
}
