## 0.1.0-dev.3

* **The NSFW model is no longer bundled** (−18 MB app size). It is downloaded on
  first use from this package's GitHub repo, verified with SHA-256 and cached.
* New `SafeImage.downloadModel(onProgress:)`, `isModelDownloaded()`,
  `deleteDownloadedModel()` and `SafeImage.model` (`SafeImageModel`) to self-host.
* `SafeImage.initialize(modelBytes:)` is now public, to ship the model yourself.
* New error code: `modelDownloadFailed`.
* Android: the plugin adds the `INTERNET` permission. macOS apps need the
  `com.apple.security.network.client` entitlement.

## 0.1.0-dev.2

* **NSFW (18+) detection, on-device.** MobileNetV2 model (GantMan/nsfw_model, MIT)
  bundled as an ONNX asset and run with `tract` (pure Rust).
* New `SafeImage.process()`: NSFW check first, then compression; unsafe images are not compressed.
* New `SafeImage.classify()`, `SafetyOptions`, `SafetyReport`, `Verdict` (safe / uncertain / unsafe).
* `SafeImage.initialize()` now also loads the model; `SafeImage.dispose()` frees it.
* New error codes: `modelLoadFailed`, `modelNotLoaded`.
* Plugin iOS minimum back to 13.0 (Xcode 27 users: see README).

## 0.1.0-dev.1

* Rust core: decode (JPEG/PNG/WebP/GIF/BMP), EXIF orientation, Lanczos3 resize, JPEG/PNG encode.
* Target-size search: output always `<= maxBytes` or `cannotMeetTarget`.
* User-defined size (`maxBytes`) and resolution (`maxWidth`/`maxHeight`) with full validation.
* Keeps inputs that already fit; lossless EXIF/GPS stripping.
* Dart API: `SafeImage.compress`, `SafeImage.info`, `SafeImageException`.
