import 'dart:async';
import 'dart:io';

import 'package:audio_session/audio_session.dart';
import 'package:flutter/material.dart';
import 'package:just_audio/just_audio.dart';
import 'package:palette_generator/palette_generator.dart';

import '../models.dart';

class PlayerController extends ChangeNotifier {
  PlayerController() {
    unawaited(_ensureInit());
  }

  final AudioPlayer _player = AudioPlayer();
  List<CatalogTrack> _queue = [];
  int _index = 0;
  Future<void>? _initFuture;
  Color _accent = const Color(0xFFA894E8);

  Color get accent => _accent;

  Future<void> _ensureInit() => _initFuture ??= _init();

  List<CatalogTrack> get queue => List.unmodifiable(_queue);
  int get index => _index;
  CatalogTrack? get current =>
      (_index >= 0 && _index < _queue.length) ? _queue[_index] : null;
  AudioPlayer get player => _player;
  Stream<Duration> get positionStream => _player.positionStream;
  Stream<Duration?> get durationStream => _player.durationStream;
  Stream<PlayerState> get playerStateStream => _player.playerStateStream;
  bool get isPlaying => _player.playing;

  Future<void> _init() async {
    final session = await AudioSession.instance;
    await session.configure(const AudioSessionConfiguration.music());
    _player.processingStateStream.listen((state) {
      if (state == ProcessingState.completed) {
        unawaited(next());
      }
    });
  }

  Future<void> playTracks(List<CatalogTrack> tracks, {int startIndex = 0}) async {
    final playable = tracks.where((t) => t.isDownloaded).toList();
    if (playable.isEmpty) return;
    _queue = playable;
    _index = startIndex.clamp(0, playable.length - 1);
    await _loadCurrent(play: true);
  }

  Future<void> playTrack(CatalogTrack track, {List<CatalogTrack>? context}) async {
    final list = (context ?? [track]).where((t) => t.isDownloaded).toList();
    if (list.isEmpty) return;
    final idx = list.indexWhere((t) => t.id == track.id);
    await playTracks(list, startIndex: idx < 0 ? 0 : idx);
  }

  Future<void> _loadCurrent({required bool play}) async {
    await _ensureInit();
    final track = current;
    if (track == null || track.localAudioPath == null) return;
    unawaited(_extractAccent(track));
    try {
      await _player.setFilePath(track.localAudioPath!);
      if (play) await _player.play();
      notifyListeners();
    } catch (e) {
      debugPrint('play error: $e');
    }
  }

  Future<void> _extractAccent(CatalogTrack track) async {
    final path = track.localCoverPath;
    if (path == null || !File(path).existsSync()) return;
    try {
      final bytes = await File(path).readAsBytes();
      final palette = await PaletteGenerator.fromImageProvider(
        MemoryImage(bytes),
        maximumColorCount: 8,
      );
      final tone = palette.dominantColor;
      if (tone == null) return;
      final hsl = HSLColor.fromColor(tone.color);
      _accent = hsl
          .withLightness(hsl.lightness.clamp(0.62, 0.80))
          .withSaturation(hsl.saturation.clamp(0.35, 0.75))
          .toColor();
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
    if (_queue.isEmpty) return;
    if (_index >= _queue.length - 1) {
      await _player.seek(Duration.zero);
      await _player.pause();
      notifyListeners();
      return;
    }
    _index += 1;
    await _loadCurrent(play: true);
  }

  Future<void> previous() async {
    if (_queue.isEmpty) return;
    if (_player.position > const Duration(seconds: 3)) {
      await _player.seek(Duration.zero);
      return;
    }
    if (_index <= 0) {
      await _player.seek(Duration.zero);
      return;
    }
    _index -= 1;
    await _loadCurrent(play: true);
  }

  Future<void> playAt(int i) async {
    if (i < 0 || i >= _queue.length) return;
    _index = i;
    await _loadCurrent(play: true);
  }

  @override
  void dispose() {
    _player.dispose();
    super.dispose();
  }
}
