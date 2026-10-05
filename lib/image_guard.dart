/// On-device NSFW detection and size control for Flutter, powered by Rust.
///
/// ```dart
/// final result = await SafeImage.process(
///   SafeImageSource.file(path),
///   options: SafeImageOptions(maxBytes: 50.kb, maxWidth: 1080, maxHeight: 1080),
/// );
/// if (result.isUnsafe) return; // 18+ → reject
/// upload(result.image!.bytes);
/// ```
library;

export 'src/exceptions.dart';
export 'src/options.dart';
export 'src/result.dart';
export 'src/safe_image.dart';
export 'src/safety.dart';
export 'src/source.dart';
