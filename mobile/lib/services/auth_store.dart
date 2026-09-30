import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../models.dart';

class AuthStore {
  AuthStore();

  static const _tokenKey = 'vynl_token';
  static const _baseUrlKey = 'vynl_base_url';

  final FlutterSecureStorage _secure = const FlutterSecureStorage();

  Future<PairingCredentials?> load() async {
    final prefs = await SharedPreferences.getInstance();
    final baseUrl = prefs.getString(_baseUrlKey);
    final token = await _secure.read(key: _tokenKey);
    if (baseUrl == null || baseUrl.isEmpty || token == null || token.isEmpty) {
      return null;
    }
    return PairingCredentials(baseUrl: baseUrl, token: token);
  }

  Future<void> save(PairingCredentials creds) async {
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString(_baseUrlKey, creds.baseUrl);
    await _secure.write(key: _tokenKey, value: creds.token);
  }

  Future<void> clear() async {
    final prefs = await SharedPreferences.getInstance();
    await prefs.remove(_baseUrlKey);
    await _secure.delete(key: _tokenKey);
  }
}
