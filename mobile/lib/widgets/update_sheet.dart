import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../services/app_update.dart';
import '../theme.dart';

class UpdateSheet extends StatelessWidget {
  const UpdateSheet({super.key});

  @override
  Widget build(BuildContext context) {
    final app = context.watch<AppUpdate>();
    final release = app.release;
    if (release == null) return const SizedBox.shrink();

    final abi = abiOfDevice();

    return Container(
      decoration: const BoxDecoration(
        color: VynlColors.surface,
        border: Border(top: BorderSide(color: VynlColors.line)),
        borderRadius: BorderRadius.vertical(
          top: Radius.circular(VynlRadius.hero),
        ),
      ),
      child: SafeArea(
        top: false,
        child: Padding(
          padding: const EdgeInsets.fromLTRB(20, 18, 20, 18),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                'UPDATE AVAILABLE',
                style: TextStyle(
                  color: VynlColors.accent,
                  fontSize: 11,
                  fontWeight: FontWeight.w800,
                  letterSpacing: 1.3,
                ),
              ),
              const SizedBox(height: 10),
              Text(
                'Vynl ${release.version}',
                style: const TextStyle(
                  fontFamily: VynlFonts.display,
                  fontSize: 21,
                  fontWeight: FontWeight.w800,
                  letterSpacing: -0.4,
                  color: VynlColors.text,
                ),
              ),
              const SizedBox(height: 4),
              Text(
                'Installed ${app.currentVersion}  ·  $abi',
                style: const TextStyle(
                  color: VynlColors.faint,
                  fontSize: 12.5,
                  fontFeatures: [FontFeature.tabularFigures()],
                ),
              ),
              if (release.notes.trim().isNotEmpty) ...[
                const SizedBox(height: 14),
                ConstrainedBox(
                  constraints: const BoxConstraints(maxHeight: 180),
                  child: SingleChildScrollView(
                    child: Text(
                      release.notes.trim(),
                      style: const TextStyle(
                        color: VynlColors.dim,
                        fontSize: 12.5,
                        height: 1.5,
                      ),
                    ),
                  ),
                ),
              ],
              const SizedBox(height: 18),
              if (app.downloading)
                Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    ClipRRect(
                      borderRadius: BorderRadius.circular(VynlRadius.control),
                      child: LinearProgressIndicator(
                        value: app.progress > 0 ? app.progress : null,
                        minHeight: 4,
                      ),
                    ),
                    const SizedBox(height: 8),
                    Text(
                      app.progress > 0
                          ? '${(app.progress * 100).round()}%  ·  downloading'
                          : 'Downloading…',
                      style: const TextStyle(
                        color: VynlColors.faint,
                        fontSize: 12,
                        fontFeatures: [FontFeature.tabularFigures()],
                      ),
                    ),
                  ],
                )
              else if (app.readyToInstall)
                FilledButton(
                  onPressed: app.install,
                  child: const Text('Install update'),
                )
              else
                FilledButton(
                  onPressed: () => app.download(abi),
                  child: const Text('Download'),
                ),
              if (app.error != null) ...[
                const SizedBox(height: 10),
                Text(
                  app.error!,
                  style: const TextStyle(color: VynlColors.danger, fontSize: 12.5),
                ),
              ],
              const SizedBox(height: 10),
              Center(
                child: TextButton(
                  onPressed: () async {
                    await app.dismiss();
                    if (context.mounted) Navigator.of(context).pop();
                  },
                  child: const Text('Later'),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}