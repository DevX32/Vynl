import 'dart:async';
import 'dart:io';

import 'package:audio_session/audio_session.dart';
import 'package:flutter/material.dart';
import 'package:just_audio/just_audio.dart';
import 'package:just_audio_background/just_audio_background.dart';
import 'package:palette_generator/palette_generator.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../models.dart';

enum RepeatPreset { off, all, one }

const _repeatCycle = [RepeatPreset.off, RepeatPreset.all, RepeatPreset.one];

const _repeatKey = 'vynl.repeat';

const _paletteWidth = 64;

LoopMode _toLoopMode(RepeatPreset mode) {
  switch (mode) {
    case RepeatPreset.off:
      return LoopMode.off;
    case RepeatPreset.all:
      return LoopMode.all;
    case RepeatPreset.one:
      return LoopMode.one;
  }
}

class PlayerController extends ChangeNotifier {
  PlayerController() {
    unawaited(_ensureInit());
  }

  final AudioPlayer _player = AudioPlayer();

  Map<String, CatalogTrack> _tracksById = {};
  ConcatenatingAudioSource? _sequence;
  Future<void>? _initFuture;
  Color _accent = const Color(0xFFA894E8);
  RepeatPreset _repeat = RepeatPreset.off;
  bool _accentFromArt = false;

  Color get accent => _accent;
  bool get accentFromArt => _accentFromArt;
  RepeatPreset get repeatMode => _repeat;
  bool get repeatActive => _repeat != RepeatPreset.off;

  Future<void> _ensureInit() => _initFuture ??= _init();

  int get _length => _player.sequence?.length ?? 0;

  CatalogTrack? _trackAt(int i) {
    final sequence = _player.sequence;
    if (sequence == null || i < 0 || i >= sequence.length) return null;
    final tag = sequence[i].tag;
    if (tag is! MediaItem) return null;
    return _tracksById[tag.id];
  }

  List<CatalogTrack> get queue => List.unmodifiable([
        for (var i = 0; i < _length; i++)
          if (_trackAt(i) case final CatalogTrack t) t,
      ]);

  int get index => _player.currentIndex ?? 0;

  CatalogTrack? get current {
    final i = _player.currentIndex;
    return i == null ? null : _trackAt(i);
  }

  Future<void> move(int from, int to) async {
    final sequence = _sequence;
    if (sequence == null || from < 0 || from >= _length) return;
    final target = to.clamp(0, _length - 1);
    if (from == target) return;
    try {
      await sequence.move(from, target);
      notifyListeners();
    } catch (e) {
      debugPrint('reorder failed: $e');
    }
  }

  AudioPlayer get player => _player;
  Stream<Duration> get positionStream => _player.positionStream;
  Stream<Duration?> get durationStream => _player.durationStream;
  Stream<PlayerState> get playerStateStream => _player.playerStateStream;
  bool get isPlaying => _player.playing;

  Future<void> _init() async {
    final session = await AudioSession.instance;
    await session.configure(const AudioSessionConfiguration.music());
    _player.currentIndexStream.listen((_) => _onTrackChanged());
    await _loadRepeat();
  }

  void _onTrackChanged() {
    _accentFromArt = false;
    final track = current;
    if (track != null) unawaited(_extractAccent(track));
    notifyListeners();
  }

  Future<void> _loadRepeat() async {
    try {
      final prefs = await SharedPreferences.getInstance();
      final saved = prefs.getString(_repeatKey);
      _repeat = RepeatPreset.values.firstWhere(
        (m) => m.name == saved,
        orElse: () => RepeatPreset.off,
      );
      await _player.setLoopMode(_toLoopMode(_repeat));
      notifyListeners();
    } catch (e) {
      debugPrint('repeat load failed: $e');
    }
  }

  Future<void> toggleRepeat() async {
    final i = _repeatCycle.indexOf(_repeat);
    _repeat = _repeatCycle[(i + 1) % _repeatCycle.length];
    await _player.setLoopMode(_toLoopMode(_repeat));
    notifyListeners();
    try {
      final prefs = await SharedPreferences.getInstance();
      await prefs.setString(_repeatKey, _repeat.name);
    } catch (e) {
      debugPrint('repeat save failed: $e');
    }
  }

  Future<void> playTracks(
    List<CatalogTrack> tracks, {
    int startIndex = 0,
  }) async {
    await _ensureInit();
    final playable = tracks.where((t) => t.isDownloaded).toList();
    if (playable.isEmpty) return;
    final start = startIndex.clamp(0, playable.length - 1);
    _tracksById = {for (final t in playable) t.id: t};
    final sequence = ConcatenatingAudioSource(
      children: [for (final t in playable) _sourceFor(t)],
    );
    _sequence = sequence;
    try {
      await _player.setAudioSource(sequence, initialIndex: start);
      await _player.play();
    } catch (e) {
      debugPrint('play error: $e');
    }
    _onTrackChanged();
  }

  Future<void> playTrack(
    CatalogTrack track, {
    List<CatalogTrack>? context,
  }) async {
    final list = (context ?? [track]).where((t) => t.isDownloaded).toList();
    if (list.isEmpty) return;
    final idx = list.indexWhere((t) => t.id == track.id);
    await playTracks(list, startIndex: idx < 0 ? 0 : idx);
  }

  AudioSource _sourceFor(CatalogTrack t) {
    final art = t.localCoverPath;
    return AudioSource.file(
      t.localAudioPath!,
      tag: MediaItem(
        id: t.id,
        title: t.title,
        artist: t.artist,
        album: t.album,
        artUri: art == null || art.isEmpty ? null : Uri.file(art),
        duration: t.duration <= 0
            ? null
            : Duration(milliseconds: (t.duration * 1000).round()),
      ),
    );
  }

  Future<void> _extractAccent(CatalogTrack track) async {
    final path = track.localCoverPath;
    if (path == null || path.isEmpty) return;
    try {
      final palette = await PaletteGenerator.fromImageProvider(
        ResizeImage(FileImage(File(path)), width: _paletteWidth),
        maximumColorCount: 8,
      );
      final tone = palette.dominantColor;
      if (tone == null) return;
      final hsl = HSLColor.fromColor(tone.color);
      _accent = hsl
          .withLightness(hsl.lightness.clamp(0.62, 0.80))
          .withSaturation(hsl.saturation.clamp(0.35, 0.75))
          .toColor();
      _accentFromArt = true;
      notifyListeners();
    } catch (e) {
      debugPrint('palette failed: $e');
    }
  }

  Future<void> toggle() async {
    if (_player.playing) {
      await _player.pause();
    } else {
      await _player.play();
    }
    notifyListeners();
  }

  Future<void> pause() async {
    await _player.pause();
    notifyListeners();
  }

  Future<void> seek(Duration position) => _player.seek(position);

  Future<void> next() async {
    if (_length == 0) return;
    await _player.seekToNext();
  }

  Future<void> previous() async {
    if (_length == 0) return;
    if (_player.position > const Duration(seconds: 3)) {
      await _player.seek(Duration.zero);
      return;
    }
    await _player.seekToPrevious();
  }

  Future<void> playAt(int i) async {
    if (i < 0 || i >= _length) return;
    await _player.seek(Duration.zero, index: i);
    if (!_player.playing) await _player.play();
  }

  @override
  void dispose() {
    _player.dispose();
    super.dispose();
  }
}
