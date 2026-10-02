import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

const kDefaultAccent = Color(0xFFA894E8);

const kAccentPresets = <Color>[
  Color(0xFFA894E8),
  Color(0xFF7AA2F7),
  Color(0xFF8FD694),
  Color(0xFFE8C86A),
  Color(0xFFE8945C),
  Color(0xFFFF6B61),
  Color(0xFFF06BA8),
];

const _accentTextDark = Color(0xFF161128);
const _accentTextLight = Color(0xFFF5F3FF);

Color accentTextFor(Color accent) =>
    accent.computeLuminance() > 0.55 ? _accentTextDark : _accentTextLight;

const kVynlOverlayStyle = SystemUiOverlayStyle(
  statusBarColor: Colors.transparent,
  statusBarIconBrightness: Brightness.light,
  systemNavigationBarColor: Colors.transparent,
  systemNavigationBarDividerColor: Colors.transparent,
  systemNavigationBarIconBrightness: Brightness.light,
  systemNavigationBarContrastEnforced: false,
);

class VynlColors {
  static const bg = Color(0xFF0E0E10);
  static const bgTop = Color(0xFF151519);
  static const surface = Color(0xFF131317);
  static const surfaceRaised = Color(0xFF151519);
  static const line = Color(0x17F0F0EC);
  static const lineStrong = Color(0x33F0F0EC);
  static const text = Color(0xFFF0F0EC);
  static const dim = Color(0xFFA3A39B);
  static const faint = Color(0xFF7A7A74);
  static const warm = Color(0xFFD9A15C);
  static const danger = Color(0xFFFF6B61);
  static const warning = Color(0xFFD9A15C);
  static const success = Color(0xFF8FD694);

  static Color accent = kDefaultAccent;
  static Color accentOn = _accentTextDark;
  static Color accentSoft = const Color(0x1CB9A7FF);

  static void setAccent(Color value) {
    accent = value;
    accentOn = accentTextFor(value);
    accentSoft = value.withValues(alpha: 0.11);
  }
}

class VynlRadius {
  static const control = 4.0;
  static const card = control;
  static const tile = control;
  static const thumb = control;
  static const art = control;

  static const hero = 10.0;
}

class VynlFonts {
  static const mono = 'Geist Mono';

  static const display = 'Geist';

  static const lyrics = 'Sora';

  static const serif = 'Fraunces';
}

class VynlMotion {
  static const fast = Duration(milliseconds: 170);
  static const normal = Duration(milliseconds: 320);
  static const slow = Duration(milliseconds: 520);

  static Curve get emphasized => Curves.easeOutCubic;

  static Curve get standard => Curves.easeOutQuart;

  static const splashEntrance = Duration(milliseconds: 1020);
  static const splashExit = Duration(milliseconds: 400);
}

const kVynlBackground = BoxDecoration(color: VynlColors.bg);

