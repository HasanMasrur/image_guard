# safe_image — সম্পূর্ণ Build Plan (Flutter + Rust FFI + On-device AI)

> **উদ্দেশ্য:** একটা Flutter package, যেটা mobile device-এর ভিতরেই (server ছাড়া)
> 1. ছবিটা 18+ / NSFW কিনা AI দিয়ে check করবে, এবং
> 2. ছবিটাকে একটা নির্দিষ্ট size-এর মধ্যে (যেমন **≤ 50 KB**) compress করবে।
>
> সব heavy কাজ (decode, resize, AI inference, compression) হবে **Rust**-এ। Dart শুধু সুন্দর async API দেবে।

এই document-টা ChatGPT conversation-এর idea গুলো পড়ে, যাচাই করে, আর যেখানে দরকার ঠিক করে লেখা। এটা পড়ে তুমি OK বললে আমি Phase 1 থেকে কাজ শুরু করব।

## ✅ Progress (2026-10-05)

| Phase | অবস্থা |
|-------|--------|
| 1 — Foundation (Flutter + Rust) | ✅ শেষ — package নাম `image_guard`, Dart class `SafeImage` |
| 2 — Image engine + dynamic size/resolution + validation | ✅ শেষ |
| 3 — AI (NSFW) | ⏳ পরের ধাপ |
| 4–6 | বাকি |

**Test:** Rust 75টা (unit 36 + scenario 36 + property 3) · Dart 27টা · iPhone simulator integration 4টা — সব pass। Android release APK build সফল।
**Speed:** 12 MP → 50 KB: Mac release ~0.4 s, iPhone simulator (debug) ~0.7 s।
**সিদ্ধান্ত যা plan থেকে বদলেছে:** iOS minimum 15.0 (Xcode 27-এর কারণে); `libc < 0.2.190` pin (0.2.190 iOS build ভাঙে); Cargo `strip = "none"` (Xcode 27-এর strip macOS dylib নষ্ট করে)।

---

## 0. শুরুতেই জানা দরকার এমন ৫টা জিনিস (ChatGPT plan-এর সংশোধন)

| # | বিষয় | কী পেলাম | কী করব |
|---|------|---------|--------|
| 1 | **নাম `safe_image`** | pub.dev-তে **`safe_image` নামটা আগেই নেওয়া** (v0.1.4, URL থেকে image loading-এর package)। ওই নামে publish করা যাবে না। | Folder-এর নাম `safe_image` থাকতে পারে, কিন্তু `pubspec.yaml`-এ অন্য নাম লাগবে। Free আছে: `image_guard`, `safe_image_guard`, `safe_image_kit`, `image_safety`, `safeimage`। **আমার পরামর্শ: `image_guard`** (ছোট, generic, future-proof)। Dart class-এর নাম তবুও `SafeImage` রাখা যায়। |
| 2 | **"≤ 50 KB guarantee"** | কোনো ছবি সব অবস্থায় দেখতে ভালো রেখে 50 KB-তে আনা যায় না। | Guarantee হবে এরকম: *হয় output ≤ maxBytes হবে, নয়তো পরিষ্কার একটা `cannotMeetTarget` error আসবে।* চুপচাপ বড় file কখনো দেবে না। |
| 3 | **কে Rust install করবে?** | Package-এ Rust থাকলে, যে developer তোমার package ব্যবহার করবে তার machine-এও সাধারণত Rust লাগে। pub.dev user-দের জন্য এটা বড় বাধা। | **Precompiled binary** (cargokit-এর feature) ব্যবহার করব: GitHub Release-এ build করা `.so`/`.a` রাখা থাকবে, user-এর Rust লাগবে না। Phase 6-এ। |
| 4 | **iPhone-এর HEIC ছবি** | iPhone camera ডিফল্টে HEIC দেয়। Rust-এর `image` crate HEIC পড়তে পারে না। `libheif` LGPL — license ঝামেলা। | V1: `image_picker`-এ `imageQuality`/`maxWidth` দিলে iOS JPEG-এ convert করে দেয় — README-তে বলব। HEIC এলে `unsupportedFormat` error। V2: iOS-এর native decoder (ImageIO) দিয়ে ছোট Swift helper। |
| 5 | **AI model-এর license** | Model package-এর ভিতরে bundle হবে, মানে তুমি model **redistribute** করছ। | শুধু MIT/BSD/Apache license-এর model নেব। নিচে Section 4-এ বিস্তারিত। |

