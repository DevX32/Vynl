import 'package:flutter/material.dart';
import 'package:package_info_plus/package_info_plus.dart';
import 'package:provider/provider.dart';
import 'package:url_launcher/url_launcher.dart';

import '../services/app_update.dart';
import '../state/app_state.dart';
import '../state/theme_controller.dart';
import '../theme.dart';
import '../widgets/accent_picker.dart';
import '../widgets/common.dart';
import '../widgets/update_sheet.dart';
import 'pair_page.dart';

class SettingsPage extends StatelessWidget {
  const SettingsPage({super.key});

  Future<void> _confirmClearCache(BuildContext context) async {
    final app = context.read<AppState>();
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('Clear offline cache?'),
        content: const Text(
          'Deletes downloaded audio and artwork from this device. '
          'You can sync again over Wi‑Fi.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: const Text('Cancel'),
          ),
          TextButton(
            onPressed: () => Navigator.pop(ctx, true),
            child: const Text(
              'Clear',
              style: TextStyle(color: VynlColors.danger),
            ),
          ),
        ],
      ),
    );
    if (ok != true || !context.mounted) return;
    await app.clearCache();
    if (context.mounted) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Offline cache cleared')),
      );
    }
  }

  Future<void> _pickAccent(BuildContext context, ThemeController theme) async {
    final picked = await showModalBottomSheet<Color>(
      context: context,
      backgroundColor: Colors.transparent,
      isScrollControlled: true,
      builder: (_) => AccentSheet(onChanged: theme.previewAccent),
    );
    if (picked != null) await theme.setAccent(picked);
  }

  Future<void> _checkForUpdates(BuildContext context) async {
  final updates = context.read<AppUpdate>();
  await updates.check(force: true);
  if (!context.mounted) return;
  final message = updates.error != null
      ? updates.error!
      : updates.available
          ? 'Update available: ${updates.release!.version}'
          : "You're on the latest version";
  ScaffoldMessenger.of(context)
      .showSnackBar(SnackBar(content: Text(message)));
}

