"""Builds assets/models/nsfw_mobilenet_v2_140_224.nnef.tar from the upstream model.

TFLite -> ONNX (tf2onnx, checked against TFLite here) -> NNEF (tract, via
`cargo run --example onnx_to_nnef`; checked against TFLite by the Rust tests).

Upstream: https://github.com/GantMan/nsfw_model (MIT), release 1.2.0,
mobilenet_v2_140_224/saved_model.tflite
SHA-256 380f98f7685f9d8a386f8cc595b6dfcb972989aae3d1b8b270d3a4a5b96fab40

Usage (Python 3, in a venv):
    pip install tensorflow tf2onnx onnx onnxruntime pillow
    python tool/convert_model.py path/to/saved_model.tflite

It also writes rust/tests/fixtures/reference.json: TFLite outputs for a few
generated 224x224 images, so the Rust tests can prove the converted model
gives the same answers as the original.
"""

import hashlib
import json
import subprocess
import sys
from pathlib import Path

import numpy as np
import onnxruntime as ort
import tensorflow as tf
from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "tool/build/nsfw_mobilenet_v2_140_224.onnx"
NNEF = ROOT / "assets/models/nsfw_mobilenet_v2_140_224.nnef.tar"
FIXTURES = ROOT / "rust/tests/fixtures"
EXPECTED_SHA = "380f98f7685f9d8a386f8cc595b6dfcb972989aae3d1b8b270d3a4a5b96fab40"
LABELS = ["drawings", "hentai", "neutral", "porn", "sexy"]


def fixture_images():
    """Deterministic 224x224 RGB test pictures (no real photos needed)."""
    rng = np.random.default_rng(42)
    x = np.linspace(0, 1, 224)
    grad = np.stack(np.meshgrid(x, x), -1)
    yield "gradient", (np.dstack([grad[..., 0], grad[..., 1], 1 - grad[..., 0]]) * 255)
    yield "noise", rng.integers(0, 256, (224, 224, 3))
    yield "skin_tone", np.tile([224, 172, 140], (224, 224, 1)) + rng.integers(-20, 20, (224, 224, 3))
    yield "white", np.full((224, 224, 3), 255)
    yield "checker", (np.indices((224, 224)).sum(0) // 16 % 2)[..., None].repeat(3, -1) * 255


def main(tflite_path: str) -> None:
    tflite = Path(tflite_path)
    sha = hashlib.sha256(tflite.read_bytes()).hexdigest()
    if sha != EXPECTED_SHA:
        sys.exit(f"Unexpected model checksum {sha}")

    OUT.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(
        [sys.executable, "-m", "tf2onnx.convert", "--tflite", str(tflite),
         "--output", str(OUT), "--opset", "13"],
        check=True,
    )

    interp = tf.lite.Interpreter(model_path=str(tflite))
    interp.allocate_tensors()
    inp, out = interp.get_input_details()[0], interp.get_output_details()[0]
    sess = ort.InferenceSession(str(OUT))
    onnx_input = sess.get_inputs()[0].name

    FIXTURES.mkdir(parents=True, exist_ok=True)
    reference = {}
    worst = 0.0
    for name, pixels in fixture_images():
        rgb = np.clip(pixels, 0, 255).astype(np.uint8)
        Image.fromarray(rgb).save(FIXTURES / f"{name}.png")
        x = (rgb.astype(np.float32) / 255.0)[None]
        interp.set_tensor(inp["index"], x)
        interp.invoke()
        ref = interp.get_tensor(out["index"])[0]
        got = sess.run(None, {onnx_input: x})[0][0]
        worst = max(worst, float(np.abs(ref - got).max()))
        reference[name] = dict(zip(LABELS, map(float, ref)))
        print(name, {k: round(v, 4) for k, v in reference[name].items()})

    (FIXTURES / "reference.json").write_text(json.dumps(reference, indent=2))
    print(f"ONNX vs TFLite max abs diff: {worst:.2e}")
    if worst > 1e-3:
        sys.exit("Converted model differs from the original")
    NNEF.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(
        ["cargo", "run", "--release", "--example", "onnx_to_nnef", "--", str(OUT), str(NNEF)],
        cwd=ROOT / "rust", check=True,
    )
    print(f"Wrote {NNEF} ({NNEF.stat().st_size / 1e6:.1f} MB)")
    print("Now run: cargo test --release --manifest-path rust/Cargo.toml --test nsfw_model")


if __name__ == "__main__":
    main(sys.argv[1])
