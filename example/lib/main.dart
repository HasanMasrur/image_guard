import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:image_guard/image_guard.dart';
import 'package:image_picker/image_picker.dart';

void main() => runApp(const MaterialApp(home: DemoPage()));

class DemoPage extends StatefulWidget {
  const DemoPage({super.key});

  @override
  State<DemoPage> createState() => _DemoPageState();
}

class _DemoPageState extends State<DemoPage> {
  final _kb = TextEditingController(text: '50');
  final _maxWidth = TextEditingController(text: '1080');
  final _maxHeight = TextEditingController(text: '1080');
  SafeImageFormat _format = SafeImageFormat.jpeg;
  bool _keepOriginal = true;

  XFile? _picked;
  Uint8List? _original;
  SafeImageResult? _result;
  String? _error;
  bool _busy = false;

  int? _parse(TextEditingController c) =>
      c.text.trim().isEmpty ? null : int.tryParse(c.text.trim());

  Future<void> _pick(ImageSource source) async {
    final file = await ImagePicker().pickImage(source: source);
    if (file == null) return;
    _picked = file;
    _original = await file.readAsBytes();
    await _run();
  }

  Future<void> _run() async {
    final file = _picked;
    if (file == null) return;
    setState(() {
      _busy = true;
      _error = null;
      _result = null;
    });
    try {
      final options = SafeImageOptions(
        maxBytes: (_parse(_kb) ?? 0) * 1024,
        maxWidth: _parse(_maxWidth),
        maxHeight: _parse(_maxHeight),
        format: _format,
        keepOriginalIfFits: _keepOriginal,
      );
      final r = await SafeImage.compress(
        SafeImageSource.file(file.path),
        options: options,
      );
      setState(() => _result = r);
    } on SafeImageException catch (e) {
      setState(() => _error = '${e.code.name}: ${e.message}');
    } finally {
      setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final r = _result;
    return Scaffold(
      appBar: AppBar(title: const Text('image_guard demo')),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          Row(
            children: [
              Expanded(child: _field(_kb, 'Max size (KB)')),
              const SizedBox(width: 8),
              Expanded(child: _field(_maxWidth, 'Max width')),
              const SizedBox(width: 8),
              Expanded(child: _field(_maxHeight, 'Max height')),
            ],
          ),
          const SizedBox(height: 8),
          Wrap(
            spacing: 8,
            crossAxisAlignment: WrapCrossAlignment.center,
            children: [
              SegmentedButton<SafeImageFormat>(
                segments: const [
                  ButtonSegment(
                    value: SafeImageFormat.jpeg,
                    label: Text('JPEG'),
                  ),
                  ButtonSegment(value: SafeImageFormat.png, label: Text('PNG')),
                ],
                selected: {_format},
                onSelectionChanged: (s) => setState(() => _format = s.first),
              ),
              FilterChip(
                label: const Text('Keep original if it fits'),
                selected: _keepOriginal,
                onSelected: (v) => setState(() => _keepOriginal = v),
              ),
            ],
          ),
          const SizedBox(height: 8),
          Wrap(
            spacing: 8,
            children: [
              FilledButton.icon(
                onPressed: _busy ? null : () => _pick(ImageSource.gallery),
                icon: const Icon(Icons.photo),
                label: const Text('Gallery'),
              ),
              FilledButton.tonalIcon(
                onPressed: _busy ? null : () => _pick(ImageSource.camera),
                icon: const Icon(Icons.camera_alt),
                label: const Text('Camera'),
              ),
              OutlinedButton(
                onPressed: _busy || _picked == null ? null : _run,
                child: const Text('Apply again'),
              ),
            ],
          ),
          const SizedBox(height: 16),
          if (_busy) const LinearProgressIndicator(),
          if (_error != null)
            Text(
              _error!,
              style: TextStyle(color: Theme.of(context).colorScheme.error),
            ),
          if (r != null) ...[
            Text(
              'Before: ${r.originalWidth}×${r.originalHeight} '
              '${r.originalFormat.name}, ${(r.originalSizeBytes / 1024).toStringAsFixed(1)} KB\n'
              'After:  ${r.width}×${r.height} ${r.format.name}, '
              '${r.sizeKb.toStringAsFixed(1)} KB'
              '${r.quality != null ? ', quality ${r.quality}' : ''}\n'
              '${r.keptOriginal ? 'Original kept (already fits)' : '${r.attempts} encode attempts'}'
              ' · ${r.elapsed.inMilliseconds} ms',
            ),
            const SizedBox(height: 12),
            Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                if (_original != null)
                  Expanded(child: _preview('Original', _original!)),
                const SizedBox(width: 8),
                Expanded(child: _preview('Result', r.bytes)),
              ],
            ),
          ],
        ],
      ),
    );
  }

  Widget _field(TextEditingController c, String label) => TextField(
    controller: c,
    keyboardType: TextInputType.number,
    decoration: InputDecoration(
      labelText: label,
      border: const OutlineInputBorder(),
    ),
  );

  Widget _preview(String title, Uint8List bytes) => Column(
    crossAxisAlignment: CrossAxisAlignment.start,
    children: [
      Text(title),
      Image.memory(bytes, fit: BoxFit.contain),
    ],
  );
}