**তোমার machine check করেছি — সব ready আছে:** Flutter 3.41.8, Rust 1.93, Android + iOS-এর সব Rust target install করা, Android NDK 29, Xcode 27।

---

## 1. পুরো Architecture এক নজরে

```text
                  App (তোমার Flutter app)
                           │
                           ▼
          ┌────────────────────────────────┐
          │  Dart API  (lib/)              │
          │  SafeImage.process(...)        │  ← async, typed result, exceptions
          └────────────────┬───────────────┘
                           │  flutter_rust_bridge (FFI, auto-generated)
                           ▼
          ┌────────────────────────────────┐
          │  Rust core  (rust/)            │
          │                                │
          │  1. Decode (+ size limits)     │
          │  2. EXIF orientation fix       │
          │  3. ছোট copy → 224×224 tensor  │──► AI classifier (tract, ONNX)
          │                                │       │
          │        verdict = unsafe? ──────┼───────┘── হ্যাঁ → এখানেই থামো, reject
          │                                │
          │  4. Resize (max width/height)  │
          │  5. Encode JPEG/WebP           │
          │  6. Target-size search ≤ 50 KB │
          └────────────────┬───────────────┘
                           ▼
            SafeImageResult { verdict, scores, bytes, size, w, h, ... }
```

**ক্রম কেন AI আগে, compression পরে?** (ChatGPT-এর এই কথাটা ঠিক)
50 KB-তে নামানো ছবি blurry/artifact-ভরা — তাতে AI ভুল করে। তাই AI দেখবে original ছবি থেকে বানানো একটা পরিষ্কার 224×224 copy। আর unsafe হলে compression-এর সময় নষ্ট করার দরকারই নেই।

---

## 2. Technology সিদ্ধান্ত (কোনটা কেন)

### 2.1 Dart ↔ Rust bridge: **flutter_rust_bridge v2** (latest 2.13.0)

| Option | সুবিধা | অসুবিধা |
|--------|-------|---------|
| **flutter_rust_bridge (FRB) ✅** | Rust function লিখলেই Dart code auto-generate হয়। Async, struct, enum, `Vec<u8>`, error, panic — সব নিজে handle করে। Android/iOS build (cargokit) template-এ আসে। Memory free করা নিয়ে ভাবতে হয় না। | একটা extra dependency + codegen step। |
| Raw `dart:ffi` + `ffigen` + `cbindgen` | পূর্ণ control, শেখার জন্য ভালো। | `malloc`/`free`, pointer, string, isolate, error — সব হাতে লিখতে হয়। ভুল হলে memory leak বা crash। |
| Dart native-assets build hooks | নতুন official পথ। | Rust-এর জন্য ecosystem এখনো FRB+cargokit-এর মতো পাকা না। |

**সিদ্ধান্ত:** FRB। ChatGPT যে "typed FFI struct + explicit free function" বলেছিল — FRB ঠিক সেটাই নিরাপদভাবে auto-generate করে। শেখার জন্য Appendix A-তে raw FFI-র ছোট উদাহরণ দিলাম, যাতে বোঝো ভিতরে কী হচ্ছে।

### 2.2 Rust crates

| কাজ | Crate | কেন |
|-----|-------|-----|
| Decode (JPEG/PNG/WebP/GIF/BMP) | `image` 0.25 | Standard। `Limits` দিয়ে "decompression bomb" আটকানো যায়। `orientation()` + `apply_orientation()` দিয়ে EXIF fix built-in। |
| Resize | `fast_image_resize` 6 | SIMD (ARM NEON) — `image`-এর resize-এর চেয়ে কয়েক গুণ দ্রুত। |
| JPEG encode | `jpeg-encoder` 0.7 | Pure Rust, দ্রুত, progressive + optimized Huffman আছে, Android/iOS-এ C toolchain লাগে না। |
| WebP encode (lossy) | `webp` 0.3 (libwebp) | `image` crate শুধু lossless WebP লেখে, তাই lossy-র জন্য libwebp লাগবে। Cargo feature `webp` দিয়ে optional রাখব। |
| AI inference | `tract-onnx` 0.23 | **Pure Rust** ONNX runtime — Android/iOS-এর জন্য আলাদা C++ library link করতে হয় না। MobileNet সাইজের model-এর জন্য যথেষ্ট দ্রুত। |
| Error | `thiserror` | Clean error enum। |

