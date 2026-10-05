# image_guard

On-device **NSFW (18+) image detection** and **compression to an exact byte limit and resolution** for Flutter, powered by a Rust core (via `flutter_rust_bridge`).

The AI check runs on the device. Images are never uploaded anywhere to be checked.

## Features

### NSFW detection
- On-device MobileNetV2 classifier ([GantMan/nsfw_model](https://github.com/GantMan/nsfw_model), MIT). It runs with [tract](https://github.com/sonos/tract), a pure-Rust inference engine, so no TensorFlow/ONNX Runtime binaries are added to your app.
- Five classes: `drawings`, `hentai`, `neutral`, `porn`, `sexy`.
- Verdict: `safe` / `uncertain` / `unsafe`, with thresholds you can configure.
- The check runs on the full-quality image **before** compression. Unsafe images are not compressed.

### Compression
- **Exact size limit**: output is always `<= maxBytes`, or a clear `cannotMeetTarget` error. It never quietly returns a file that is too big.
- **Resolution limit**: `maxWidth` / `maxHeight`. The aspect ratio is kept and images are never upscaled.
- **Never makes small images worse**: a 100 KB photo with a 500 KB limit comes back unchanged.
- Picks the **highest quality that fits**. Resolution is reduced only if needed.
- Fixes **EXIF rotation** and strips **EXIF/GPS** metadata.
- Reads JPEG, PNG, WebP, GIF and BMP, and writes JPEG or PNG.
- Rejects broken, oversized (> 100 MP) and malicious input without crashing.

## Usage

```dart
import 'package:image_guard/image_guard.dart';

void main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await SafeImage.initialize(); // optional: loads the model up front (~0.2 s)
  runApp(const MyApp());
}

final result = await SafeImage.process(
  SafeImageSource.file(xfile.path),
  options: SafeImageOptions(
    maxBytes: 50.kb,   // user-chosen size
    maxWidth: 1080,    // user-chosen resolution (optional)
    maxHeight: 1080,
  ),
);

switch (result.verdict) {
  case Verdict.unsafe:
    showError('18+ content is not allowed'); // result.image is null
  case Verdict.uncertain:
    // Your policy: reject, or send to manual review.
    // (A case must not be empty: in Dart an empty case falls through to the next one.)
    showError('This photo needs review');
  case Verdict.safe:
    await api.upload(result.image!.bytes);   // guaranteed <= 50 KB
}

print(result.safety); // SafetyReport(safe, nsfwScore: 0.012, porn: 0.004, ...)
```

Other calls:

```dart
final report = await SafeImage.classify(source);   // NSFW check only
final small  = await SafeImage.compress(source);   // compression only (no NSFW check!)
await SafeImage.dispose();                         // free model memory
```

### Safety options

| Option | Default | Meaning |
|---|---|---|
| `rejectThreshold` | `0.7` | `nsfwScore >= this` → `unsafe` |
| `reviewThreshold` | `0.3` | `nsfwScore >= this` → `uncertain` |
| `suggestiveWeight` | `0.5` | How much `sexy` (swimwear, lingerie) counts. `0` = ignore, `1` = same as explicit |
| `compressUnsafe` | `false` | Also compress unsafe images |

`nsfwScore = porn + hentai + sexy × suggestiveWeight`. `SafetyOptions.strict` is a stricter preset.

> **Accuracy:** about 92 % validation accuracy on the upstream dataset. Like every classifier it makes mistakes (false positives and false negatives). For user-generated content, combine it with server-side moderation and/or reporting. This is **not** a CSAM detection tool.

### Errors

```dart
try {
  await SafeImage.process(source, options: options);
} on SafeImageException catch (e) {
  switch (e.code) {
    case SafeImageErrorCode.invalidOptions:   // bad maxBytes / quality / thresholds
    case SafeImageErrorCode.unsupportedFormat: // e.g. HEIC, PDF
    case SafeImageErrorCode.corruptImage:
    case SafeImageErrorCode.cannotMeetTarget:  // limit too small for minDimension/minQuality
    case SafeImageErrorCode.modelLoadFailed:
    default:
  }
}
```

### Compression options

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

## Model & license

The bundled model `assets/models/nsfw_mobilenet_v2_140_224.nnef.tar` is converted from [GantMan/nsfw_model](https://github.com/GantMan/nsfw_model) release 1.2.0 (MIT License, Copyright (c) 2020 The nsfw_model Developers). See `NOTICE` and `tool/convert_model.py`.

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
