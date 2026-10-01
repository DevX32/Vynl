import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../models.dart';
import '../services/player_controller.dart';
import '../state/app_state.dart';
import '../theme.dart';
import '../widgets/artwork.dart';
import '../widgets/common.dart';

class PlaylistsPage extends StatelessWidget {
  const PlaylistsPage({super.key});

  @override
  Widget build(BuildContext context) {
    final app = context.watch<AppState>();

    if (app.playlists.isEmpty) {
      return const Center(
        child: Padding(
          padding: EdgeInsets.symmetric(horizontal: 44),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(
                Icons.playlist_play_rounded,
                size: 44,
                color: VynlColors.faint,
              ),
              SizedBox(height: 16),
              Text(
                'No playlists synced yet.',
                style: TextStyle(
                  color: VynlColors.dim,
                  fontSize: 14,
                  fontWeight: FontWeight.w600,
                ),
              ),
              SizedBox(height: 6),
              Text(
                'Playlists from your desktop show up here after a sync.',
                textAlign: TextAlign.center,
                style: TextStyle(
                  color: VynlColors.faint,
                  fontSize: 12.5,
                  height: 1.5,
                ),
              ),
            ],
          ),
        ),
      );
    }

    return ListView(
      padding: const EdgeInsets.fromLTRB(4, 0, 4, 30),
      children: [
        const SectionHeader(title: 'YOUR PLAYLISTS'),
        for (final pl in app.playlists)
          _PlaylistRow(
            playlist: pl,
            trackCount: pl.trackIds.length,
            onTap: () => Navigator.of(context).push(
              MaterialPageRoute(
                builder: (_) => PlaylistDetailPage(playlist: pl),
              ),
            ),
          ),
      ],
    );
  }
}

class _PlaylistArt extends StatelessWidget {
  const _PlaylistArt({required this.tracks, this.size = 60});

  final List<CatalogTrack> tracks;
  final double size;

  @override
  Widget build(BuildContext context) {
    final cells = <Widget>[];
    for (var i = 0; i < 4; i++) {
      final t = i < tracks.length ? tracks[i] : null;
      cells.add(
        t == null
            ? const ColoredBox(color: VynlColors.surfaceRaised)
            : ArtTile(
                coverPath: t.localCoverPath,
                size: size / 2,
                radius: 0,
                accent: VynlColors.accent,
              ),
      );
    }

    return ClipRRect(
      borderRadius: BorderRadius.circular(VynlRadius.hero),
      child: SizedBox(
        width: size,
        height: size,
        child: Column(
          children: [
            Row(children: [cells[0], cells[1]]),
            Row(children: [cells[2], cells[3]]),
          ],
        ),
      ),
    );
  }
}

class _PlaylistRow extends StatelessWidget {
  const _PlaylistRow({
    required this.playlist,
    required this.trackCount,
    required this.onTap,
  });

  final PlaylistDto playlist;
  final int trackCount;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final app = context.read<AppState>();
    final art = app.tracksForPlaylist(playlist).take(4).toList();

    return Material(
      color: Colors.transparent,
      borderRadius: BorderRadius.circular(VynlRadius.tile),
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 9),
          child: Row(
            children: [
              _PlaylistArt(tracks: art),
              const SizedBox(width: 15),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Text(
                      playlist.name,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: const TextStyle(
                        color: VynlColors.text,
                        fontWeight: FontWeight.w700,
                        fontSize: 15,
                        letterSpacing: -0.2,
                        fontFamily: VynlFonts.display,
                      ),
                    ),
                    const SizedBox(height: 4),
                    Text(
                      '$trackCount tracks',
                      style: const TextStyle(
                        color: VynlColors.dim,
                        fontSize: 12.5,
                      ),
                    ),
                  ],
                ),
              ),
              const Icon(
                Icons.chevron_right_rounded,
                size: 22,
                color: VynlColors.faint,
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class PlaylistDetailPage extends StatelessWidget {
  const PlaylistDetailPage({super.key, required this.playlist});

  final PlaylistDto playlist;

  @override
  Widget build(BuildContext context) {
    final app = context.watch<AppState>();
    final player = context.watch<PlayerController>();
    final current = player.current;
    final tracks = app.tracksForPlaylist(playlist);
    final art = tracks.take(4).toList();

    return Container(
      decoration: const BoxDecoration(gradient: kVynlBackground),
      child: Scaffold(
        backgroundColor: Colors.transparent,
        appBar: AppBar(title: Text(playlist.name)),
        body: tracks.isEmpty
            ? const Center(
                child: Text(
                  'No downloaded tracks in this playlist yet.',
                  style: TextStyle(color: VynlColors.dim),
                ),
              )
            : Column(
                children: [
                  Padding(
                    padding: const EdgeInsets.fromLTRB(20, 4, 20, 16),
                    child: Row(
                      children: [
                        _PlaylistArt(tracks: art, size: 84),
                        const SizedBox(width: 18),
                        Expanded(
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Text(
                                playlist.name,
                                maxLines: 2,
                                overflow: TextOverflow.ellipsis,
                                style: const TextStyle(
                                  fontFamily: VynlFonts.display,
                                  fontSize: 19,
                                  fontWeight: FontWeight.w800,
                                  height: 1.15,
                                  letterSpacing: -0.4,
                                ),
                              ),
                              const SizedBox(height: 6),
                              Text(
                                tracks.length == 1
                                    ? '1 track'
                                    : '${tracks.length} tracks',
                                style: const TextStyle(
                                  color: VynlColors.dim,
                                  fontSize: 13,
                                ),
                              ),
                            ],
                          ),
                        ),
                        const SizedBox(width: 12),
                        _PlayFab(
                          onTap: () => player.playTracks(tracks),
                        ),
                      ],
                    ),
                  ),
                  Expanded(
                    child: ListView.builder(
                      padding: const EdgeInsets.fromLTRB(4, 0, 4, 28),
                      itemCount: tracks.length,
                      itemBuilder: (context, i) {
                        final track = tracks[i];
                        return TrackTile(
                          track: track,
                          active: current?.id == track.id,
                          showStatus: false,
                          onTap: () => player.playTrack(
                            track,
                            context: tracks,
                          ),
                        );
                      },
                    ),
                  ),
                ],
              ),
      ),
    );
  }
}

class _PlayFab extends StatelessWidget {
  const _PlayFab({required this.onTap});

  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return VynlControl(
      onTap: onTap,
      style: VynlControlStyle.accent,
      size: 44,
      iconSize: 26,
      icon: Icons.play_arrow_rounded,
    );
  }
}
