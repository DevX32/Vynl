import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../models.dart';
import '../theme.dart';
import 'artwork.dart';

class TrackTile extends StatelessWidget {
  const TrackTile({
    super.key,
    required this.track,
    this.onTap,
    this.trailing,
    this.subtitle,
    this.active = false,
    this.durationLabel,
    this.showStatus = true,
  });

  final CatalogTrack track;
  final VoidCallback? onTap;
  final Widget? trailing;
  final String? subtitle;
  final bool active;
  final String? durationLabel;
  final bool showStatus;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;

    final body = Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
      child: Row(
        children: [
          Stack(
            children: [
              ArtTile(
                coverPath: track.localCoverPath,
                size: 56,
                accent: scheme.primary,
              ),
              if (active)
                Positioned.fill(
                  child: DecoratedBox(
                    decoration: BoxDecoration(
                      borderRadius: BorderRadius.circular(VynlRadius.thumb),
                      border: Border.all(
                        color: scheme.primary.withValues(alpha: 0.55),
                        width: 1.5,
                      ),
                    ),
                  ),
                ),
            ],
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
                  style: TextStyle(
                    color: active ? scheme.primary : VynlColors.text,
                    fontWeight: FontWeight.w700,
                    fontSize: 14.5,
                    letterSpacing: -0.1,
                  ),
                ),
                const SizedBox(height: 2),
                Text(
                  track.artist,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: const TextStyle(
                    color: VynlColors.dim,
                    fontSize: 12.5,
                  ),
                ),
                const SizedBox(height: 1),
                Text(
                  subtitle ??
                      (track.isDownloaded
                          ? track.album
                          : '${track.album}  ·  not downloaded'),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: const TextStyle(
                    color: VynlColors.faint,
                    fontSize: 11.5,
                  ),
                ),
              ],
            ),
          ),
          if (trailing != null) ...[
            const SizedBox(width: 10),
            trailing!,
          ] else ...[
            const SizedBox(width: 10),
            Column(
              crossAxisAlignment: CrossAxisAlignment.end,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  durationLabel ?? formatDurationSeconds(track.duration),
                  style: const TextStyle(
                    color: VynlColors.faint,
                    fontSize: 11.5,
                    fontFeatures: [FontFeature.tabularFigures()],
                  ),
                ),
                const SizedBox(height: 6),
                showStatus
                    ? Icon(
                        track.isDownloaded
                            ? Icons.download_done_rounded
                            : Icons.cloud_download_outlined,
                        size: 15,
                        color: track.isDownloaded
                            ? VynlColors.success.withValues(alpha: 0.75)
                            : VynlColors.faint,
                      )
                    : const SizedBox(width: 15, height: 15),
              ],
            ),
          ],
        ],
      ),
    );

    if (onTap == null) return body;

    return Material(
      color: Colors.transparent,
      borderRadius: BorderRadius.circular(VynlRadius.tile),
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: onTap,
        child: active
            ? DecoratedBox(
                decoration: BoxDecoration(
                  color: scheme.primary.withValues(alpha: 0.07),
                  borderRadius: BorderRadius.circular(VynlRadius.tile),
                ),
                child: body,
              )
            : body,
      ),
    );
  }
}

class SectionHeader extends StatelessWidget {
  const SectionHeader({super.key, required this.title, this.trailing});

  final String title;
  final Widget? trailing;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.fromLTRB(20, 18, 16, 8),
      child: Row(
        children: [
          Expanded(
            child: Text(
              title,
              style: const TextStyle(
                color: VynlColors.faint,
                fontSize: 11,
                fontWeight: FontWeight.w800,
                letterSpacing: 1.3,
              ),
            ),
          ),
          if (trailing != null) trailing!,
        ],
      ),
    );
  }
}

enum VynlControlStyle { accent, raised, plain }

class VynlSwitch extends StatelessWidget {
  const VynlSwitch({
    super.key,
    required this.value,
    required this.onChanged,
    this.accent,
  });

  final bool value;
  final ValueChanged<bool>? onChanged;
  final Color? accent;

  static const _trackW = 36.0;
  static const _trackH = 20.0;
  static const _knob = 14.0;

