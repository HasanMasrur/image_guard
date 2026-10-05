## 0.1.0-dev.1

* Rust core: decode (JPEG/PNG/WebP/GIF/BMP), EXIF orientation, Lanczos3 resize, JPEG/PNG encode.
* Target-size search: output always `<= maxBytes` or `cannotMeetTarget`.
* User-defined size (`maxBytes`) and resolution (`maxWidth`/`maxHeight`) with full validation.
* Keeps inputs that already fit; lossless EXIF/GPS stripping.
* Dart API: `SafeImage.compress`, `SafeImage.info`, `SafeImageException`.
