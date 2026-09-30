import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter_local_notifications/flutter_local_notifications.dart';

class SyncNotifications {
  SyncNotifications();

  final FlutterLocalNotificationsPlugin _plugin =
      FlutterLocalNotificationsPlugin();

  static const _channelId = 'vynl_sync';
  static const _channelName = 'Library sync';
  static const _notifId = 17865;

  bool _available = false;

  bool get isAvailable => _available;

  Future<void> init() async {
    if (kIsWeb || Platform.isWindows) return;
    const android = AndroidInitializationSettings('@mipmap/ic_launcher');
    try {
      await _plugin.initialize(
        const InitializationSettings(android: android),
      );
    } catch (e) {
      debugPrint('notifications unavailable: $e');
      return;
    }
    final androidPlugin = _plugin
        .resolvePlatformSpecificImplementation<
            AndroidFlutterLocalNotificationsPlugin>();
    await androidPlugin?.requestNotificationsPermission();
    await androidPlugin?.createNotificationChannel(
      const AndroidNotificationChannel(
        _channelId,
        _channelName,
        description: 'Progress while downloading your Vynl library',
        importance: Importance.low,
      ),
    );
    _available = true;
  }

  Future<void> showProgress({
    required int done,
    required int total,
    String? detail,
  }) async {
    if (!_available) return;
    final android = AndroidNotificationDetails(
      _channelId,
      _channelName,
      channelDescription: 'Progress while downloading your Vynl library',
      importance: Importance.low,
      priority: Priority.low,
      onlyAlertOnce: true,
      showProgress: true,
      maxProgress: total <= 0 ? 1 : total,
      progress: done.clamp(0, total <= 0 ? 1 : total),
      ongoing: done < total,
    );
    await _plugin.show(
      _notifId,
      'Syncing library',
      detail ?? '$done / $total tracks',
      NotificationDetails(android: android),
    );
  }

  Future<void> showDone({required int count, String? error}) async {
    if (!_available) return;
    final android = AndroidNotificationDetails(
      _channelId,
      _channelName,
      importance: Importance.defaultImportance,
      priority: Priority.defaultPriority,
    );
    await _plugin.show(
      _notifId,
      error == null ? 'Sync complete' : 'Sync finished with errors',
      error ?? '$count tracks ready offline',
      NotificationDetails(android: android),
    );
  }

  Future<void> cancel() async {
    if (!_available) return;
    await _plugin.cancel(_notifId);
  }
}
