import 'dart:async';
import 'dart:io';
import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:just_audio/just_audio.dart';
import 'package:provider/provider.dart';

import '../models.dart';
import '../services/lyrics.dart';
import '../state/app_state.dart';
import '../services/player_controller.dart';
import '../theme.dart';
import '../widgets/artwork.dart';
import '../widgets/common.dart';

class PlayerPage extends StatefulWidget {
  const PlayerPage({super.key});

  @override
  State<PlayerPage> createState() => _PlayerPageState();
}

class _PlayerPageState extends State<PlayerPage> {
  bool _lyricsOn = false;

  void _toggleLyrics() => setState(() => _lyricsOn = !_lyricsOn);

  @override
  Widget build(BuildContext context) {
    final player = context.watch<PlayerController>();
    final track = player.current;
    if (track == null) {
      return const Scaffold(
        backgroundColor: VynlColors.bg,
        body: Center(
          child: Text(
            'Nothing playing',
            style: TextStyle(color: VynlColors.dim),
          ),
        ),
      );
    }

    final hasLyrics = (track.lyrics ?? '').trim().isNotEmpty;
    final lyricsOn = hasLyrics && _lyricsOn;

    return ArtBackdrop(
      coverPath: track.localCoverPath,
      tint: VynlColors.accent,
      blur: lyricsOn ? 120 : 80,
      dim: lyricsOn ? 0.42 : 0.62,
      child: Scaffold(
        backgroundColor: Colors.transparent,
        appBar: AppBar(
          leading: IconButton(
            icon: const Icon(Icons.keyboard_arrow_down_rounded, size: 30),
            onPressed: () => Navigator.of(context).maybePop(),
          ),
          titleSpacing: 0,
          title: player.queue.isEmpty
              ? null
              : Text(
                  '${player.index + 1} / ${player.queue.length}',
                  textAlign: TextAlign.center,
                  style: const TextStyle(
                    fontSize: 12,
                    fontWeight: FontWeight.w700,
                    letterSpacing: 1.4,
                    color: VynlColors.dim,
                  ),
                ),
          centerTitle: true,
          actions: [
            IconButton(
              icon: const Icon(
                Icons.queue_music_rounded,
                size: 24,
                color: VynlColors.dim,
              ),
              tooltip: 'Queue',
              onPressed: () => _showQueueSheet(context, player),
            ),
          ],
        ),
        body: SafeArea(
          child: GestureDetector(
            behavior: HitTestBehavior.opaque,
            onDoubleTap: hasLyrics ? _toggleLyrics : null,
            child: Stack(
              children: [
                AnimatedSwitcher(
                  duration: VynlMotion.slow,
                  switchInCurve: VynlMotion.emphasized,
                  switchOutCurve: Curves.easeIn,
                  layoutBuilder: (current, previous) => Stack(
                    fit: StackFit.expand,
                    children: [...previous, if (current != null) current],
                  ),
                  child: _PlayerBody(
                    key: ValueKey(lyricsOn ? 'lyr' : 'art'),
                    player: player,
                    track: track,
                    lyricsOn: lyricsOn,
                  ),
                ),
                if (hasLyrics)
                  Positioned(
                    left: 0,
                    right: 0,
                    bottom: 6,
                    child: IgnorePointer(
                      child: Center(
                        child: AnimatedOpacity(
                          duration: VynlMotion.fast,
                          opacity: lyricsOn ? 0 : 0.5,
                          child: const Text(
                            'DOUBLE TAP FOR LYRICS',
                            style: TextStyle(
                              color: VynlColors.faint,
                              fontSize: 10,
                              fontWeight: FontWeight.w700,
                              letterSpacing: 1.4,
                            ),
                          ),
                        ),
                      ),
                    ),
                  ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _LyricsOverlay extends StatefulWidget {
  const _LyricsOverlay({required this.player});

  final PlayerController player;

  @override
  State<_LyricsOverlay> createState() => _LyricsOverlayState();
}

class _LyricsOverlayState extends State<_LyricsOverlay> {
  final _scroll = ScrollController();
  final _cardScroll = ScrollController();
  final Map<int, GlobalKey> _lineKeys = {};

  LyricsResult? _remote;
  String? _shownFor;
  String? _triedFor;
  DateTime? _failedAt;
  int _request = 0;
  bool _userScrolling = false;
  Timer? _resume;

  static const _retryAfter = Duration(seconds: 20);

  @override
  void initState() {
    super.initState();
    _scroll.addListener(_onScroll);
  }

  bool _shouldAttempt(String trackId) {
    if (_triedFor != trackId) return true;
    final at = _failedAt;
    if (at == null) return false; // already succeeded
    return DateTime.now().difference(at) >= _retryAfter;
  }

  Future<void> _maybeFetch() async {
    final track = widget.player.current;
    final api = context.read<AppState>().api;
    if (track == null || api == null) return;

    if (_shownFor != null && _shownFor != track.id) {
      _remote = null;
      _shownFor = null;
      _lineKeys.clear();
    }

    if (!_shouldAttempt(track.id)) return;
    _triedFor = track.id;

    final req = ++_request;
    final res = await api.syncedLyrics(track.id);
    if (!mounted || req != _request) return;

    if (res == null) {
      _failedAt = DateTime.now();
      return;
    }

    final parsed = lyricsFromRemote(track.id, res.text);
    if (parsed != null) {
      setState(() {
        _remote = parsed;
        _shownFor = track.id;
        _failedAt = null;
      });
    }
  }

  @override
  void dispose() {
    _resume?.cancel();
    _scroll.removeListener(_onScroll);
    _scroll.dispose();
    _cardScroll.dispose();
    super.dispose();
  }

  void _onScroll() {
    if (!_scroll.hasClients) return;
    if (_scroll.position.userScrollDirection == ScrollDirection.idle) return;
    if (!_userScrolling) {
      _userScrolling = true;
      if (mounted) setState(() {});
    }
    _resume?.cancel();
    _resume = Timer(const Duration(seconds: 3), () {
      if (mounted) setState(() => _userScrolling = false);
    });
  }

  void _centre(int index, int count) {
    if (_userScrolling || index < 0 || index >= count) return;

    void snap() {
      if (!mounted || _userScrolling || !_scroll.hasClients) return;
      final ctx = _lineKeys[index]?.currentContext;
      if (ctx == null) return;
      Scrollable.ensureVisible(
        ctx,
        alignment: 0.34,
        duration: VynlMotion.normal,
        curve: VynlMotion.emphasized,
      );
    }

    final ctx = _lineKeys[index]?.currentContext;
    if (ctx != null) {
      snap();
      return;
    }

    if (!_scroll.hasClients) return;
    final viewport = _scroll.position.viewportDimension;
    final estimate = index * _estimatedExtent - viewport * 0.34;
    final max = _scroll.position.maxScrollExtent;
    final to = estimate.clamp(0.0, max > 0 ? max : 0.0);
    if ((_scroll.offset - to).abs() < 4) {
      WidgetsBinding.instance.addPostFrameCallback((_) => snap());
      return;
    }
    _scroll.animateTo(
      to,
      duration: VynlMotion.normal,
      curve: VynlMotion.emphasized,
    );
    WidgetsBinding.instance.addPostFrameCallback((_) => snap());
  }

  double get _estimatedExtent {
    if (_lineKeys.isEmpty) return 38;
    var sum = 0.0;
    for (final key in _lineKeys.values) {
      final box = key.currentContext?.findRenderObject() as RenderBox?;
      if (box != null && box.hasSize) sum += box.size.height;
    }
    return _lineKeys.isEmpty ? 38 : (sum / _lineKeys.length).clamp(28.0, 64.0);
  }

  Key _keyFor(int index) =>
      _lineKeys.putIfAbsent(index, () => GlobalKey(debugLabel: 'lyr$index'));

  @override
  Widget build(BuildContext context) {
    _maybeFetch();

    final track = widget.player.current;
    final embedded = lyricsFromTrack(track?.lyrics);
    final result = _remote ?? embedded;
    if (track == null || result == null) {
      return const SizedBox.shrink();
    }

    final canSync = result.synced;

    final cover = track.localCoverPath;
    final hasArt = cover != null && File(cover).existsSync();

    return Container(
      decoration: BoxDecoration(
        color: VynlColors.bg.withValues(alpha: hasArt ? 0.30 : 0.62),
        borderRadius: BorderRadius.circular(VynlRadius.art),
      ),
      clipBehavior: Clip.antiAlias,
      child: Stack(
        fit: StackFit.expand,
        children: [
          const ColoredBox(color: Color(0x660E0E10)),
          canSync
              ? _SyncedLines(
                  player: widget.player,
                  lines: toLyricLines(result),
                  scroll: _scroll,
                  keyFor: _keyFor,
                  onActive: _centre,
                )
              : _PlainStanzas(result: result, scroll: _cardScroll),
        ],
      ),
    );
  }
}

class _PlainStanzas extends StatelessWidget {
  const _PlainStanzas({required this.result, required this.scroll});

  final LyricsResult result;
  final ScrollController scroll;

  @override
  Widget build(BuildContext context) {
    final stanzas = stanzasOf(result.text);
    if (stanzas.isEmpty) {
      return const Center(
        child: Text(
          'Instrumental',
          style: TextStyle(color: VynlColors.dim, fontSize: 15),
        ),
      );
    }
    return ListView.builder(
        controller: scroll,
        padding: const EdgeInsets.fromLTRB(16, 22, 16, 26),
        itemCount: stanzas.length,
        itemBuilder: (context, i) => Padding(
          padding: EdgeInsets.only(bottom: i == stanzas.length - 1 ? 0 : 16),
          child: Column(
            children: [
              for (final line in stanzas[i])
                Padding(
                  padding: const EdgeInsets.only(bottom: 5),
                  child: Text(
                    line,
                    textAlign: TextAlign.center,
                    style: const TextStyle(
                      fontFamily: VynlFonts.lyrics,
                      color: VynlColors.text,
                      fontSize: 14.5,
                      height: 1.45,
                      fontVariations: [FontVariation.weight(460)],
                      letterSpacing: -0.2,
                      shadows: [
                        Shadow(color: Color(0x73000000), blurRadius: 5),
                      ],
                    ),
                  ),
                ),
            ],
          ),
        ),
      );
  }
}

class _SyncedLines extends StatefulWidget {
  const _SyncedLines({
    required this.player,
    required this.lines,
    required this.scroll,
    required this.keyFor,
    required this.onActive,
  });

  final PlayerController player;
  final List<LyricLine> lines;
  final ScrollController scroll;
  final Key Function(int index) keyFor;
  final void Function(int index, int count) onActive;

  @override
  State<_SyncedLines> createState() => _SyncedLinesState();
}

class _SyncedLinesState extends State<_SyncedLines> {
  int _lastActive = -2;

  @override
  Widget build(BuildContext context) {
    final lines = widget.lines;
    return StreamBuilder<int>(
      stream: widget.player.positionStream
          .map((p) => activeLineIndex(lines, p))
          .distinct(),
      builder: (context, snap) {
        final active = snap.data ?? -1;

        if (active != _lastActive) {
          _lastActive = active;
          WidgetsBinding.instance.addPostFrameCallback((_) {
            if (mounted) widget.onActive(active, lines.length);
          });
        }

        return ListView.builder(
          controller: widget.scroll,
          padding: const EdgeInsets.fromLTRB(14, 34, 14, 44),
          itemCount: lines.length,
          itemBuilder: (context, i) {
            final line = lines[i];
            final isActive = i == active;
            final instrumental = isInstrumental(line.text);

            final TextStyle base = TextStyle(
              fontFamily: VynlFonts.lyrics,
              color: isActive
                  ? VynlColors.text
                  : (active >= 0 && i < active)
                      ? VynlColors.faint
                      : VynlColors.dim,
              fontSize: isActive ? 16 : 12.5,
              height: 1.32,
              fontVariations: [FontVariation.weight(isActive ? 700 : 520)],
              letterSpacing: isActive ? -0.4 : -0.15,
              shadows: const [
                Shadow(color: Color(0x8A000000), blurRadius: 6),
                Shadow(color: Color(0x40000000), blurRadius: 1),
              ],
            );

            return GestureDetector(
              behavior: HitTestBehavior.opaque,
              onTap: line.time < 0
                  ? null
                  : () => widget.player.seek(
                        Duration(milliseconds: (line.time * 1000).round()),
                      ),
              child: Padding(
                key: widget.keyFor(i),
                padding: const EdgeInsets.symmetric(vertical: 5),
                child: AnimatedScale(
                  duration: VynlMotion.fast,
                  curve: VynlMotion.emphasized,
                  scale: isActive ? 1.0 : 0.96,
                  child: _LineBody(
                    player: widget.player,
                    line: line,
                    style: base,
                    instrumental: instrumental,
                    active: isActive,
                  ),
                ),
              ),
            );
          },
        );
      },
    );
  }
}

class _LineBody extends StatelessWidget {
  const _LineBody({
    required this.player,
    required this.line,
    required this.style,
    required this.instrumental,
    required this.active,
  });

  final PlayerController player;
  final LyricLine line;
  final TextStyle style;
  final bool instrumental;
  final bool active;

  @override
  Widget build(BuildContext context) {
    if (instrumental) {
      return _InstrumentalDots(active: active);
    }

    final words = line.words;
    if (active && words != null && words.isNotEmpty) {
      return _KaraokeWords(player: player, words: words, style: style);
    }

    return Text(line.text, textAlign: TextAlign.center, style: style);
  }
}

class _KaraokeWords extends StatelessWidget {
  const _KaraokeWords({
    required this.player,
    required this.words,
    required this.style,
  });

  final PlayerController player;
  final List<LyricWord> words;
  final TextStyle style;

  @override
  Widget build(BuildContext context) {
    return StreamBuilder<int>(
      stream: player.positionStream
          .map((p) => activeWordIndex(words, p))
          .distinct(),
      builder: (context, snap) {
        final sung = snap.data ?? -1;
        return Text.rich(
          TextSpan(
            children: [
              for (var i = 0; i < words.length; i++)
                TextSpan(
                  text: words[i].text,
                  style: TextStyle(
                    color: i <= sung ? VynlColors.accent : style.color,
                    fontVariations: [
                      FontVariation.weight(i <= sung ? 700 : 520),
                    ],
                  ),
                ),
            ],
          ),
          textAlign: TextAlign.center,
          style: style,
        );
      },
    );
  }
}

class _InstrumentalDots extends StatefulWidget {
  const _InstrumentalDots({required this.active});

  final bool active;

  @override
  State<_InstrumentalDots> createState() => _InstrumentalDotsState();
}

class _InstrumentalDotsState extends State<_InstrumentalDots>
    with SingleTickerProviderStateMixin {
  late final AnimationController _c = AnimationController(
    vsync: this,
    duration: const Duration(milliseconds: 1400),
  );

  @override
  void initState() {
    super.initState();
    if (widget.active) _c.repeat();
  }

  @override
  void didUpdateWidget(covariant _InstrumentalDots old) {
    super.didUpdateWidget(old);
    if (widget.active && !_c.isAnimating) {
      _c.repeat();
    } else if (!widget.active && _c.isAnimating) {
      _c.stop();
      _c.value = 0;
    }
  }

  @override
  void dispose() {
    _c.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    if (!widget.active) return _row(baseAlpha: 0.3);

    return AnimatedBuilder(
      animation: _c,
      builder: (context, _) {
        double wave(int i) {
          final phase = (_c.value + i * 0.18) % 1.0;
          return (phase - 0.5).abs() * 2;
        }

        return _row(baseAlpha: 1.0, wave: wave);
      },
    );
  }

  Widget _row({required double baseAlpha, double Function(int)? wave}) {
    return Row(
      mainAxisAlignment: MainAxisAlignment.center,
      children: List.generate(3, (i) {
        final w = wave?.call(i);
        return Padding(
          padding: const EdgeInsets.symmetric(horizontal: 5),
          child: Transform.translate(
            offset: Offset(0, w == null ? 0.0 : -4 * (1 - w)),
            child: Container(
              width: 6,
              height: 6,
              decoration: BoxDecoration(
                shape: BoxShape.circle,
                color: VynlColors.text.withValues(
                  alpha: w == null ? baseAlpha : baseAlpha * (0.35 + 0.65 * (1 - w)),
                ),
              ),
            ),
          ),
        );
      }),
    );
  }
}

class _PlayerBody extends StatelessWidget {
  const _PlayerBody({
    super.key,
    required this.player,
    required this.track,
    required this.lyricsOn,
  });

  final PlayerController player;
  final CatalogTrack track;
  final bool lyricsOn;

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final artSize = math.min(
          constraints.maxWidth - 72,
          constraints.maxHeight * 0.42,
        );
        return Column(
          children: [
            const Spacer(flex: 1),
            AnimatedSwitcher(
              duration: VynlMotion.slow,
              switchInCurve: VynlMotion.emphasized,
              switchOutCurve: Curves.easeIn,
              child: lyricsOn
                  ? SizedBox(
                      key: const ValueKey('lyrics'),
                      width: artSize,
                      height: artSize,
                      child: _LyricsOverlay(player: player),
                    )
                  : Center(
                      key: const ValueKey('art'),
                      child: Hero(
                        tag: 'art-${track.id}',
                        child: ArtTile(
                          coverPath: track.localCoverPath,
                          size: artSize,
                          radius: VynlRadius.art,
                          accent: VynlColors.accent,
                        ),
                      ),
                    ),
            ),
            const Spacer(flex: 1),
                  Padding(
                    padding: const EdgeInsets.symmetric(horizontal: 32),
                    child: Column(
                      children: [
                        Text(
                          track.title,
                          maxLines: 2,
                          overflow: TextOverflow.ellipsis,
                          textAlign: TextAlign.center,
                          style: const TextStyle(
                            fontFamily: VynlFonts.display,
                            fontSize: 23,
                            fontWeight: FontWeight.w800,
                            height: 1.15,
                            letterSpacing: -0.5,
                            color: VynlColors.text,
                          ),
                        ),
                        const SizedBox(height: 7),
                        Text(
                          '${track.artist}  ·  ${track.album}',
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          textAlign: TextAlign.center,
                          style: const TextStyle(
                            fontSize: 13.5,
                            color: VynlColors.dim,
                          ),
                        ),
                      ],
                    ),
                  ),
                  const SizedBox(height: 26),
                  Padding(
                    padding: const EdgeInsets.symmetric(horizontal: 26),
                    child: _SeekBar(player: player),
                  ),
                  const SizedBox(height: 16),
                  _Transport(player: player),
                  const SizedBox(height: 24),
                  Padding(
                    padding: const EdgeInsets.symmetric(horizontal: 26),
                    child: Row(
                      children: [
                        VynlControl(
                          onTap: player.toggleRepeat,
                          style: VynlControlStyle.plain,
                          size: 34,
                          iconSize: 20,
                          color: player.repeatActive
                              ? VynlColors.accent.withValues(alpha: 0.16)
                              : null,
                          iconColor:
                              player.repeatActive ? VynlColors.accent : null,
                          tooltip: player.repeatMode == RepeatPreset.one
                              ? 'Repeat one'
                              : player.repeatMode == RepeatPreset.all
                                  ? 'Repeat all'
                                  : 'Repeat off',
                          icon: player.repeatMode == RepeatPreset.one
                              ? Icons.repeat_one_rounded
                              : Icons.repeat_rounded,
                        ),
                        const Spacer(),
                        VynlControl(
                          onTap: () => player.playTracks(
                            [...player.queue]..shuffle(),
                            startIndex: 0,
                          ),
                          style: VynlControlStyle.plain,
                          size: 34,
                          iconSize: 20,
                          icon: Icons.shuffle_rounded,
                        ),
                      ],
                    ),
                  ),
                  const Spacer(flex: 1),
                ],
              );
      },
    );
  }
}

void _showQueueSheet(BuildContext context, PlayerController player) {
  showModalBottomSheet<void>(
      context: context,
      backgroundColor: VynlColors.surface,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(
          top: Radius.circular(VynlRadius.control),
        ),
      ),
      builder: (ctx) {
        return SafeArea(
          child: SizedBox(
            height: 420,
            child: ListenableBuilder(
              listenable: player,
              builder: (context, _) {
                final queue = player.queue;
                final current = player.current;
                final index = player.index;
                final upcoming = queue.asMap().entries
                    .where((e) => e.key > index || current == null)
                    .toList(growable: false);

                return Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Padding(
                      padding: const EdgeInsets.fromLTRB(22, 20, 22, 10),
                      child: Row(
                        children: [
                          const Expanded(
                            child: Text(
                              'QUEUE',
                              style: TextStyle(
                                color: VynlColors.faint,
                                fontSize: 11,
                                fontWeight: FontWeight.w800,
                                letterSpacing: 1.3,
                              ),
                            ),
                          ),
                          if (queue.isNotEmpty)
                            Text(
                              '${index + 1} / ${queue.length}',
                              style: const TextStyle(
                                color: VynlColors.dim,
                                fontSize: 11,
                                fontWeight: FontWeight.w700,
                                letterSpacing: 1.1,
                              ),
                            ),
                        ],
                      ),
                    ),
                    if (current != null)
                      Padding(
                        padding: const EdgeInsets.fromLTRB(10, 0, 10, 10),
                        child: DecoratedBox(
                          decoration: BoxDecoration(
                            color: VynlColors.accentSoft,
                            borderRadius:
                                BorderRadius.circular(VynlRadius.tile),
                          ),
                          child: TrackTile(
                            track: current,
                            active: true,
                            subtitle: current.artist,
                            showStatus: false,
                            durationLabel:
                                formatDurationSeconds(current.duration),
                          ),
                        ),
                      ),
                    if (upcoming.isNotEmpty)
                      const Padding(
                        padding: EdgeInsets.fromLTRB(22, 0, 22, 2),
                        child: Text(
                          'NEXT UP',
                          style: TextStyle(
                            color: VynlColors.faint,
                            fontSize: 11,
                            fontWeight: FontWeight.w800,
                            letterSpacing: 1.3,
                          ),
                        ),
                      ),
                    Expanded(
                      child: upcoming.isEmpty
                          ? const Center(
                              child: Text(
                                'Nothing queued',
                                style: TextStyle(
                                  color: VynlColors.faint,
                                  fontSize: 12.5,
                                ),
                              ),
                            )
                          : ListView.builder(
                              padding: const EdgeInsets.only(bottom: 20),
                              itemCount: upcoming.length,
                              itemBuilder: (context, n) {
                                final entry = upcoming[n];
                                return TrackTile(
                                  track: entry.value,
                                  subtitle: entry.value.artist,
                                  showStatus: false,
                                  onTap: () {
                                    player.playAt(entry.key);
                                    Navigator.of(ctx).pop();
                                  },
                                );
                              },
                            ),
                    ),
                  ],
                );
              },
            ),
          ),
        );
      },
  );
}

class _SeekBar extends StatefulWidget {
  const _SeekBar({required this.player});

  final PlayerController player;

  @override
  State<_SeekBar> createState() => _SeekBarState();
}

class _SeekBarState extends State<_SeekBar> {
  static const _trackHeight = 3.0;
  static const _touchHeight = 30.0;
  static const _restingSide = 9.0;
  static const _activeSide = 13.0;

  double? _preview;
  bool _dragging = false;

  void _begin(double dx, double width, Duration dur) {
    setState(() {
      _dragging = true;
      _preview = width <= 0 || dur.inMilliseconds <= 0
          ? null
          : (dx / width).clamp(0.0, 1.0);
    });
  }

  void _move(double dx, double width, Duration dur) {
    if (_preview == null || width <= 0 || dur.inMilliseconds <= 0) return;
    final next = (dx / width).clamp(0.0, 1.0);
    if (next == _preview) return;
    setState(() => _preview = next);
  }

  void _end() {
    final dur = widget.player.player.duration ?? Duration.zero;
    final preview = _preview;
    final target = preview == null || dur.inMilliseconds <= 0
        ? null
        : Duration(milliseconds: (preview * dur.inMilliseconds).round());
    setState(() {
      _preview = null;
      _dragging = false;
    });
    if (target != null) widget.player.seek(target);
  }

  void _cancel() {
    if (_preview == null && !_dragging) return;
    setState(() {
      _preview = null;
      _dragging = false;
    });
  }

  @override
  Widget build(BuildContext context) {
    final player = widget.player;
    final accent = VynlColors.accent;

    return StreamBuilder<Duration?>(
      stream: player.durationStream,
      builder: (context, durSnap) {
        final dur = durSnap.data ?? Duration.zero;
        final total = dur.inMilliseconds;
        return StreamBuilder<Duration>(
          stream: player.positionStream,
          builder: (context, snap) {
            final pos = snap.data ?? Duration.zero;
            final live =
                total <= 0 ? 0.0 : (pos.inMilliseconds / total).clamp(0.0, 1.0);
            final progress = _preview ?? live;
            final shown = _preview != null && total > 0
                ? Duration(milliseconds: (_preview! * total).round())
                : pos;
            final buffered = total <= 0
                ? 0.0
                : (player.player.bufferedPosition.inMilliseconds / total)
                    .clamp(0.0, 1.0);

            return Row(
              crossAxisAlignment: CrossAxisAlignment.center,
              children: [
                _Stamp(formatDuration(shown), width: 42),
                const SizedBox(width: 11),
                Expanded(
                  child: LayoutBuilder(
                    builder: (context, c) {
                      final width = c.maxWidth;
                      final top = (_touchHeight - _trackHeight) / 2;
                      final side = _dragging ? _activeSide : _restingSide;
                      return Semantics(
                        slider: true,
                        label: 'Seek',
                        value: formatDuration(shown),
                        child: GestureDetector(
                          behavior: HitTestBehavior.opaque,
                          onPanDown: (d) =>
                              _begin(d.localPosition.dx, width, dur),
                          onPanUpdate: (d) =>
                              _move(d.localPosition.dx, width, dur),
                          onPanEnd: (_) => _end(),
                          onPanCancel: _cancel,
                          child: SizedBox(
                            height: _touchHeight,
                            child: Stack(
                              clipBehavior: Clip.none,
                              children: [
                                _seg(
                                  0,
                                  top,
                                  width,
                                  VynlColors.text.withValues(alpha: 0.2),
                                ),
                                if (buffered > 0)
                                  _seg(
                                    0,
                                    top,
                                    width * buffered,
                                    VynlColors.text.withValues(alpha: 0.34),
                                  ),
                                if (progress > 0)
                                  _seg(0, top, width * progress, accent),
                                Positioned(
                                  left: width * progress - side / 2,
                                  top: (_touchHeight - side) / 2,
                                  width: side,
                                  height: side,
                                  child: IgnorePointer(
                                    child: Transform.rotate(
                                      angle: math.pi / 4,
                                      child: Container(
                                        decoration: BoxDecoration(
                                          color: accent,
                                          borderRadius:
                                              BorderRadius.circular(2),
                                          boxShadow: [
                                            BoxShadow(
                                              color: accent.withValues(
                                                alpha: _dragging ? 0.34 : 0.2,
                                              ),
                                              blurRadius: 6,
                                              spreadRadius: 3,
                                            ),
                                          ],
                                        ),
                                      ),
                                    ),
                                  ),
                                ),
                              ],
                            ),
                          ),
                        ),
                      );
                    },
                  ),
                ),
                const SizedBox(width: 11),
                _Stamp(formatDuration(dur)),
              ],
            );
          },
        );
      },
    );
  }

  Widget _seg(double left, double top, double width, Color color) {
    return Positioned(
      left: left,
      top: top,
      width: width < 0 ? 0 : width,
      height: _trackHeight,
      child: DecoratedBox(
        decoration: BoxDecoration(
          color: color,
          borderRadius: BorderRadius.circular(VynlRadius.control),
        ),
      ),
    );
  }
}

class _Stamp extends StatelessWidget {
  const _Stamp(this.text, {this.width});

  final String text;
  final double? width;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: width,
      child: Text(
        text,
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
        style: const TextStyle(
          color: VynlColors.dim,
          fontSize: 12,
          fontFeatures: [FontFeature.tabularFigures()],
        ),
      ),
    );
  }
}

class _Transport extends StatelessWidget {
  const _Transport({required this.player});

