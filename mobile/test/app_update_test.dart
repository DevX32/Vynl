import 'package:flutter_test/flutter_test.dart';
import 'package:vynl_mobile/services/app_update.dart';

void main() {
  group('AppUpdate.compare', () {
    test('treats a newer patch as newer', () {
      expect(AppUpdate.compare('0.4.5', '0.4.4'), greaterThan(0));
    });

    test('treats the same version as equal', () {
      expect(AppUpdate.compare('0.4.4', '0.4.4'), 0);
    });

    test('does not offer an update for the same release', () {
      expect(AppUpdate.compare('0.4.4+2026', '0.4.4+2026'), 0);
    });

    test('offers an update when the build number moves', () {
      expect(AppUpdate.compare('0.4.4+2027', '0.4.4+2026'), greaterThan(0));
    });

    test('a desktop manifest never beats a newer phone build', () {
      expect(AppUpdate.compare('0.4.4', '0.4.4+2026'), lessThan(0));
    });

    test('handles missing components', () {
      expect(AppUpdate.compare('0.5', '0.4.9'), greaterThan(0));
      expect(AppUpdate.compare('0.4.4', '0.4.4.1'), lessThan(0));
    });
  });

  group('AppUpdate.abiFor', () {
    test('maps the android abi names', () {
      expect(AppUpdate.abiFor('androidArm64'), 'arm64-v8a');
      expect(AppUpdate.abiFor('androidArm'), 'armeabi-v7a');
      expect(AppUpdate.abiFor('androidX64'), 'x86_64');
    });

    test('falls back for unknown names', () {
      expect(AppUpdate.abiFor('windowsX64'), isNull);
    });
  });
}
