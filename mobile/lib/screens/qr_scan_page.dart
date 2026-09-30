import 'dart:async';

import 'package:flutter/material.dart';
import 'package:mobile_scanner/mobile_scanner.dart';

import '../theme.dart';

class QrScanPage extends StatefulWidget {
  const QrScanPage({super.key});

  @override
  State<QrScanPage> createState() => _QrScanPageState();
}

class _QrScanPageState extends State<QrScanPage> {
  late final MobileScannerController _controller;
  bool _done = false;

  @override
  void initState() {
    super.initState();
    _controller = MobileScannerController(
      autoStart: false,
      detectionSpeed: DetectionSpeed.noDuplicates,
    );
    unawaited(_controller.start());
  }

  @override
  void dispose() {
    unawaited(_controller.dispose());
    super.dispose();
  }

  Future<void> _allowCamera() async {
    await _controller.start();
  }

  void _onDetect(BarcodeCapture capture) {
    if (_done) return;
    for (final barcode in capture.barcodes) {
      final raw = barcode.rawValue;
      if (raw == null) continue;
      final trimmed = raw.trim();
      if (trimmed.startsWith('vynl://') || trimmed.contains('http')) {
        _done = true;
        Navigator.of(context).pop(trimmed);
        return;
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Scan QR')),
      body: MobileScanner(
        controller: _controller,
        errorBuilder: _buildError,
        onDetect: _onDetect,
      ),
      bottomNavigationBar: const SafeArea(
        child: Padding(
          padding: EdgeInsets.all(16),
          child: Text(
            'Point at the QR shown in Vynl desktop Settings → Mobile Sync.',
            textAlign: TextAlign.center,
            style: TextStyle(color: VynlColors.dim),
          ),
        ),
      ),
    );
  }

  Widget _buildError(
    BuildContext context,
    MobileScannerException error,
    Widget? child,
  ) {
    debugPrint(
      'mobile_scanner ${error.errorCode.name} '
      'code=${error.errorDetails?.code} '
      'message=${error.errorDetails?.message} '
      'details=${error.errorDetails?.details}',
    );

    final denied =
        error.errorCode == MobileScannerErrorCode.permissionDenied;
    final unsupported = error.errorCode == MobileScannerErrorCode.unsupported;

    final title = unsupported
        ? 'Scanning unavailable'
        : denied
            ? 'Camera access needed'
            : 'Camera unavailable';

    final message = unsupported
        ? 'This device has no camera Vynl can scan with.'
        : denied
            ? 'Vynl needs the camera to read the pairing code.'
            : 'The camera could not start. Try again.';

    return _ScannerMessage(
      icon: denied || unsupported
          ? Icons.no_photography_outlined
          : Icons.videocam_off_outlined,
      title: title,
      message: message,
      actionLabel: unsupported ? null : 'Try again',
      onAction: unsupported ? null : _allowCamera,
    );
  }
}

class _ScannerMessage extends StatelessWidget {
  const _ScannerMessage({
    required this.icon,
    required this.title,
    required this.message,
    this.actionLabel,
    this.onAction,
  });

  final IconData icon;
  final String title;
  final String message;
  final String? actionLabel;
  final VoidCallback? onAction;

  @override
  Widget build(BuildContext context) {
    final action = actionLabel;
    return ColoredBox(
      color: VynlColors.bg,
      child: Center(
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 34),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(icon, size: 34, color: VynlColors.faint),
              const SizedBox(height: 18),
              Text(
                title,
                textAlign: TextAlign.center,
                style: const TextStyle(
                  fontFamily: VynlFonts.display,
                  fontSize: 19,
                  fontWeight: FontWeight.w700,
                  letterSpacing: -0.3,
                  color: VynlColors.text,
                ),
              ),
              const SizedBox(height: 8),
              Text(
                message,
                textAlign: TextAlign.center,
                style: const TextStyle(
                  color: VynlColors.dim,
                  fontSize: 13.5,
                  height: 1.55,
                ),
              ),
              if (action != null && onAction != null) ...[
                const SizedBox(height: 22),
                OutlinedButton(
                  onPressed: onAction,
                  style: OutlinedButton.styleFrom(
                    minimumSize: const Size(0, 44),
                    padding: const EdgeInsets.symmetric(horizontal: 22),
                  ),
                  child: Text(action),
                ),
              ],
            ],
          ),
        ),
      ),
    );
  }
}
