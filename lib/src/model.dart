import 'dart:async';
import 'dart:io';

import 'package:crypto/crypto.dart';
import 'package:path_provider/path_provider.dart';

import 'exceptions.dart';

/// Called while the model downloads. [total] is `null` when the server does
/// not send a size.
typedef ModelDownloadProgress = void Function(int received, int? total);

/// Where the NSFW model is downloaded from and cached.
///
/// The model (~18 MB) is not bundled in your app. It is downloaded once on
/// first use, checked against [sha256], and cached in [directory].
class SafeImageModel {
  final Uri url;

  /// Lowercase hex SHA-256 of the file at [url]. A download that does not
  /// match is discarded.
  final String sha256;

  /// Cache folder. Defaults to `<application support>/image_guard`.
  final String? directory;

  const SafeImageModel({
    required this.url,
    required this.sha256,
    this.directory,
  });

  /// GantMan nsfw_model 1.2.0 (MIT), converted to NNEF, served from this
  /// package's GitHub repo at the `model-v1` tag (never moved; a new model
  /// gets a new tag).
  static final standard = SafeImageModel(
    url: Uri.parse(
      'https://raw.githubusercontent.com/HasanMasrur/image_guard/model-v1/'
      'assets/models/nsfw_mobilenet_v2_140_224.nnef.tar',
    ),
    sha256: '8b07fb6e5c401d27782f08c6c17622d1083b74af1fd1738ffebd1c839d818294',
  );

  /// Larger responses are aborted (the real model is ~18 MB).
  static const maxDownloadBytes = 64 * 1024 * 1024;

  // The hash is in the name, so a new model version never reuses an old file.
  String get _fileName => 'nsfw-${sha256.substring(0, 16)}.nnef.tar';

  Future<Directory> _dir() async => Directory(
    directory ?? '${(await getApplicationSupportDirectory()).path}/image_guard',
  );

  Future<File> cachedFile() async => File('${(await _dir()).path}/$_fileName');

  @override
  String toString() => 'SafeImageModel($url)';
}

final _downloads = <String, Future<File>>{};

/// Returns the cached model file, downloading it first if needed.
/// Concurrent calls for the same model share one download.
Future<File> ensureModelFile(
  SafeImageModel model, {
  ModelDownloadProgress? onProgress,
}) async {
  final file = await model.cachedFile();
  if (await file.exists()) return file;
  return _downloads[file
      .path] ??= _download(model, file, onProgress).whenComplete(() {
    // Block body: returning the removed future would make it wait on itself.
    _downloads.remove(file.path);
  });
}

Never _fail(String m) =>
    throw SafeImageException(SafeImageErrorCode.modelDownloadFailed, m);

Future<File> _download(
  SafeImageModel model,
  File file,
  ModelDownloadProgress? onProgress,
) async {
  await file.parent.create(recursive: true);
  final part = File('${file.path}.part');
  final client = HttpClient()..connectionTimeout = const Duration(seconds: 30);
  try {
    final HttpClientResponse res;
    try {
      res = await (await client.getUrl(model.url)).close();
    } on Object catch (e) {
      _fail('Cannot download NSFW model from ${model.url}: $e');
    }
    if (res.statusCode != HttpStatus.ok) {
      _fail(
        'Cannot download NSFW model from ${model.url}: HTTP ${res.statusCode}',
      );
    }
    final total = res.contentLength >= 0 ? res.contentLength : null;
    final digest = _DigestSink();
    final hasher = sha256.startChunkedConversion(digest);
    final sink = part.openWrite();
    var received = 0;
    try {
      await for (final chunk in res.timeout(const Duration(seconds: 60))) {
        received += chunk.length;
        if (received > SafeImageModel.maxDownloadBytes) {
          _fail(
            'NSFW model download is larger than '
            '${SafeImageModel.maxDownloadBytes} bytes',
          );
        }
        sink.add(chunk);
        hasher.add(chunk);
        onProgress?.call(received, total);
      }
    } on SafeImageException {
      rethrow;
    } on Object catch (e) {
      _fail('NSFW model download interrupted after $received bytes: $e');
    } finally {
      await sink.close();
    }
    hasher.close();
    final actual = digest.value.toString();
    if (actual != model.sha256.toLowerCase()) {
      _fail(
        'NSFW model checksum mismatch: expected ${model.sha256}, got $actual',
      );
    }
    // Rename is atomic: a half-written file is never seen as the model.
    final done = await part.rename(file.path);
    await _deleteOtherModels(file);
    return done;
  } finally {
    client.close(force: true);
    if (await part.exists()) await part.delete();
  }
}

/// Removes models left over from older package versions.
Future<void> _deleteOtherModels(File keep) async {
  try {
    await for (final f in keep.parent.list()) {
      final name = f.uri.pathSegments.last;
      if (f is File && f.path != keep.path && name.startsWith('nsfw-')) {
        await f.delete();
      }
    }
  } on FileSystemException {
    // Best effort only.
  }
}

class _DigestSink implements Sink<Digest> {
  late Digest value;

  @override
  void add(Digest data) => value = data;

  @override
  void close() {}
}
