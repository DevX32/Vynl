import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../state/app_state.dart';
import '../theme.dart';
import '../widgets/mini_player.dart';
import 'library_page.dart';
import 'playlists_page.dart';
import 'settings_page.dart';

class HomeShell extends StatefulWidget {
  const HomeShell({super.key});

  @override
  State<HomeShell> createState() => _HomeShellState();
}

class _HomeShellState extends State<HomeShell> {
  int _tab = 0;

  static const _titles = ['Library', 'Playlists', 'Settings'];

  @override
  Widget build(BuildContext context) {
    final app = context.watch<AppState>();

    if (app.loading) {
      return const Scaffold(
        backgroundColor: VynlColors.bg,
        body: Center(
          child: SizedBox(
            width: 26,
            height: 26,
            child: CircularProgressIndicator(
              strokeWidth: 2.4,
              color: VynlColors.accent,
            ),
          ),
        ),
      );
    }

    const pages = [
      LibraryPage(),
      PlaylistsPage(),
      SettingsPage(),
    ];

    return Container(
      decoration: const BoxDecoration(gradient: kVynlBackground),
      child: Scaffold(
        backgroundColor: Colors.transparent,
        appBar: AppBar(
          title: Text(_titles[_tab]),
          actions: [
            if (app.syncing)
              Padding(
                padding: const EdgeInsets.only(right: 18),
                child: Center(
                  child: SizedBox(
                    width: 17,
                    height: 17,
                    child: CircularProgressIndicator(
                      strokeWidth: 2.2,
                      color: VynlColors.accent,
                    ),
                  ),
                ),
              )
            else if (app.isPaired)
              IconButton(
                tooltip: 'Sync library',
                onPressed: () => app.runSync(),
                icon: const Icon(Icons.sync_rounded, size: 22),
              ),
          ],
        ),
        body: IndexedStack(index: _tab, children: pages),
        bottomNavigationBar: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const MiniPlayer(),
            NavigationBar(
              selectedIndex: _tab,
              onDestinationSelected: (i) => setState(() => _tab = i),
              destinations: const [
                NavigationDestination(
                  icon: Icon(Icons.music_note_outlined),
                  selectedIcon: Icon(Icons.music_note_rounded),
                  label: 'Library',
                ),
                NavigationDestination(
                  icon: Icon(Icons.playlist_play_outlined),
                  selectedIcon: Icon(Icons.playlist_play_rounded),
                  label: 'Playlists',
                ),
                NavigationDestination(
                  icon: Icon(Icons.tune_outlined),
                  selectedIcon: Icon(Icons.tune_rounded),
                  label: 'Settings',
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}