**AI runtime-এর বিকল্প কেন না:** `ort` (ONNX Runtime) বেশি দ্রুত আর GPU/NNAPI/CoreML পারে, কিন্তু mobile-এর জন্য prebuilt C++ binary link করা ঝামেলা আর binary বড়। TFLite-এর Rust binding দুর্বল। তাই V1-এ `tract`; পরে দরকার হলে `Classifier` trait-এর পিছনে `ort` backend যোগ করা যাবে — Dart API বদলাবে না।

### 2.3 Platform minimums
- Android: `minSdk 21`, ABIs: `arm64-v8a`, `armeabi-v7a`, `x86_64` (emulator)
- iOS: 15.0+ (Xcode 27 এর নিচে build করে না), device (`aarch64-apple-ios`) + simulator (`aarch64-apple-ios-sim`, `x86_64-apple-ios`)

---

## 3. Folder Structure

```text
safe_image/                         ← repo (pubspec name: image_guard বা যেটা ঠিক করবে)
├── pubspec.yaml
├── README.md  CHANGELOG.md  LICENSE  NOTICE        ← NOTICE-এ model-এর license
├── flutter_rust_bridge.yaml
├── assets/
│   └── models/
│       └── nsfw_mobilenet_v2.onnx                  ← ~3–10 MB
├── lib/
│   ├── safe_image.dart                             ← public export
│   └── src/
│       ├── safe_image.dart                         ← SafeImage class (facade)
│       ├── config.dart                             ← SafeImageConfig, OutputFormat
│       ├── result.dart                             ← SafeImageResult, Verdict, SafetyScores
│       ├── source.dart                             ← SafeImageSource.file / .bytes
│       ├── exceptions.dart                         ← SafeImageException + codes
│       └── rust/                                   ← FRB auto-generated (হাতে edit না)
├── rust/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── api/                                    ← শুধু এখানকার fn Dart-এ expose হয়
│       │   ├── mod.rs
│       │   └── engine.rs                           ← init / process / classify / compress
│       ├── error.rs
│       ├── config.rs
│       ├── decode.rs                               ← decode + limits + orientation
│       ├── resize.rs
│       ├── encode.rs                               ← jpeg / webp
│       ├── target_size.rs                          ← ≤ maxBytes algorithm
│       ├── classifier/
│       │   ├── mod.rs                              ← trait Classifier
│       │   ├── preprocess.rs                       ← 224×224, normalize
│       │   └── tract_backend.rs
│       └── pipeline.rs                             ← সব জোড়া লাগায়
├── rust_builder/                                   ← cargokit glue (template থেকে আসে)
├── android/   ios/                                 ← plugin glue (template থেকে আসে)
├── example/                                        ← demo app (image_picker দিয়ে)
├── test/                                           ← Dart unit tests
├── integration_test/                               ← real device tests
├── tool/
│   └── convert_model.py                            ← Keras → ONNX script
└── .github/workflows/ci.yml
```

---

## 4. AI Model — কোনটা নেব

| Model | License | Size | Output | মন্তব্য |
|-------|---------|------|--------|---------|
| **GantMan `nsfw_model` MobileNetV2 (224×224)** ✅ | MIT | ~9 MB (quantize করলে আরও ছোট) | 5 class: `drawings, hentai, neutral, porn, sexy` | Mobile-এর জন্য ছোট ও দ্রুত, license redistribution-এ সমস্যা নেই, multi-category (ChatGPT যে বলেছিল শুধু binary-র চেয়ে category ভালো)। |
| Yahoo `open_nsfw` | BSD-2 | ~23 MB | 1 score (sfw/nsfw) | পুরনো, ভারী (ResNet-50), category নেই। `nsfw_detector_flutter` এটা ব্যবহার করে। |
| Falconsai ViT | Apache-2.0 | ~330 MB | 2 class | Mobile-এর জন্য অনেক বড়। |
| NudeNet | license নিজে verify করতে হবে | — | Detector (box) | V2-তে ভাবা যায়, license আগে check। |

