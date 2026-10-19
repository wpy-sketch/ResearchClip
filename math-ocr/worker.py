"""Fixed local worker, launched by Rust with -I -c. No model-generated code is executed.

ONNX preprocessing/encoder-decoder conventions adapted from RapidAI/RapidLaTeXOCR
v0.0.9, Copyright (c) 2023 RapidAI, MIT (see LICENSE-RapidLaTeXOCR).
We use current ONNX/NumPy/Pillow/tokenizers without the old package's auto-download,
OpenCV, YAML or arbitrary model paths. CPU-only, two inference threads, 512 tokens.
"""
import hashlib
import json
import os
from pathlib import Path
import sys
import time

MAX_IMAGE_BYTES = 20 * 1024 * 1024
MAX_PIXELS = 4_194_304


def ordinary(path):
    if path.is_symlink() or path.is_junction():
        raise ValueError("UNSAFE_PATH")
    return path


def verify_models(root, manifest):
    ordinary(root)
    for item in manifest["files"]:
        p = ordinary(root / item["name"])
        if not p.is_file() or p.stat().st_size != item["size"]:
            raise ValueError("MODEL_MISSING")
        h = hashlib.sha256()
        count = 0
        with p.open("rb") as f:
            for chunk in iter(lambda: f.read(1024 * 1024), b""):
                h.update(chunk)
                count += len(chunk)
                if count > item["size"]:
                    raise ValueError("MODEL_INVALID")
        if h.hexdigest() != item["sha256"]:
            raise ValueError("MODEL_INVALID")


def download(root, manifest, staging):
    import tempfile
    import urllib.request
    import urllib.parse
    class Redirect(urllib.request.HTTPRedirectHandler):
        def redirect_request(self, req, fp, code, msg, headers, url):
            p = urllib.parse.urlsplit(url)
            if p.scheme != "https" or not (p.hostname == "github.com" or (p.hostname or "").endswith(".githubusercontent.com")):
                raise ValueError("DOWNLOAD_REDIRECT_DENIED")
            return super().redirect_request(req, fp, code, msg, headers, url)
    opener = urllib.request.build_opener(Redirect())
    root.mkdir(parents=True, exist_ok=True)
    ordinary(root)
    ordinary(staging)
    for item in manifest["files"]:
        target = ordinary(root / item["name"])
        if target.exists():
            # An already valid file may be reused; never replace an unknown existing file.
            if target.stat().st_size == item["size"]:
                verify_models(root, {"files": [item]})
                continue
            raise ValueError("MODEL_CONFLICT")
        with tempfile.NamedTemporaryFile(dir=staging, prefix="download-", suffix=".part", delete=False) as f:
            stage = Path(f.name)
            try:
                h, count = hashlib.sha256(), 0
                with opener.open(item["url"], timeout=20) as response:
                    while True:
                        chunk = response.read(128 * 1024)
                        if not chunk:
                            break
                        count += len(chunk)
                        if count > item["size"]:
                            raise ValueError("DOWNLOAD_TOO_LARGE")
                        f.write(chunk)
                        h.update(chunk)
                f.flush()
                os.fsync(f.fileno())
                f.close()
                if count != item["size"] or h.hexdigest() != item["sha256"]:
                    raise ValueError("MODEL_INVALID")
                # Windows rename refuses an existing destination; never overwrite it.
                stage.rename(target)
            finally:
                f.close()
                if stage.exists():
                    stage.unlink()
    return {"downloaded": True, "model": manifest["id"]}


