import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:just_audio_background/just_audio_background.dart';

import 'theme.dart';

const MethodChannel _installChannel = MethodChannel('vynl/install');

Future<bool> canRequestPackageInstalls() async {
  if (kIsWeb || !Platform.isAndroid) return true;
  try {
    final allowed = await _installChannel.invokeMethod<bool>(
      'canRequestInstalls',
    );
    return allowed ?? true;
  } catch (e) {
    debugPrint('install permission check failed: $e');
    return true;
  }
}

Future<void> configurePlatform() async {
  if (kIsWeb || !Platform.isAndroid) return;
  await SystemChrome.setEnabledSystemUIMode(SystemUiMode.edgeToEdge);
  SystemChrome.setSystemUIOverlayStyle(kVynlOverlayStyle);
  await JustAudioBackground.init(
    androidNotificationChannelId: 'com.vynl.vynl_mobile.channel.playback',
    androidNotificationChannelName: 'Now playing',
    androidNotificationChannelDescription: 'Playback controls',
    androidNotificationOngoing: true,
    androidStopForegroundOnPause: false,
  );
}