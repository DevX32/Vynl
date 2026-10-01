import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/painting.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../services/player_controller.dart';
import '../theme.dart';

final _hexPattern = RegExp(r'^#?([0-9a-fA-F]{6})$');

class ThemeController extends ChangeNotifier {
  ThemeController() {
    unawaited(_restore());
  }

  static const _accentKey = 'vynl.accent';
  static const _dynamicKey = 'vynl.dynamic_accent';

  Color _accent = kDefaultAccent;
  bool _dynamic = false;
  PlayerController? _player;
  bool _restored = false;

  Color get accent => VynlColors.accent;
  Color get chosen => _accent;
  bool get dynamicAccent => _dynamic;
  bool get restored => _restored;
  bool get isPreset => kAccentPresets.contains(_accent);

  void bind(PlayerController player) {
    if (identical(_player, player)) return;
    _player?.removeListener(_syncFromPlayer);
    _player = player;
    player.addListener(_syncFromPlayer);
    _syncFromPlayer();
  }

  void _syncFromPlayer() {
    if (!_dynamic) return;
    final dynamic = _player?.accentFromArt == true ? _player?.accent : null;
    if (dynamic == null || dynamic == VynlColors.accent) return;
    VynlColors.setAccent(dynamic);
    notifyListeners();
  }

  Color get _effective => _dynamic && _player?.accentFromArt == true
      ? _player!.accent
      : _accent;

  Future<void> _restore() async {
    try {
      final prefs = await SharedPreferences.getInstance();
      _dynamic = prefs.getBool(_dynamicKey) ?? false;
      final stored = prefs.getString(_accentKey);
      if (stored != null) {
        final parsed = parseHexColor(stored);
        if (parsed != null) _accent = parsed;
      }
      VynlColors.setAccent(_effective);
    } catch (e) {
      debugPrint('theme restore failed: $e');
    }
    _restored = true;
    notifyListeners();
  }

  Future<void> setAccent(Color value) async {
    _accent = value;
    VynlColors.setAccent(_effective);
    notifyListeners();
    try {
      final prefs = await SharedPreferences.getInstance();
      await prefs.setString(_accentKey, hexOf(value));
    } catch (e) {
      debugPrint('theme save failed: $e');
    }
  }

  void previewAccent(Color value) {
    _accent = value;
    VynlColors.setAccent(_effective);
    notifyListeners();
  }

  Future<void> setDynamicAccent(bool value) async {
    if (_dynamic == value) return;
    _dynamic = value;
    VynlColors.setAccent(_effective);
    notifyListeners();
    try {
      final prefs = await SharedPreferences.getInstance();
      await prefs.setBool(_dynamicKey, value);
    } catch (e) {
      debugPrint('theme save failed: $e');
    }
  }
}

Color? parseHexColor(String raw) {
  final match = _hexPattern.firstMatch(raw.trim());
  if (match == null) return null;
  final digits = match.group(1)!;
  final value = int.tryParse(digits, radix: 16);
  if (value == null) return null;
  return Color(0xFF000000 | value);
}

String hexOf(Color color) {
  String part(double channel) => (channel.clamp(0.0, 1.0) * 255)
      .round()
      .toRadixString(16)
      .padLeft(2, '0')
      .toUpperCase();
  return '#${part(color.r)}${part(color.g)}${part(color.b)}';
}