import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:vynl_mobile/theme.dart';

void main() {
  test('theme builds', () {
    final theme = buildVynlTheme();
    expect(theme.brightness, Brightness.dark);
  });
}
