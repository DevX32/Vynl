import 'package:flutter/material.dart';

import '../state/theme_controller.dart';
import '../theme.dart';

(double, double, double) hsvOf(Color color) {
  final channels = [color.r, color.g, color.b];
  final max = channels.reduce((a, b) => a > b ? a : b);
  final min = channels.reduce((a, b) => a < b ? a : b);
  final delta = max - min;
  var hue = 0.0;
  if (delta > 0) {
    if (max == color.r) {
      hue = 60 * (((color.g - color.b) / delta) % 6);
    } else if (max == color.g) {
      hue = 60 * (((color.b - color.r) / delta) + 2);
    } else {
      hue = 60 * (((color.r - color.g) / delta) + 4);
    }
  }
  if (hue < 0) hue += 360;
  final sat = max == 0 ? 0.0 : delta / max;
  return (hue, sat * 100, max * 100);
}

Color colorOfHsv(double hue, double sat, double val) {
  final h = hue % 360;
  final s = (sat / 100).clamp(0.0, 1.0);
  final v = (val / 100).clamp(0.0, 1.0);
  final c = v * s;
  final x = c * (1 - (((h / 60) % 2) - 1).abs());
  final m = v - c;
  var r = 0.0;
  var g = 0.0;
  var b = 0.0;
  if (h < 60) {
    r = c;
    g = x;
  } else if (h < 120) {
    r = x;
    g = c;
  } else if (h < 180) {
    g = c;
    b = x;
  } else if (h < 240) {
    g = x;
    b = c;
  } else if (h < 300) {
    r = x;
    b = c;
  } else {
    r = c;
    b = x;
  }
  return Color.from(alpha: 1, red: r + m, green: g + m, blue: b + m);
}

class AccentSwatch extends StatelessWidget {
  const AccentSwatch({
    super.key,
    required this.color,
    required this.selected,
    this.onTap,
    this.size = 30,
    this.checked = true,
    this.muted = false,
  });

  final Color color;
  final bool selected;
  final VoidCallback? onTap;
  final double size;
  final bool checked;
  final bool muted;

  @override
  Widget build(BuildContext context) {
    return Semantics(
      button: true,
      selected: selected,
      label: 'Accent ${hexOf(color)}',
      child: GestureDetector(
        onTap: onTap,
        behavior: HitTestBehavior.opaque,
        child: AnimatedOpacity(
          duration: VynlMotion.fast,
          opacity: muted ? 0.35 : 1,
          child: AnimatedContainer(
            duration: VynlMotion.fast,
            curve: VynlMotion.emphasized,
            width: size,
            height: size,
            decoration: BoxDecoration(
              color: color,
              borderRadius: BorderRadius.circular(VynlRadius.control),
              border: Border.all(
                color: selected ? VynlColors.text : Colors.transparent,
                width: 2,
              ),
              boxShadow: selected
                  ? [
                      BoxShadow(
                        color: color.withValues(alpha: 0.45),
                        blurRadius: 10,
                        spreadRadius: 1,
                      ),
                    ]
                  : null,
            ),
          child: selected && checked
                ? DecoratedBox(
                    decoration: BoxDecoration(
                      color: accentTextFor(color).withValues(alpha: 0.92),
                      borderRadius: BorderRadius.circular(VynlRadius.control),
                    ),
                    child: Icon(
                      Icons.check_rounded,
                      size: size * 0.6,
                      color: color,
                    ),
                  )
                : null,
          ),
        ),
      ),
    );
  }
}

class AccentSheet extends StatefulWidget {
  const AccentSheet({super.key, required this.onChanged});

  final ValueChanged<Color> onChanged;

  @override
  State<AccentSheet> createState() => _AccentSheetState();
}

class _AccentSheetState extends State<AccentSheet> {
  late Color _value;
  late double _hue;
  late double _sat;
  late double _val;

  @override
  void initState() {
    super.initState();
    _value = VynlColors.accent;
    final hsv = hsvOf(_value);
    _hue = hsv.$1;
    _sat = hsv.$2;
    _val = hsv.$3;
  }