**সিদ্ধান্ত:** GantMan MobileNetV2 → `tf2onnx` দিয়ে ONNX-এ convert (`tool/convert_model.py`) → ঐচ্ছিকভাবে int8 quantize → accuracy আবার মাপা।
⚠️ Implementation-এর সময় আমি model-এর আসল repo থেকে license ফাইল, input format (NHWC `[1,224,224,3]`, pixel/255) আর class order নিজে verify করব, আর `NOTICE`-এ attribution দেব।

### Score থেকে Verdict

```text
explicit   = porn + hentai
suggestive = sexy
nsfwScore  = explicit + suggestive × suggestiveWeight      (default weight 0.5)

nsfwScore ≥ rejectThreshold (0.70)  → Verdict.unsafe     → reject
nsfwScore ≥ reviewThreshold (0.30)  → Verdict.uncertain  → app ঠিক করবে (review/allow/reject)
otherwise                            → Verdict.safe
```

সব category score result-এ থাকবে, তাই app চাইলে নিজের rule বানাতে পারবে।

**সতর্কতা (README-তে থাকবে):** On-device classifier 100% সঠিক না — false positive/negative হবে। User-generated content platform হলে server-side moderation দ্বিতীয় স্তর হিসেবে রাখো। এটা CSAM detection tool না; সে ধরনের আইনি দায়িত্বের জন্য আলাদা specialized service লাগে।

### Model কীভাবে ship হবে
- Model থাকবে Flutter **asset** হিসেবে (`assets/models/...`)। `SafeImage.initialize()` সেটা `rootBundle` থেকে পড়ে একবার Rust-এ পাঠাবে; Rust model optimize করে memory-তে রাখবে (একবারই load)।
- কেন Rust binary-তে `include_bytes!` না: Android-এ ৩টা ABI-র জন্য ৩টা `.so` — model ৩ বার copy হতো।

---

## 5. Public Dart API (developer যা দেখবে)

```dart
import 'package:image_guard/safe_image.dart';

// app start-এ একবার (model load, ~100–300 ms)
await SafeImage.initialize();

final result = await SafeImage.process(
  SafeImageSource.file(pickedFile.path),
  config: const SafeImageConfig(
    maxBytes: 50 * 1024,
    maxWidth: 1600,
    maxHeight: 1600,
    format: OutputFormat.jpeg,
    rejectThreshold: 0.70,
    reviewThreshold: 0.30,
  ),
);

switch (result.verdict) {
  case Verdict.unsafe:
    showError('এই ছবিটা upload করা যাবে না');
  case Verdict.uncertain:
    // তোমার app-এর policy
  case Verdict.safe:
    await api.upload(result.bytes!);          // ≤ 50 KB নিশ্চিত
}
```

### Result

```dart
class SafeImageResult {
  final Verdict verdict;            // safe / uncertain / unsafe
  final double nsfwScore;           // 0.0 – 1.0
  final SafetyScores scores;        // drawings, hentai, neutral, porn, sexy
  final Uint8List? bytes;           // unsafe হলে null (compressUnsafe=false হলে)
  final int? sizeBytes;
  final int? width, height;
  final OutputFormat? format;
  final int? qualityUsed;           // encoder quality যেটাতে target মিলেছে
  final int originalBytes;
  final ProcessTimings timings;     // decode / inference / encode ms
  bool get isSafe => verdict == Verdict.safe;
}
```

### আলাদা আলাদা ব্যবহারও করা যাবে

```dart
final report = await SafeImage.classify(source);     // শুধু AI
final small  = await SafeImage.compress(source, config: CompressConfig(maxBytes: 50 * 1024)); // শুধু compression
await SafeImage.dispose();                            // model memory ছেড়ে দাও
```

### Errors

```dart
class SafeImageException implements Exception {
  final SafeImageErrorCode code; // notInitialized, fileNotFound, unsupportedFormat,
                                 // corruptImage, imageTooLarge, cannotMeetTarget,
                                 // modelLoadFailed, internal
  final String message;
}
```

### Config default গুলো

