// NSFW detection end-to-end (host Rust library + bundled model).
import 'dart:io';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:image/image.dart' as img;
import 'package:image_guard/image_guard.dart';

import 'helpers.dart';

const modelPath = 'assets/models/nsfw_mobilenet_v2_140_224.nnef.tar';

Matcher throwsCode(SafeImageErrorCode code) =>
    throwsA(isA<SafeImageException>().having((e) => e.code, 'code', code));

void main() {
  setUpAll(() async {
    await initNative();
    await SafeImage.initialize(modelBytes: File(modelPath).readAsBytesSync());
  });

  test('classify a neutral picture', () async {
    final r = await SafeImage.classify(
      SafeImageSource.bytes(jpeg(photo(800, 600))),
    );
    final sum = r.scores.toMap().values.reduce((a, b) => a + b);
    expect(sum, closeTo(1.0, 0.01));
    expect(r.nsfwScore, inInclusiveRange(0.0, 1.0));
    // A smooth gradient is not explicit.
    expect(r.verdict, isNot(Verdict.unsafe));
  });

  test('plain white and black images are safe', () async {
    for (final c in [255, 0]) {
      final i = img.Image(width: 300, height: 300)
        ..clear(img.ColorRgb8(c, c, c));
      final r = await SafeImage.classify(SafeImageSource.bytes(png(i)));
      expect(r.verdict, isNot(Verdict.unsafe), reason: 'color $c: $r');
    }
  });

  test('process: safe image is compressed', () async {
    final r = await SafeImage.process(
      SafeImageSource.bytes(jpeg(photo(2000, 1500))),
      options: SafeImageOptions(maxBytes: 50.kb),
      safety: const SafetyOptions(rejectThreshold: 1, reviewThreshold: 1),
    );
    expect(r.verdict, Verdict.safe);
    expect(r.image, isNotNull);
    expect(r.image!.sizeBytes, lessThanOrEqualTo(50.kb));
  });

  test('process: unsafe verdict skips compression', () async {
    // Threshold 0 makes every image "unsafe" — tests the flow, not the model.
    final r = await SafeImage.process(
      SafeImageSource.bytes(jpeg(photo(400, 300))),
      safety: const SafetyOptions(rejectThreshold: 0, reviewThreshold: 0),
    );
    expect(r.verdict, Verdict.unsafe);
    expect(r.image, isNull);
  });

  test('process: compressUnsafe still returns an image', () async {
    final r = await SafeImage.process(
      SafeImageSource.bytes(jpeg(photo(400, 300))),
      safety: const SafetyOptions(
        rejectThreshold: 0,
        reviewThreshold: 0,
        compressUnsafe: true,
      ),
    );
    expect(r.verdict, Verdict.unsafe);
    expect(r.image, isNotNull);
  });

  test('same scores for file and bytes', () async {
    final bytes = jpeg(photo(640, 480));
    final dir = await Directory.systemTemp.createTemp('ig_nsfw');
    addTearDown(() => dir.delete(recursive: true));
    final f = File('${dir.path}/a.jpg')..writeAsBytesSync(bytes);
    final a = await SafeImage.classify(SafeImageSource.bytes(bytes));
    final b = await SafeImage.classify(SafeImageSource.file(f.path));
    expect(a.nsfwScore, b.nsfwScore);
  });

  test('invalid safety options rejected before native call', () {
    expect(
      SafeImage.process(
        SafeImageSource.bytes(Uint8List(0)),
        safety: const SafetyOptions(rejectThreshold: 2),
      ),
      throwsCode(SafeImageErrorCode.invalidOptions),
    );
  });

  test('bad image gives a clear error, not a verdict', () {
    expect(
      SafeImage.classify(SafeImageSource.bytes(Uint8List.fromList([1, 2, 3]))),
      throwsCode(SafeImageErrorCode.unsupportedFormat),
    );
  });

  test('parallel classification', () async {
    final input = jpeg(photo(600, 400));
    final results = await Future.wait([
      for (var i = 0; i < 6; i++)
        SafeImage.classify(SafeImageSource.bytes(input)),
    ]);
    for (final r in results) {
      expect(r.nsfwScore, closeTo(results.first.nsfwScore, 1e-6));
    }
  });

  test('dispose then reload', () async {
    await SafeImage.dispose();
    await SafeImage.initialize(modelBytes: File(modelPath).readAsBytesSync());
    final r = await SafeImage.classify(
      SafeImageSource.bytes(jpeg(photo(100, 100))),
    );
    expect(r.scores.neutral, greaterThanOrEqualTo(0));
  });

  test('invalid model bytes', () async {
    await SafeImage.dispose();
    await expectLater(
      SafeImage.initialize(modelBytes: Uint8List.fromList([1, 2, 3])),
      throwsCode(SafeImageErrorCode.modelLoadFailed),
    );
    // Recover for any later test.
    await SafeImage.initialize(modelBytes: File(modelPath).readAsBytesSync());
  });
}
