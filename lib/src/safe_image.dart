import 'package:flutter/foundation.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart'
    show ExternalLibrary, PanicException;

import 'exceptions.dart';
import 'options.dart';
import 'result.dart';
import 'rust/api/compress.dart' as rust;
import 'rust/api/types.dart' as rust;
import 'rust/frb_generated.dart';
import 'source.dart';

/// Entry point of the package. All heavy work runs in Rust on a background
/// thread, so these calls never block the UI.
abstract final class SafeImage {
  static Future<void>? _init;

  /// Loads the native library. Optional — every method calls it on first use.
  /// Safe to call many times.
  static Future<void> initialize({
    @visibleForTesting ExternalLibrary? externalLibrary,
  }) {
    return _init ??= RustLib.init(externalLibrary: externalLibrary).catchError((
      Object e,
    ) {
      _init = null;
      throw SafeImageException(
        SafeImageErrorCode.internal,
        'Failed to load native library: $e',
      );
    });
  }

  /// Resizes/re-encodes [source] so it fits [options].
  ///
  /// * Output size is always `<= options.maxBytes`.
  /// * Output resolution is always within `maxWidth` × `maxHeight`.
  /// * If the input already fits, it is returned as-is (see
  ///   [SafeImageOptions.keepOriginalIfFits]).
  ///
  /// Throws [SafeImageException].
  static Future<SafeImageResult> compress(
    SafeImageSource source, {
    SafeImageOptions options = const SafeImageOptions(),
  }) async {
    options.validate();
    _validateSource(source);
    await initialize();
    final o = _toRust(options);
    final r = await _guard(
      () => switch (source) {
        FileSource(:final path) => rust.compressFile(path: path, options: o),
        BytesSource(:final bytes) => rust.compressBytes(
          bytes: bytes,
          options: o,
        ),
      },
    );
    return SafeImageResult(
      bytes: r.bytes,
      width: r.width,
      height: r.height,
      format: _fromRustFormat(r.format),
      quality: r.quality,
      originalSizeBytes: r.originalSizeBytes,
      originalWidth: r.originalWidth,
      originalHeight: r.originalHeight,
      originalFormat: _fromRustInput(r.originalFormat),
      keptOriginal: r.keptOriginal,
      attempts: r.attempts,
      elapsed: Duration(milliseconds: r.elapsedMs),
    );
  }

  /// Reads format and dimensions from the header without decoding pixels.
  static Future<SafeImageInfo> info(SafeImageSource source) async {
    _validateSource(source);
    await initialize();
    final i = await _guard(
      () => switch (source) {
        FileSource(:final path) => rust.imageInfoFile(path: path),
        BytesSource(:final bytes) => rust.imageInfoBytes(bytes: bytes),
      },
    );
    return SafeImageInfo(
      format: _fromRustInput(i.format),
      width: i.width,
      height: i.height,
      sizeBytes: i.sizeBytes,
      hasOrientation: i.hasOrientation,
    );
  }

  static void _validateSource(SafeImageSource source) {
    switch (source) {
      case FileSource(:final path) when path.trim().isEmpty:
        throw const SafeImageException(
          SafeImageErrorCode.fileNotFound,
          'File path is empty',
        );
      case BytesSource(:final bytes) when bytes.isEmpty:
        throw const SafeImageException(
          SafeImageErrorCode.emptyInput,
          'Input is empty (0 bytes)',
        );
      default:
    }
  }

  static Future<T> _guard<T>(Future<T> Function() call) async {
    try {
      return await call();
    } on rust.ImageGuardError catch (e) {
      throw SafeImageException(_fromRustCode(e.code), e.message);
    } on PanicException catch (e) {
      throw SafeImageException(SafeImageErrorCode.internal, e.message);
    }
  }

  static rust.CompressOptions _toRust(SafeImageOptions o) =>
      rust.CompressOptions(
        maxBytes: o.maxBytes,
        maxWidth: o.maxWidth,
        maxHeight: o.maxHeight,
        format: switch (o.format) {
          SafeImageFormat.jpeg => rust.OutputFormat.jpeg,
          SafeImageFormat.png => rust.OutputFormat.png,
        },
        minQuality: o.minQuality,
        maxQuality: o.maxQuality,
        minDimension: o.minDimension,
        keepOriginalIfFits: o.keepOriginalIfFits,
        stripMetadata: o.stripMetadata,
      );

  static SafeImageFormat _fromRustFormat(rust.OutputFormat f) => switch (f) {
    rust.OutputFormat.jpeg => SafeImageFormat.jpeg,
    rust.OutputFormat.png => SafeImageFormat.png,
  };

  static SafeImageInputFormat _fromRustInput(rust.InputFormat f) => switch (f) {
    rust.InputFormat.jpeg => SafeImageInputFormat.jpeg,
    rust.InputFormat.png => SafeImageInputFormat.png,
    rust.InputFormat.webP => SafeImageInputFormat.webp,
    rust.InputFormat.gif => SafeImageInputFormat.gif,
    rust.InputFormat.bmp => SafeImageInputFormat.bmp,
  };

  static SafeImageErrorCode _fromRustCode(rust.ErrorCode c) => switch (c) {
    rust.ErrorCode.invalidOptions => SafeImageErrorCode.invalidOptions,
    rust.ErrorCode.emptyInput => SafeImageErrorCode.emptyInput,
    rust.ErrorCode.fileNotFound => SafeImageErrorCode.fileNotFound,
    rust.ErrorCode.fileReadFailed => SafeImageErrorCode.fileReadFailed,
    rust.ErrorCode.inputTooLarge => SafeImageErrorCode.inputTooLarge,
    rust.ErrorCode.unsupportedFormat => SafeImageErrorCode.unsupportedFormat,
    rust.ErrorCode.corruptImage => SafeImageErrorCode.corruptImage,
    rust.ErrorCode.imageTooLarge => SafeImageErrorCode.imageTooLarge,
    rust.ErrorCode.cannotMeetTarget => SafeImageErrorCode.cannotMeetTarget,
    rust.ErrorCode.encodingFailed => SafeImageErrorCode.encodingFailed,
    rust.ErrorCode.internal => SafeImageErrorCode.internal,
  };
}