| Field | Default | মানে |
|-------|---------|------|
| `maxBytes` | `50 * 1024` | Output-এর সর্বোচ্চ size |
| `maxWidth` / `maxHeight` | `1600` | শুরুর resize সীমা (aspect ratio ঠিক থাকে) |
| `minDimension` | `320` | এর নিচে নামতে হলে `cannotMeetTarget` |
| `minQuality` / `maxQuality` | `40` / `90` | Quality search-এর সীমা |
| `format` | `jpeg` | `jpeg` বা `webp` |
| `rejectThreshold` / `reviewThreshold` | `0.70` / `0.30` | Verdict |
| `suggestiveWeight` | `0.5` | `sexy` class কতটা গুনবে |
| `compressUnsafe` | `false` | Unsafe হলে compress করে সময় নষ্ট না |
| `stripMetadata` | `true` | GPS সহ EXIF বাদ (privacy) — re-encode করলে এমনিতেই বাদ যায় |

**কেন file path পাঠাই, bytes না?** 8 MB ছবি Dart memory-তে পড়ে আবার Rust-এ copy করার দরকার নেই — Rust সরাসরি file পড়ে। (`SafeImageSource.bytes(...)` ও থাকবে, যেমন camera/network থেকে আসা data-র জন্য।)

---

## 6. Rust Core — মূল logic

### 6.1 Decode (নিরাপদভাবে)

```rust
pub fn decode(bytes: &[u8]) -> Result<DynamicImage, SafeImageError> {
    let mut reader = ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
    let mut limits = Limits::default();
    limits.max_image_width  = Some(12_000);
    limits.max_image_height = Some(12_000);
    limits.max_alloc        = Some(512 * 1024 * 1024); // decompression bomb থেকে বাঁচতে
    reader.limits(limits);

    let mut decoder = reader.into_decoder()?;
    let orientation = decoder.orientation()?;          // EXIF
    let mut img = DynamicImage::from_decoder(decoder)?;
    img.apply_orientation(orientation);                // ঘুরে থাকা ছবি সোজা করো
    Ok(img)
}
```

### 6.2 Classifier

```rust
pub trait Classifier: Send + Sync {
    fn classify(&self, img: &DynamicImage) -> Result<SafetyScores, SafeImageError>;
}

pub struct TractClassifier { plan: TypedRunnableModel<TypedModel> }

impl TractClassifier {
    pub fn load(model_bytes: &[u8]) -> Result<Self, SafeImageError> {
        let plan = tract_onnx::onnx()
            .model_for_read(&mut Cursor::new(model_bytes))?
            .with_input_fact(0, f32::fact([1, 224, 224, 3]).into())?
            .into_optimized()?
            .into_runnable()?;
        Ok(Self { plan })
    }
}
// classify(): fast_image_resize দিয়ে 224×224 → f32 tensor (/255) → run → softmax output → SafetyScores
```
(আসল API implementation-এর সময় tract version অনুযায়ী মিলিয়ে নেব।)

### 6.3 Target-size algorithm (≤ 50 KB)

ChatGPT-এর idea (quality কমাও → তারপর dimension কমাও) ঠিক ছিল; আমি সেটাকে **binary search** দিয়ে দ্রুত করব:

```rust
pub fn encode_to_target(original: &RgbImage, cfg: &Config) -> Result<Encoded, SafeImageError> {
    let (mut w, mut h) = fit_within(original.dimensions(), cfg.max_width, cfg.max_height);

    loop {
        let img = resize(original, w, h);   // সবসময় original থেকে resize (বারবার resize করলে ঝাপসা হয়)
        let (mut lo, mut hi, mut best) = (cfg.min_quality, cfg.max_quality, None);

        while lo <= hi {                    // সর্বোচ্চ ~6 বার encode
            let q = (lo + hi) / 2;
            let out = encode(&img, cfg.format, q)?;
            if out.len() <= cfg.max_bytes { best = Some((q, out)); lo = q + 1; } // আরও ভালো quality চেষ্টা
            else { hi = q - 1; }
        }
        if let Some((q, bytes)) = best { return Ok(Encoded { bytes, quality: q, width: w, height: h }); }

        // সবচেয়ে কম quality-তেও বড় → dimension 15% কমাও
        w = (w as f32 * 0.85) as u32;
        h = (h as f32 * 0.85) as u32;
        if w.max(h) < cfg.min_dimension { return Err(SafeImageError::CannotMeetTarget); }
    }
}
```

