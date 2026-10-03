import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../state/app_state.dart';
import '../theme.dart';
import 'qr_scan_page.dart';

class PairingPayload {
  const PairingPayload({required this.host, required this.pin, this.alts = const []});

  final String host;
  final String pin;
  final List<String> alts;
}

PairingPayload? parsePairingPayload(String raw) {
  final uri = Uri.tryParse(raw);
  if (uri == null) return null;
  if (uri.scheme != 'vynl' || uri.host != 'pair') return null;

  final host = uri.queryParameters['h'];
  final pin = uri.queryParameters['pin'];
  if (host == null || host.isEmpty || pin == null || pin.isEmpty) return null;

  final port = uri.queryParameters['p'];
  final portPart = (port == null || port.isEmpty) ? '' : ':$port';

  String withPort(String value) {
    if (value.isEmpty) return value;
    if (RegExp(r':\d+$').hasMatch(value)) return value;
    return '$value$portPart';
  }

  final alts = (uri.queryParameters['a'] ?? '')
      .split(',')
      .map((value) => value.trim())
      .where((value) => value.isNotEmpty)
      .map(withPort)
      .toList();

  return PairingPayload(host: withPort(host), pin: pin, alts: alts);
}

class PairPage extends StatefulWidget {
  const PairPage({super.key});

  @override
  State<PairPage> createState() => _PairPageState();
}

class _PairPageState extends State<PairPage> {
  bool _busy = false;

  Future<void> _scan() async {
    if (_busy) return;
    if (kIsWeb || Platform.isWindows) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(
          content: Text('QR scanning needs a phone camera — use Android.'),
        ),
      );
      return;
    }
    setState(() => _busy = true);
    try {
      final raw = await Navigator.of(context).push<String>(
        MaterialPageRoute(builder: (_) => const QrScanPage()),
      );
      if (raw == null || raw.trim().isEmpty) return;

      final parsed = parsePairingPayload(raw.trim());
      if (parsed == null) {
        if (mounted) {
          ScaffoldMessenger.of(context).showSnackBar(
            const SnackBar(content: Text("That isn't a Vynl pairing code.")),
          );
        }
        return;
      }

      if (!mounted) return;
      final app = context.read<AppState>();
      final messenger = ScaffoldMessenger.of(context);
      final navigator = Navigator.of(context);
      await app.pair(parsed.host, parsed.pin, alts: parsed.alts);
      if (!mounted) return;
      app.setTab(AppState.tabLibrary);
      navigator.maybePop();
      messenger.showSnackBar(
        const SnackBar(content: Text('Paired — syncing library…')),
      );
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(AppState.describePairFailure(e))),
        );
      }
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    return Container(
      decoration: kVynlBackground,
      child: Scaffold(
        backgroundColor: Colors.transparent,
        body: SafeArea(
          child: Column(
            children: [
              Align(
                alignment: Alignment.centerLeft,
                child: IconButton(
                  icon: const Icon(Icons.arrow_back_rounded),
                  onPressed: () => Navigator.of(context).maybePop(),
                ),
              ),
              const Spacer(),
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 34),
                child: Column(
                  children: [
                    Container(
                      width: 108,
                      height: 108,
                      decoration: BoxDecoration(
                        borderRadius:
                            BorderRadius.circular(VynlRadius.control),
                        gradient: LinearGradient(
                          begin: Alignment.topLeft,
                          end: Alignment.bottomRight,
                          colors: [
                            VynlColors.accent.withValues(alpha: 0.33),
                            VynlColors.accent.withValues(alpha: 0.10),
                          ],
                        ),
                      ),
                      child: const Icon(
                        Icons.qr_code_scanner_rounded,
                        size: 48,
                        color: VynlColors.text,
                      ),
                    ),
                    const SizedBox(height: 30),
                    const Text(
                      'Pair with desktop',
                      style: TextStyle(
                        fontFamily: VynlFonts.display,
                        fontSize: 26,
                        fontWeight: FontWeight.w800,
                        letterSpacing: -0.7,
                        color: VynlColors.text,
                      ),
                    ),
                    const SizedBox(height: 12),
                    const Text(
                      'On your PC open Vynl → Settings → Mobile Sync, '
                      'then point your camera at the code shown there.',
                      textAlign: TextAlign.center,
                      style: TextStyle(
                        color: VynlColors.dim,
                        fontSize: 14,
                        height: 1.6,
                      ),
                    ),
                  ],
                ),
              ),
              const Spacer(flex: 2),
              Padding(
                padding: const EdgeInsets.fromLTRB(24, 0, 24, 26),
                child: FilledButton.icon(
                  onPressed: _busy ? null : _scan,
                  icon: _busy
                      ? SizedBox(
                          width: 18,
                          height: 18,
                          child: CircularProgressIndicator(
                            strokeWidth: 2.2,
                            color: VynlColors.accentOn,
                          ),
                        )
                      : const Icon(Icons.qr_code_scanner_rounded),
                  label: Text(_busy ? 'Scanning…' : 'Scan pairing code'),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
