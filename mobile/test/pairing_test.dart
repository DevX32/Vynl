import 'package:flutter_test/flutter_test.dart';
import 'package:vynl_mobile/screens/pair_page.dart';

void main() {
  group('parsePairingPayload', () {
    test('parses host, port and pin', () {
      final p = parsePairingPayload('vynl://pair?h=devx32.local&p=17865&pin=483920');
      expect(p, isNotNull);
      expect(p!.host, 'devx32.local:17865');
      expect(p.pin, '483920');
    });

    test('omits port when absent', () {
      final p = parsePairingPayload('vynl://pair?h=10.0.0.5&pin=111111');
      expect(p!.host, '10.0.0.5');
    });

    test('ignores an empty port param', () {
      final p = parsePairingPayload('vynl://pair?h=10.0.0.5&p=&pin=111111');
      expect(p!.host, '10.0.0.5');
    });

    test('handles a bare ipv4 host', () {
      final p = parsePairingPayload('vynl://pair?h=192.168.1.64&p=17865&pin=000042');
      expect(p!.host, '192.168.1.64:17865');
      expect(p.pin, '000042');
    });

    test('rejects a missing pin', () {
      expect(parsePairingPayload('vynl://pair?h=devx32.local&p=17865'), isNull);
    });

    test('rejects an empty pin', () {
      expect(parsePairingPayload('vynl://pair?h=devx32.local&pin='), isNull);
    });

    test('rejects a missing host', () {
      expect(parsePairingPayload('vynl://pair?p=17865&pin=483920'), isNull);
    });

    test('rejects a plain http url so it falls back to manual entry', () {
      expect(parsePairingPayload('http://192.168.1.64:17865'), isNull);
    });

    test('rejects a vynl url with the wrong action', () {
      expect(parsePairingPayload('vynl://other?h=a&pin=1'), isNull);
    });

    test('rejects garbage', () {
      expect(parsePairingPayload('not a url at all'), isNull);
      expect(parsePairingPayload(''), isNull);
    });
  });
}