ThemeData buildVynlTheme({Color? seed}) {
  final accent = seed ?? VynlColors.accent;
  final accentOn = accentTextFor(accent);
  final scheme = ColorScheme.fromSeed(
    seedColor: accent,
    brightness: Brightness.dark,
  ).copyWith(
    primary: accent,
    onPrimary: accentOn,
    secondary: VynlColors.warm,
    onSecondary: const Color(0xFF161128),
    surface: VynlColors.bg,
    onSurface: VynlColors.text,
    outline: VynlColors.line,
    error: VynlColors.danger,
    surfaceContainer: VynlColors.surface,
    surfaceContainerHigh: VynlColors.surfaceRaised,
  );

  final base = ThemeData(
    useMaterial3: true,
    brightness: Brightness.dark,
    colorScheme: scheme,
    scaffoldBackgroundColor: VynlColors.bg,
    splashFactory: InkSparkle.splashFactory,
    fontFamily: VynlFonts.mono,
  );

  return base.copyWith(
    textTheme: base.textTheme.apply(
      bodyColor: VynlColors.text,
      displayColor: VynlColors.text,
    ),
    appBarTheme: const AppBarTheme(
      backgroundColor: Colors.transparent,
      surfaceTintColor: Colors.transparent,
      foregroundColor: VynlColors.text,
      elevation: 0,
      scrolledUnderElevation: 0,
      centerTitle: false,
      titleTextStyle: TextStyle(
        color: VynlColors.text,
        fontFamily: VynlFonts.display,
        fontSize: 21,
        fontWeight: FontWeight.w600,
        letterSpacing: -0.4,
      ),
      systemOverlayStyle: kVynlOverlayStyle,
    ),
    inputDecorationTheme: InputDecorationTheme(
      filled: true,
      fillColor: VynlColors.surface,
      isDense: true,
      contentPadding: const EdgeInsets.symmetric(horizontal: 18, vertical: 14),
      prefixIconColor: VynlColors.faint,
      suffixIconColor: VynlColors.faint,
      border: OutlineInputBorder(
        borderRadius: BorderRadius.circular(VynlRadius.control),
        borderSide: BorderSide.none,
      ),
      enabledBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(VynlRadius.control),
        borderSide: BorderSide.none,
      ),
      focusedBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(VynlRadius.control),
        borderSide: BorderSide.none,
      ),
      labelStyle: const TextStyle(color: VynlColors.dim),
      hintStyle: const TextStyle(color: VynlColors.faint),
    ),
    filledButtonTheme: FilledButtonThemeData(
      style: FilledButton.styleFrom(
        backgroundColor: accent,
        foregroundColor: accentOn,
        minimumSize: const Size.fromHeight(54),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(VynlRadius.control),
        ),
        textStyle: const TextStyle(
          fontSize: 15,
          fontWeight: FontWeight.w700,
          letterSpacing: 0.2,
        ),
      ),
    ),
    outlinedButtonTheme: OutlinedButtonThemeData(
      style: OutlinedButton.styleFrom(
        foregroundColor: VynlColors.text,
        side: const BorderSide(color: VynlColors.line),
        minimumSize: const Size.fromHeight(54),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(VynlRadius.control),
        ),
        textStyle: const TextStyle(fontSize: 15, fontWeight: FontWeight.w600),
      ),
    ),
    textButtonTheme: TextButtonThemeData(
      style: TextButton.styleFrom(
        foregroundColor: VynlColors.text,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(VynlRadius.control),
        ),
      ),
    ),
    iconButtonTheme: IconButtonThemeData(
      style: IconButton.styleFrom(
        foregroundColor: VynlColors.dim,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(VynlRadius.control),
        ),
      ),
    ),
    navigationBarTheme: NavigationBarThemeData(
      backgroundColor: VynlColors.surface,
      surfaceTintColor: Colors.transparent,
      indicatorColor: accent.withValues(alpha: 0.16),
      indicatorShape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(VynlRadius.control * 2),
      ),
      height: 66,
      elevation: 0,
      labelBehavior: NavigationDestinationLabelBehavior.alwaysShow,
      labelTextStyle: WidgetStateProperty.resolveWith((states) {
        final selected = states.contains(WidgetState.selected);
        return TextStyle(
          fontSize: 10.5,
          height: 1.15,
          fontWeight: selected ? FontWeight.w700 : FontWeight.w600,
          letterSpacing: 0.1,
          color: selected ? VynlColors.text : VynlColors.faint,
        );
      }),
      iconTheme: WidgetStateProperty.resolveWith((states) {
        final selected = states.contains(WidgetState.selected);
        return IconThemeData(
          size: 22,
          color: selected ? accent : VynlColors.faint,
        );
      }),
    ),
    snackBarTheme: SnackBarThemeData(
      backgroundColor: VynlColors.surfaceRaised,
      contentTextStyle: const TextStyle(color: VynlColors.text, fontSize: 13.5),
      behavior: SnackBarBehavior.floating,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(VynlRadius.tile),
      ),
    ),
    dialogTheme: DialogThemeData(
      backgroundColor: VynlColors.surface,
      surfaceTintColor: Colors.transparent,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(VynlRadius.card),
      ),
    ),
    dividerTheme: const DividerThemeData(
      color: VynlColors.line,
      thickness: 1,
      space: 1,
    ),
    sliderTheme: SliderThemeData(
      activeTrackColor: VynlColors.text,
      inactiveTrackColor: VynlColors.line,
      thumbColor: VynlColors.text,
      overlayColor: VynlColors.accent.withValues(alpha: 0.14),
      trackHeight: 3,
      thumbShape: const RoundSliderThumbShape(enabledThumbRadius: 5),
      overlayShape: const RoundSliderOverlayShape(overlayRadius: 16),
    ),
    listTileTheme: const ListTileThemeData(
      iconColor: VynlColors.dim,
      textColor: VynlColors.text,
      contentPadding: EdgeInsets.symmetric(horizontal: 16, vertical: 4),
    ),
    progressIndicatorTheme: ProgressIndicatorThemeData(
      color: accent,
      linearTrackColor: VynlColors.line,
    ),
  );
}
