import 'package:flutter/material.dart';
import 'package:just_audio/just_audio.dart';
import 'package:provider/provider.dart';

import '../screens/player_page.dart';
import '../services/player_controller.dart';
import '../theme.dart';
import 'artwork.dart';
import 'common.dart';

Route<void> _playerRoute() {
  return PageRouteBuilder<void>(
    transitionDuration: VynlMotion.normal,
    reverseTransitionDuration: VynlMotion.normal,
    opaque: false,
    barrierColor: Colors.transparent,
    pageBuilder: (_, __, ___) => const PlayerPage(),
    transitionsBuilder: (_, anim, __, child) {
      final curved = CurvedAnimation(
        parent: anim,
        curve: VynlMotion.emphasized,
        reverseCurve: VynlMotion.emphasized,
      );
      return FadeTransition(
        opacity: Tween<double>(begin: 0, end: 1).animate(
          CurvedAnimation(
            parent: anim,
            curve: const Interval(0, 0.7, curve: Curves.easeOut),
          ),
        ),
        child: ScaleTransition(
          scale: Tween<double>(begin: 0.94, end: 1).animate(curved),
          child: SlideTransition(
            position: Tween<Offset>(
              begin: const Offset(0, 0.035),
              end: Offset.zero,
            ).animate(curved),
            child: child,
          ),
        ),
      );
    },
  );
}

class MiniPlayer extends StatelessWidget {
  const MiniPlayer({super.key});

  static const _height = 66.0;

  @override
  Widget build(BuildContext context) {
    final player = context.watch<PlayerController>();
    final track = player.current;
    if (track == null) return const SizedBox.shrink();

    final scheme = Theme.of(context).colorScheme;

    return Padding(
      padding: const EdgeInsets.fromLTRB(10, 0, 10, 6),
      child: Material(
        color: VynlColors.surfaceRaised,
        borderRadius: BorderRadius.circular(VynlRadius.card),
        clipBehavior: Clip.antiAlias,
        elevation: 6,
        shadowColor: Colors.black.withValues(alpha: 0.4),
        child: InkWell(
          onTap: () => Navigator.of(context).push(_playerRoute()),
          child: SizedBox(
            height: _height,
            child: Stack(
              children: [
                Positioned(
                  left: 0,
                  right: 0,
                  top: 0,
                  child: StreamBuilder<Duration>(
                    stream: player.positionStream,
                    builder: (context, snap) {
                      final pos = snap.data ?? Duration.zero;
                      final dur = player.player.duration ?? Duration.zero;
                      final v = dur.inMilliseconds == 0
                          ? 0.0
                          : pos.inMilliseconds / dur.inMilliseconds;
                      return TweenAnimationBuilder<double>(
                        tween: Tween(end: v.clamp(0.0, 1.0)),
                        duration: const Duration(milliseconds: 280),
                        curve: VynlMotion.standard,
                        builder: (context, value, _) =>
                            LinearProgressIndicator(
                          value: value,
                          minHeight: 2,
                          backgroundColor: Colors.transparent,
                          color: scheme.primary,
                        ),
                      );
                    },
                  ),
                ),
                Padding(
                  padding: const EdgeInsets.fromLTRB(8, 4, 6, 0),
                  child: Row(
                    children: [
                      ArtTile(
                        coverPath: track.localCoverPath,
                        size: 46,
                        accent: VynlColors.accent,
                      ),
                      const SizedBox(width: 12),
                      Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          mainAxisAlignment: MainAxisAlignment.center,
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            Text(
                              track.title,
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                              style: const TextStyle(
                                fontWeight: FontWeight.w700,
                                fontSize: 13.5,
                                letterSpacing: -0.1,
                                color: VynlColors.text,
                                fontFamily: VynlFonts.display,
                              ),
                            ),
                            const SizedBox(height: 2),
                            Text(
                              track.artist,
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                              style: const TextStyle(
                                color: VynlColors.dim,
                                fontSize: 12,
                              ),
                            ),
                          ],
                        ),
                      ),
                      StreamBuilder<PlayerState>(
                        stream: player.playerStateStream,
                        builder: (context, snap) {
                          final playing = snap.data?.playing ?? false;
                          return Row(
                            mainAxisSize: MainAxisSize.min,
                            children: [
                              VynlControl(
                                onTap: player.previous,
                                style: VynlControlStyle.plain,
                                size: 30,
                                iconSize: 23,
                                icon: Icons.skip_previous_rounded,
                              ),
                              const SizedBox(width: 6),
                              VynlControl(
                                onTap: player.toggle,
                                style: VynlControlStyle.accent,
                                color: VynlColors.accent,
                                size: 36,
                                iconSize: 22,
                                icon: playing
                                    ? Icons.pause_rounded
                                    : Icons.play_arrow_rounded,
                              ),
                              const SizedBox(width: 6),
                              VynlControl(
                                onTap: player.next,
                                style: VynlControlStyle.plain,
                                size: 30,
                                iconSize: 23,
                                icon: Icons.skip_next_rounded,
                              ),
                            ],
                          );
                        },
                      ),
                    ],
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
