# image_guard — pub.dev-তে Upload করার সম্পূর্ণ Guide

> শেষ check: 2026-10-05 · Package: `image_guard` · Version: `0.1.0-dev.1`
> GitHub: https://github.com/HasanMasrur/image_guard

---

## ১. আমি কী কী check করেছি (ফলাফল)

| # | Check | ফলাফল |
|---|-------|-------|
| 1 | pub.dev-তে `image_guard` নাম খালি আছে কিনা | ✅ খালি (এখনো কেউ নেয়নি) |
| 2 | **pana** (pub.dev-এর নিজের scoring tool) | ✅ **160 / 160 points**, পুরো নম্বর |
| 3 | `dart pub publish --dry-run` | ✅ Archive **282 KB**, কোনো error নেই |
| 4 | `flutter analyze` | ✅ কোনো issue নেই |
| 5 | Rust test (unit + scenario + property) | ✅ 75টা pass |
| 6 | Rust `clippy` (কঠোর lint) | ✅ পরিষ্কার |
| 7 | Dart test | ✅ 27টা pass |
| 8 | **নতুন Flutter app-এ package বসিয়ে iOS-এ চালানো** (ঠিক যেভাবে অন্য developer ব্যবহার করবে) | ✅ Build হয়ে simulator-এ চালু হয়েছে |
| 9 | **নতুন Flutter app-এ package বসিয়ে Android build** | ✅ ৪টা ABI-র জন্যই Rust compile হয়েছে |
| 10 | LICENSE, CHANGELOG, README, example | ✅ সব আছে |
| 11 | GitHub-এর সাথে sync | ✅ (এই guide-এর সাথে নতুন পরিবর্তনও push করা হয়েছে) |

### Check করতে গিয়ে যে সমস্যা পেয়ে ঠিক করেছি

**সমস্যা:** Plugin iOS **15.0** চাইছিল, কিন্তু নতুন Flutter app ডিফল্টে iOS **13.0** ধরে নেয়। ফলে কেউ package যোগ করলেই iOS build fail হতো।

**সমাধান:** Plugin-এর minimum আবার **iOS 13.0** করেছি। পুরনো Xcode-এ iOS 13-এর app কোনো ঝামেলা ছাড়াই চলবে। Xcode 27 যে iOS 15 চায়, সেটা যেকোনো Flutter app-এর নিয়ম, আমাদের package-এর না। কী করতে হবে তা README-তে লিখে দিয়েছি।

### দুটো warning থাকবে, এগুলো নিয়ে চিন্তা নেই

1. **`flutter_rust_bridge should allow more than one version`**: ইচ্ছা করে রাখা। Bridge-এর Dart আর Rust অংশের version হুবহু না মিললে package চলে না।
2. **"files are modified in git"**: Publish-এর আগে সব commit করা থাকলে এটা আসবে না। আমি commit করে দিয়েছি।

---

## ২. তোমার যা যা লাগবে

| জিনিস | আছে কিনা | কী করতে হবে |
|------|---------|-------------|
| Google account | — | pub.dev-তে এই account-এর নামেই package থাকবে। যে account সবসময় ব্যবহার করবে সেটা নাও |
| Mac-এ pub login | ❌ এখনো করা হয়নি | প্রথমবার `dart pub publish` চালালে browser খুলে login চাইবে |
| Terminal | ✅ | VS Code-এর terminal-ই যথেষ্ট |
| Internet | ✅ | |

---

## ৩. Upload করার ধাপ (৫ মিনিটের কাজ)

### ধাপ ১: Project folder-এ যাও

VS Code-এ `safe_image` folder খোলা থাকলে terminal এমনিতেই ওখানে থাকবে। না হলে:

```bash
cd ~/Desktop/package/safe_image
```

### ধাপ ২: শেষবার check করো

```bash
git pull
dart pub publish --dry-run
```

শেষে `Package has 1 warning.` দেখালে ঠিক আছে। ওটা flutter_rust_bridge-এর warning।

### ধাপ ৩: Publish করো

```bash
dart pub publish
```

এরপর যা হবে:

```text
1. File-এর লম্বা একটা list দেখাবে
2. flutter_rust_bridge-এর warning দেখাবে (স্বাভাবিক)
3. প্রশ্ন আসবে:
   Do you want to publish image_guard 0.1.0-dev.1 to https://pub.dev (y/N)?
   → y লিখে Enter চাপো

4. প্রথমবার হলে একটা link দেখাবে আর browser খুলবে:
   Pub needs your authorization to upload packages on your behalf.
   → Google account বেছে নাও → "Allow" চাপো
   → Browser-এ "Pub Authorized Successfully" দেখাবে

5. Terminal-এ দেখাবে:
   Successfully uploaded https://pub.dev/packages/image_guard version 0.1.0-dev.1
```

### ধাপ ৪: যাচাই করো

