import 'dart:async';
import 'dart:io';
import 'dart:math' as math;
import 'dart:ui' show ImageFilter;

import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../models.dart';
import '../services/player_controller.dart';
import '../state/app_state.dart';
import '../theme.dart';
import '../widgets/artwork.dart';
import '../widgets/common.dart';

const _quotes = <String>[
  'where words fail, music speaks',
  'music is the shorthand of emotion',
  'life is a song. love is the music',
  'without music, life would be a mistake',
  'music is the universal language of mankind',
  'one good thing about music, when it hits you, you feel no pain',
  'music expresses that which cannot be put into words',
];

const _recentLimit = 6;
const _featuredHeight = 152.0;

String _greetingFor(DateTime now) {
  final hour = now.hour;
  if (hour < 6) return 'Good Night';
  if (hour < 12) return 'Good Morning';
  if (hour < 18) return 'Good Afternoon';
  return 'Good Evening';
}

class HomePage extends StatefulWidget {
  const HomePage({super.key});

  @override
  State<HomePage> createState() => _HomePageState();
}

class _HomePageState extends State<HomePage> {
  Timer? _midnight;

  @override
  void initState() {
    super.initState();
    _scheduleMidnight();
  }

  @override
  void dispose() {
    _midnight?.cancel();
    super.dispose();
  }

  void _scheduleMidnight() {
    final now = DateTime.now();
    final next = DateTime(now.year, now.month, now.day + 1);
    _midnight = Timer(next.difference(now), () {
      if (!mounted) return;
      setState(() {});
      _scheduleMidnight();
    });
  }

  void _play(CatalogTrack track, List<CatalogTrack> pool) {
    context.read<PlayerController>().playTrack(track, context: pool);
  }

  void _shuffleAll(List<CatalogTrack> pool) {
    context.read<PlayerController>().playTracks([...pool]..shuffle());
  }

  Future<void> _sync() async {
    final state = context.read<AppState>();
    if (!state.isPaired) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(
          content: Text('Pair with your desktop under Settings first.'),
        ),
      );
      return;
    }
    await state.runSync();
  }

  void _openLibrary() => context.read<AppState>().setTab(AppState.tabLibrary);

  static List<CatalogTrack> _recent(List<CatalogTrack> pool) {
    final sorted = [...pool]
      ..sort((a, b) {
        final byTime = (b.mtime ?? 0).compareTo(a.mtime ?? 0);
        return byTime != 0 ? byTime : b.id.compareTo(a.id);
      });
    return sorted.take(_recentLimit).toList();
  }

  static CatalogTrack _featured(
    List<CatalogTrack> pool,
    CatalogTrack? current,
    DateTime now,
  ) {
    if (current != null) {
      for (final t in pool) {
        if (t.id == current.id) return t;
      }
    }
    return pool[math.Random(now.day * 31 + now.month).nextInt(pool.length)];
  }

  @override
  Widget build(BuildContext context) {
    final app = context.watch<AppState>();
    final player = context.watch<PlayerController>();

    final now = DateTime.now();
    final pool = app.downloadedTracks;
    final recent = _recent(pool);
    final featured =
        pool.isEmpty ? null : _featured(pool, player.current, now);
    final header = _Header(
      greeting: _greetingFor(now),
      quote: _quotes[(now.day - 1) % _quotes.length],
      name: app.displayName,
    );

    return RefreshIndicator(
      onRefresh: () async {
        final state = context.read<AppState>();
        if (state.isPaired && !state.syncing) await state.runSync();
      },
      color: VynlColors.accent,
      backgroundColor: VynlColors.surface,
      displacement: 24,
      child: featured == null
          ? Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Padding(
                  padding: const EdgeInsets.fromLTRB(20, 4, 20, 0),
                  child: header,
                ),
                Expanded(child: _emptyView(app)),
              ],
            )
          : ListView(
              padding: const EdgeInsets.fromLTRB(20, 4, 20, 28),
              physics: const AlwaysScrollableScrollPhysics(),
              children: [
                header,
                const SizedBox(height: 18),
                _FeaturedCard(
                  track: featured,
                  onTap: () => _play(featured, pool),
                ),
                const SizedBox(height: 18),
                Wrap(
                  spacing: 8,
                  runSpacing: 8,
                  children: [
                    _ActionPill(
                      icon: Icons.shuffle_rounded,
                      label: 'Shuffle All',
                      onTap: () => _shuffleAll(pool),
                    ),
                    _ActionPill(
                      icon: app.syncing
                          ? Icons.sync_rounded
                          : Icons.cloud_sync_outlined,
                      label: app.syncing ? 'Syncing' : 'Sync',
                      onTap: app.syncing ? null : _sync,
                    ),
                    _ActionPill(
                      icon: Icons.library_music_outlined,
                      label: 'Library',
                      onTap: _openLibrary,
                    ),
                  ],
                ),
                if (recent.isNotEmpty) ...[
                  SectionHeader(
                    title: 'RECENTLY ADDED',
                    padding: const EdgeInsets.fromLTRB(0, 24, 0, 10),
                    trailing: TextButton(
                      onPressed: _openLibrary,
                      style: TextButton.styleFrom(
                        foregroundColor: VynlColors.faint,
                        padding: const EdgeInsets.symmetric(horizontal: 8),
                        minimumSize: const Size(0, 30),
                      ),
                      child: const Text(
                        'View all',
                        style: TextStyle(fontSize: 11.5),
                      ),
                    ),
                  ),
                  _RecentGrid(
                    tracks: recent,
                    activeId: player.current?.id,
                    onTap: (track) => _play(track, pool),
                  ),
                ],
              ],
            ),
    );
  }

  Widget _emptyView(AppState app) {
    final syncedNothing = app.tracks.isEmpty;
    return VynlEmptyState(
      icon: syncedNothing
          ? Icons.music_off_rounded
          : Icons.cloud_download_outlined,
      title: syncedNothing ? 'Nothing synced yet' : 'Nothing downloaded yet',
      subtitle: syncedNothing
          ? 'Pair with your desktop under Settings, then pull down to sync.'
          : 'Download a few tracks on Wi‑Fi so they can play offline.',
      action: _ActionPill(
        icon: Icons.cloud_sync_outlined,
        label: 'Sync now',
        onTap: app.syncing ? null : _sync,
      ),
    );
  }
}

