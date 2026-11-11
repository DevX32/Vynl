import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../models.dart';
import '../services/player_controller.dart';
import '../state/app_state.dart';
import '../theme.dart';
import '../widgets/artwork.dart';
import '../widgets/common.dart';

class BrowsePage extends StatefulWidget {
  const BrowsePage({super.key});

  @override
  State<BrowsePage> createState() => _BrowsePageState();
}

class _BrowsePageState extends State<BrowsePage> {
  final _controller = TextEditingController();

  @override
  void initState() {
    super.initState();
    _controller.text = context.read<AppState>().browseQuery;
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  Future<void> _search(String value) async {
    await context.read<AppState>().browse(value);
  }

  @override
  Widget build(BuildContext context) {
    final app = context.watch<AppState>();
    final player = context.watch<PlayerController>();
    final results = app.browseResults;

    return Column(
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(16, 4, 16, 10),
          child: TextField(
            controller: _controller,
            textInputAction: TextInputAction.search,
            onSubmitted: _search,
            style: const TextStyle(fontSize: 14, color: VynlColors.text),
            decoration: InputDecoration(
              hintText: 'Search the catalogue',
              hintStyle: const TextStyle(
                fontSize: 14,
                color: VynlColors.faint,
              ),
              prefixIcon: const Icon(
                Icons.search_rounded,
                color: VynlColors.faint,
                size: 20,
              ),
              suffixIcon: app.browseSearching
                  ? const Padding(
                      padding: EdgeInsets.all(14),
                      child: SizedBox(
                        width: 16,
                        height: 16,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      ),
                    )
                  : null,
              filled: true,
              fillColor: VynlColors.surface,
              border: OutlineInputBorder(
                borderRadius: BorderRadius.circular(VynlRadius.control),
                borderSide: const BorderSide(color: VynlColors.line),
              ),
              enabledBorder: OutlineInputBorder(
                borderRadius: BorderRadius.circular(VynlRadius.control),
                borderSide: const BorderSide(color: VynlColors.line),
              ),
            ),
          ),
        ),
        Expanded(
          child: app.browseError != null
              ? _Message(
                  title: 'Search failed',
                  body: app.browseError!,
                )
              : results == null
                  ? const _Message(
                      title: 'Browse',
                      body: 'Search for a song, album, or artist and play it '
                          'straight from your desktop.',
                    )
                  : ListView(
                      padding: const EdgeInsets.fromLTRB(4, 0, 4, 30),
                      children: [
                        if (results.songs.isNotEmpty) ...[
                          const SectionHeader(title: 'SONGS'),
                          for (final song in results.songs)
                            TrackTile(
                              track: _playable(app, song),
                              onTap: () => _playSongs(app, player, results.songs),
                            ),
                        ],
                        if (results.albums.isNotEmpty) ...[
                          const SectionHeader(title: 'ALBUMS'),
                          _CardRow(
                            items: results.albums
                                .map(
                                  (a) => _CardData(
                                    id: a.id,
                                    title: a.title,
                                    subtitle: a.year?.toString() ?? a.artist,
                                    coverUrl: a.cover,
                                  ),
                                )
                                .toList(),
                            onOpen: (id) => _openAlbum(context, id),
                          ),
                        ],
                        if (results.playlists.isNotEmpty) ...[
                          const SectionHeader(title: 'PLAYLISTS'),
                          _CardRow(
                            items: results.playlists
                                .map(
                                  (p) => _CardData(
                                    id: p.id,
                                    title: p.title,
                                    subtitle: p.author ?? 'Playlist',
                                    coverUrl: p.cover,
                                  ),
                                )
                                .toList(),
                            onOpen: (id) => _openPlaylist(context, id),
                          ),
                        ],
                        if (results.songs.isEmpty &&
                            results.albums.isEmpty &&
                            results.playlists.isEmpty)
                          const _Message(
                            title: 'No results',
                            body: 'Try a different song, album, or artist.',
                          ),
                      ],
                    ),
        ),
      ],
    );
  }

  CatalogTrack _playable(AppState app, StreamTrack song) {
    final list = app.streamTracks([song]);
    return list.isEmpty
        ? CatalogTrack(
            id: 'ytm:${song.id}',
            title: song.title,
            artist: song.artist,
            album: song.album ?? '',
            duration: song.duration ?? 0,
            ext: 'webm',
            hasCover: song.cover != null,
            coverUrl: song.cover,
          )
        : list.first;
  }

  Future<void> _playSongs(
    AppState app,
    PlayerController player,
    List<StreamTrack> songs,
  ) async {
    final list = await app.playableStreamTracks(songs);
    if (list.isEmpty) return;
    await player.playTracks(list);
  }

  Future<void> _openAlbum(BuildContext context, String id) async {
    final app = context.read<AppState>();
    try {
      final detail = await app.openAlbum(id);
      if (!context.mounted) return;
      await showModalBottomSheet<void>(
        context: context,
        backgroundColor: VynlColors.surface,
        isScrollControlled: true,
        builder: (_) => _CatalogueSheet(
          title: detail.album.title,
          subtitle: detail.album.artist,
          coverUrl: detail.album.cover,
          tracks: detail.tracks,
        ),
      );
    } catch (e) {
      if (!context.mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Could not open that album: $e')),
      );
    }
  }

  Future<void> _openPlaylist(BuildContext context, String id) async {
    final app = context.read<AppState>();
    try {
      final detail = await app.openPlaylist(id);
      if (!context.mounted) return;
      await showModalBottomSheet<void>(
        context: context,
        backgroundColor: VynlColors.surface,
        isScrollControlled: true,
        builder: (_) => _CatalogueSheet(
          title: detail.playlist.title,
          subtitle: detail.playlist.author ?? 'Playlist',
          coverUrl: detail.playlist.cover,
          tracks: detail.tracks,
        ),
      );
    } catch (e) {
      if (!context.mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Could not open that playlist: $e')),
      );
    }
  }
}