Future<void> _openRepo(BuildContext context) async {
    final uri = Uri.parse('https://github.com/DevX32/Vynl');
    try {
      if (!await launchUrl(uri, mode: LaunchMode.externalApplication)) {
        throw StateError('could not open $uri');
      }
    } catch (e) {
      if (!context.mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Could not open the link.')),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final app = context.watch<AppState>();
    final theme = context.watch<ThemeController>();
    final updates = context.watch<AppUpdate>();
    final paired = app.isPaired;

    return ListView(
      padding: const EdgeInsets.fromLTRB(0, 4, 0, 36),
      children: [
        if (app.lastError != null)
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 8, 16, 4),
            child: _Banner(
              text: app.lastError!,
              color: VynlColors.danger,
              icon: Icons.error_outline_rounded,
            ),
          ),
        if (app.syncProgress?.error != null)
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 8, 16, 4),
            child: _Banner(
              text: app.syncProgress!.error!,
              color: VynlColors.warning,
              icon: Icons.warning_amber_rounded,
            ),
          ),

        const _Kicker('PROFILE'),
        _Group(
          children: const [_DisplayNameField()],
        ),

        const _Kicker('APPEARANCE'),
        _Group(
          children: [
            _Row(
              label: 'Accent colour',
              hint: theme.dynamicAccent
                  ? 'Turn off Dynamic Accent to change this'
                  : theme.isPreset
                      ? 'Preset'
                      : 'Custom ${hexOf(theme.chosen)}',
              onTap: theme.dynamicAccent
                  ? null
                  : () => _pickAccent(context, theme),
              trailing: AccentSwatch(
                color: theme.accent,
                selected: false,
                checked: false,
                size: 26,
                muted: theme.dynamicAccent,
                onTap: theme.dynamicAccent
                    ? null
                    : () => _pickAccent(context, theme),
              ),
            ),
            _Row(
              label: 'Dynamic accent',
              hint: theme.dynamicAccent
                  ? 'Matching the current track artwork'
                  : 'Use the accent colour above',
              trailing: VynlSwitch(
                value: theme.dynamicAccent,
                onChanged: theme.setDynamicAccent,
              ),
            ),
          ],
        ),

        const _Kicker('DESKTOP'),
        _Group(
          children: [
            _Row(
              label: 'Connection',
              hint: paired
                  ? 'Synced over your local network'
                  : 'Not paired yet',
              trailing: _Status(text: paired ? 'PAIRED' : 'OFF', on: paired),
            ),
            if (paired) ...[
              _Row(
                label: 'Sync library',
                hint: app.syncing
                    ? '${app.syncProgress?.phase ?? 'Starting'}…'
                    : 'Pull new and changed tracks from the desktop app',
                onTap: app.syncing ? null : () => app.runSync(),
                trailing: app.syncing
                    ? const SizedBox(
                        width: 17,
                        height: 17,
                        child: CircularProgressIndicator(strokeWidth: 2.2),
                      )
                    : const _Chip(label: 'Sync'),
              ),
              if (app.syncing && app.syncProgress != null)
                const _SyncProgress(),
              _Row(
                label: 'Unpair',
                labelColor: VynlColors.danger,
                hint: 'Forget this desktop and its library',
                onTap: () => app.unpair(),
                trailing: const _Chip(label: 'Unpair', danger: true),
              ),
            ] else
              _Row(
                label: 'Pair with desktop',
                hint: 'Scan the QR code shown in Vynl › Settings',
                onTap: () => Navigator.of(context).push(
                  MaterialPageRoute(builder: (_) => const PairPage()),
                ),
                trailing: const _Chip(label: 'Scan', accent: true),
              ),
          ],
        ),

        const _Kicker('STORAGE'),
        _Group(
          children: [
            _Row(
              label: 'Offline cache',
              hint: '${formatBytes(app.cacheBytes)}  ·  '
                  '${app.downloadedTracks.length} tracks ready offline',
            ),
            _Row(
              label: 'Clear offline cache',
              labelColor: VynlColors.warning,
              hint: 'Remove downloaded audio and artwork from this device',
              onTap: () => _confirmClearCache(context),
              trailing: const _Chip(label: 'Clear'),
            ),
          ],
        ),

        const _Kicker('ABOUT'),
        _Group(
          children: [
            const _AboutHeader(),
            _Row(
              label: updates.available
                  ? 'Update to ${updates.release!.version}'
                  : 'Check for updates',
              hint: updates.error != null
                  ? updates.error!
                  : updates.available
                      ? 'A newer version is ready to install'
                      : updates.downloading
                          ? 'Looking for the latest release…'
                          : updates.checked
                              ? 'You are on the latest version'
                              : 'Tap to look for a newer release',
              labelColor: updates.available
                  ? VynlColors.accent
                  : updates.error != null
                      ? VynlColors.danger
                      : null,
              onTap: updates.available
                  ? () => showModalBottomSheet<void>(
                        context: context,
                        backgroundColor: Colors.transparent,
                        isScrollControlled: true,
                        builder: (_) => const UpdateSheet(),
                      )
                  : () => _checkForUpdates(context),
              trailing: updates.downloading
                  ? const SizedBox(
                      width: 17,
                      height: 17,
                      child: CircularProgressIndicator(strokeWidth: 2.2),
                    )
                  : _Chip(
                      label: updates.available ? 'Update' : 'Check',
                      accent: updates.available,
                    ),
            ),
            _Row(
              label: 'Source code',
              hint: 'github.com/DevX32/Vynl',
              onTap: () => _openRepo(context),
              trailing: const Icon(
                Icons.open_in_new_rounded,
                size: 17,
                color: VynlColors.faint,
              ),
            ),
          ],
        ),
      ],
    );
  }
}