class _Header extends StatelessWidget {
  const _Header({required this.greeting, required this.quote, this.name});

  final String greeting;
  final String quote;
  final String? name;

  @override
  Widget build(BuildContext context) {
    final displayName = name?.trim() ?? '';

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        Text.rich(
          TextSpan(
            children: [
              TextSpan(text: greeting),
              if (displayName.isNotEmpty) ...[
                const TextSpan(text: ', '),
                TextSpan(
                  text: displayName,
                  style: TextStyle(
                    fontFamily: VynlFonts.serif,
                    fontStyle: FontStyle.italic,
                    fontWeight: FontWeight.w700,
                    letterSpacing: 0,
                    color: VynlColors.accent,
                  ),
                ),
              ],
            ],
          ),
          style: const TextStyle(
            fontFamily: VynlFonts.display,
            fontSize: 27,
            fontWeight: FontWeight.w600,
            letterSpacing: -0.6,
            height: 1.2,
            color: VynlColors.text,
          ),
        ),
        const SizedBox(height: 8),
        Container(
          padding: const EdgeInsets.only(left: 10),
          decoration: const BoxDecoration(
            border: Border(left: BorderSide(color: VynlColors.lineStrong, width: 2)),
          ),
          child: Text(
            quote,
            style: const TextStyle(
              fontSize: 12.5,
              fontStyle: FontStyle.italic,
              height: 1.35,
              color: VynlColors.dim,
            ),
          ),
        ),
      ],
    );
  }
}

/// Mirrors the desktop card's `saturate(1.25) brightness(0.9)` filter so the
/// artwork stays recognisable instead of turning into a flat smear.
ColorFilter _coverFilter({
  double saturation = 1.25,
  double brightness = 0.9,
}) {
  // Luminance-preserving Rec.709 saturation, then a brightness scale.
  final sr = (1 - saturation) * 0.2126;
  final sg = (1 - saturation) * 0.7152;
  final sb = (1 - saturation) * 0.0722;
  return ColorFilter.matrix(<double>[
    (saturation + sr) * brightness, sr * brightness, sr * brightness, 0, 0,
    sg * brightness, (saturation + sg) * brightness, sg * brightness, 0, 0,
    sb * brightness, sb * brightness, (saturation + sb) * brightness, 0, 0,
    0, 0, 0, 1, 0,
  ]);
}

const _featuredScrim = LinearGradient(
  begin: Alignment.topCenter,
  end: Alignment.bottomCenter,
  colors: [
    Color(0x0008080C),
    Color(0x4708080C),
    Color(0xB808080C),
    Color(0xE608080C),
  ],
  stops: [0.0, 0.55, 0.82, 1.0],
);

class _FeaturedCard extends StatelessWidget {
  const _FeaturedCard({required this.track, required this.onTap});

