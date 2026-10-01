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
  static const _titles = ['Library', 'Playlists'];

  void _openSettings() {
    Navigator.of(context).push(
      MaterialPageRoute<void>(builder: (_) => const _SettingsShell()),
    );
  }

  @override
  Widget build(BuildContext context) {
    final app = context.watch<AppState>();

    if (app.loading) {
      return Scaffold(
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
    ];

    return Container(
      decoration: kVynlBackground,
      child: Scaffold(
        backgroundColor: Colors.transparent,
        appBar: AppBar(
          title: Text(_titles[app.tabIndex.clamp(0, _titles.length - 1)]),
          actions: [
            IconButton(
              tooltip: 'Settings',
              onPressed: _openSettings,
              icon: const Icon(Icons.tune_rounded, size: 22),
            ),
          ],
        ),
        body: IndexedStack(index: app.tabIndex, children: pages),
        bottomNavigationBar: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const MiniPlayer(),
            NavigationBar(
              selectedIndex: app.tabIndex.clamp(0, _titles.length - 1),
              onDestinationSelected: app.setTab,
              destinations: const [
                NavigationDestination(
                  icon: Icon(Icons.library_music_outlined),
                  selectedIcon: Icon(Icons.library_music_rounded),
                  label: 'Library',
                ),
                NavigationDestination(
                  icon: Icon(Icons.queue_music_outlined),
                  selectedIcon: Icon(Icons.queue_music_rounded),
                  label: 'Playlists',
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

class _SettingsShell extends StatelessWidget {
  const _SettingsShell();

  @override
  Widget build(BuildContext context) {
    return Container(
      decoration: kVynlBackground,
      child: Scaffold(
        backgroundColor: Colors.transparent,
        appBar: AppBar(title: const Text('Settings')),
        body: const SafeArea(top: false, child: SettingsPage()),
      ),
    );
  }
}
