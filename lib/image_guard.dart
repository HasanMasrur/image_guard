/// On-device image safety and size control for Flutter, powered by Rust.
///
/// ```dart
/// final result = await SafeImage.compress(
///   SafeImageSource.file(path),
///   options: SafeImageOptions(maxBytes: 50.kb, maxWidth: 1080, maxHeight: 1080),
/// );
/// ```
library;

export 'src/exceptions.dart';
export 'src/options.dart';
export 'src/result.dart';
export 'src/safe_image.dart';
export 'src/source.dart';
