# image_guard

On-device image compression for Flutter with a **hard byte limit** and **resolution limit**, powered by a Rust core (via `flutter_rust_bridge`).

> Status: `0.1.0-dev` — compression engine done. On-device NSFW detection is the next milestone.

## Features

- **Exact size limit** — output is always `<= maxBytes`, or a clear `cannotMeetTarget` error. It never quietly returns a file that is too big.
- **Resolution limit** — `maxWidth` / `maxHeight`. The aspect ratio is kept and images are never upscaled.
- **Never makes small images worse** — a 100 KB photo with a 500 KB limit comes back unchanged.
- Picks the **highest quality that fits**: binary search on JPEG quality first, then the resolution is reduced only if needed.
- Fixes **EXIF rotation** and strips **EXIF/GPS** metadata.
- Reads JPEG, PNG, WebP, GIF and BMP, and writes JPEG or PNG.
- Rejects broken, oversized (> 100 MP) and malicious input without crashing.
- Runs on a Rust worker thread, so the UI isolate is never blocked.

## Usage

```dart
import 'package:image_guard/image_guard.dart';

final result = await SafeImage.compress(
  SafeImageSource.file(xfile.path),
  options: SafeImageOptions(
    maxBytes: 50.kb,      // user-chosen size
    maxWidth: 1080,       // user-chosen resolution (optional)
    maxHeight: 1080,
  ),
);

await api.upload(result.bytes);   // guaranteed <= 50 KB
print(result);                    // 1080x810 jpeg, 48.7 KB, quality: 72 ...
```

### Errors

```dart
try {
  await SafeImage.compress(source, options: options);
} on SafeImageException catch (e) {
  switch (e.code) {
    case SafeImageErrorCode.invalidOptions:   // bad maxBytes / quality / size
    case SafeImageErrorCode.unsupportedFormat: // e.g. HEIC, PDF
    case SafeImageErrorCode.corruptImage:
    case SafeImageErrorCode.cannotMeetTarget:  // limit too small for minDimension/minQuality
    default:
  }
}
```

### Options

| Option | Default | Allowed |
|---|---|---|
| `maxBytes` | `50 * 1024` | 1 KB – 100 MB |
| `maxWidth` / `maxHeight` | `null` (no limit) | 1 – 16384, and `>= minDimension` |
| `format` | `jpeg` | `jpeg`, `png` |
| `minQuality` / `maxQuality` | `40` / `90` | 1 – 100, min <= max |
| `minDimension` | `64` | 1 – 4096 |
| `keepOriginalIfFits` | `true` | |
| `stripMetadata` | `true` | |

## HEIC on iPhone

HEIC is not decoded yet. When you use `image_picker`, pass `imageQuality` (or `maxWidth`). iOS then converts the photo to JPEG before returning it.

## Platform support & requirements

| Platform | Minimum |
|---|---|
| Android | arm64-v8a, armeabi-v7a, x86_64, x86 |
| iOS | 13.0 (15.0 with Xcode 27+) |
| macOS | 10.15 (for development) |

**Rust toolchain required (for now).** The native library is compiled while your app builds:

1. Install Rust: https://rustup.rs
2. Android: the Android NDK (Android Studio → SDK Manager → NDK).
3. iOS: Xcode.

The first build compiles Rust and takes a few minutes. Later builds are cached. Precompiled binaries are planned, and then Rust will no longer be needed.

**Xcode 27+:** Xcode 27 no longer builds for iOS 13. Set iOS 15 in `ios/Podfile`:

```ruby
platform :ios, '15.0'

post_install do |installer|
  installer.pods_project.targets.each do |target|
    flutter_additional_ios_build_settings(target)
    target.build_configurations.each do |config|
      config.build_settings['IPHONEOS_DEPLOYMENT_TARGET'] = '15.0'
    end
  end
end
```

Also set **iOS Deployment Target = 15.0** for the Runner target in Xcode.

## Development

```bash
cargo test --manifest-path rust/Cargo.toml          # Rust unit + scenario + property tests
cargo build --release --manifest-path rust/Cargo.toml
flutter test                                         # Dart tests (uses the host build above)
cd example && flutter test integration_test -d <device>
flutter_rust_bridge_codegen generate                 # after changing rust/src/api
```
