import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../state/app_state.dart';
import '../theme.dart';
import '../widgets/common.dart';
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

  @override
  Widget build(BuildContext context) {
    final app = context.watch<AppState>();
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

        const _Section(child: _Kicker('DESKTOP')),
        _Section(
          child: _Row(
            label: 'Connection',
            hint: paired
                ? (app.credentials?.baseUrl ?? '')
                : 'Not paired yet',
            hintMono: paired,
            trailing: _Status(text: paired ? 'PAIRED' : 'OFF', on: paired),
          ),
        ),
        if (paired) ...[
          _Section(
            child: _Row(
              label: 'Sync library',
              hint: app.syncing
                  ? '${app.syncProgress?.phase ?? 'Starting'}…'
                  : 'Pull new and changed tracks from the desktop app',
              onTap: app.syncing
                  ? null
                  : () => app.runSync(),
              trailing: app.syncing
                  ? const SizedBox(
                      width: 17,
                      height: 17,
                      child: CircularProgressIndicator(strokeWidth: 2.2),
                    )
                  : const _Chip(label: 'Sync'),
            ),
          ),
          if (app.syncing && app.syncProgress != null)
            _Section(
              tight: true,
              child: ClipRRect(
                borderRadius: BorderRadius.circular(VynlRadius.control),
                child: LinearProgressIndicator(
                  value: app.syncProgress!.fraction,
                  minHeight: 4,
                ),
              ),
            ),
          _Section(
            child: _Row(
              label: 'Unpair',
              labelColor: VynlColors.danger,
              hint: 'Forget this desktop and its library',
              onTap: () => app.unpair(),
              trailing: const _Chip(label: 'Unpair', danger: true),
            ),
          ),
        ] else
          _Section(
            child: _Row(
              label: 'Pair with desktop',
              hint: 'Scan the QR code shown in Vynl › Settings',
              onTap: () => Navigator.of(context).push(
                MaterialPageRoute(builder: (_) => const PairPage()),
              ),
              trailing: const _Chip(label: 'Scan', accent: true),
            ),
          ),

        const _Section(child: _Kicker('STORAGE')),
        _Section(
          child: _Row(
            label: 'Offline cache',
            hint: '${formatBytes(app.cacheBytes)}  ·  '
                '${app.downloadedTracks.length} tracks ready offline',
          ),
        ),
        _Section(
          child: _Row(
            label: 'Clear offline cache',
            labelColor: VynlColors.warning,
            hint: 'Remove downloaded audio and artwork from this device',
            onTap: () => _confirmClearCache(context),
            trailing: const _Chip(label: 'Clear'),
          ),
        ),

        const _Section(child: _Kicker('ABOUT')),
        const _Section(
          child: _Row(
            label: 'Vynl',
            hint: 'Offline companion for Vynl desktop',
          ),
        ),
      ],
    );
  }
}

class _Section extends StatelessWidget {
  const _Section({required this.child, this.tight = false});

  final Widget child;
  final bool tight;

  @override
  Widget build(BuildContext context) {
    return Container(
      width: double.infinity,
      decoration: const BoxDecoration(
        border: Border(top: BorderSide(color: VynlColors.line)),
      ),
      padding: tight
          ? const EdgeInsets.fromLTRB(16, 10, 16, 6)
          : const EdgeInsets.fromLTRB(16, 15, 16, 15),
      child: child,
    );
  }
}

class _Kicker extends StatelessWidget {
  const _Kicker(this.text);

  final String text;

  @override
  Widget build(BuildContext context) {
    return Text(
      text,
      style: const TextStyle(
        color: VynlColors.faint,
        fontSize: 11,
        fontWeight: FontWeight.w800,
        letterSpacing: 1.3,
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
      fg = const Color(0xFF17131A);
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
    this.hintMono = false,
  });

  final String label;
  final String hint;
  final Widget? trailing;
  final VoidCallback? onTap;
  final Color? labelColor;
  final bool hintMono;

  @override
  Widget build(BuildContext context) {
    final content = Row(
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
                style: TextStyle(
                  color: VynlColors.faint,
                  fontSize: hintMono ? 12 : 12.5,
                  letterSpacing: hintMono ? 0.2 : 0,
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
