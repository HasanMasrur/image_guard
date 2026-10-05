import 'dart:io';

import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';
import 'package:image/image.dart' as img;
import 'package:image_guard/image_guard.dart';

/// Loads the Rust library built on the host with
/// `cargo build --release --manifest-path rust/Cargo.toml`.
Future<void> initNative() async {
  final name = Platform.isMacOS
      ? 'libimage_guard.dylib'
      : Platform.isWindows
      ? 'image_guard.dll'
      : 'libimage_guard.so';
  final path = 'rust/target/release/$name';
  if (!File(path).existsSync()) {
    throw StateError(
      '$path not found. Run: cargo build --release --manifest-path rust/Cargo.toml',
    );
  }
  await SafeImage.initialize(externalLibrary: ExternalLibrary.open(path));
}

/// Photo-like test picture (gradients + noise).
img.Image photo(int w, int h, {int seed = 7}) {
  var s = seed;
  final out = img.Image(width: w, height: h);
  for (final p in out) {
    s = (s * 1103515245 + 12345) & 0x7fffffff;
    final n = (s % 24) - 12;
    p
      ..r = (255 * p.x / w + n).clamp(0, 255)
      ..g = (255 * p.y / h + n).clamp(0, 255)
      ..b = (128 + n * 4).clamp(0, 255);
  }
  return out;
}

Uint8List jpeg(img.Image i, {int quality = 90}) =>
    img.encodeJpg(i, quality: quality);

Uint8List png(img.Image i) => img.encodePng(i);

img.Image decode(Uint8List bytes) => img.decodeImage(bytes)!;
