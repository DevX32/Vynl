import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../models.dart';
import '../services/player_controller.dart';
import '../state/app_state.dart';
import '../theme.dart';
import '../widgets/artwork.dart';
import '../widgets/common.dart';

class RemotePanel extends StatelessWidget {
  const RemotePanel({super.key});

  @override
  Widget build(BuildContext context) {
    final app = context.watch<AppState>();
    final player = context.watch<PlayerController>();
    final remote = app.remotePlayer;
    final track = remote?.track;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        SectionHeader(
          title: 'ON YOUR DESKTOP',
          padding: const EdgeInsets.fromLTRB(0, 24, 0, 10),
        ),
        Material(
          color: VynlColors.surfaceRaised,
          borderRadius: BorderRadius.circular(VynlRadius.card),
          clipBehavior: Clip.antiAlias,
          child: Padding(
            padding: const EdgeInsets.fromLTRB(14, 14, 14, 12),
            child: Column(
              children: [
                Row(
                  children: [
                    ArtTile(
                      coverPath: null,
                      coverUrl: track?.cover,
                      size: 52,
                      accent: Theme.of(context).colorScheme.primary,
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            track?.title ?? 'Nothing playing',
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: const TextStyle(
                              fontSize: 14,
                              fontWeight: FontWeight.w600,
                              color: VynlColors.text,
                            ),
                          ),
                          const SizedBox(height: 2),
                          Text(
                            track?.artist ?? 'Start something on your desktop',
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: const TextStyle(
                              fontSize: 12,
                              color: VynlColors.faint,
                            ),
                          ),
                        ],
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 10),
                Row(
                  mainAxisAlignment: MainAxisAlignment.center,
                  children: [
                    _RemoteButton(
                      icon: Icons.skip_previous_rounded,
                      onTap: () => app.remoteAction('previous'),
                    ),
                    const SizedBox(width: 14),
                    _RemoteButton(
                      icon: remote?.playing == true
                          ? Icons.pause_rounded
                          : Icons.play_arrow_rounded,
                      accent: true,
                      onTap: () => app.remoteAction('toggle'),
                    ),
                    const SizedBox(width: 14),
                    _RemoteButton(
                      icon: Icons.skip_next_rounded,
                      onTap: () => app.remoteAction('next'),
                    ),
                  ],
                ),
              ],
            ),
          ),
        ),
        if (app.remoteTaste != null && app.remoteTaste!.liked.isNotEmpty) ...[
          SectionHeader(
            title: 'LIKED SONGS',
            padding: const EdgeInsets.fromLTRB(0, 24, 0, 10),
          ),
          for (final liked in app.remoteTaste!.liked.take(10))
            TrackTile(
              track: _track(app, liked),
              onTap: () {
                final list = app.streamTracks(app.remoteTaste!.liked);
                if (list.isEmpty) return;
                final index = list.indexWhere((t) => t.id == 'ytm:${liked.id}');
                player.playTracks(list, startIndex: index < 0 ? 0 : index);
              },
            ),
        ],
      ],
    );
  }

  CatalogTrack _track(AppState app, StreamTrack liked) {
    final list = app.streamTracks([liked]);
    return list.isEmpty
        ? CatalogTrack(
            id: 'ytm:${liked.id}',
            title: liked.title,
            artist: liked.artist,
            album: liked.album ?? '',
            duration: liked.duration ?? 0,
            ext: 'webm',
            hasCover: liked.cover != null,
            coverUrl: liked.cover,
          )
        : list.first;
  }
}

class _RemoteButton extends StatelessWidget {
  const _RemoteButton({
    required this.icon,
    required this.onTap,
    this.accent = false,
  });

  final IconData icon;
  final VoidCallback onTap;
  final bool accent;

  @override
  Widget build(BuildContext context) {
    return VynlControl(
      onTap: onTap,
      size: accent ? 44 : 38,
      iconSize: accent ? 24 : 20,
      icon: icon,
      style: accent ? VynlControlStyle.accent : VynlControlStyle.plain,
    );
  }
}
