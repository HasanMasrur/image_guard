// Runs on a real device / simulator: exercises the native library that
// cargokit built for that platform.
//   cd example && flutter test integration_test -d <device>
import 'dart:io';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:image/image.dart' as img;
import 'package:image_guard/image_guard.dart';
import 'package:integration_test/integration_test.dart';

Uint8List photoJpeg(int w, int h, {int quality = 92}) {
  var s = 7;
  final out = img.Image(width: w, height: h);
  for (final p in out) {
    s = (s * 1103515245 + 12345) & 0x7fffffff;
    final n = (s % 24) - 12;
    p
      ..r = (255 * p.x / w + n).clamp(0, 255)
      ..g = (255 * p.y / h + n).clamp(0, 255)
      ..b = (128 + n * 4).clamp(0, 255);
  }
  return img.encodeJpg(out, quality: quality);
}

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  test('12 MP photo → 50 KB on device', () async {
    final input = photoJpeg(4000, 3000);
    final r = await SafeImage.compress(
      SafeImageSource.bytes(input),
      options: SafeImageOptions(maxBytes: 50.kb),
    );
    // ignore: avoid_print
    print('device: $r');
    expect(r.sizeBytes, lessThanOrEqualTo(50.kb));
    expect(img.decodeJpg(r.bytes), isNotNull);
  });

  test('resolution + size from a file', () async {
    final dir = await Directory.systemTemp.createTemp('ig');
    final f = File('${dir.path}/a.jpg')
      ..writeAsBytesSync(photoJpeg(3000, 2000));
    final r = await SafeImage.compress(
      SafeImageSource.file(f.path),
      options: SafeImageOptions(
        maxBytes: 80.kb,
        maxWidth: 1080,
        maxHeight: 1080,
      ),
    );
    expect(r.width, lessThanOrEqualTo(1080));
    expect(r.height, lessThanOrEqualTo(1080));
    expect(r.sizeBytes, lessThanOrEqualTo(80.kb));
    await dir.delete(recursive: true);
  });

  test('small image with large limit is kept', () async {
    final input = photoJpeg(300, 200, quality: 80);
    final r = await SafeImage.compress(
      SafeImageSource.bytes(input),
      options: SafeImageOptions(maxBytes: 500.kb),
    );
    expect(r.keptOriginal, isTrue);
  });

  test('model loads from the package asset and classifies', () async {
    await SafeImage.initialize();
    final r = await SafeImage.classify(
      SafeImageSource.bytes(photoJpeg(800, 600)),
    );
    // ignore: avoid_print
    print('device classify: $r');
    expect(r.verdict, isNot(Verdict.unsafe));
    final sum = r.scores.toMap().values.reduce((a, b) => a + b);
    expect(sum, closeTo(1, 0.01));
  });

  test('process: 12 MP photo → safe + 50 KB on device', () async {
    final r = await SafeImage.process(
      SafeImageSource.bytes(photoJpeg(4000, 3000)),
      options: SafeImageOptions(maxBytes: 50.kb),
    );
    // ignore: avoid_print
    print('device process: $r');
    expect(r.isUnsafe, isFalse);
    expect(r.image!.sizeBytes, lessThanOrEqualTo(50.kb));
  });

  test('process: unsafe verdict blocks the image', () async {
    final r = await SafeImage.process(
      SafeImageSource.bytes(photoJpeg(300, 200)),
      safety: const SafetyOptions(rejectThreshold: 0, reviewThreshold: 0),
    );
    expect(r.isUnsafe, isTrue);
    expect(r.image, isNull);
  });

  test('errors map to SafeImageException', () async {
    await expectLater(
      SafeImage.compress(SafeImageSource.bytes(Uint8List.fromList([1, 2, 3]))),
      throwsA(isA<SafeImageException>()),
    );
  });
}
