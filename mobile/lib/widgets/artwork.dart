import 'dart:io';
import 'dart:ui';

import 'package:flutter/material.dart';

import '../theme.dart';

class ArtBackdrop extends StatelessWidget {
  const ArtBackdrop({
    super.key,
    required this.coverPath,
    required this.child,
    this.radius = 0,
    this.blur = 72,
    this.dim = 0.66,
    this.tint,
    this.vignette = true,
  });

  final String? coverPath;
  final Widget child;
  final double radius;
  final double blur;
  final double dim;
  final Color? tint;
  final bool vignette;

  static const _maxShrink = 8.0;

  static const _vignetteDecoration = BoxDecoration(
    gradient: RadialGradient(
      radius: 0.95,
      colors: [
        Color(0x000E0E10),
        Color(0x800E0E10),
        Color(0xD90E0E10),
      ],
      stops: [0.0, 0.7, 1.0],
    ),
  );

  @override
  Widget build(BuildContext context) {
    final path = coverPath;
    final hasArt = path != null && path.isNotEmpty;

    return ClipRRect(
      borderRadius: BorderRadius.circular(radius),
      child: LayoutBuilder(
        builder: (context, constraints) {
          final shrink = (blur / 4).clamp(1.0, _maxShrink);
          final sigma = blur / shrink;
          final width = constraints.maxWidth / shrink;
          final height = constraints.maxHeight / shrink;
          final ratio = MediaQuery.devicePixelRatioOf(context);

          final source = hasArt
              ? Image.file(
                  File(path),
                  fit: BoxFit.cover,
                  width: width,
                  height: height,
                  cacheWidth: (width * ratio).round(),
                  gaplessPlayback: true,
                  errorBuilder: (_, __, ___) => _fallback(),
                )
              : _fallback();

          final picture = RepaintBoundary(
            child: FittedBox(
              fit: BoxFit.fill,
              child: SizedBox(
                width: width,
                height: height,
                child: shrink <= 1.0
                    ? source
                    : ImageFiltered(
                        imageFilter: ImageFilter.blur(
                          sigmaX: sigma,
                          sigmaY: sigma,
                        ),
                        child: source,
                      ),
              ),
            ),
          );

          return Stack(
            fit: StackFit.expand,
            children: [
              const ColoredBox(color: VynlColors.bg),
              ClipRect(child: picture),
              ColoredBox(color: VynlColors.bg.withValues(alpha: dim)),
              if (vignette) const DecoratedBox(decoration: _vignetteDecoration),
              child,
            ],
          );
        },
      ),
    );
  }

  Widget _fallback() {
    return DecoratedBox(
      decoration: BoxDecoration(
        gradient: LinearGradient(
          begin: Alignment.topCenter,
          end: Alignment.bottomCenter,
          colors: [
            (tint ?? VynlColors.accent).withValues(alpha: 0.26),
            VynlColors.bg,
          ],
        ),
      ),
    );
  }
}

class ArtTile extends StatelessWidget {
  const ArtTile({
    super.key,
    required this.coverPath,
    this.size = 56,
    this.radius = VynlRadius.thumb,
    this.icon = Icons.music_note_rounded,
    this.accent,
  });

  final String? coverPath;
  final double size;
  final double radius;
  final IconData icon;
  final Color? accent;

  @override
  Widget build(BuildContext context) {
    final path = coverPath;
    final hasArt = path != null && path.isNotEmpty;
    final ratio = MediaQuery.devicePixelRatioOf(context);

    return ClipRRect(
      borderRadius: BorderRadius.circular(radius),
      child: SizedBox(
        width: size,
        height: size,
        child: hasArt
            ? Image.file(
                File(path),
                fit: BoxFit.cover,
                cacheWidth: (size * ratio).round(),
                errorBuilder: (_, __, ___) => _fallback(),
              )
            : _fallback(),
      ),
    );
  }

  Widget _fallback() {
    return DecoratedBox(
      decoration: BoxDecoration(
        gradient: LinearGradient(
          begin: Alignment.topLeft,
          end: Alignment.bottomRight,
          colors: [
            (accent ?? VynlColors.accent).withValues(alpha: 0.20),
            VynlColors.surfaceRaised,
          ],
        ),
      ),
      child: Icon(icon, color: VynlColors.faint, size: size * 0.42),
    );
  }
}
