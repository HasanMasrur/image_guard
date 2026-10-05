# image_guard — pub.dev-তে Publish করার Guide

## 🟢 এখনকার অবস্থা (আপডেট)

**যা করা হয়ে গেছে:**

| কাজ | অবস্থা |
|-----|--------|
| `pubspec.yaml` থেকে `publish_to: none` মুছে ফেলা | ✅ |
| LICENSE → MIT (Copyright: HasanMasrur) | ✅ নাম বদলাতে চাইলে `LICENSE` file-এ গিয়ে বদলে নাও |
| `pubspec.yaml`-এ `repository` + `issue_tracker` | ✅ |
| `.pubignore` (build ফাইল আর এই বাংলা guide গুলো package-এ যাবে না) | ✅ |
| `.gitignore`-এ `rust/target` | ✅ |
| Git repo + প্রথম commit + GitHub-এ push | ✅ https://github.com/HasanMasrur/image_guard |
| `dart pub publish --dry-run` | ✅ archive 281 KB, শুধু একটা warning, যেটা উপেক্ষা করা যায় (নিচে ব্যাখ্যা আছে) |

তোমার paste করা GitHub command-এর `echo "# image_guard" >> README.md` লাইনটা আমি চালাইনি। ওটা চালালে আমাদের লেখা README-এর শেষে একটা বাড়তি লাইন জুড়ে যেত। বাকি command গুলোর কাজ (`git init`, commit, `branch -M main`, `remote add`, `push`) সব করা হয়েছে।

### 👉 এখন তোমার কাজ: শুধু publish command চালানো

VS Code-এর terminal-এ project folder (`safe_image`) থেকে:

```bash
dart pub publish
```

1. File-এর একটা list আর flutter_rust_bridge-এর warning দেখাবে। এটা স্বাভাবিক।
2. প্রশ্ন আসবে `Do you want to publish image_guard 0.1.0-dev.1 ...? (y/N)` → **y** লিখে Enter চাপো
3. Browser খুলবে → Google account দিয়ে login করো → **Allow** চাপো
4. ২–৫ মিনিট পরে দেখো: https://pub.dev/packages/image_guard

> ⚠️ একবার publish করলে version আর মুছে ফেলা যায় না। মনে রেখো, এটা `-dev` version, আর এখন ব্যবহারকারীর machine-এ Rust লাগবে (README-তে লেখা আছে)।

Publish হয়ে গেলে আমাকে জানাও। পরের ধাপ: **precompiled binary** (ধাপ ২), যাতে ব্যবহারকারীর Rust না লাগে।

---

> আমি তোমার package-এর একটা কপিতে `dart pub publish --dry-run` চালিয়ে দেখেছি। এটা আসলে publish করে না, শুধু check করে। ফল: package publish করার মতো অবস্থায় আছে, archive size মাত্র **292 KB**। তবে publish করার আগে নিচের কয়েকটা জিনিস ঠিক করতে হবে।

---

## ধাপ ১: Publish-এর আগে যা ঠিক করতে হবে

| # | কী করতে হবে | কেন |
|---|-------------|-----|
| 1 | `pubspec.yaml` থেকে `publish_to: none` লাইনটা মুছে ফেলা | এই লাইন থাকলে publish করা যায় না। আমিই রেখেছিলাম, যাতে ভুল করে publish না হয়ে যায় |
| 2 | **LICENSE** file ঠিক করা | এখন শুধু `TODO: Add your license here.` লেখা আছে। পুরো license text না থাকলে pub.dev-তে score কমে যায়। **MIT** license দেওয়া ভালো |
| 3 | **GitHub repo** বানিয়ে `pubspec.yaml`-এ `repository:` field দেওয়া | Dry-run এটা না থাকায় warning দিয়েছে। এই repo পরে precompiled binary রাখার জন্যও লাগবে |
| 4 | `.pubignore` file যোগ করা | যাতে `rust/target` (কয়েকশো MB build ফাইল) ভুল করেও package-এ ঢুকে না যায় |
| 5 | Version ঠিক করা | এখন `0.1.0-dev.1`। AI অংশ এখনো বাকি, তাই এই "dev" version রাখাই ঠিক |

### একটা warning উপেক্ষা করা যায়

Dry-run আরেকটা warning দিয়েছে:

```
Your dependency on "flutter_rust_bridge" should allow more than one version.
```

এটা **ইচ্ছা করেই** করা। `flutter_rust_bridge`-এর Dart অংশ আর Rust অংশের version হুবহু না মিললে bridge চলে না। তাই `2.13.0`-এ আটকে রাখতে হয়।

---

## ধাপ ২: সবচেয়ে বড় বিষয়: যারা package ব্যবহার করবে, তাদের Rust লাগবে কিনা

