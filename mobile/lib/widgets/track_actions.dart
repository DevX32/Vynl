import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../models.dart';
import '../services/player_controller.dart';
import '../state/app_state.dart';
import '../theme.dart';
import 'artwork.dart';

enum _TrackAction { remove, download }

Future<void> showTrackActions(BuildContext context, CatalogTrack track) async {
  final app = context.read<AppState>();
  final player = context.read<PlayerController>();
  final downloaded = track.isDownloaded;

  final action = await showModalBottomSheet<_TrackAction>(
    context: context,
    backgroundColor: VynlColors.surface,
    shape: const RoundedRectangleBorder(
      borderRadius: BorderRadius.vertical(
        top: Radius.circular(VynlRadius.hero),
      ),
    ),
    builder: (context) {
      return SafeArea(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Padding(
              padding: const EdgeInsets.fromLTRB(20, 18, 20, 12),
              child: Row(
                children: [
                  ArtTile(
                    coverPath: track.localCoverPath,
                    size: 48,
                    accent: VynlColors.accent,
                  ),
                  const SizedBox(width: 14),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Text(
                          track.title,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: const TextStyle(
                            color: VynlColors.text,
                            fontWeight: FontWeight.w700,
                            fontSize: 15,
                          ),
                        ),
                        const SizedBox(height: 3),
                        Text(
                          track.artist,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: const TextStyle(
                            color: VynlColors.dim,
                            fontSize: 12.5,
                          ),
                        ),
                      ],
                    ),
                  ),
                ],
              ),
            ),
            if (downloaded)
              ListTile(
                leading: const Icon(
                  Icons.delete_outline_rounded,
                  color: VynlColors.danger,
                ),
                title: const Text(
                  'Remove from this device',
                  style: TextStyle(
                    color: VynlColors.danger,
                    fontWeight: FontWeight.w600,
                    fontSize: 14.5,
                  ),
                ),
                subtitle: const Text(
                  'Keeps it in your desktop library',
                  style: TextStyle(color: VynlColors.faint, fontSize: 12.5),
                ),
                onTap: () => Navigator.pop(context, _TrackAction.remove),
              )
            else
              ListTile(
                enabled: app.isPaired,
                leading: Icon(
                  Icons.download_rounded,
                  color: app.isPaired ? VynlColors.accent : VynlColors.faint,
                ),
                title: Text(
                  'Download to this device',
                  style: TextStyle(
                    color: app.isPaired ? VynlColors.accent : VynlColors.faint,
                    fontWeight: FontWeight.w600,
                    fontSize: 14.5,
                  ),
                ),
                subtitle: Text(
                  app.isPaired
                      ? 'Pull it back down over Wi-Fi'
                      : 'Pair with your desktop first',
                  style: const TextStyle(
                    color: VynlColors.faint,
                    fontSize: 12.5,
                  ),
                ),
                onTap: app.isPaired
                    ? () => Navigator.pop(context, _TrackAction.download)
                    : null,
              ),
            const SizedBox(height: 8),
          ],
        ),
      );
    },
  );

  if (action == null || !context.mounted) return;

  if (action == _TrackAction.remove) {
    final ok = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Remove from device?'),
        content: Text(
          '"${track.title}" will be deleted from this phone and hidden from '
          'your library here. It stays in your desktop library.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: const Text('Cancel'),
          ),
          TextButton(
            onPressed: () => Navigator.pop(context, true),
            child: const Text(
              'Remove',
              style: TextStyle(color: VynlColors.danger),
            ),
          ),
        ],
      ),
    );
    if (ok != true || !context.mounted) return;

    await player.removeTrack(track.id);
    await app.removeTrackFromDevice(track);
    if (context.mounted) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Removed "${track.title}" from this device')),
      );
    }
    return;
  }

  final messenger = ScaffoldMessenger.of(context);
  messenger.showSnackBar(
    SnackBar(content: Text('Downloading "${track.title}"…')),
  );
  try {
    await app.downloadTrackToDevice(track);
    messenger.showSnackBar(
      SnackBar(content: Text('Downloaded "${track.title}"')),
    );
  } catch (e) {
    messenger.showSnackBar(
      SnackBar(content: Text('Couldn’t download: $e')),
    );
  }
}