**Speed optimization (Phase 5):** শুরুর dimension আন্দাজ করা — JPEG-এ মোটামুটি quality 70-এ ~0.6–1.0 bit/pixel লাগে। 50 KB ≈ 409,600 bit → ~500k–700k pixel (যেমন ~900×700)। তাই 4000×3000 থেকে সরাসরি 1600 দিয়ে শুরু না করে কাছাকাছি size থেকে শুরু করলে loop কম ঘুরবে। Original ইতিমধ্যে ≤ maxBytes আর format ঠিক থাকলে re-encode না করে (metadata strip করে) ফেরত দেওয়া যায়।

### 6.4 Pipeline

```text
process(source, config):
  bytes  = read(source)                       → fileNotFound / imageTooLarge
  img    = decode(bytes)                      → unsupportedFormat / corruptImage
  scores = classifier.classify(&img)          → notInitialized যদি model load না হয়
  verdict = decide(scores, config)
  if verdict == unsafe && !compressUnsafe → return (bytes = None)
  rgb    = img.to_rgb8()                      → alpha থাকলে সাদা background-এ flatten (JPEG-এ alpha নেই)
  enc    = encode_to_target(&rgb, config)
  return Result { verdict, scores, enc, timings }
```

### 6.5 Thread ও memory
- FRB প্রতিটা call Rust-এর worker thread-এ চালায় → Flutter UI জমে যাবে না।
- Model থাকবে `static ENGINE: RwLock<Option<Arc<TractClassifier>>>`-এ — একবার load, অনেক thread একসাথে পড়তে পারবে (একসাথে ৫টা ছবি process করলেও OK)।
- Rust-এর `Vec<u8>` → Dart-এ `Uint8List` FRB zero-copy-র মতো efficiently পাঠায়, আর ownership Dart GC-র হাতে যায় — আলাদা `free` লাগে না।
- Rust panic → FRB ধরে Dart exception বানায়, app crash করবে না। তবুও কোড-এ `unwrap()` এড়াব।
- `Cargo.toml` release profile: `lto = true`, `codegen-units = 1`, `opt-level = 3`, `strip = true` — binary ছোট ও দ্রুত।

---

## 7. ধাপে ধাপে Build Plan

প্রতিটা Phase শেষে কাজ চলছে কিনা প্রমাণ (✅ Done when) থাকবে। আগের phase পাকা না হলে পরেরটায় যাব না।

### Phase 1 — Foundation (Flutter + Rust জোড়া লাগানো)
1. `flutter_rust_bridge_codegen create safe_image --template plugin` দিয়ে plugin skeleton (cargokit সহ)।
2. `pubspec.yaml`: নাম, description, `flutter: plugin: platforms: android/ios: ffiPlugin: true`।
3. Rust-এ একটা dummy fn: `fn image_info(path: String) -> Result<ImageInfo>` (শুধু width/height/format ফেরত দেয়)।
4. Codegen চালিয়ে Dart binding বানানো।
5. Example app: একটা button → ছবি বেছে নাও → info দেখাও।

✅ Done when: Android emulator, Android ফোন, iOS simulator, iPhone — চারটাতেই example app-এ ছবির width/height দেখায়।

### Phase 2 — Image engine (AI ছাড়া)
1. `decode.rs` (limits + EXIF)। 2. `resize.rs`। 3. `encode.rs` (JPEG; WebP feature flag-এর পিছনে)।
4. `target_size.rs` algorithm। 5. `SafeImage.compress()` Dart API।
6. Rust unit tests: test-image fixture (ছোট/বড়/ঘোরানো EXIF/PNG-with-alpha/corrupt/0-byte/panorama 12000×800)।

✅ Done when: সব fixture-এ output ≤ 50 KB **অথবা** `cannotMeetTarget`; EXIF-ঘোরানো ছবি সোজা আসে; corrupt file-এ crash না, error আসে; `cargo test` সবুজ।

### Phase 3 — AI moderation
1. `tool/convert_model.py`: model download → ONNX → (int8 quantize) → `assets/models/`।
2. License verify + `NOTICE`।
3. `classifier/` (trait + tract backend + preprocess)।
4. `SafeImage.initialize()` / `classify()` / `dispose()`।
5. Evaluation: নিজের একটা ছোট labeled set (শুধু safe/বৈধ ছবি দিয়ে false-positive মাপা; NSFW side-এর জন্য public research benchmark-এর রিপোর্টকৃত মান + অল্প manual check)। Float vs int8 model-এর output তুলনা।