  final PlayerController player;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 26),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          VynlControl(
            onTap: player.previous,
            style: VynlControlStyle.raised,
            size: 42,
            iconSize: 25,
            icon: Icons.skip_previous_rounded,
          ),
          const SizedBox(width: 18),
          StreamBuilder<PlayerState>(
            stream: player.playerStateStream,
            builder: (context, snap) {
              final playing = snap.data?.playing ?? false;
              final buffering =
                  snap.data?.processingState == ProcessingState.buffering;
              return _PlayButton(
                playing: playing,
                buffering: buffering,
                color: VynlColors.accent,
                onTap: player.toggle,
              );
            },
          ),
          const SizedBox(width: 18),
          VynlControl(
            onTap: player.next,
            style: VynlControlStyle.raised,
            size: 42,
            iconSize: 25,
            icon: Icons.skip_next_rounded,
          ),
        ],
      ),
    );
  }
}

class _PlayButton extends StatelessWidget {
  const _PlayButton({
    required this.playing,
    required this.buffering,
    required this.color,
    required this.onTap,
  });

  final bool playing;
  final bool buffering;
  final Color color;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return VynlControl(
      onTap: onTap,
      style: VynlControlStyle.accent,
      color: color,
      size: 58,
      child: buffering
          ? SizedBox(
              width: 26,
              height: 26,
              child: CircularProgressIndicator(
                strokeWidth: 2.4,
                color: VynlColors.accentOn,
              ),
            )
          : Icon(
              playing ? Icons.pause_rounded : Icons.play_arrow_rounded,
              size: 33,
              color: VynlColors.accentOn,
            ),
    );
  }
}
