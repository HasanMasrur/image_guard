// End-to-end through Dart → FFI → Rust (host build of the Rust library).
import 'dart:io';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:image_guard/image_guard.dart';

import 'helpers.dart';

Matcher throwsCode(SafeImageErrorCode code) =>
    throwsA(isA<SafeImageException>().having((e) => e.code, 'code', code));

void main() {
  setUpAll(initNative);

  test('initialize is idempotent', () async {
    await SafeImage.initialize();
    await SafeImage.initialize();
  });

  test('big photo → 50 KB', () async {
    final input = jpeg(photo(3000, 2000));
    final r = await SafeImage.compress(
      SafeImageSource.bytes(input),
      options: SafeImageOptions(maxBytes: 50.kb),
    );
    expect(r.sizeBytes, lessThanOrEqualTo(50.kb));
    expect(r.keptOriginal, isFalse);
    expect(r.originalWidth, 3000);
    expect(r.originalFormat, SafeImageInputFormat.jpeg);
    final out = decode(r.bytes);
    expect((out.width, out.height), (r.width, r.height));
  });

  test('100 KB-ish image with a 500 KB limit is returned unchanged', () async {
    final input = jpeg(photo(500, 400), quality: 90);
    expect(input.length, lessThan(500.kb));
    final r = await SafeImage.compress(
      SafeImageSource.bytes(input),
      options: SafeImageOptions(maxBytes: 500.kb),
    );
    expect(r.keptOriginal, isTrue);
    expect(r.sizeBytes, lessThanOrEqualTo(input.length));
    expect((r.width, r.height), (500, 400));
  });

  test('resolution limit is applied with aspect ratio', () async {
    final r = await SafeImage.compress(
      SafeImageSource.bytes(jpeg(photo(2000, 1000))),
      options: SafeImageOptions(
        maxBytes: 5.mb,
        maxWidth: 1080,
        maxHeight: 1080,
      ),
    );
    expect((r.width, r.height), (1080, 540));
  });

  test('user-chosen KB values are all respected', () async {
    final input = jpeg(photo(1600, 1200));
    for (final kb in [10, 25, 50, 100, 200]) {
      final r = await SafeImage.compress(
        SafeImageSource.bytes(input),
        options: SafeImageOptions(maxBytes: kb.kb, keepOriginalIfFits: false),
      );
      expect(r.sizeBytes, lessThanOrEqualTo(kb.kb), reason: '$kb KB');
    }
  });

  test('png output keeps format', () async {
    final r = await SafeImage.compress(
      SafeImageSource.bytes(png(photo(300, 300))),
      options: SafeImageOptions(
        maxBytes: 100.kb,
        format: SafeImageFormat.png,
        keepOriginalIfFits: false,
      ),
    );
    expect(r.format, SafeImageFormat.png);
    expect(r.quality, isNull);
    expect(r.fileExtension, '.png');
  });

  test('file source and saveTo', () async {
    final dir = await Directory.systemTemp.createTemp('image_guard_test');
    addTearDown(() => dir.delete(recursive: true));
    final file = File('${dir.path}/in.jpg')
      ..writeAsBytesSync(jpeg(photo(1200, 900)));

    final info = await SafeImage.info(SafeImageSource.file(file.path));
    expect(
      (info.width, info.height, info.format),
      (1200, 900, SafeImageInputFormat.jpeg),
    );

    final r = await SafeImage.compress(
      SafeImageSource.file(file.path),
      options: SafeImageOptions(maxBytes: 30.kb),
    );
    final saved = await r.saveTo('${dir.path}/out${r.fileExtension}');
    expect(saved.lengthSync(), r.sizeBytes);
  });

  group('errors', () {
    test('invalid options are rejected in Dart before native code runs', () {
      expect(
        SafeImage.compress(
          SafeImageSource.bytes(Uint8List(0)),
          options: const SafeImageOptions(maxBytes: 10),
        ),
        throwsCode(SafeImageErrorCode.invalidOptions),
      );
    });
    test('missing file', () {
      expect(
        SafeImage.compress(const SafeImageSource.file('/no/such/file.jpg')),
        throwsCode(SafeImageErrorCode.fileNotFound),
      );
    });
    test('empty path', () {
      expect(
        SafeImage.compress(const SafeImageSource.file('')),
        throwsCode(SafeImageErrorCode.fileNotFound),
      );
    });
    test('empty bytes', () {
      expect(
        SafeImage.compress(SafeImageSource.bytes(Uint8List(0))),
        throwsCode(SafeImageErrorCode.emptyInput),
      );
    });
    test('not an image', () {
      expect(
        SafeImage.compress(
          SafeImageSource.bytes(Uint8List.fromList('hello world'.codeUnits)),
        ),
        throwsCode(SafeImageErrorCode.unsupportedFormat),
      );
    });
    test('impossible target', () {
      expect(
        SafeImage.compress(
          SafeImageSource.bytes(jpeg(photo(1500, 1500))),
          options: const SafeImageOptions(maxBytes: 1024, minDimension: 1500),
        ),
        throwsCode(SafeImageErrorCode.cannotMeetTarget),
      );
    });
    test('message is helpful', () async {
      try {
        await SafeImage.compress(
          SafeImageSource.bytes(jpeg(photo(1500, 1500))),
          options: const SafeImageOptions(maxBytes: 1024, minDimension: 1500),
        );
        fail('should throw');
      } on SafeImageException catch (e) {
        expect(e.message, contains('Increase maxBytes'));
      }
    });
  });

  test('many calls in parallel', () async {
    final input = jpeg(photo(1200, 800));
    final results = await Future.wait([
      for (var kb = 20; kb <= 90; kb += 10)
        SafeImage.compress(
          SafeImageSource.bytes(input),
          options: SafeImageOptions(maxBytes: kb.kb, keepOriginalIfFits: false),
        ),
    ]);
    for (final (i, r) in results.indexed) {
      expect(r.sizeBytes, lessThanOrEqualTo((20 + i * 10).kb));
    }
  });
}
