import 'dart:io';
import 'dart:typed_data';

import 'options.dart';

/// Format detected from the input.
enum SafeImageInputFormat { jpeg, png, webp, gif, bmp }

class SafeImageResult {
  /// The encoded output. Upload this or save it with [saveTo].
  final Uint8List bytes;
  final int width;
  final int height;
  final SafeImageFormat format;

  /// JPEG quality used; `null` for PNG or when [keptOriginal] is true.
  final int? quality;

  final int originalSizeBytes;

  /// Original width/height after EXIF rotation.
  final int originalWidth;
  final int originalHeight;
  final SafeImageInputFormat originalFormat;

  /// `true` when the input already met every limit and was returned unchanged
  /// (only metadata may have been removed).
  final bool keptOriginal;

  /// Encode attempts used by the size search.
  final int attempts;
  final Duration elapsed;

  const SafeImageResult({
    required this.bytes,
    required this.width,
    required this.height,
    required this.format,
    required this.quality,
    required this.originalSizeBytes,
    required this.originalWidth,
    required this.originalHeight,
    required this.originalFormat,
    required this.keptOriginal,
    required this.attempts,
    required this.elapsed,
  });

  int get sizeBytes => bytes.length;
  double get sizeKb => sizeBytes / 1024;

  /// Output size / original size (e.g. 0.05 = 95 % smaller).
  double get ratio => sizeBytes / originalSizeBytes;

  /// `.jpg` or `.png`.
  String get fileExtension => format == SafeImageFormat.png ? '.png' : '.jpg';

  Future<File> saveTo(String path) =>
      File(path).writeAsBytes(bytes, flush: true);

  @override
  String toString() =>
      'SafeImageResult(${width}x$height ${format.name}, '
      '${sizeKb.toStringAsFixed(1)} KB, quality: $quality, '
      'original: ${originalWidth}x$originalHeight ${originalFormat.name} '
      '${(originalSizeBytes / 1024).toStringAsFixed(1)} KB, '
      'keptOriginal: $keptOriginal, ${elapsed.inMilliseconds} ms)';
}

class SafeImageInfo {
  final SafeImageInputFormat format;

  /// Width/height after EXIF rotation.
  final int width;
  final int height;
  final int sizeBytes;

  /// `true` when the file carries an EXIF rotation/flip.
  final bool hasOrientation;

  const SafeImageInfo({
    required this.format,
    required this.width,
    required this.height,
    required this.sizeBytes,
    required this.hasOrientation,
  });

  @override
  String toString() =>
      'SafeImageInfo(${format.name} ${width}x$height, $sizeBytes bytes, '
      'hasOrientation: $hasOrientation)';
}