def infer(root, image_path, manifest):
    # Fail closed before importing native inference dependencies or reading pixels.
    verify_models(root, manifest)
    ordinary(image_path)
    if image_path.stat().st_size > MAX_IMAGE_BYTES:
        raise ValueError("IMAGE_LIMIT")
    def offline(event, args):
        if event in ("socket.connect", "socket.getaddrinfo", "subprocess.Popen", "os.system", "os.exec"):
            raise PermissionError("OFFLINE_INFERENCE")
    sys.addaudithook(offline)
    os.environ["HF_HUB_OFFLINE"] = "1"
    os.environ["TOKENIZERS_PARALLELISM"] = "false"
    import numpy as np
    import onnxruntime as ort
    from PIL import Image
    from tokenizers import Tokenizer
    Image.MAX_IMAGE_PIXELS = MAX_PIXELS
    with Image.open(image_path) as image:
        if image.format != "PNG" or image.width * image.height > MAX_PIXELS or max(image.size) > 4096:
            raise ValueError("IMAGE_LIMIT")
        rgba = image.convert("RGBA")
        white = Image.new("RGBA", image.size, "white")
        white.alpha_composite(rgba)
        image = white.convert("L")
    data = np.asarray(image, dtype=np.float32)
    low, high = float(data.min()), float(data.max())
    if high - low < 8:
        raise ValueError("NO_FORMULA")
    data = ((data - low) / (high - low) * 255).astype(np.uint8)
    if data.mean() < 128:
        data = 255 - data

    def pad(im):
        a = np.asarray(im.convert("L"))
        yy, xx = np.nonzero(a < 128)
        if not len(xx):
            raise ValueError("NO_FORMULA")
        crop = im.crop((int(xx.min()), int(yy.min()), int(xx.max()) + 1, int(yy.max()) + 1))
        out = Image.new("L", tuple(((s + 31) // 32) * 32 for s in crop.size), 255)
        out.paste(crop, (0, 0))
        return out

    def minmax(im):
        ratio = max(im.width / 672, im.height / 192, 1)
        if ratio > 1:
            im = im.resize((max(1, int(im.width / ratio)), max(1, int(im.height / ratio))), Image.Resampling.BILINEAR)
        out = Image.new("L", (max(32, im.width), max(32, im.height)), 255)
        out.paste(im, (0, 0))
        return out

    options = ort.SessionOptions()
    options.intra_op_num_threads = 2
    options.inter_op_num_threads = 1
    options.log_severity_level = 3
    options.enable_cpu_mem_arena = False
    sessions = [ort.InferenceSession(str(root / name), sess_options=options, providers=["CPUExecutionProvider"]) for name in ("image_resizer.onnx", "encoder.onnx", "decoder.onnx")]
    def run(session, values):
        if len(session.get_inputs()) != len(values):
            raise ValueError("MODEL_CONTRACT")
        return session.run(None, {i.name: v for i, v in zip(session.get_inputs(), values)})[0]
    im = minmax(pad(Image.fromarray(data)))
    width, height, ratio = im.width, im.height, 1.0
    tensor = None
    for _ in range(10):
        height = max(1, int(height * ratio))
        resized = im.resize((width, height), Image.Resampling.BILINEAR if ratio > 1 else Image.Resampling.LANCZOS)
        bounded = pad(minmax(resized))
        tensor = ((np.asarray(bounded, dtype=np.float32) - 0.7931 * 255) / (0.1738 * 255))[None, None, :, :]
        width = (int(np.argmax(run(sessions[0], [tensor]))) + 1) * 32
        if width == bounded.width:
            break
        ratio = width / bounded.width
    context = run(sessions[1], [tensor])
    tokens = np.array([[1]], dtype=np.int64)
    finished = False
    for _ in range(512):
        logits = run(sessions[2], [tokens, np.ones_like(tokens, dtype=bool), context])[:, -1, :]
        if not np.isfinite(logits).all():
            raise ValueError("INFERENCE_FAILED")
        token = int(np.argmax(logits))  # Deterministic greedy decoding, no random sampling.
        tokens = np.concatenate([tokens, np.array([[token]], dtype=np.int64)], axis=1)
        if token == 2:
            finished = True
            break
    if not finished:
        raise ValueError("TOKEN_LIMIT")
    text = Tokenizer.from_file(str(root / "tokenizer.json")).decode(tokens[0, 1:].tolist())
    text = "".join(text.split(" ")).replace("Ġ", " ").replace("[EOS]", "").replace("[BOS]", "").replace("[PAD]", "").strip()
    if not text or len(text) > 8192 or "\0" in text:
        raise ValueError("INVALID_OUTPUT")
    return {"latex": text, "model": manifest["id"], "verified": False, "tokens": len(tokens[0]) - 1}


def main():
    # Rust passes this fixed manifest as JSON, never a user-controlled URL/model path.
    mode, root, manifest_json = sys.argv[1:4]
    manifest = json.loads(manifest_json)
    root = Path(root)
    started = time.monotonic()
    try:
        if mode == "download":
            result = download(root, manifest, Path(sys.argv[4]))
        elif mode == "recognize":
            result = infer(root, Path(sys.argv[4]), manifest)
        elif mode == "probe":
            import importlib.metadata as metadata
            import onnxruntime, numpy, PIL, tokenizers
            result = {"python": sys.version.split()[0], "dependencies": {n: metadata.version(n) for n in ("onnxruntime", "numpy", "pillow", "tokenizers")}}
        else:
            raise ValueError("INVALID_OPERATION")
        print(json.dumps({"ok": True, "result": result, "seconds": time.monotonic() - started}, ensure_ascii=True))
    except Exception as error:
        # Never print screenshots, model output, exception paths, or a traceback.
        code = str(error) if isinstance(error, ValueError) and str(error).isupper() else "RUNTIME_UNAVAILABLE"
        print(json.dumps({"ok": False, "code": code[:80]}))


if __name__ == "__main__":
    main()