### এখনকার অবস্থা

যে কেউ তোমার package ব্যবহার করলে app build করার সময় **তার machine-এ Rust compile হবে**। এর মানে:

- তাকে Rust install করতে হবে (`rustup`)
- Android/iOS-এর Rust target গুলোও install করতে হবে
- প্রথম build-এ বেশ কয়েক মিনিট লাগবে (আমাদের Android build-এ **৬ মিনিট** লেগেছিল)

এতে বেশিরভাগ Flutter developer package-টা আর ব্যবহার করবে না।

### সমাধান: Precompiled binary

```text
তুমি GitHub-এ নতুন version push করো
        │
        ▼
GitHub Actions সব platform-এর জন্য Rust library build করে
(Android arm64 / armv7 / x86_64, iOS device / simulator, macOS)
        │
        ▼
Signing key দিয়ে sign করে GitHub Release-এ upload করে
        │
        ▼
Package ব্যবহারকারী app build করে
        │
        ▼
Cargokit GitHub থেকে তৈরি binary download করে, signature যাচাই করে
→ ব্যবহারকারীর Rust লাগে না, build দ্রুত হয়
```

এটা set up করতে লাগবে:

| জিনিস | কে করবে |
|------|---------|
| একটা **public** GitHub repo | তুমি (github.com-এ "New repository") |
| Signing key (private + public) | আমি বানিয়ে দেব |
| Private key-টা GitHub Secret-এ রাখা | তুমি (Repo → Settings → Secrets → Actions) |
| CI workflow file (`.github/workflows/...`) | আমি লিখে দেব |
| `cargokit.yaml`-এ public key আর repo-র ঠিকানা | আমি লিখে দেব |

---

## ধাপ ৩: Publish করা

এই ধাপ **তোমাকেই** চালাতে হবে, কারণ এখানে তোমার Google account দিয়ে login লাগে।

Project folder-এ terminal খুলে:

```bash
# ১. আরেকবার check
dart pub publish --dry-run

# ২. আসল publish
dart pub publish
```

`dart pub publish` চালালে:

1. সব file-এর list দেখাবে
2. জিজ্ঞেস করবে `Do you want to publish image_guard 0.1.0-dev.1 to https://pub.dev (y/N)?` → **y** লিখে Enter
3. প্রথমবার হলে browser খুলবে → Google account দিয়ে login → Allow
4. কয়েক মিনিট পরে `https://pub.dev/packages/image_guard`-এ package দেখা যাবে

> ⚠️ **সাবধান:** একবার publish করলে সেই version **আর মুছে ফেলা যায় না**। বড়জোর "retracted" (৭ দিনের মধ্যে) বা "discontinued" mark করা যায়। তাই publish-এর আগে সব ঠিক আছে কিনা ভালো করে দেখে নিও।

### Publish-এর পরে

- pub.dev page-এ **"Pub Points"** দেখো (সর্বোচ্চ 160)। কোথায় পয়েন্ট কেটেছে সেটাও ওখানে লেখা থাকে।
- প্রথমবার publish করার পর চাইলে একটা **Verified Publisher** বানাতে পারো (নিজের domain লাগে)। এতে package-এর পাশে ✓ চিহ্ন আসে। এটা না করলেও চলে।

---

## আমার পরামর্শ: কোন ক্রমে কাজ করবে

| ক্রম | কাজ | Version |
|-----|-----|---------|
| 1 | GitHub repo বানানো + ধাপ ১-এর ৫টা কাজ + `git init` | — |
| 2 | **এখনই dev version publish করো**, যাতে `image_guard` নামটা অন্য কেউ নিয়ে না নেয়। README-তে লেখা থাকবে যে এটা dev version আর এখন Rust লাগে | `0.1.0-dev.1` |
| 3 | Precompiled binary set up করা (ধাপ ২) | `0.1.0-dev.2` |
| 4 | Phase 3: AI দিয়ে 18+ ছবি ধরা | `0.1.0-dev.3` |
| 5 | Android ফোন + iPhone-এ example app দিয়ে test → আসল release | **`0.1.0`** |

---

## তোমার কাছ থেকে যা লাগবে

1. GitHub-এ একটা **খালি public repo** বানাও, নাম `image_guard`। README বা LICENSE যোগ করো না, খালি রাখো।
2. আমাকে repo-র link দাও, যেমন `https://github.com/<তোমার-username>/image_guard`
3. LICENSE-এ কোন নাম যাবে বলো (তোমার নাম বা company-র নাম)

এগুলো পেলে আমি ধাপ ১-এর ৫টা কাজ করে দেব, আর `git init` করে প্রথম commit বানিয়ে দেব। **GitHub-এ push করার আগে তোমাকে জিজ্ঞেস করে নেব।**