✅ Done when: Rust-এর output Python reference-এর সাথে একই ছবিতে ±0.01 মেলে; mid-range Android-এ inference সময় মাপা হয়েছে।

### Phase 4 — পূর্ণ Pipeline + Public API
1. `pipeline.rs` + `SafeImage.process()`।
2. Dart model class গুলো (result, config, exceptions) + dartdoc comment।
3. Error mapping Rust → `SafeImageErrorCode`।
4. Example app সুন্দর করা: before/after ছবি, size, verdict badge, category score bar, timing।

✅ Done when: example app দিয়ে গ্যালারি/ক্যামেরার ছবি → verdict + ≤ 50 KB file; unsafe-এ upload button বন্ধ।

### Phase 5 — Production quality
- **Performance:** benchmark (`criterion` Rust-এ, আর device-এ real timing)। লক্ষ্য (আন্দাজ, মেপে ঠিক করব): 12 MP ছবি mid-range Android-এ মোট ~1 সেকেন্ডের নিচে।
- **Memory:** 50 MP ছবিতে peak memory মাপা; Android Studio profiler / Xcode Instruments দিয়ে leak check (১০০টা ছবি loop-এ)।
- **Concurrency:** একসাথে ১০টা `process()` — crash/deadlock নেই।
- **Robustness:** `cargo fuzz` দিয়ে decoder-এ random bytes।
- **Binary size:** release `.so`/`.a` size মাপা; দরকার হলে `opt-level = "s"`।
- `flutter analyze`, `dart format`, `cargo clippy -- -D warnings` সব পরিষ্কার।

### Phase 6 — Publish-ready
1. **Precompiled binaries** (cargokit): GitHub Actions-এ সব target build → sign → GitHub Release-এ upload। Package user-এর Rust লাগবে না।
2. README: install, quick start, config table, accuracy disclaimer, HEIC note, model license, privacy ("ছবি device ছেড়ে যায় না")।
3. CHANGELOG, LICENSE (MIT), NOTICE, `example/`, pubspec-এ `topics`, `repository`, `issue_tracker`।
4. CI (`.github/workflows/ci.yml`): Rust fmt/clippy/test → Dart analyze/test → example Android APK build → iOS build (macOS runner)।
5. `dart pub publish --dry-run`, `pana` দিয়ে pub points check → তারপর publish `0.1.0`।
6. আগে নিজের app-এ ব্যবহার করে real data-তে threshold ঠিক করা (ChatGPT-এর এই পরামর্শটা ভালো)।

### V2 idea (পরে)
Blur detection, image quality score, perceptual hash (duplicate), face detection, violence model, `ort` backend (NNAPI/CoreML acceleration), HEIC native decode, video frame scan, web support (WASM)।

---

## 8. Testing Strategy (সারসংক্ষেপ)

| স্তর | Tool | কী test |
|------|------|---------|
| Rust unit | `cargo test` | decode, orientation, resize ratio, target size, score → verdict |
| Rust property | `proptest` | যেকোনো random size/maxBytes-এ "≤ maxBytes বা error" নিয়ম ভাঙে না |
| Rust fuzz | `cargo fuzz` | Corrupt input-এ panic/crash নেই |
| Dart unit | `flutter test` | Config validation, result mapping, exception |
| Integration | `integration_test` (real device) | পুরো pipeline Android + iOS-এ |
| Benchmark | `criterion` + device timing | speed regression ধরা |

---

## 9. ঝুঁকি ও সমাধান

