import 'package:flutter/material.dart';

import '../theme.dart';

const _logoSize = 120.0;

const _logoMs = 900.0;
const _logoDelayMs = 120.0;

const _entranceCurve = Cubic(0.16, 1, 0.3, 1);
const _standardCurve = Cubic(0.4, 0, 0.2, 1);

class SplashScreen extends StatefulWidget {
  const SplashScreen({super.key, this.exiting = false, this.onExited});

  final bool exiting;
  final VoidCallback? onExited;

  @override
  State<SplashScreen> createState() => _SplashScreenState();
}

class _SplashScreenState extends State<SplashScreen>
    with TickerProviderStateMixin {
  late final AnimationController _enter = AnimationController(
    vsync: this,
    duration: VynlMotion.splashEntrance,
  );

  late final AnimationController _exit = AnimationController(
    vsync: this,
    duration: VynlMotion.splashExit,
  );

  late final CurvedAnimation _logo = _segment(_logoDelayMs, _logoMs, _entranceCurve);

  late final Animation<double> _logoScale =
      Tween<double>(begin: 0.5, end: 1).animate(_logo);
  late final Animation<double> _logoTurn = Tween<double>(
    begin: -10 * (3.14159265358979 / 180),
    end: 0,
  ).animate(_logo);

  CurvedAnimation _segment(double delayMs, double ms, Curve curve) {
    final total = VynlMotion.splashEntrance.inMilliseconds;
    return CurvedAnimation(
      parent: _enter,
      curve: Interval(delayMs / total, (delayMs + ms) / total, curve: curve),
    );
  }

  @override
  void initState() {
    super.initState();
    _enter.forward();
    if (widget.exiting) _startExit();
  }

  @override
  void didUpdateWidget(SplashScreen oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.exiting && !oldWidget.exiting) _startExit();
  }

  void _startExit() {
    _exit.forward().whenComplete(() => widget.onExited?.call());
  }

  @override
  void dispose() {
    _logo.dispose();
    _enter.dispose();
    _exit.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return ColoredBox(
      color: VynlColors.bg,
      child: AnimatedBuilder(
        animation: _exit,
        child: Center(
          child: FadeTransition(
            opacity: _logo,
            child: ScaleTransition(
              scale: _logoScale,
              child: RotationTransition(
                turns: _logoTurn,
                child: Image.asset(
                  'assets/icon/splash_icon.png',
                  width: _logoSize,
                  height: _logoSize,
                  fit: BoxFit.contain,
                  errorBuilder: (_, __, ___) => const SizedBox.shrink(),
                ),
              ),
            ),
          ),
        ),
        builder: (context, child) {
          final t = _standardCurve.transform(_exit.value);
          return Opacity(
            opacity: 1 - t,
            child: Transform.scale(
              scale: 1 + 0.05 * t,
              child: child,
            ),
          );
        },
      ),
    );
  }
}