- ২–১০ মিনিট পরে খোলো: **https://pub.dev/packages/image_guard**
- "Scores" tab-এ pub points আসতে আরও কিছুক্ষণ লাগে। 160 আশা করা যায়।

> ⚠️ **খুব জরুরি:** একবার publish করলে সেই version **কখনো মুছে ফেলা যায় না**।
> - ৭ দিনের মধ্যে **Retract** করা যায় (Admin tab থেকে)। তখন নতুন কেউ ওই version পাবে না।
> - পুরো package **Discontinued** mark করা যায়।
> - নাম একবার নিলে আর ছেড়ে দেওয়া যায় না।

---

## ৪. Publish-এর পরে যা যা লাগবে / করতে হবে

### ক) এখনকার সবচেয়ে বড় সীমাবদ্ধতা: ব্যবহারকারীর Rust লাগবে

এখন কেউ `image_guard` ব্যবহার করলে তার machine-এ এগুলো লাগবে:

- **Rust** (https://rustup.rs থেকে install)
- Android-এর জন্য **Android NDK**
- প্রথম build-এ **৩–৬ মিনিট** বেশি সময়

README-তে এটা লেখা আছে। তবে বেশিরভাগ developer এই ঝামেলা নিতে চায় না।

**সমাধান: Precompiled binary।** GitHub Actions আগে থেকে সব platform-এর Rust library build করে GitHub Release-এ রেখে দেবে। ব্যবহারকারীর machine সেখান থেকে download করবে, Rust লাগবে না।

| কাজ | কে করবে |
|-----|---------|
| Signing key (private + public) বানানো | আমি |
| Private key GitHub-এ রাখা: repo → **Settings → Secrets and variables → Actions → New repository secret**, নাম `IMAGE_GUARD_PRIVATE_KEY` | **তুমি** (key আমি দেব) |
| GitHub Actions workflow file লেখা | আমি |
| `rust/cargokit.yaml`-এ public key আর repo-র ঠিকানা | আমি |
| GitHub-এ একটা release tag push করে test | আমি (তোমার অনুমতি নিয়ে) |

### খ) Phase 3: AI দিয়ে 18+ ছবি ধরা

Plan document (`SAFE_IMAGE_PLAN.md`) অনুযায়ী এটাই package-এর মূল feature। Model license verify, model convert করা, Rust-এ inference, তারপর test।

### গ) নিজের Android ফোনে একবার test

আমি Android build সফল হওয়া পর্যন্ত দেখেছি, কিন্তু আসল ফোনে চালাইনি (তোমার Mac-এ emulator নেই)। ফোন USB দিয়ে connect করে:

```bash
cd example
flutter run
```

App-এ KB আর resolution লিখে gallery থেকে ছবি নাও, তারপর ফলাফল দেখো।

### ঘ) পরের version কীভাবে publish করবে (ভবিষ্যতের জন্য)

প্রতিবার নতুন version দিতে:

```text
1. pubspec.yaml-এ version বাড়াও   → যেমন 0.1.0-dev.1 → 0.1.0-dev.2
2. CHANGELOG.md-এর একদম উপরে নতুন version-এর অংশ লেখো
3. git commit + git push
4. dart pub publish --dry-run
5. dart pub publish
```

Version-এর নিয়ম:

| পরিবর্তন | উদাহরণ |
|---------|---------|
| Development চলছে | `0.1.0-dev.1` → `0.1.0-dev.2` |
| প্রথম ঠিকঠাক release | `0.1.0` |
| Bug fix | `0.1.0` → `0.1.1` |
| নতুন feature | `0.1.1` → `0.2.0` |
| পুরনো API ভেঙে গেলে (1.0-এর পরে) | `1.2.0` → `2.0.0` |

### ঙ) ঐচ্ছিক: Verified Publisher

pub.dev-তে package-এর পাশে ✓ চিহ্ন দেখাতে নিজের একটা domain লাগে (যেমন `hasanmasrur.dev`)। Google Search Console-এ domain verify করে pub.dev-তে publisher বানাতে হয়। এখন না করলেও চলে, পরে package publisher-এ transfer করা যায়।

---

## ৫. আমার পরামর্শ: কোন ক্রমে কাজ করবে

| ক্রম | কাজ | Version |
|-----|-----|---------|
| 1 | ✅ সব check শেষ | — |
| 2 | 👉 **তুমি `dart pub publish` চালাও**, নামটা নিজের করে নাও | `0.1.0-dev.1` |
| 3 | Precompiled binary (ব্যবহারকারীর Rust লাগবে না) | `0.1.0-dev.2` |
| 4 | Phase 3: AI দিয়ে 18+ ছবি ধরা | `0.1.0-dev.3` |
| 5 | Android ফোন + iPhone-এ test → আসল release | **`0.1.0`** |

Publish হয়ে গেলে আমাকে জানাও। তখন ৩ নম্বর থেকে শুরু করব।