class _VynlMark extends StatelessWidget {
  const _VynlMark({required this.size});

  final double size;

  @override
  Widget build(BuildContext context) {
    return Image.asset(
      'assets/icon/app_icon.png',
      width: size,
      height: size,
      filterQuality: FilterQuality.medium,
    );
  }
}

class _AboutHeader extends StatefulWidget {
  const _AboutHeader();

  @override
  State<_AboutHeader> createState() => _AboutHeaderState();
}

class _AboutHeaderState extends State<_AboutHeader> {
  String _version = '';

  @override
  void initState() {
    super.initState();
    PackageInfo.fromPlatform().then((info) {
      if (mounted) setState(() => _version = info.version);
    });
  }

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 14, 16, 16),
      child: Row(
        children: [
          const _VynlMark(size: 42),
          const SizedBox(width: 14),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                const Text(
                  'Vynl',
                  style: TextStyle(
                    fontFamily: VynlFonts.display,
                    fontSize: 20,
                    fontWeight: FontWeight.w800,
                    height: 1.1,
                    letterSpacing: -0.4,
                    color: VynlColors.text,
                  ),
                ),
                const SizedBox(height: 3),
                Text(
                  _version.isEmpty ? '—' : 'Version $_version',
                  style: const TextStyle(color: VynlColors.faint, fontSize: 12),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _Group extends StatelessWidget {
  const _Group({required this.children});

  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    return Container(
      width: double.infinity,
      padding: const EdgeInsets.symmetric(vertical: 2),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: children,
      ),
    );
  }
}

class _DisplayNameField extends StatefulWidget {
  const _DisplayNameField();

  @override
  State<_DisplayNameField> createState() => _DisplayNameFieldState();
}

class _DisplayNameFieldState extends State<_DisplayNameField> {
  late final TextEditingController _ctrl = TextEditingController(
    text: context.read<AppState>().displayName,
  );

  @override
  void dispose() {
    _ctrl.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final app = context.watch<AppState>();

    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 12, 16, 14),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Text(
            'DISPLAY NAME',
            style: TextStyle(
              color: VynlColors.faint,
              fontSize: 11,
              fontWeight: FontWeight.w800,
              letterSpacing: 1.3,
            ),
          ),
          const SizedBox(height: 10),
          TextField(
            controller: _ctrl,
            onChanged: app.setDisplayName,
            textInputAction: TextInputAction.done,
            maxLength: 32,
            style: const TextStyle(fontSize: 14, color: VynlColors.text),
            decoration: const InputDecoration(
              hintText: 'e.g. Cosmic Panda',
              counterText: '',
            ),
          ),
          const SizedBox(height: 8),
          Text(
            app.displayName.isEmpty
                ? 'Leave blank for a plain greeting'
                : 'Shown in your home greeting',
            style: const TextStyle(color: VynlColors.faint, fontSize: 12),
          ),
        ],
      ),
    );
  }
}

class _SyncProgress extends StatelessWidget {
  const _SyncProgress();

  @override
  Widget build(BuildContext context) {
    final app = context.watch<AppState>();
    final value = app.syncProgress?.fraction;
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 2, 16, 12),
      child: ClipRRect(
        borderRadius: BorderRadius.circular(VynlRadius.control),
        child: LinearProgressIndicator(
          value: value,
          minHeight: 4,
          color: VynlColors.accent,
        ),
      ),
    );
  }
}

class _Kicker extends StatelessWidget {
  const _Kicker(this.text);

  final String text;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 20, 16, 2),
      child: Text(
        text,
        style: const TextStyle(
          color: VynlColors.faint,
          fontSize: 11,
          fontWeight: FontWeight.w800,
          letterSpacing: 1.3,
        ),
      ),
    );
  }
}

class _Chip extends StatefulWidget {
  const _Chip({required this.label, this.accent = false, this.danger = false});

