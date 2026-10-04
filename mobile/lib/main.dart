import 'dart:async';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:sqflite_common_ffi/sqflite_ffi.dart';

import 'platform_setup.dart';
import 'screens/home_shell.dart';
import 'services/app_update.dart';
import 'services/auth_store.dart';
import 'services/library_db.dart';
import 'services/notifications.dart';
import 'services/player_controller.dart';
import 'state/app_state.dart';
import 'state/theme_controller.dart';
import 'theme.dart';
import 'widgets/update_sheet.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();

  PaintingBinding.instance.imageCache
    ..maximumSize = 3000
    ..maximumSizeBytes = 300 << 20;

  await configurePlatform();

  if (!kIsWeb && (Platform.isWindows || Platform.isLinux || Platform.isMacOS)) {
    sqfliteFfiInit();
    databaseFactory = databaseFactoryFfi;
  }

  final authStore = AuthStore();
  final db = LibraryDb();
  final notifications = SyncNotifications();
  final appState = AppState(
    authStore: authStore,
    db: db,
    notifications: notifications,
  );
  final player = PlayerController();
  final themeController = ThemeController()..bind(player);
  final appUpdate = AppUpdate();

  runApp(
    MultiProvider(
      providers: [
        ChangeNotifierProvider.value(value: appState),
        ChangeNotifierProvider.value(value: player),
        ChangeNotifierProvider.value(value: themeController),
        ChangeNotifierProvider.value(value: appUpdate),
      ],
      child: const VynlApp(),
    ),
  );

  unawaited(appState.bootstrap());
  unawaited(_promptForUpdate(appUpdate));
}

Future<void> _promptForUpdate(AppUpdate appUpdate) async {
  await Future<void>.delayed(const Duration(seconds: 4));
  await appUpdate.check();
}

final _navigatorKey = GlobalKey<NavigatorState>();

class VynlApp extends StatelessWidget {
  const VynlApp({super.key});

  @override
  Widget build(BuildContext context) {
    return Consumer<ThemeController>(
      builder: (context, theme, _) {
        return MaterialApp(
          title: 'Vynl',
          debugShowCheckedModeBanner: false,
          navigatorKey: _navigatorKey,
          theme: buildVynlTheme(),
          builder: (context, child) => _UpdatePrompt(
            child: child ?? const SizedBox.shrink(),
          ),
          home: const HomeShell(),
        );
      },
    );
  }
}

class _UpdatePrompt extends StatefulWidget {
  const _UpdatePrompt({required this.child});

  final Widget child;

  @override
  State<_UpdatePrompt> createState() => _UpdatePromptState();
}

class _UpdatePromptState extends State<_UpdatePrompt> {
  bool _shown = false;

  @override
  Widget build(BuildContext context) {
    return Consumer<AppUpdate>(
      builder: (context, app, child) {
        if (app.available && !_shown) {
          _shown = true;
          WidgetsBinding.instance.addPostFrameCallback((_) {
            if (!mounted) return;
            final ctx = _navigatorKey.currentContext;
            if (ctx == null) return;
            showModalBottomSheet<void>(
              context: ctx,
              backgroundColor: Colors.transparent,
              isScrollControlled: true,
              builder: (_) => const UpdateSheet(),
            );
          });
        }
        return child ?? const SizedBox.shrink();
      },
      child: widget.child,
    );
  }
}