  @override
  Widget build(BuildContext context) {
    final tint = accent ?? VynlColors.accent;
    final enabled = onChanged != null;
    return Semantics(
      toggled: value,
      button: true,
      enabled: enabled,
      child: GestureDetector(
        onTap: enabled ? () => onChanged!(!value) : null,
        behavior: HitTestBehavior.opaque,
        child: Center(
          widthFactor: 1,
          child: SizedBox(
            width: 48,
            height: 44,
            child: Center(
              child: AnimatedOpacity(
                duration: VynlMotion.fast,
                opacity: enabled ? 1 : 0.4,
                child: AnimatedContainer(
                  duration: VynlMotion.fast,
                  curve: VynlMotion.emphasized,
                  width: _trackW,
                  height: _trackH,
                  padding: const EdgeInsets.all(3),
                  decoration: BoxDecoration(
                    color: value
                        ? tint.withValues(alpha: 0.11)
                        : VynlColors.text.withValues(alpha: 0.20),
                    borderRadius: BorderRadius.circular(VynlRadius.control),
                  ),
                  child: AnimatedAlign(
                    duration: VynlMotion.fast,
                    curve: VynlMotion.emphasized,
                    alignment:
                        value ? Alignment.centerRight : Alignment.centerLeft,
                    child: Container(
                      width: _knob,
                      height: _knob,
                      decoration: BoxDecoration(
                        color: value ? tint : VynlColors.faint,
                        borderRadius: BorderRadius.circular(3),
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class VynlControl extends StatefulWidget {
  const VynlControl({
    super.key,
    required this.onTap,
    this.icon,
    this.child,
    this.style = VynlControlStyle.plain,
    this.size = 40,
    this.iconSize = 20,
    this.iconColor,
    this.color,
    this.enabled = true,
    this.tooltip,
  }) : assert(icon != null || child != null);

  final VoidCallback? onTap;
  final IconData? icon;
  final Widget? child;
  final VynlControlStyle style;
  final double size;
  final double iconSize;
  final Color? iconColor;
  final Color? color;
  final bool enabled;
  final String? tooltip;

  static double extent(double side) => side * math.sqrt2;

  @override
  State<VynlControl> createState() => _VynlControlState();
}

class _VynlControlState extends State<VynlControl> {
  bool _pressed = false;

  @override
  Widget build(BuildContext context) {
    late final Color bg;
    late final Color fg;
    late final BorderSide side;

    switch (widget.style) {
      case VynlControlStyle.accent:
        bg = widget.color ?? VynlColors.accent;
        fg = widget.iconColor ?? VynlColors.accentOn;
        side = BorderSide.none;
      case VynlControlStyle.raised:
        bg = widget.color ?? VynlColors.surfaceRaised;
        fg = widget.iconColor ?? VynlColors.dim;
        side = const BorderSide(color: VynlColors.line);
      case VynlControlStyle.plain:
        bg = widget.color ?? Colors.transparent;
        fg = widget.iconColor ?? VynlColors.dim;
        side = BorderSide.none;
    }

    final shape = RoundedRectangleBorder(
      borderRadius: BorderRadius.circular(VynlRadius.control),
      side: side,
    );

    final button = Material(
      color: bg,
      shape: shape,
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: widget.enabled ? widget.onTap : null,
        onHighlightChanged: widget.enabled
            ? (v) => setState(() => _pressed = v)
            : null,
        customBorder: shape,
        splashColor: fg.withValues(alpha: 0.16),
        highlightColor: fg.withValues(alpha: 0.10),
        child: SizedBox(
          width: widget.size,
          height: widget.size,
          child: Center(
            child: AnimatedScale(
              scale: _pressed ? 0.92 : 1,
              duration: VynlMotion.fast,
              curve: VynlMotion.emphasized,
              child: Transform.rotate(
                angle: -math.pi / 4,
                child: widget.child ??
                    Icon(widget.icon, size: widget.iconSize, color: fg),
              ),
            ),
          ),
        ),
      ),
    );

    Widget result = SizedBox(
      width: VynlControl.extent(widget.size),
      height: VynlControl.extent(widget.size),
      child: Center(
        child: Transform.rotate(angle: math.pi / 4, child: button),
      ),
    );

    if (!widget.enabled) result = Opacity(opacity: 0.4, child: result);
    if (widget.tooltip != null) {
      result = Tooltip(message: widget.tooltip!, child: result);
    }
    return result;
  }
}

String formatDurationSeconds(double seconds) {
  final d = Duration(seconds: seconds.round());
  return formatDuration(d);
}

String formatDuration(Duration d) {
  final m = d.inMinutes.remainder(60).toString().padLeft(2, '0');
  final s = d.inSeconds.remainder(60).toString().padLeft(2, '0');
  final h = d.inHours;
  if (h > 0) return '$h:$m:$s';
  return '${d.inMinutes}:$s';
}

String formatBytes(int bytes) {
  if (bytes < 1024) return '$bytes B';
  if (bytes < 1024 * 1024) return '${(bytes / 1024).toStringAsFixed(1)} KB';
  if (bytes < 1024 * 1024 * 1024) {
    return '${(bytes / (1024 * 1024)).toStringAsFixed(1)} MB';
  }
  return '${(bytes / (1024 * 1024 * 1024)).toStringAsFixed(2)} GB';
}
