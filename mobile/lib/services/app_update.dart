import 'dart:async';
import 'dart:convert';
import 'dart:ffi' as ffi;
import 'dart:io';

import 'package:android_intent_plus/android_intent.dart';
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

const _minApkBytes = 1024 * 1024;

Future<void> _pause(int attempt) {
  return Future<void>.delayed(
    attempt < _backoff.length ? _backoff[attempt] : const Duration(seconds: 10),
  );
}

String abiOfDevice() =>
    AppUpdate.abiFor(ffi.Abi.current().toString()) ?? 'arm64-v8a';

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
  bool _needsPermission = false;
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
  bool get busy => _busy;
  String get currentVersion => _current;

  bool get needsInstallPermission => _needsPermission;

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
    final s = raw.toLowerCase();
    if (s.contains('x86_64') ||
        s.contains('amd64') ||
        s.contains('androidx64')) {
      return 'x86_64';
    }
    if (s.contains('arm64') || s.contains('aarch64')) return 'arm64-v8a';
    if (s.contains('armeabi') ||
        s.contains('armv7') ||
        s.contains('androidarm')) {
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
    _available = null;
    try {
      final text = await _fetchText(_manifestUrl);
      final body = jsonDecode(text) as Map<String, dynamic>;
      final latest = body['version'] as String? ?? '';
      if (latest.isEmpty) throw const FormatException('no version');
      final current = await _installedVersion();
      _current = current;
      final dismissed = !force && latest == _dismissedVersion;
      if (compare(latest, current) > 0 && !dismissed) {
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
      final url = 'https://github.com/DevX32/Vynl/releases/download/v$version/'
          'Vynl-mobile-$version-$abi.apk';
      final dir = await getTemporaryDirectory();
      final dest = File('${dir.path}/Vynl-mobile-$version-$abi.apk');

      await _download(url, dest);
      if (!await _isUsableApk(dest)) {
        throw const FormatException('downloaded file is not a valid APK');
      }
      _apkPath = dest.path;
      _needsPermission = false;
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
    if (!await _isUsableApk(File(path))) {
      _error = 'The downloaded update is incomplete. Download it again.';
      _apkPath = null;
      notifyListeners();
      return;
    }
    try {
      final result = await OpenFilex.open(
        path,
        type: 'application/vnd.android.package-archive',
      );
      if (result.type == ResultType.done) return;
      _needsPermission = result.type == ResultType.permissionDenied;
      _error = switch (result.type) {
        ResultType.noAppToOpen => 'No installer available on this device.',
        ResultType.permissionDenied => 'Vynl needs permission to install apps.',
        ResultType.fileNotFound => 'The downloaded update went missing.',
        _ => 'Could not open the installer.',
      };
      notifyListeners();
    } catch (e) {
      _error = 'Could not open the installer.';
      debugPrint('update install failed: $e');
      notifyListeners();
    }
  }

  Future<void> openInstallSettings() async {
    try {
      final info = await PackageInfo.fromPlatform();
      await AndroidIntent(
        action: 'android.settings.MANAGE_UNKNOWN_APP_SOURCES',
        data: 'package:${info.packageName}',
      ).launch();
    } catch (e) {
      debugPrint('update install settings failed: $e');
    }
  }

  Future<bool> _isUsableApk(File file) async {
    try {
      if (!await file.exists()) return false;
      if (await file.length() < _minApkBytes) return false;
      final handle = await file.open();
      try {
        final head = await handle.read(2);
        return head.length == 2 && head[0] == 0x50 && head[1] == 0x4B;
      } finally {
        await handle.close();
      }
    } catch (_) {
      return false;
    }
  }
}
