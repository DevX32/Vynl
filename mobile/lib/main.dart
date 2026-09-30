import 'dart:async';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:sqflite_common_ffi/sqflite_ffi.dart';

import 'screens/home_shell.dart';
import 'services/auth_store.dart';
import 'services/library_db.dart';
import 'services/notifications.dart';
import 'services/player_controller.dart';
import 'state/app_state.dart';
import 'theme.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();

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

  runApp(
    MultiProvider(
      providers: [
        ChangeNotifierProvider.value(value: appState),
        ChangeNotifierProvider.value(value: player),
      ],
      child: const VynlApp(),
    ),
  );

  unawaited(appState.bootstrap());
}

class VynlApp extends StatelessWidget {
  const VynlApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Vynl',
      debugShowCheckedModeBanner: false,
      theme: buildVynlTheme(),
      home: const HomeShell(),
    );
  }
}
