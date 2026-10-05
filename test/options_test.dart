// Pure Dart: no native library needed.
import 'package:flutter_test/flutter_test.dart';
import 'package:image_guard/image_guard.dart';

Matcher throwsInvalid(String field) => throwsA(
  isA<SafeImageException>()
      .having((e) => e.code, 'code', SafeImageErrorCode.invalidOptions)
      .having((e) => e.message, 'message', contains(field)),
);

void main() {
  test('defaults are valid and match the documented values', () {
    const o = SafeImageOptions();
    o.validate();
    expect(o.maxBytes, 50 * 1024);
    expect(o.format, SafeImageFormat.jpeg);
    expect(o.keepOriginalIfFits, isTrue);
    expect(o.stripMetadata, isTrue);
  });

  test('size units', () {
    expect(50.kb, 51200);
    expect(2.mb, 2097152);
  });

  group('maxBytes', () {
    test('lower bound 1 KB', () {
      expect(
        () => const SafeImageOptions(maxBytes: 1023).validate(),
        throwsInvalid('maxBytes'),
      );
      const SafeImageOptions(maxBytes: 1024).validate();
    });
    test('upper bound 100 MB', () {
      SafeImageOptions(maxBytes: 100.mb).validate();
      expect(
        () => SafeImageOptions(maxBytes: 100.mb + 1).validate(),
        throwsInvalid('maxBytes'),
      );
    });
    test('zero and negative', () {
      expect(
        () => const SafeImageOptions(maxBytes: 0).validate(),
        throwsInvalid('maxBytes'),
      );
      expect(
        () => const SafeImageOptions(maxBytes: -5).validate(),
        throwsInvalid('maxBytes'),
      );
    });
  });

  group('quality', () {
    test('range 1..100', () {
      expect(
        () => const SafeImageOptions(minQuality: 0).validate(),
        throwsInvalid('minQuality'),
      );
      expect(
        () => const SafeImageOptions(maxQuality: 101).validate(),
        throwsInvalid('maxQuality'),
      );
      // Would wrap around to 44 if it reached the u8 field unchecked.
      expect(
        () => const SafeImageOptions(maxQuality: 300).validate(),
        throwsInvalid('maxQuality'),
      );
    });
    test('min must not exceed max', () {
      expect(
        () => const SafeImageOptions(minQuality: 80, maxQuality: 70).validate(),
        throwsInvalid('minQuality'),
      );
      const SafeImageOptions(minQuality: 70, maxQuality: 70).validate();
    });
  });

  group('resolution', () {
    test('positive and within 16384', () {
      expect(
        () => const SafeImageOptions(maxWidth: 0).validate(),
        throwsInvalid('maxWidth'),
      );
      expect(
        () => const SafeImageOptions(maxHeight: -1).validate(),
        throwsInvalid('maxHeight'),
      );
      expect(
        () => const SafeImageOptions(maxWidth: 16385).validate(),
        throwsInvalid('maxWidth'),
      );
      const SafeImageOptions(maxWidth: 1080, maxHeight: 1920).validate();
    });
    test('cannot be smaller than minDimension', () {
      expect(
        () =>
            const SafeImageOptions(maxWidth: 100, minDimension: 200).validate(),
        throwsInvalid('maxWidth'),
      );
    });
    test('minDimension range', () {
      expect(
        () => const SafeImageOptions(minDimension: 0).validate(),
        throwsInvalid('minDimension'),
      );
      expect(
        () => const SafeImageOptions(minDimension: 4097).validate(),
        throwsInvalid('minDimension'),
      );
    });
  });

  test('copyWith changes only given fields', () {
    const a = SafeImageOptions(maxBytes: 1024 * 100, maxWidth: 800);
    final b = a.copyWith(maxQuality: 75);
    expect(b.maxBytes, a.maxBytes);
    expect(b.maxWidth, 800);
    expect(b.maxQuality, 75);
  });

  test('sources describe themselves', () {
    expect(const SafeImageSource.file('/a.jpg').toString(), contains('/a.jpg'));
  });
}
