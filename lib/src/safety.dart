import 'exceptions.dart';
import 'result.dart';

/// Decision made from the NSFW score.
enum Verdict {
  /// Below [SafetyOptions.reviewThreshold].
  safe,

  /// Between the review and reject thresholds — your app decides
  /// (e.g. send to manual review, or treat as unsafe).
  uncertain,

  /// At or above [SafetyOptions.rejectThreshold]. Do not upload.
  unsafe,
}

/// How strict the NSFW check is.
class SafetyOptions {
  /// `nsfwScore >= rejectThreshold` → [Verdict.unsafe]. Range 0–1.
  final double rejectThreshold;

  /// `nsfwScore >= reviewThreshold` → [Verdict.uncertain]. Range 0–1.
  final double reviewThreshold;

  /// How much the "sexy" (suggestive, not explicit) class counts. Range 0–1.
  /// `0` ignores swimwear/lingerie-style photos, `1` treats them as explicit.
  final double suggestiveWeight;

  /// Also compress unsafe images (normally skipped to save time).
  final bool compressUnsafe;

  const SafetyOptions({
    this.rejectThreshold = 0.7,
    this.reviewThreshold = 0.3,
    this.suggestiveWeight = 0.5,
    this.compressUnsafe = false,
  });

  /// Stricter preset: more images become uncertain/unsafe.
  static const strict = SafetyOptions(
    rejectThreshold: 0.5,
    reviewThreshold: 0.2,
    suggestiveWeight: 0.8,
  );

  /// Throws [SafeImageException] with [SafeImageErrorCode.invalidOptions].
  void validate() {
    Never fail(String m) =>
        throw SafeImageException(SafeImageErrorCode.invalidOptions, m);
    for (final (name, v) in [
      ('rejectThreshold', rejectThreshold),
      ('reviewThreshold', reviewThreshold),
      ('suggestiveWeight', suggestiveWeight),
    ]) {
      if (v.isNaN || v < 0 || v > 1) {
        fail('$name must be between 0.0 and 1.0, got $v');
      }
    }
    if (reviewThreshold > rejectThreshold) {
      fail(
        'reviewThreshold ($reviewThreshold) must not be greater than '
        'rejectThreshold ($rejectThreshold)',
      );
    }
  }

  @override
  String toString() =>
      'SafetyOptions(reject: $rejectThreshold, review: $reviewThreshold, '
      'suggestiveWeight: $suggestiveWeight, compressUnsafe: $compressUnsafe)';
}

/// Model probabilities for each class (they add up to ~1).
class SafetyScores {
  final double drawings;
  final double hentai;
  final double neutral;
  final double porn;
  final double sexy;

  const SafetyScores({
    required this.drawings,
    required this.hentai,
    required this.neutral,
    required this.porn,
    required this.sexy,
  });

  Map<String, double> toMap() => {
    'drawings': drawings,
    'hentai': hentai,
    'neutral': neutral,
    'porn': porn,
    'sexy': sexy,
  };

  @override
  String toString() => toMap().entries
      .map((e) => '${e.key}: ${e.value.toStringAsFixed(3)}')
      .join(', ');
}

class SafetyReport {
  final Verdict verdict;

  /// `porn + hentai + sexy × suggestiveWeight`, 0–1.
  final double nsfwScore;
  final SafetyScores scores;
  final Duration elapsed;

  const SafetyReport({
    required this.verdict,
    required this.nsfwScore,
    required this.scores,
    required this.elapsed,
  });

  bool get isSafe => verdict == Verdict.safe;
  bool get isUnsafe => verdict == Verdict.unsafe;

  @override
  String toString() =>
      'SafetyReport(${verdict.name}, nsfwScore: ${nsfwScore.toStringAsFixed(3)}, '
      '$scores, ${elapsed.inMilliseconds} ms)';
}

/// Result of [SafeImage.process]: the safety decision and, unless the image
/// was unsafe, the compressed image.
class SafeImageProcessResult {
  final SafetyReport safety;

  /// `null` when [safety] is unsafe (and `compressUnsafe` is false).
  final SafeImageResult? image;

  const SafeImageProcessResult({required this.safety, required this.image});

  Verdict get verdict => safety.verdict;
  bool get isSafe => safety.isSafe;
  bool get isUnsafe => safety.isUnsafe;

  @override
  String toString() => 'SafeImageProcessResult($safety, image: $image)';
}