  final CatalogTrack track;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return Material(
      color: VynlColors.surfaceRaised,
      borderRadius: BorderRadius.circular(VynlRadius.card),
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: onTap,
        child: SizedBox(
          height: _featuredHeight,
          child: Stack(
            fit: StackFit.expand,
            children: [
              _cover(),
              const DecoratedBox(
                decoration: BoxDecoration(gradient: _featuredScrim),
              ),
              Padding(
                padding: const EdgeInsets.fromLTRB(18, 16, 82, 16),
                child: Column(
                  mainAxisAlignment: MainAxisAlignment.end,
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    const Text(
                      'Jump back in',
                      style: TextStyle(
                        fontSize: 11.5,
                        color: Color(0xBFF0F0EC),
                      ),
                    ),
                    const SizedBox(height: 5),
                    Text(
                      track.title,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: const TextStyle(
                        fontSize: 17,
                        fontWeight: FontWeight.w600,
                        color: Colors.white,
                      ),
                    ),
                    const SizedBox(height: 2),
                    Text(
                      track.artist,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: const TextStyle(
                        fontSize: 13,
                        color: Color(0xB3FFFFFF),
                      ),
                    ),
                  ],
                ),
              ),
              Positioned(
                right: 16,
                bottom: 18,
                child: VynlControl(
                  onTap: onTap,
                  style: VynlControlStyle.accent,
                  size: 42,
                  iconSize: 24,
                  icon: Icons.play_arrow_rounded,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  Widget _cover() {
    final path = track.localCoverPath;
    if (path == null || path.isEmpty) {
      return const DecoratedBox(
        decoration: BoxDecoration(
          gradient: LinearGradient(
            begin: Alignment.topCenter,
            end: Alignment.bottomCenter,
            colors: [Color(0x3DB9A7FF), Color(0xFF0E0E10)],
          ),
        ),
      );
    }

    return LayoutBuilder(
      builder: (context, constraints) {
        final ratio = MediaQuery.devicePixelRatioOf(context);
        final width = constraints.maxWidth.isFinite ? constraints.maxWidth : 600.0;

        return ImageFiltered(
          imageFilter: ImageFilter.blur(sigmaX: 2, sigmaY: 2),
          child: ColorFiltered(
            colorFilter: _coverFilter(),
            child: Image.file(
              File(path),
              fit: BoxFit.cover,
              gaplessPlayback: true,
              cacheWidth: (width * ratio).round(),
              errorBuilder: (_, __, ___) => const ColoredBox(
                color: VynlColors.surfaceRaised,
              ),
            ),
          ),
        );
      },
    );
  }
}

class _ActionPill extends StatelessWidget {
  const _ActionPill({
    required this.icon,
    required this.label,
    required this.onTap,
  });

  final IconData icon;
  final String label;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final enabled = onTap != null;
    final shape = RoundedRectangleBorder(
      borderRadius: BorderRadius.circular(VynlRadius.control),
      side: const BorderSide(color: VynlColors.line),
    );

    return Opacity(
      opacity: enabled ? 1 : 0.45,
      child: Material(
        color: VynlColors.surfaceRaised,
        shape: shape,
        clipBehavior: Clip.antiAlias,
        child: InkWell(
          onTap: onTap,
          customBorder: shape,
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 9),
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Icon(icon, size: 16, color: VynlColors.dim),
                const SizedBox(width: 8),
                Text(
                  label,
                  style: const TextStyle(fontSize: 12, color: VynlColors.dim),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _RecentGrid extends StatelessWidget {
  const _RecentGrid({
    required this.tracks,
    required this.activeId,
    required this.onTap,
  });

  final List<CatalogTrack> tracks;
  final String? activeId;
  final ValueChanged<CatalogTrack> onTap;

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        const gap = 12.0;
        final columns =
            (constraints.maxWidth / 148).floor().clamp(2, 4).toInt();
        final cardWidth =
            (constraints.maxWidth - gap * (columns - 1)) / columns;

        return Wrap(
          spacing: gap,
          runSpacing: 16,
          children: [
            for (final track in tracks)
              SizedBox(
                width: cardWidth,
                child: _RecentCard(
                  track: track,
                  artSize: cardWidth - 8,
                  active: track.id == activeId,
                  onTap: () => onTap(track),
                ),
              ),
          ],
        );
      },
    );
  }
}

class _RecentCard extends StatelessWidget {
  const _RecentCard({
    required this.track,
    required this.artSize,
    required this.active,
    required this.onTap,
  });

  final CatalogTrack track;
  final double artSize;
  final bool active;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final accent = Theme.of(context).colorScheme.primary;

    return Material(
      color: Colors.transparent,
      borderRadius: BorderRadius.circular(VynlRadius.card),
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.all(4),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: [
              Stack(
                clipBehavior: Clip.none,
                children: [
                  Stack(
                    children: [
                      ArtTile(
                        coverPath: track.localCoverPath,
                        size: artSize,
                        accent: accent,
                      ),
                      if (active)
                        Positioned.fill(
                          child: DecoratedBox(
                            decoration: BoxDecoration(
                              borderRadius:
                                  BorderRadius.circular(VynlRadius.thumb),
                              border: Border.all(
                                color: accent.withValues(alpha: 0.55),
                                width: 1.5,
                              ),
                            ),
                          ),
                        ),
                    ],
                  ),
                  Positioned(
                    right: 4,
                    bottom: 4,
                    child: VynlControl(
                      onTap: onTap,
                      style: VynlControlStyle.accent,
                      size: 30,
                      iconSize: 19,
                      icon: Icons.play_arrow_rounded,
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 8),
              Text(
                track.title,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: TextStyle(
                  color: active ? accent : VynlColors.text,
                  fontSize: 12.5,
                  fontWeight: FontWeight.w700,
                  letterSpacing: -0.1,
                ),
              ),
              const SizedBox(height: 2),
              Text(
                track.artist,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: const TextStyle(color: VynlColors.faint, fontSize: 11),
              ),
            ],
          ),
        ),
      ),
    );
  }
}