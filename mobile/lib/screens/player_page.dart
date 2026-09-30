import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:just_audio/just_audio.dart';
import 'package:provider/provider.dart';

import '../services/player_controller.dart';
import '../theme.dart';
import '../widgets/artwork.dart';
import '../widgets/common.dart';

class PlayerPage extends StatelessWidget {
  const PlayerPage({super.key});

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

    return ArtBackdrop(
      coverPath: track.localCoverPath,
      tint: player.accent,
      blur: 80,
      dim: 0.62,
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
          child: LayoutBuilder(
            builder: (context, constraints) {
              final artSize = math.min(
                constraints.maxWidth - 72,
                constraints.maxHeight * 0.42,
              );
              return Column(
                children: [
                  const Spacer(flex: 1),
                  Center(
                    child: Hero(
                      tag: 'art-${track.id}',
                      child: ArtTile(
                        coverPath: track.localCoverPath,
                        size: artSize,
                        radius: VynlRadius.art,
                        accent: player.accent,
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
                    child: _WaveSeek(player: player),
                  ),
                  const SizedBox(height: 14),
                  _Transport(player: player),
                  const SizedBox(height: 24),
                  Padding(
                    padding: const EdgeInsets.symmetric(horizontal: 26),
                    child: Row(
                      children: [
                        Container(
                          padding: const EdgeInsets.symmetric(
                            horizontal: 12,
                            vertical: 7,
                          ),
                          decoration: BoxDecoration(
                            color: Colors.white.withValues(alpha: 0.08),
                            borderRadius:
                                BorderRadius.circular(VynlRadius.control),
                          ),
                          child: Row(
                            mainAxisSize: MainAxisSize.min,
                            children: [
                              Icon(
                                Icons.headphones_rounded,
                                size: 14,
                                color: player.accent,
                              ),
                              const SizedBox(width: 7),
                              Text(
                                '${player.queue.length} in queue',
                                style: const TextStyle(
                                  fontSize: 11.5,
                                  fontWeight: FontWeight.w600,
                                  color: VynlColors.dim,
                                ),
                              ),
                            ],
                          ),
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
          ),
        ),
      ),
    );
  }

  static void _showQueueSheet(BuildContext context, PlayerController player) {
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
}

class _WaveSeek extends StatelessWidget {
  const _WaveSeek({required this.player});

  final PlayerController player;

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final width = constraints.maxWidth;

        void seekAt(double dx) {
          final dur = player.player.duration ?? Duration.zero;
          if (dur.inMilliseconds <= 0 || width <= 0) return;
          final f = (dx / width).clamp(0.0, 1.0);
          player.seek(Duration(milliseconds: (f * dur.inMilliseconds).round()));
        }

        return StreamBuilder<Duration>(
          stream: player.positionStream,
          builder: (context, snap) {
            final pos = snap.data ?? Duration.zero;
            final dur = player.player.duration ?? Duration.zero;
            final progress = dur.inMilliseconds <= 0
                ? 0.0
                : pos.inMilliseconds / dur.inMilliseconds;

            return GestureDetector(
              behavior: HitTestBehavior.opaque,
              onHorizontalDragUpdate: (d) => seekAt(d.localPosition.dx),
              onTapDown: (d) => seekAt(d.localPosition.dx),
              child: SizedBox(
                height: 46,
                width: double.infinity,
                child: CustomPaint(
                  painter: _WavePainter(
                    seed: player.current?.id ?? '',
                    progress: progress.clamp(0.0, 1.0),
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

class _WavePainter extends CustomPainter {
  _WavePainter({required this.seed, required this.progress});

  final String seed;
  final double progress;

  @override
  void paint(Canvas canvas, Size size) {
    const barW = 2.4;
    const gap = 2.6;
    final count = (size.width / (barW + gap)).floor();
    if (count <= 0) return;

    var state = seed.hashCode & 0x7fffffff;
    if (state == 0) state = 12345;

    final mid = size.height / 2;
    final played = VynlColors.text.withValues(alpha: 0.92);
    final rest = VynlColors.text.withValues(alpha: 0.16);
    final paint = Paint()..strokeCap = StrokeCap.round;

    final reached = progress * count;

    for (var i = 0; i < count; i++) {
      state = (state * 1103515245 + 12345) & 0x7fffffff;
      final r = state / 0x7fffffff;

      final envelope = 0.45 + 0.55 * math.sin((i / count) * math.pi * 3.1);
      var h = (6 + r * 32 * envelope).clamp(5.0, size.height);
      if (i > reached && i < reached + 2) h = 5;

      paint.color = i <= reached ? played : rest;
      final x = i * (barW + gap) + barW / 2;
      canvas.drawLine(
        Offset(x, mid - h / 2),
        Offset(x, mid + h / 2),
        paint,
      );
    }
  }

  @override
  bool shouldRepaint(_WavePainter old) =>
      old.seed != seed || old.progress != progress;
}

class _Transport extends StatelessWidget {
  const _Transport({required this.player});

  final PlayerController player;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 26),
      child: Row(
        children: [
          StreamBuilder<Duration>(
            stream: player.positionStream,
            builder: (context, snap) => Text(
              formatDuration(snap.data ?? Duration.zero),
              style: const TextStyle(
                color: VynlColors.dim,
                fontSize: 12,
                fontFeatures: [FontFeature.tabularFigures()],
              ),
            ),
          ),
          const Spacer(),
          VynlControl(
            onTap: player.previous,
            style: VynlControlStyle.raised,
            size: 40,
            iconSize: 24,
            icon: Icons.skip_previous_rounded,
          ),
          const SizedBox(width: 10),
          StreamBuilder<PlayerState>(
            stream: player.playerStateStream,
            builder: (context, snap) {
              final playing = snap.data?.playing ?? false;
              final buffering =
                  snap.data?.processingState == ProcessingState.buffering;
              return _PlayButton(
                playing: playing,
                buffering: buffering,
                color: player.accent,
                onTap: player.toggle,
              );
            },
          ),
          const SizedBox(width: 10),
          VynlControl(
            onTap: player.next,
            style: VynlControlStyle.raised,
            size: 40,
            iconSize: 24,
            icon: Icons.skip_next_rounded,
          ),
          const Spacer(),
          StreamBuilder<Duration?>(
            stream: player.durationStream,
            builder: (context, snap) => Text(
              formatDuration(snap.data ?? Duration.zero),
              style: const TextStyle(
                color: VynlColors.dim,
                fontSize: 12,
                fontFeatures: [FontFeature.tabularFigures()],
              ),
            ),
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
      size: 52,
      child: buffering
          ? const SizedBox(
              width: 24,
              height: 24,
              child: CircularProgressIndicator(
                strokeWidth: 2.4,
                color: Color(0xFF17131A),
              ),
            )
          : Icon(
              playing ? Icons.pause_rounded : Icons.play_arrow_rounded,
              size: 30,
              color: const Color(0xFF17131A),
            ),
    );
  }
}
