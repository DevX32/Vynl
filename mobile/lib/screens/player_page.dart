import 'dart:async';
import 'dart:io';
import 'dart:math' as math;
import 'dart:ui';

import 'package:flutter/material.dart';
import 'package:just_audio/just_audio.dart';
import 'package:provider/provider.dart';

import '../models.dart';
import '../services/api.dart';
import '../services/lyrics.dart';
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
              icon: const Icon(Icons.more_horiz_rounded, size: 24),
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
  LyricsResult? _remote;
  String? _remoteFor;
  bool _fetching = false;
  bool _userScrolling = false;
  Timer? _resume;

  @override
  void initState() {
    super.initState();
    _scroll.addListener(_onScroll);
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _maybeFetch();
  }

  Future<void> _maybeFetch() async {
    final track = widget.player.current;
    final api = context.read<VynlApi?>();
    if (track == null || api == null) return;
    if (_remoteFor == track.id || _fetching) return;
    _fetching = true;
    _remoteFor = track.id;
    final res = await api.syncedLyrics(track.id);
    if (!mounted || widget.player.current?.id != track.id) return;
    _fetching = false;
    final parsed = res == null ? null : lyricsFromRemote(track.id, res.text);
    if (parsed != null && mounted) {
      setState(() => _remote = parsed);
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
    final atEnd =
        _scroll.position.pixels >= _scroll.position.maxScrollExtent - 24;
    if (!_userScrolling && !atEnd) {
      _userScrolling = true;
      _resume?.cancel();
      _resume = Timer(const Duration(seconds: 4), () {
        if (mounted) _userScrolling = false;
      });
    }
  }

  void _centre(int index, int count) {
    if (_userScrolling || !_scroll.hasClients || index < 0 || index >= count) {
      return;
    }
    final viewport = _scroll.position.viewportDimension;
    final target = index * 38.0 - viewport * 0.66;
    final max = _scroll.position.maxScrollExtent;
    final to = target.clamp(0.0, max > 0 ? max : 0.0);
    if ((_scroll.offset - to).abs() < 6) return;
    _scroll.animateTo(to, duration: VynlMotion.normal, curve: VynlMotion.emphasized);
  }

  @override
  Widget build(BuildContext context) {
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
        border: Border.all(color: VynlColors.text.withValues(alpha: 0.07)),
      ),
      clipBehavior: Clip.antiAlias,
      child: Stack(
        fit: StackFit.expand,
        children: [
          if (hasArt)
            ImageFiltered(
              imageFilter: ImageFilter.blur(sigmaX: 26, sigmaY: 26),
              child: Transform.scale(
                scale: 1.5,
                child: Image.file(
                  File(cover),
                  fit: BoxFit.cover,
                  errorBuilder: (_, __, ___) => const SizedBox.shrink(),
                ),
              ),
            ),
          DecoratedBox(
            decoration: BoxDecoration(
              gradient: LinearGradient(
                begin: Alignment.topCenter,
                end: Alignment.bottomCenter,
                colors: [
                  const Color(0xFF171310).withValues(alpha: 0.30),
                  const Color(0xFF100D0B).withValues(alpha: 0.62),
                ],
              ),
            ),
          ),
          canSync
              ? _SyncedLines(
                  player: widget.player,
                  lines: toLyricLines(result),
                  scroll: _scroll,
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
        padding: const EdgeInsets.fromLTRB(13, 15, 13, 20),
        itemCount: stanzas.length,
        itemBuilder: (context, i) => Padding(
          padding: EdgeInsets.only(bottom: i == stanzas.length - 1 ? 0 : 13),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              for (final line in stanzas[i])
                Padding(
                  padding: const EdgeInsets.only(bottom: 2.5),
                  child: Text(
                    line,
                    style: const TextStyle(
                      fontFamily: VynlFonts.display,
                      color: VynlColors.text,
                      fontSize: 12,
                      height: 1.38,
                      letterSpacing: -0.15,
                      shadows: [
                        Shadow(color: Color(0x59000000), blurRadius: 3),
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

class _SyncedLines extends StatelessWidget {
  const _SyncedLines({
    required this.player,
    required this.lines,
    required this.scroll,
    required this.onActive,
  });

  final PlayerController player;
  final List<LyricLine> lines;
  final ScrollController scroll;
  final void Function(int index, int count) onActive;

  @override
  Widget build(BuildContext context) {
    return StreamBuilder<Duration>(
      stream: player.positionStream,
      builder: (context, snap) {
        final active = activeLineIndex(lines, snap.data ?? Duration.zero);
        WidgetsBinding.instance.addPostFrameCallback((_) {
          onActive(active, lines.length);
        });
        return ListView.builder(
          controller: scroll,
          padding: const EdgeInsets.fromLTRB(14, 34, 14, 44),
          itemCount: lines.length,
          itemBuilder: (context, i) {
            final line = lines[i];
            final isActive = i == active;
            return GestureDetector(
              behavior: HitTestBehavior.opaque,
              onTap: line.time < 0
                  ? null
                  : () => player.seek(
                        Duration(milliseconds: (line.time * 1000).round()),
                      ),
              child: Padding(
                padding: const EdgeInsets.symmetric(vertical: 5),
                child: Text(
                  line.text.isEmpty ? '♪' : line.text,
                  textAlign: TextAlign.center,
                  style: TextStyle(
                    fontFamily: VynlFonts.display,
                    color: isActive
                        ? VynlColors.text
                        : (active >= 0 && i < active)
                            ? VynlColors.faint
                            : VynlColors.dim,
                    fontSize: isActive ? 16 : 12.5,
                    height: 1.32,
                    fontWeight: isActive ? FontWeight.w800 : FontWeight.w600,
                    letterSpacing: isActive ? -0.4 : -0.15,
                    shadows: const [
                      Shadow(color: Color(0x8A000000), blurRadius: 6),
                      Shadow(color: Color(0x40000000), blurRadius: 1),
                    ],
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
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const Padding(
                  padding: EdgeInsets.fromLTRB(22, 20, 22, 6),
                  child: Text(
                    'UP NEXT',
                    style: TextStyle(
                      color: VynlColors.faint,
                      fontSize: 11,
                      fontWeight: FontWeight.w800,
                      letterSpacing: 1.3,
                    ),
                  ),
                ),
                Expanded(
                  child: ListView.builder(
                    padding: const EdgeInsets.only(bottom: 20),
                    itemCount: player.queue.length,
                    itemBuilder: (context, i) {
                      final t = player.queue[i];
                      return TrackTile(
                        track: t,
                        active: i == player.index,
                        showStatus: false,
                        onTap: () {
                          player.playAt(i);
                          Navigator.of(ctx).pop();
                        },
                      );
                    },
                  ),
                ),
              ],
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
          ? const SizedBox(
              width: 26,
              height: 26,
              child: CircularProgressIndicator(
                strokeWidth: 2.4,
                color: Color(0xFF17131A),
              ),
            )
          : Icon(
              playing ? Icons.pause_rounded : Icons.play_arrow_rounded,
              size: 33,
              color: const Color(0xFF17131A),
            ),
    );
  }
}
