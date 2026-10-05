import 'exceptions.dart';

/// Output encoding.
enum SafeImageFormat {
  /// Lossy, small. Transparent areas become white.
  jpeg,

  /// Lossless, keeps transparency. Size is reduced by resolution only.
  png,
}

/// Size helpers: `50.kb`, `2.mb`.
extension SafeImageSizeUnits on int {
  int get kb => this * 1024;
  int get mb => this * 1024 * 1024;
}

/// Allowed ranges. The Rust core enforces the same limits.
abstract final class SafeImageLimits {
  static const int minMaxBytes = 1024; // 1 KB
  static const int maxMaxBytes = 100 * 1024 * 1024; // 100 MB
  static const int maxDimension = 16384;
  static const int maxMinDimension = 4096;
}

/// What the output must look like.
class SafeImageOptions {
  /// Output is guaranteed to be at most this many bytes
  /// (or a [SafeImageErrorCode.cannotMeetTarget] error is thrown).
  final int maxBytes;

  /// Maximum output width in pixels. Aspect ratio is kept; images are never upscaled.
  final int? maxWidth;

  /// Maximum output height in pixels. Aspect ratio is kept; images are never upscaled.
  final int? maxHeight;

  final SafeImageFormat format;

  /// JPEG quality range (1–100) the search may use. Higher quality is preferred;
  /// resolution is reduced only when [minQuality] is still too big.
  final int minQuality;
  final int maxQuality;

  /// The longest side is never reduced below this many pixels.
  final int minDimension;

  /// If the input already meets every limit (size, resolution, format) it is
  /// returned unchanged — a 100 KB photo with a 500 KB limit stays 100 KB.
  final bool keepOriginalIfFits;

  /// Remove EXIF/XMP/IPTC metadata such as GPS location.
  final bool stripMetadata;

  const SafeImageOptions({
    this.maxBytes = 50 * 1024,
    this.maxWidth,
    this.maxHeight,
    this.format = SafeImageFormat.jpeg,
    this.minQuality = 40,
    this.maxQuality = 90,
    this.minDimension = 64,
    this.keepOriginalIfFits = true,
    this.stripMetadata = true,
  });

  SafeImageOptions copyWith({
    int? maxBytes,
    int? maxWidth,
    int? maxHeight,
    SafeImageFormat? format,
    int? minQuality,
    int? maxQuality,
    int? minDimension,
    bool? keepOriginalIfFits,
    bool? stripMetadata,
  }) => SafeImageOptions(
    maxBytes: maxBytes ?? this.maxBytes,
    maxWidth: maxWidth ?? this.maxWidth,
    maxHeight: maxHeight ?? this.maxHeight,
    format: format ?? this.format,
    minQuality: minQuality ?? this.minQuality,
    maxQuality: maxQuality ?? this.maxQuality,
    minDimension: minDimension ?? this.minDimension,
    keepOriginalIfFits: keepOriginalIfFits ?? this.keepOriginalIfFits,
    stripMetadata: stripMetadata ?? this.stripMetadata,
  );

  /// Throws [SafeImageException] with [SafeImageErrorCode.invalidOptions]
  /// when a value is out of range. Called automatically by [SafeImage].
  void validate() {
    Never fail(String message) =>
        throw SafeImageException(SafeImageErrorCode.invalidOptions, message);

    if (maxBytes < SafeImageLimits.minMaxBytes ||
        maxBytes > SafeImageLimits.maxMaxBytes) {
      fail(
        'maxBytes must be between ${SafeImageLimits.minMaxBytes} and '
        '${SafeImageLimits.maxMaxBytes} bytes (1 KB – 100 MB), got $maxBytes',
      );
    }
    for (final (name, q) in [
      ('minQuality', minQuality),
      ('maxQuality', maxQuality),
    ]) {
      if (q < 1 || q > 100) fail('$name must be between 1 and 100, got $q');
    }
    if (minQuality > maxQuality) {
      fail(
        'minQuality ($minQuality) must not be greater than maxQuality ($maxQuality)',
      );
    }
    if (minDimension < 1 || minDimension > SafeImageLimits.maxMinDimension) {
      fail(
        'minDimension must be between 1 and ${SafeImageLimits.maxMinDimension}, got $minDimension',
      );
    }
    for (final (name, v) in [
      ('maxWidth', maxWidth),
      ('maxHeight', maxHeight),
    ]) {
      if (v == null) continue;
      if (v < 1 || v > SafeImageLimits.maxDimension) {
        fail(
          '$name must be between 1 and ${SafeImageLimits.maxDimension}, got $v',
        );
      }
      if (v < minDimension) {
        fail(
          '$name ($v) must not be smaller than minDimension ($minDimension)',
        );
      }
    }
  }

  @override
  String toString() =>
      'SafeImageOptions(maxBytes: $maxBytes, maxWidth: $maxWidth, '
      'maxHeight: $maxHeight, format: ${format.name}, '
      'quality: $minQuality–$maxQuality, minDimension: $minDimension, '
      'keepOriginalIfFits: $keepOriginalIfFits, stripMetadata: $stripMetadata)';
}
