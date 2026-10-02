import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../state/app_state.dart';
import '../theme.dart';
import '../widgets/mini_player.dart';
import '../widgets/splash.dart';
import 'library_page.dart';
import 'playlists_page.dart';
import 'settings_page.dart';

class HomeShell extends StatefulWidget {
  const HomeShell({super.key});

  @override
  State<HomeShell> createState() => _HomeShellState();
}

class _HomeShellState extends State<HomeShell> with SingleTickerProviderStateMixin {
  static const _titles = ['Library', 'Playlists'];

  bool _exiting = false;
  bool _showSplash = true;

  late final AnimationController _reveal = AnimationController(
    vsync: this,
    duration: VynlMotion.splashExit,
  );

  late final CurvedAnimation _revealFade = CurvedAnimation(
    parent: _reveal,
    curve: const Cubic(0.4, 0, 0.2, 1),
  );

  late final Animation<double> _revealScale =
      Tween<double>(begin: 0.985, end: 1).animate(_revealFade);

  @override
  void initState() {
    super.initState();
    context.read<AppState>().addListener(_onState);
  }

  @override
  void dispose() {
    context.read<AppState>().removeListener(_onState);
    _reveal.dispose();
    _revealFade.dispose();
    super.dispose();
  }

  void _onState() {
    if (!_showSplash || _exiting || context.read<AppState>().loading) return;
    setState(() => _exiting = true);
  }

  void _openSettings() {
    Navigator.of(context).push(
      MaterialPageRoute<void>(builder: (_) => const _SettingsShell()),
    );
  }

  @override
  Widget build(BuildContext context) {
    final app = context.watch<AppState>();

    if (_showSplash) {
      return SplashScreen(
        exiting: _exiting,
        onExited: () {
          if (!mounted) return;
          setState(() => _showSplash = false);
          _reveal.forward();
        },
      );
    }

    const pages = [
      LibraryPage(),
      PlaylistsPage(),
    ];

    return FadeTransition(
      opacity: _revealFade,
      child: ScaleTransition(
        scale: _revealScale,
        child: Container(
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
                  selectedIndex:
                      app.tabIndex.clamp(0, _titles.length - 1),
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
