// Pure Dart: no native library needed.
import 'package:flutter_test/flutter_test.dart';
import 'package:image_guard/image_guard.dart';

Matcher throwsInvalid(String field) => throwsA(
  isA<SafeImageException>()
      .having((e) => e.code, 'code', SafeImageErrorCode.invalidOptions)
      .having((e) => e.message, 'message', contains(field)),
);

void main() {
  test('defaults', () {
    const s = SafetyOptions();
    s.validate();
    expect(s.rejectThreshold, 0.7);
    expect(s.reviewThreshold, 0.3);
    expect(s.compressUnsafe, isFalse);
    SafetyOptions.strict.validate();
  });

  test('thresholds must be 0..1', () {
    expect(
      () => const SafetyOptions(rejectThreshold: 1.5).validate(),
      throwsInvalid('rejectThreshold'),
    );
    expect(
      () => const SafetyOptions(reviewThreshold: -0.1).validate(),
      throwsInvalid('reviewThreshold'),
    );
    expect(
      () => const SafetyOptions(suggestiveWeight: 2).validate(),
      throwsInvalid('suggestiveWeight'),
    );
    expect(
      () => const SafetyOptions(rejectThreshold: double.nan).validate(),
      throwsInvalid('rejectThreshold'),
    );
  });

  test('review must not exceed reject', () {
    expect(
      () => const SafetyOptions(
        reviewThreshold: 0.8,
        rejectThreshold: 0.5,
      ).validate(),
      throwsInvalid('reviewThreshold'),
    );
    const SafetyOptions(reviewThreshold: 0.5, rejectThreshold: 0.5).validate();
  });

  test('scores map and report helpers', () {
    const scores = SafetyScores(
      drawings: 0.1,
      hentai: 0,
      neutral: 0.9,
      porn: 0,
      sexy: 0,
    );
    expect(scores.toMap().keys, [
      'drawings',
      'hentai',
      'neutral',
      'porn',
      'sexy',
    ]);
    const report = SafetyReport(
      verdict: Verdict.safe,
      nsfwScore: 0,
      scores: scores,
      elapsed: Duration.zero,
    );
    expect(report.isSafe, isTrue);
    expect(report.isUnsafe, isFalse);
  });
}
