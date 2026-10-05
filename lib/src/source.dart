import 'dart:typed_data';

/// Where the image comes from.
sealed class SafeImageSource {
  const SafeImageSource();

  /// A file on disk, e.g. `XFile.path` from `image_picker`.
  /// Rust reads the file directly, so the bytes are never copied through Dart.
  const factory SafeImageSource.file(String path) = FileSource;

  /// Encoded image bytes (JPEG, PNG, WebP, GIF or BMP), e.g. from the network.
  const factory SafeImageSource.bytes(Uint8List bytes) = BytesSource;
}

final class FileSource extends SafeImageSource {
  final String path;
  const FileSource(this.path);

  @override
  String toString() => 'SafeImageSource.file($path)';
}

final class BytesSource extends SafeImageSource {
  final Uint8List bytes;
  const BytesSource(this.bytes);

  @override
  String toString() => 'SafeImageSource.bytes(${bytes.length} bytes)';
}
