import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:vynl_mobile/theme.dart';

void main() {
  final theme = buildVynlTheme();

  group('button shapes', () {
    test('the control radius is the desktop 4px token', () {
      expect(VynlRadius.control, 4.0);
      expect(VynlRadius.card, 4.0);
      expect(VynlRadius.tile, 4.0);
    });

    test('icon buttons resolve to a 4px square, never a circle', () {
      final style = theme.iconButtonTheme.style!;
      final shape = style.shape!.resolve(const <WidgetState>{});

      expect(shape, isA<RoundedRectangleBorder>());
      expect(shape, isNot(isA<StadiumBorder>()));
      expect(shape, isNot(isA<CircleBorder>()));

      final radius =
          (shape as RoundedRectangleBorder).borderRadius as BorderRadius;
      expect(radius, BorderRadius.circular(VynlRadius.control));
    });

    test('text and filled buttons share the same radius', () {
      final text = theme.textButtonTheme.style!.shape!
          .resolve(const <WidgetState>{}) as RoundedRectangleBorder;
      final filled = theme.filledButtonTheme.style!.shape!
          .resolve(const <WidgetState>{}) as RoundedRectangleBorder;

      expect(text.borderRadius, BorderRadius.circular(VynlRadius.control));
      expect(filled.borderRadius, BorderRadius.circular(VynlRadius.control));
    });

    test('the nav indicator is square, not a pill', () {
      expect(theme.navigationBarTheme.indicatorShape, isNot(isA<CircleBorder>()));
      expect(theme.navigationBarTheme.indicatorShape, isNot(isA<StadiumBorder>()));
      expect(
        theme.navigationBarTheme.indicatorShape,
        isA<RoundedRectangleBorder>(),
      );
    });
  });

  testWidgets('the icon button ink is clipped to a 4px square', (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        theme: buildVynlTheme(),
        home: Scaffold(
          body: IconButton(
            onPressed: () {},
            icon: const Icon(Icons.arrow_back),
          ),
        ),
      ),
    );

    final ink = tester.widget<InkResponse>(
      find.descendant(
        of: find.byType(IconButton),
        matching: find.byWidgetPredicate((w) => w is InkResponse),
      ),
    );

    expect(ink.customBorder, isNotNull);
    expect(ink.customBorder, isA<RoundedRectangleBorder>());
    expect(
      (ink.customBorder! as RoundedRectangleBorder).borderRadius,
      BorderRadius.circular(VynlRadius.control),
    );
  });
}
