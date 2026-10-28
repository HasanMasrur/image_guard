// Model download + cache, against a local HTTP server.
import 'dart:io';

import 'package:crypto/crypto.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:image_guard/image_guard.dart';

import 'helpers.dart';

Matcher throwsCode(SafeImageErrorCode code) =>
    throwsA(isA<SafeImageException>().having((e) => e.code, 'code', code));

void main() {
  final modelBytes = File(modelPath).readAsBytesSync();
  final modelSha = sha256.convert(modelBytes).toString();
  late HttpServer server;
  late Directory cache;
  var requests = 0;

  Uri url(String path) => Uri.parse('http://127.0.0.1:${server.port}$path');

  setUpAll(() async {
    await initNative();
    server = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
    server.listen((req) async {
      requests++;
      final res = req.response;
      switch (req.uri.path) {
        case '/model.tar':
          res.contentLength = modelBytes.length;
          res.add(modelBytes);
        case '/redirect':
          await res.redirect(url('/model.tar'));
          return;
        default:
          res.statusCode = HttpStatus.notFound;
      }
      await res.close();
    });
  });

  tearDownAll(() => server.close(force: true));

  setUp(() async {
    requests = 0;
    cache = await Directory.systemTemp.createTemp('image_guard_test');
    await SafeImage.dispose();
  });

  tearDown(() => cache.delete(recursive: true));

  SafeImageModel model(String path, {String? sha}) => SafeImageModel(
    url: url(path),
    sha256: sha ?? modelSha,
    directory: cache.path,
  );

  test('downloads once, reports progress, then loads from cache', () async {
    SafeImage.model = model('/model.tar');
    expect(await SafeImage.isModelDownloaded(), isFalse);

    final progress = <int>[];
    await SafeImage.downloadModel(
      onProgress: (received, total) {
        expect(total, modelBytes.length);
        progress.add(received);
      },
    );
    expect(progress.last, modelBytes.length);
    expect(await SafeImage.isModelDownloaded(), isTrue);
    expect(
      cache.listSync().map((f) => f.path),
      everyElement(isNot(endsWith('.part'))),
    );

    await SafeImage.initialize();
    final r = await SafeImage.classify(
      SafeImageSource.bytes(jpeg(photo(200, 200))),
    );
    expect(r.scores.neutral, greaterThanOrEqualTo(0));
    expect(requests, 1);
  });

  test('initialize downloads on first use and follows redirects', () async {
    SafeImage.model = model('/redirect');
    await SafeImage.initialize();
    expect(await SafeImage.isModelDownloaded(), isTrue);
    expect(requests, 2);
  });

  test('concurrent calls share one download', () async {
    SafeImage.model = model('/model.tar');
    await Future.wait([for (var i = 0; i < 4; i++) SafeImage.downloadModel()]);
    expect(requests, 1);
  });

  test('checksum mismatch is rejected and nothing is cached', () async {
    SafeImage.model = model('/model.tar', sha: '0' * 64);
    await expectLater(
      SafeImage.initialize(),
      throwsCode(SafeImageErrorCode.modelDownloadFailed),
    );
    expect(await SafeImage.isModelDownloaded(), isFalse);
    expect(cache.listSync(), isEmpty);
  });

  test('HTTP error', () async {
    SafeImage.model = model('/missing');
    await expectLater(
      SafeImage.downloadModel(),
      throwsCode(SafeImageErrorCode.modelDownloadFailed),
    );
  });

  test('no server', () async {
    SafeImage.model = SafeImageModel(
      url: Uri.parse('http://127.0.0.1:1/model.tar'),
      sha256: modelSha,
      directory: cache.path,
    );
    await expectLater(
      SafeImage.downloadModel(),
      throwsCode(SafeImageErrorCode.modelDownloadFailed),
    );
  });

  test('a damaged cache file is deleted so it downloads again', () async {
    SafeImage.model = model('/model.tar');
    await SafeImage.downloadModel();
    final file = cache.listSync().single as File;
    file.writeAsBytesSync([1, 2, 3]);
    await expectLater(
      SafeImage.initialize(),
      throwsCode(SafeImageErrorCode.modelLoadFailed),
    );
    expect(file.existsSync(), isFalse);
    await SafeImage.initialize();
    expect(requests, 2);
  });

  test('deleteDownloadedModel', () async {
    SafeImage.model = model('/model.tar');
    await SafeImage.initialize();
    await SafeImage.deleteDownloadedModel();
    expect(await SafeImage.isModelDownloaded(), isFalse);
  });

  test('old model versions are cleaned up', () async {
    File('${cache.path}/nsfw-0000000000000000.nnef.tar').writeAsBytesSync([0]);
    SafeImage.model = model('/model.tar');
    await SafeImage.downloadModel();
    expect(cache.listSync(), hasLength(1));
  });
}
