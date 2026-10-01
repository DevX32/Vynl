import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:just_audio_background/just_audio_background.dart';

import 'theme.dart';

Future<void> configurePlatform() async {
  if (kIsWeb || !Platform.isAndroid) return;
  await SystemChrome.setEnabledSystemUIMode(SystemUiMode.edgeToEdge);
  SystemChrome.setSystemUIOverlayStyle(kVynlOverlayStyle);
  await JustAudioBackground.init(
    androidNotificationChannelId: 'com.vynl.vynl_mobile.channel.playback',
    androidNotificationChannelName: 'Now playing',
    androidNotificationChannelDescription: 'Playback controls',
    androidNotificationOngoing: true,
    androidStopForegroundOnPause: true,
  );
}