  void _emit(Color value) {
    widget.onChanged(value);
  }

  void _applyHsv() {
    setState(() => _value = colorOfHsv(_hue, _sat, _val));
    _emit(_value);
  }

  void _commitHex(String raw) {
    final parsed = parseHexColor(raw);
    if (parsed == null) return;
    final hsv = hsvOf(parsed);
    setState(() {
      _value = parsed;
      _hue = hsv.$1;
      _sat = hsv.$2;
      _val = hsv.$3;
    });
    _emit(_value);
  }

  @override
  Widget build(BuildContext context) {
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
          padding: const EdgeInsets.fromLTRB(20, 18, 20, 20),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const Text(
                'ACCENT COLOR',
                style: TextStyle(
                  color: VynlColors.faint,
                  fontSize: 11,
                  fontWeight: FontWeight.w800,
                  letterSpacing: 1.3,
                ),
              ),
              const SizedBox(height: 14),
              Wrap(
                spacing: 10,
                runSpacing: 10,
                children: [
                  for (final preset in kAccentPresets)
                    AccentSwatch(
                      color: preset,
                      selected: _value == preset,
                      onTap: () {
                      final next = preset;
                      setState(() {
                        _value = next;
                        final hsv = hsvOf(next);
                        _hue = hsv.$1;
                        _sat = hsv.$2;
                        _val = hsv.$3;
                      });
                      _emit(next);
                    },
                    ),
                ],
              ),
              const SizedBox(height: 22),
              _SvField(
                hue: _hue,
                sat: _sat,
                val: _val,
                onChanged: (s, v) {
                  setState(() {
                    _sat = s;
                    _val = v;
                    _applyHsv();
                  });
                },
              ),
              const SizedBox(height: 16),
              _HueField(
                hue: _hue,
                onChanged: (h) {
                  setState(() {
                    _hue = h;
                    _applyHsv();
                  });
                },
              ),
              const SizedBox(height: 16),
              Row(
                children: [
                  Container(
                    width: 44,
                    height: 44,
                    decoration: BoxDecoration(
                      color: _value,
                      borderRadius: BorderRadius.circular(VynlRadius.control),
                      border: Border.all(color: VynlColors.line),
                    ),
                  ),
                  const SizedBox(width: 12),
                  Expanded(child: _HexField(value: _value, onCommit: _commitHex)),
                ],
              ),
              const SizedBox(height: 20),
              FilledButton(
                onPressed: () => Navigator.of(context).pop(_value),
                child: const Text('Done'),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _SvField extends StatelessWidget {
  const _SvField({
    required this.hue,
    required this.sat,
    required this.val,
    required this.onChanged,
  });

  final double hue;
  final double sat;
  final double val;
  final void Function(double sat, double val) onChanged;

  @override
  Widget build(BuildContext context) {
    return ClipRRect(
      borderRadius: BorderRadius.circular(VynlRadius.control),
      child: SizedBox(
        height: 132,
        width: double.infinity,
        child: LayoutBuilder(
          builder: (context, c) {
            final w = c.maxWidth;
            final h = c.maxHeight;
            void report(Offset p) => onChanged(
              (p.dx / w * 100).clamp(0.0, 100.0),
              ((1 - p.dy / h) * 100).clamp(0.0, 100.0),
            );
            return GestureDetector(
              behavior: HitTestBehavior.opaque,
              onPanDown: (d) => report(d.localPosition),
              onPanUpdate: (d) => report(d.localPosition),
              child: Stack(
                children: [
                  Positioned.fill(
                    child: DecoratedBox(
                      decoration: BoxDecoration(
                        gradient: LinearGradient(
                          colors: [
                            Colors.white,
                            HSVColor.fromAHSV(1, hue, 1, 1).toColor(),
                          ],
                        ),
                      ),
                    ),
                  ),
                  Positioned.fill(
                    child: DecoratedBox(
                      decoration: BoxDecoration(
                        gradient: const LinearGradient(
                          begin: Alignment.topCenter,
                          end: Alignment.bottomCenter,
                          colors: [Colors.transparent, Colors.black],
                        ),
                      ),
                    ),
                  ),
                  Positioned(
                    left: w * sat / 100 - 7,
                    top: h * (1 - val / 100) - 7,
                    child: _Thumb(color: colorOfHsv(hue, sat, val)),
                  ),
                ],
              ),
            );
          },
        ),
      ),
    );
  }
}

class _HueField extends StatelessWidget {
  const _HueField({required this.hue, required this.onChanged});

  final double hue;
  final void Function(double hue) onChanged;

  @override
  Widget build(BuildContext context) {
    return ClipRRect(
      borderRadius: BorderRadius.circular(VynlRadius.control),
      child: SizedBox(
        height: 20,
        width: double.infinity,
        child: LayoutBuilder(
          builder: (context, c) {
            final w = c.maxWidth;
            void report(Offset p) => onChanged((p.dx / w * 360).clamp(0, 360));
            return GestureDetector(
              behavior: HitTestBehavior.opaque,
              onPanDown: (d) => report(d.localPosition),
              onPanUpdate: (d) => report(d.localPosition),
              child: Stack(
                children: [
                  Positioned.fill(
                    child: DecoratedBox(
                      decoration: BoxDecoration(
                        gradient: LinearGradient(
                          colors: [
                            for (var i = 0; i <= 12; i++)
                              HSVColor.fromAHSV(1, i * 30.0, 1, 1).toColor(),
                          ],
                        ),
                      ),
                    ),
                  ),
                  Positioned(
                    left: (w - 16) * hue / 360,
                    top: 2,
                    child: Container(
                      width: 16,
                      height: 16,
                      decoration: BoxDecoration(
                        color: HSVColor.fromAHSV(1, hue, 1, 1).toColor(),
                        borderRadius: BorderRadius.circular(3),
                        border: Border.all(color: Colors.white, width: 2),
                        boxShadow: const [
                          BoxShadow(
                            color: Color(0x66000000),
                            blurRadius: 4,
                            offset: Offset(0, 1),
                          ),
                        ],
                      ),
                    ),
                  ),
                ],
              ),
            );
          },
        ),
      ),
    );
  }
}

class _Thumb extends StatelessWidget {
  const _Thumb({required this.color});

  final Color color;

  @override
  Widget build(BuildContext context) {
    return Container(
      width: 14,
      height: 14,
      decoration: BoxDecoration(
        color: color,
        borderRadius: BorderRadius.circular(3),
        border: Border.all(color: Colors.white, width: 2),
        boxShadow: const [
          BoxShadow(color: Color(0x66000000), blurRadius: 4, offset: Offset(0, 1)),
        ],
      ),
    );
  }
}

class _HexField extends StatefulWidget {
  const _HexField({required this.value, required this.onCommit});

  final Color value;
  final void Function(String raw) onCommit;

  @override
  State<_HexField> createState() => _HexFieldState();
}

class _HexFieldState extends State<_HexField> {
  late final TextEditingController _controller = TextEditingController(
    text: hexOf(widget.value).substring(1),
  );

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Container(
      height: 44,
      padding: const EdgeInsets.symmetric(horizontal: 14),
      decoration: BoxDecoration(
        color: VynlColors.surfaceRaised,
        border: Border.all(color: VynlColors.line),
        borderRadius: BorderRadius.circular(VynlRadius.control),
      ),
      child: Row(
        children: [
          const Text('#', style: TextStyle(color: VynlColors.faint, fontSize: 13)),
          const SizedBox(width: 2),
          Expanded(
            child: TextField(
              controller: _controller,
              maxLength: 6,
              autofocus: false,
              textCapitalization: TextCapitalization.characters,
              style: const TextStyle(
                color: VynlColors.text,
                fontSize: 13,
                letterSpacing: 0.6,
              ),
              decoration: const InputDecoration(
                counterText: '',
                border: InputBorder.none,
                isDense: true,
                contentPadding: EdgeInsets.zero,
                fillColor: Colors.transparent,
                filled: false,
              ),
              onSubmitted: widget.onCommit,
            ),
          ),
        ],
      ),
    );
  }
}