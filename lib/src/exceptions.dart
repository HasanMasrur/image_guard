/// Why an operation failed.
enum SafeImageErrorCode {
  /// An option is out of range or contradicts another option.
  invalidOptions,

  /// The input has 0 bytes.
  emptyInput,
  fileNotFound,

  /// The path exists but cannot be read (directory, permission, I/O error).
  fileReadFailed,

  /// The input file is larger than 100 MB.
  inputTooLarge,

  /// Not JPEG, PNG, WebP, GIF or BMP (e.g. HEIC or a PDF).
  unsupportedFormat,

  /// The file is damaged or truncated.
  corruptImage,

  /// More than 30 000 px per side or 100 megapixels.
  imageTooLarge,

  /// Even at `minQuality` and `minDimension` the output is bigger than `maxBytes`.
  cannotMeetTarget,
  encodingFailed,
  internal,
}

class SafeImageException implements Exception {
  final SafeImageErrorCode code;
  final String message;

  const SafeImageException(this.code, this.message);

  @override
  String toString() => 'SafeImageException(${code.name}): $message';
}