  final String label;
  final bool accent;
  final bool danger;

  @override
  State<_Chip> createState() => _ChipState();
}

class _ChipState extends State<_Chip> {
  bool _hover = false;

  @override
  Widget build(BuildContext context) {
    final hover = _hover;
    final Color bg;
    final Color fg;
    final Color border;

    if (widget.accent) {
      bg = hover
          ? Color.lerp(VynlColors.accent, Colors.white, 0.18)!
          : VynlColors.accent;
      fg = accentTextFor(bg);
      border = bg;
    } else if (widget.danger) {
      bg = hover
          ? VynlColors.danger.withValues(alpha: 0.10)
          : Colors.transparent;
      fg = VynlColors.danger;
      border = hover
          ? VynlColors.danger
          : VynlColors.danger.withValues(alpha: 0.5);
    } else {
      bg = hover ? VynlColors.surface : Colors.transparent;
      fg = hover ? VynlColors.text : VynlColors.dim;
      border = VynlColors.line;
    }

    return MouseRegion(
      opaque: false,
      onEnter: (_) => setState(() => _hover = true),
      onExit: (_) => setState(() => _hover = false),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 13, vertical: 9),
        decoration: BoxDecoration(
          color: bg,
          border: Border.all(color: border),
          borderRadius: BorderRadius.circular(VynlRadius.control),
        ),
        child: Text(
          widget.label,
          style: TextStyle(
            color: fg,
            fontSize: 12.5,
            fontWeight: FontWeight.w700,
            letterSpacing: 0.3,
          ),
        ),
      ),
    );
  }
}

class _Status extends StatelessWidget {
  const _Status({required this.text, required this.on});

  final String text;
  final bool on;

  @override
  Widget build(BuildContext context) {
    return Text(
      text,
      style: TextStyle(
        color: on ? VynlColors.accent : VynlColors.faint,
        fontSize: 11,
        fontWeight: FontWeight.w700,
        letterSpacing: 1.1,
      ),
    );
  }
}

class _Row extends StatelessWidget {
  const _Row({
    required this.label,
    required this.hint,
    this.trailing,
    this.onTap,
    this.labelColor,
  });

  final String label;
  final String hint;
  final Widget? trailing;
  final VoidCallback? onTap;
  final Color? labelColor;

  @override
  Widget build(BuildContext context) {
    final content = Padding(
      padding: const EdgeInsets.fromLTRB(16, 13, 16, 13),
      child: Row(
        children: [
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: [
              Text(
                label,
                style: TextStyle(
                  color: labelColor ?? VynlColors.text,
                  fontWeight: FontWeight.w600,
                  fontSize: 14.5,
                ),
              ),
              const SizedBox(height: 4),
              Text(
                hint,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: const TextStyle(
                  color: VynlColors.faint,
                  fontSize: 12.5,
                ),
              ),
            ],
          ),
        ),
        if (trailing != null) ...[
            const SizedBox(width: 14),
            trailing!,
          ],
        ],
      ),
    );

    if (onTap == null) return content;
    return Material(
      color: Colors.transparent,
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: onTap,
        hoverColor: Colors.transparent,
        highlightColor: Colors.transparent,
        focusColor: Colors.transparent,
        splashColor: Colors.transparent,
        splashFactory: NoSplash.splashFactory,
        child: content,
      ),
    );
  }
}

class _Banner extends StatelessWidget {
  const _Banner({required this.text, required this.color, required this.icon});

  final String text;
  final Color color;
  final IconData icon;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(13),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.10),
        border: Border.all(color: color.withValues(alpha: 0.28)),
        borderRadius: BorderRadius.circular(VynlRadius.control),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(icon, size: 16, color: color),
          const SizedBox(width: 11),
          Expanded(
            child: Text(
              text,
              style: TextStyle(color: color, fontSize: 12.5, height: 1.45),
            ),
          ),
        ],
      ),
    );
  }
}