class _CatalogueSheet extends StatefulWidget {
  const _CatalogueSheet({
    required this.title,
    required this.subtitle,
    this.coverUrl,
    required this.tracks,
  });

  final String title;
  final String subtitle;
  final String? coverUrl;
  final List<StreamTrack> tracks;

  @override
  State<_CatalogueSheet> createState() => _CatalogueSheetState();
}

class _CatalogueSheetState extends State<_CatalogueSheet> {
  Future<void> _playAll(
    AppState app,
    PlayerController player,
    int startIndex,
  ) async {
    final list = await app.playableStreamTracks(widget.tracks);
    if (list.isEmpty) return;
    await player.playTracks(list, startIndex: startIndex);
  }

  @override
  Widget build(BuildContext context) {
    final app = context.read<AppState>();
    final player = context.watch<PlayerController>();
    final playable = app.streamTracks(widget.tracks);

    return DraggableScrollableSheet(
      expand: false,
      initialChildSize: 0.7,
      maxChildSize: 0.92,
      builder: (context, controller) => ListView(
        controller: controller,
        padding: const EdgeInsets.fromLTRB(16, 18, 16, 30),
        children: [
          Row(
            children: [
              ArtTile(
                coverPath: null,
                coverUrl: widget.coverUrl,
                size: 84,
                radius: VynlRadius.art,
              ),
              const SizedBox(width: 14),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      widget.title,
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                      style: const TextStyle(
                        fontSize: 20,
                        fontWeight: FontWeight.w700,
                        color: VynlColors.text,
                      ),
                    ),
                    const SizedBox(height: 4),
                    Text(
                      widget.subtitle,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: const TextStyle(
                        fontSize: 13,
                        color: VynlColors.dim,
                      ),
                    ),
                  ],
                ),
              ),
            ],
          ),
          const SizedBox(height: 18),
          for (var i = 0; i < playable.length; i++)
            TrackTile(
              track: playable[i],
              onTap: () => _playAll(app, player, i),
            ),
          if (playable.isEmpty)
            const Padding(
              padding: EdgeInsets.symmetric(vertical: 24),
              child: Text(
                'Nothing to play here.',
                style: TextStyle(color: VynlColors.faint, fontSize: 13),
              ),
            ),
        ],
      ),
    );
  }
}

class _CardData {
  const _CardData({
    required this.id,
    required this.title,
    required this.subtitle,
    this.coverUrl,
  });

  final String id;
  final String title;
  final String subtitle;
  final String? coverUrl;
}

class _CardRow extends StatelessWidget {
  const _CardRow({required this.items, required this.onOpen});

  final List<_CardData> items;
  final ValueChanged<String> onOpen;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      height: 148,
      child: ListView.separated(
        scrollDirection: Axis.horizontal,
        padding: const EdgeInsets.symmetric(horizontal: 12),
        itemCount: items.length,
        separatorBuilder: (_, __) => const SizedBox(width: 10),
        itemBuilder: (context, index) {
          final item = items[index];
          return GestureDetector(
            onTap: () => onOpen(item.id),
            child: SizedBox(
              width: 118,
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  ArtTile(
                    coverPath: null,
                    coverUrl: item.coverUrl,
                    size: 118,
                    radius: VynlRadius.art,
                  ),
                  const SizedBox(height: 8),
                  Text(
                    item.title,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: const TextStyle(
                      fontSize: 12.5,
                      fontWeight: FontWeight.w600,
                      color: VynlColors.text,
                    ),
                  ),
                  const SizedBox(height: 2),
                  Text(
                    item.subtitle,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: const TextStyle(fontSize: 11.5, color: VynlColors.faint),
                  ),
                ],
              ),
            ),
          );
        },
      ),
    );
  }
}

class _Message extends StatelessWidget {
  const _Message({required this.title, required this.body});

  final String title;
  final String body;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 44),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Icon(
              Icons.travel_explore_rounded,
              size: 44,
              color: VynlColors.faint,
            ),
            const SizedBox(height: 16),
            Text(
              title,
              style: const TextStyle(
                color: VynlColors.dim,
                fontSize: 14,
                fontWeight: FontWeight.w600,
              ),
            ),
            const SizedBox(height: 6),
            Text(
              body,
              textAlign: TextAlign.center,
              style: const TextStyle(
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
}
