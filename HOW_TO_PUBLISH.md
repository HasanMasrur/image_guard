# কীভাবে Publish করবে: ধাপে ধাপে

> শুধু নিচের ধাপগুলো একটা একটা করে করো। সব check আগেই করা আছে, package তৈরি।

---

## ধাপ ১: VS Code-এ Terminal খোলো

VS Code-এর উপরের menu থেকে **Terminal → New Terminal** বেছে নাও।

নিচে একটা terminal খুলবে। নিশ্চিত হও তুমি সঠিক folder-এ আছ। এই command লিখে Enter চাপো:

```bash
cd ~/Desktop/package/safe_image
```

---

## ধাপ ২: শেষবার check করো

এই command লিখে Enter চাপো:

```bash
dart pub publish --dry-run
```

একদম শেষে এই লাইনটা দেখবে:

```text
Package has 1 warning.
```

✅ **এটা ঠিক আছে।** Warning-টা `flutter_rust_bridge` নিয়ে, ইচ্ছা করে রাখা।

❌ যদি `error` লেখা আসে, তাহলে থামো আর আমাকে লেখাটা পাঠাও।

---

## ধাপ ৩: Publish করো

এই command লিখে Enter চাপো:

```bash
dart pub publish
```

### ৩.১: প্রশ্ন আসবে

অনেক লেখার পরে শেষে এরকম একটা প্রশ্ন আসবে:

```text
Do you want to publish image_guard 0.1.0-dev.1 to https://pub.dev (y/N)?
```

👉 **`y`** লিখে **Enter** চাপো।

### ৩.২: Google login (শুধু প্রথমবার)

Terminal-এ এরকম লেখা আসবে, আর browser নিজে থেকে খুলবে:

```text
Pub needs your authorization to upload packages on your behalf.
In a web browser, go to https://accounts.google.com/...
Then click "Allow access".
Waiting for your authorization...
```

Browser-এ:

1. তোমার **Google account** বেছে নাও (এই account-এর নামেই package থাকবে)
2. **Allow** / **Continue** চাপো
3. Browser-এ দেখাবে: **"Pub Authorized Successfully"**

> Browser নিজে না খুললে terminal-এর লম্বা `https://accounts.google.com/...` link-টা copy করে browser-এ paste করো।

### ৩.৩: সফল হলে

Terminal-এ দেখাবে:

```text
Successfully uploaded https://pub.dev/packages/image_guard version 0.1.0-dev.1.
```

🎉 **হয়ে গেছে!**

---

## ধাপ ৪: pub.dev-তে দেখো

৫–১০ মিনিট পরে browser-এ খোলো:

**https://pub.dev/packages/image_guard**

- তোমার package-এর page দেখা যাবে
- **Scores** tab-এ নম্বর আসতে আরও কিছুক্ষণ লাগবে (আশা করা যায় 160/160)

---

## ⚠️ মনে রাখো

- একবার publish করলে সেই version **আর মুছে ফেলা যায় না**
- ভুল হলে ৭ দিনের মধ্যে pub.dev-এর package page → **Admin** tab থেকে **Retract** করা যায়
- পরের বার নতুন version দিতে হলে আগে `pubspec.yaml`-এ `version` বাড়াতে হবে (যেমন `0.1.0-dev.2`)

---

## সমস্যা হলে

| দেখলে | মানে | কী করবে |
|------|------|---------|
| `Package has 2 warnings` আর `files are modified in git` | কোনো file বদলেছে কিন্তু commit হয়নি | আমাকে বলো, ঠিক করে দেব |
| `image_guard is already taken` / `not allowed` | অন্য কেউ নামটা নিয়ে নিয়েছে | আমাকে বলো, নতুন নাম ঠিক করব |
| `Authentication failed` | Login হয়নি | আবার `dart pub publish` চালাও |
| অন্য যেকোনো `error` | — | পুরো লেখাটা copy করে আমাকে পাঠাও |

---

Publish হয়ে গেলে আমাকে জানাও। পরের কাজ: **precompiled binary**, যাতে তোমার package যারা ব্যবহার করবে তাদের Rust install করতে না হয়।
