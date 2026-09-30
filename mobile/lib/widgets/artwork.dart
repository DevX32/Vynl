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
  });

  final String? coverPath;
  final Widget child;
  final double radius;
  final double blur;
  final double dim;
  final Color? tint;

  @override
  Widget build(BuildContext context) {
    final path = coverPath;
    final hasArt = path != null && File(path).existsSync();

    Widget backdrop;
    if (hasArt) {
      backdrop = ImageFiltered(
        imageFilter: ImageFilter.blur(sigmaX: blur, sigmaY: blur),
        child: Transform.scale(
          scale: 1.4,
          child: Image.file(
            File(path),
            fit: BoxFit.cover,
            width: double.infinity,
            height: double.infinity,
            errorBuilder: (_, __, ___) => const SizedBox.shrink(),
          ),
        ),
      );
    } else {
      backdrop = DecoratedBox(
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

    return ClipRRect(
      borderRadius: BorderRadius.circular(radius),
      child: Stack(
        fit: StackFit.expand,
        children: [
          const ColoredBox(color: VynlColors.bg),
          backdrop,
          ColoredBox(color: const Color(0xFF140F0C).withValues(alpha: dim)),
          child,
        ],
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
    final hasArt = path != null && File(path).existsSync();

    return ClipRRect(
      borderRadius: BorderRadius.circular(radius),
      child: SizedBox(
        width: size,
        height: size,
        child: hasArt
            ? Image.file(
                File(path),
                fit: BoxFit.cover,
                cacheWidth: (size * 2).round(),
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
