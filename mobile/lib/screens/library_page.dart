import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../services/player_controller.dart';
import '../state/app_state.dart';
import '../theme.dart';
import '../widgets/common.dart';

class LibraryPage extends StatefulWidget {
  const LibraryPage({super.key});

  @override
  State<LibraryPage> createState() => _LibraryPageState();
}

class _LibraryPageState extends State<LibraryPage> {
  final _searchCtrl = TextEditingController();
  String _query = '';

  @override
  void dispose() {
    _searchCtrl.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final app = context.watch<AppState>();
    final player = context.watch<PlayerController>();
    final current = player.current;
    final q = _query.trim().toLowerCase();
    final tracks = app.tracks.where((t) {
      if (q.isEmpty) return true;
      return t.title.toLowerCase().contains(q) ||
          t.artist.toLowerCase().contains(q) ||
          t.album.toLowerCase().contains(q);
    }).toList();
    final downloaded = app.downloadedTracks;

    return Column(
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(16, 2, 16, 6),
          child: TextField(
            controller: _searchCtrl,
            onChanged: (v) => setState(() => _query = v),
            textInputAction: TextInputAction.search,
            style: const TextStyle(fontSize: 14, color: VynlColors.text),
            decoration: InputDecoration(
              hintText: 'Search songs, artists, albums',
              prefixIcon: const Icon(Icons.search_rounded, size: 21),
              suffixIcon: q.isEmpty
                  ? null
                  : IconButton(
                      icon: const Icon(Icons.cancel_rounded, size: 19),
                      onPressed: () {
                        _searchCtrl.clear();
                        setState(() => _query = '');
                      },
                    ),
            ),
          ),
        ),
        SectionHeader(
          title: q.isEmpty ? 'TRACKS' : 'RESULTS',
          trailing: q.isEmpty && tracks.isNotEmpty
              ? TextButton.icon(
                  onPressed: () {
                    final pool = downloaded.isNotEmpty ? downloaded : tracks;
                    player.playTracks(pool);
                  },
                  icon: const Icon(Icons.shuffle_rounded, size: 16),
                  label: const Text(
                    'Shuffle all',
                    style: TextStyle(
                      fontSize: 12.5,
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                  style: TextButton.styleFrom(
                    foregroundColor: VynlColors.accent,
                    padding: const EdgeInsets.symmetric(horizontal: 10),
                    minimumSize: const Size(0, 34),
                  ),
                )
              : null,
        ),
        if (app.syncing && app.syncProgress != null)
          Padding(
            padding: const EdgeInsets.fromLTRB(20, 0, 20, 12),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                ClipRRect(
                  borderRadius: BorderRadius.circular(3),
                  child: LinearProgressIndicator(
                    value: app.syncProgress!.fraction,
                    minHeight: 5,
                  ),
                ),
                const SizedBox(height: 8),
                Text(
                  '${app.syncProgress!.currentTitle ?? app.syncProgress!.phase}'
                  '  (${app.syncProgress!.done}/${app.syncProgress!.total})',
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: const TextStyle(
                    color: VynlColors.faint,
                    fontSize: 12,
                  ),
                ),
              ],
            ),
          ),
        Expanded(
          child: tracks.isEmpty
              ? _EmptyState(
                  icon: q.isEmpty
                      ? Icons.music_off_rounded
                      : Icons.search_off_rounded,
                  title: q.isEmpty ? 'Nothing synced yet' : 'No matches',
                  subtitle: q.isEmpty
                      ? 'Pair with your desktop under Settings to pull your library across.'
                      : 'Try a different title, artist or album.',
                )
              : ListView.builder(
                  padding: const EdgeInsets.fromLTRB(4, 0, 4, 28),
                  itemCount: tracks.length,
                  itemBuilder: (context, i) {
                    final track = tracks[i];
                    final playable =
                        tracks.where((t) => t.isDownloaded).toList();
                    return TrackTile(
                      track: track,
                      active: current?.id == track.id,
                      onTap: track.isDownloaded
                          ? () => player.playTrack(track, context: playable)
                          : () {
                              ScaffoldMessenger.of(context).showSnackBar(
                                const SnackBar(
                                  content: Text(
                                    'Not downloaded yet — tap Sync on Wi‑Fi.',
                                  ),
                                ),
                              );
                            },
                    );
                  },
                ),
        ),
      ],
    );
  }
}

class _EmptyState extends StatelessWidget {
  const _EmptyState({
    required this.icon,
    required this.title,
    required this.subtitle,
  });

  final IconData icon;
  final String title;
  final String subtitle;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 44),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Container(
              width: 84,
              height: 84,
              decoration: BoxDecoration(
                borderRadius: BorderRadius.circular(VynlRadius.control),
                color: VynlColors.surfaceRaised,
              ),
              child: Icon(icon, size: 34, color: VynlColors.faint),
            ),
            const SizedBox(height: 22),
            Text(
              title,
              style: const TextStyle(
                fontFamily: VynlFonts.display,
                fontSize: 17,
                fontWeight: FontWeight.w800,
                letterSpacing: -0.2,
                color: VynlColors.text,
              ),
            ),
            const SizedBox(height: 8),
            Text(
              subtitle,
              textAlign: TextAlign.center,
              style: const TextStyle(
                color: VynlColors.dim,
                fontSize: 13.5,
                height: 1.55,
              ),
            ),
          ],
        ),
      ),
    );
  }
}