| ঝুঁকি | প্রভাব | সমাধান |
|-------|-------|--------|
| Model ভুল করে (false positive/negative) | ভালো ছবি reject, খারাপ ছবি pass | `uncertain` zone, configurable threshold, সব score expose, server-side দ্বিতীয় স্তর |
| `tract` slow কিছু device-এ | UX খারাপ | ছোট model, int8 quantize, `Classifier` trait-এর পিছনে `ort` backend বিকল্প |
| Binary size বড় (tract + image) | App size বাড়ে | LTO, strip, অপ্রয়োজনীয় `image` feature বন্ধ, মাপা |
| HEIC | iPhone ছবি fail | V1 workaround + V2 native decode |
| Package user-এর Rust নেই | Install fail | Cargokit precompiled binaries |
| বিশাল ছবি (50–100 MP) | Out of memory | `Limits`, আগে header পড়ে dimension check, দরকারে JPEG-এর scaled decode |
| Model license | Publish-এ আইনি সমস্যা | শুধু MIT/BSD/Apache; NOTICE |

---

## 10. তোমার কাছ থেকে যে সিদ্ধান্ত গুলো লাগবে

1. **pub.dev নাম:** `image_guard` (আমার পছন্দ) / `safe_image_guard` / `safe_image_kit` / অন্য কিছু? (`safe_image` নেওয়া)
2. **WebP V1-এ লাগবে?** নাকি শুধু JPEG দিয়ে শুরু করি (সহজ, সব জায়গায় চলে)। আমার পরামর্শ: JPEG দিয়ে শুরু, WebP optional feature হিসেবে Phase 2-এর শেষে।
3. **Default `maxBytes` 50 KB** আর **max dimension 1600** ঠিক আছে? (50 KB-তে 1600px প্রায়ই সম্ভব না; বাস্তবে ~900–1100px-এ নামবে — profile photo/chat-এর জন্য ঠিক আছে।)
4. **GitHub repo** কোথায় হবে (precompiled binary আর CI-র জন্য লাগবে)?

এগুলো জানালে (বা "তোমার পরামর্শ মতোই করো" বললে) আমি **Phase 1** শুরু করব।

---

## Appendix A — Raw FFI কীভাবে কাজ করে (শেখার জন্য, আমরা FRB ব্যবহার করব)

Rust:
```rust
#[repr(C)]
pub struct SiBuffer { pub ptr: *mut u8, pub len: usize, pub error_code: i32 }

#[no_mangle]
pub extern "C" fn si_compress(path: *const c_char, max_bytes: u32) -> SiBuffer {
    let result = std::panic::catch_unwind(|| { /* ... */ });  // panic যেন FFI boundary পার না হয়
    // Vec<u8> → raw pointer; ownership Dart-কে দিলাম
    let mut v = result.unwrap_or_default().into_boxed_slice();
    let buf = SiBuffer { ptr: v.as_mut_ptr(), len: v.len(), error_code: 0 };
    std::mem::forget(v);
    buf
}

#[no_mangle]
pub extern "C" fn si_free_buffer(buf: SiBuffer) {
    if !buf.ptr.is_null() {
        unsafe { drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(buf.ptr, buf.len))) };
    }
}
```

Dart:
```dart
final lib = DynamicLibrary.open('libsafe_image.so');
final pathPtr = path.toNativeUtf8();
try {
  final buf = _compress(pathPtr, 50 * 1024);          // ব্যাকগ্রাউন্ডে চালাতে Isolate.run লাগবে
  final bytes = Uint8List.fromList(buf.ptr.asTypedList(buf.len)); // copy করে নাও
  _freeBuffer(buf);                                   // না দিলে memory leak!
} finally {
  malloc.free(pathPtr);
}
```

দেখো — প্রতিটা `alloc`-এর জোড়া `free`, panic আটকানো, isolate — সব হাতে। FRB ঠিক এগুলোই auto-generate করে, তাই আমরা FRB নিচ্ছি।

## Appendix B — pub.dev-তে কাছাকাছি package (তুলনা)

| Package | কী করে | কী নেই |
|---------|--------|--------|
| `nsfw_detector_flutter` | On-device NSFW (TFLite, open_nsfw) | Compression, Rust |
| `nsfw_detect` | On-device NSFW, image/video | Target-size compression |
| `flutter_image_compress` | Native (Kotlin/Swift) compression | AI, target-byte guarantee |
| `image_compression_flutter` | Compression/resize | AI |
| `rust_image_compress` | Rust compression | AI; নিজেই বলে production-ready না |

**তোমার package-এর জায়গা:** এক call-এ *AI moderation + guaranteed byte-limit compression*, Rust core, পুরো on-device — এরকম একসাথে pub.dev-তে নেই।
