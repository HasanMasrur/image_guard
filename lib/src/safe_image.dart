import 'package:flutter/foundation.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart'
    show ExternalLibrary, PanicException;

import 'exceptions.dart';
import 'model.dart';
import 'options.dart';
import 'result.dart';
import 'rust/api/compress.dart' as rust;
import 'rust/api/safety.dart' as rust;
import 'rust/api/types.dart' as rust;
import 'rust/frb_generated.dart';
import 'safety.dart';
import 'source.dart';

/// Entry point of the package. All heavy work runs in Rust on a background
/// thread, so these calls never block the UI.
abstract final class SafeImage {
  /// Where the NSFW model is downloaded from. Change it before first use to
  /// host the model yourself.
  static SafeImageModel model = SafeImageModel.standard;

  static Future<void>? _native;
  static Future<void>? _model;

  /// Loads the native library and the NSFW model.
  ///
  /// On the first run the model (~18 MB) is downloaded and cached; later runs
  /// load it from disk (~100–300 ms). Call [downloadModel] earlier (e.g. after
  /// login) to show progress or to avoid the wait on the first photo.
  ///
  /// [modelBytes] loads a model you ship yourself instead of downloading.
  /// Optional: every method initializes on first use. Safe to call many times.
  static Future<void> initialize({
    bool loadModel = true,
    Uint8List? modelBytes,
    @visibleForTesting ExternalLibrary? externalLibrary,
  }) async {
    await (_native ??= RustLib.init(externalLibrary: externalLibrary)
        .catchError((Object e) {
          _native = null;
          throw SafeImageException(
            SafeImageErrorCode.internal,
            'Failed to load native library: $e',
          );
        }));
    if (loadModel) await _ensureModel(modelBytes);
  }

  /// Downloads the model if it is not cached yet. Does not load it.
  ///
  /// Throws [SafeImageException] with [SafeImageErrorCode.modelDownloadFailed].
  static Future<void> downloadModel({ModelDownloadProgress? onProgress}) =>
      ensureModelFile(model, onProgress: onProgress);

  /// Whether the model is already cached on this device.
  static Future<bool> isModelDownloaded() async =>
      (await model.cachedFile()).exists();

  /// Deletes the cached model. It is downloaded again on next use.
  static Future<void> deleteDownloadedModel() async {
    await dispose();
    final file = await model.cachedFile();
    if (await file.exists()) await file.delete();
  }

  static Future<void> _ensureModel([Uint8List? modelBytes]) {
    return _model ??= () async {
      try {
        if (modelBytes != null) {
          await _guard(() => rust.loadModel(model: modelBytes));
          return;
        }
        final file = await ensureModelFile(model);
        try {
          final bytes = await file.readAsBytes();
          await _guard(() => rust.loadModel(model: bytes));
        } on SafeImageException {
          // A damaged cache would fail forever; download it again next time.
          await file.delete().catchError((_) => file);
          rethrow;
        }
      } catch (e) {
        _model = null;
        if (e is SafeImageException) rethrow;
        throw SafeImageException(
          SafeImageErrorCode.modelLoadFailed,
          'Cannot read NSFW model: $e',
        );
      }
    }();
  }

  /// Frees the model memory (~20 MB). It is loaded again on next use.
  static Future<void> dispose() async {
    final model = _model;
    _model = null;
    if (model != null) {
      await model.catchError((_) {});
      rust.unloadModel();
    }
  }

  /// Checks [source] for NSFW content, then (unless it is unsafe) compresses
  /// it to fit [options]. The image is decoded only once.
  ///
  /// ```dart
  /// final r = await SafeImage.process(SafeImageSource.file(path),
  ///     options: SafeImageOptions(maxBytes: 50.kb));
  /// if (r.isUnsafe) return showError('18+ content is not allowed');
  /// await upload(r.image!.bytes);
  /// ```
  ///
  /// Throws [SafeImageException].
  static Future<SafeImageProcessResult> process(
    SafeImageSource source, {
    SafeImageOptions options = const SafeImageOptions(),
    SafetyOptions safety = const SafetyOptions(),
  }) async {
    options.validate();
    safety.validate();
    _validateSource(source);
    await initialize();
    final o = _toRust(options);
    final s = _safetyToRust(safety);
    final r = await _guard(
      () => switch (source) {
        FileSource(:final path) => rust.processFile(
          path: path,
          options: o,
          safety: s,
        ),
        BytesSource(:final bytes) => rust.processBytes(
          bytes: bytes,
          options: o,
          safety: s,
        ),
      },
    );
    final image = r.image;
    return SafeImageProcessResult(
      safety: _fromRustReport(r.safety),
      image: image == null ? null : _fromRustResult(image),
    );
  }

  /// NSFW check only (no compression).
  static Future<SafetyReport> classify(
    SafeImageSource source, {
    SafetyOptions safety = const SafetyOptions(),
  }) async {
    safety.validate();
    _validateSource(source);
    await initialize();
    final s = _safetyToRust(safety);
    final r = await _guard(
      () => switch (source) {
        FileSource(:final path) => rust.classifyFile(path: path, safety: s),
        BytesSource(:final bytes) => rust.classifyBytes(
          bytes: bytes,
          safety: s,
        ),
      },
    );
    return _fromRustReport(r);
  }

  /// Compression only — **no NSFW check**. Use [process] for user uploads.
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
    await initialize(loadModel: false);
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
    return _fromRustResult(r);
  }

  /// Reads format and dimensions from the header without decoding pixels.
  static Future<SafeImageInfo> info(SafeImageSource source) async {
    _validateSource(source);
    await initialize(loadModel: false);
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

  static rust.SafetyOptions _safetyToRust(SafetyOptions s) =>
      rust.SafetyOptions(
        rejectThreshold: s.rejectThreshold,
        reviewThreshold: s.reviewThreshold,
        suggestiveWeight: s.suggestiveWeight,
        compressUnsafe: s.compressUnsafe,
      );

  static SafeImageResult _fromRustResult(rust.CompressResult r) =>
      SafeImageResult(
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

  static SafetyReport _fromRustReport(rust.SafetyReport r) => SafetyReport(
    verdict: switch (r.verdict) {
      rust.Verdict.safe => Verdict.safe,
      rust.Verdict.uncertain => Verdict.uncertain,
      rust.Verdict.unsafe => Verdict.unsafe,
    },
    nsfwScore: r.nsfwScore,
    scores: SafetyScores(
      drawings: r.scores.drawings,
      hentai: r.scores.hentai,
      neutral: r.scores.neutral,
      porn: r.scores.porn,
      sexy: r.scores.sexy,
    ),
    elapsed: Duration(milliseconds: r.elapsedMs),
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
    rust.ErrorCode.modelNotLoaded => SafeImageErrorCode.modelNotLoaded,
    rust.ErrorCode.modelLoadFailed => SafeImageErrorCode.modelLoadFailed,
    rust.ErrorCode.internal => SafeImageErrorCode.internal,
  };
}
