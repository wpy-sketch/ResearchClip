"""Fixed local CPU embedding worker. No dynamic code, model hub or remote image inputs."""
import hashlib
import json
import os
from pathlib import Path
import struct
import sys
import time

os.environ.update(OMP_NUM_THREADS="2", OPENBLAS_NUM_THREADS="1", MKL_NUM_THREADS="2",
                  TOKENIZERS_PARALLELISM="false", HF_HUB_OFFLINE="1")


def verify(root, manifest):
    for item in manifest["files"]:
        path = root / item["name"]
        if path.is_symlink() or not path.is_file():
            raise ValueError("MODEL_MISSING")
        if path.stat().st_size != item["size"]:
            raise ValueError("MODEL_INVALID")
        with path.open("rb") as stream:
            digest = hashlib.file_digest(stream, "sha256").hexdigest()
        if digest != item["sha256"]:
            raise ValueError("MODEL_INVALID")


def normalized(value):
    import numpy as np
    value = np.asarray(value, dtype=np.float32).reshape(-1)
    norm = np.linalg.norm(value)
    if value.size != 512 or not np.isfinite(value).all() or not np.isfinite(norm) or norm < 1e-8:
        raise ValueError("VECTOR_INVALID")
    return (value / norm).tolist()


class Encoder:
    def __init__(self, root, kind):
        import onnxruntime as ort
        self.root = root
        options = ort.SessionOptions()
        options.intra_op_num_threads = 2
        options.inter_op_num_threads = 1
        options.execution_mode = ort.ExecutionMode.ORT_SEQUENTIAL
        options.add_session_config_entry("session.intra_op.allow_spinning", "0")
        options.add_session_config_entry("session.inter_op.allow_spinning", "0")
        self.session = ort.InferenceSession(str(root / ("vision.onnx" if kind == "image" else "text.onnx")),
                                           sess_options=options, providers=["CPUExecutionProvider"])
        self.kind = kind
        if kind == "text":
            import numpy as np
            from tokenizers import Tokenizer
            self.tokenizer = Tokenizer.from_file(str(root / "tokenizer.json"))
            self.tokenizer.enable_truncation(max_length=128)
            data = (root / "dense.safetensors").read_bytes()
            length = struct.unpack("<Q", data[:8])[0]
            meta = json.loads(data[8:8+length])["linear.weight"]
            if meta != {"dtype": "F32", "shape": [512, 768], "data_offsets": [0, 1572864]}:
                raise ValueError("MODEL_INVALID")
            self.dense = np.frombuffer(data, dtype="<f4", offset=8+length).reshape(512, 768)

    def encode(self, value):
        import numpy as np
        if self.kind == "image":
            from PIL import Image
            Image.MAX_IMAGE_PIXELS = 16_777_216
            with Image.open(value) as image:
                if image.width * image.height > Image.MAX_IMAGE_PIXELS:
                    raise ValueError("IMAGE_LIMIT")
                image = image.convert("RGB")
                # CLIP: resize shortest edge 224, bicubic, center crop, fixed channel normalization.
                w, h = image.size
                size = (224, int(224*h/w)) if w <= h else (int(224*w/h), 224)
                if size[0] * size[1] > 16_777_216:
                    raise ValueError("IMAGE_LIMIT")
                image = image.resize(size, Image.Resampling.BICUBIC)
                left, top = round((size[0]-224)/2), round((size[1]-224)/2)
                image = image.crop((left, top, left+224, top+224))
                pixels = np.asarray(image, dtype=np.float32)/255
            pixels = (pixels - [0.48145466, 0.4578275, 0.40821073])/[0.26862954, 0.26130258, 0.27577711]
            output = self.session.run(["image_embeds"], {"pixel_values": pixels.transpose(2, 0, 1)[None].astype(np.float32)})[0]
        else:
            if not isinstance(value, str) or not value.strip() or len(value) > 500 or any(ord(c) < 32 for c in value):
                raise ValueError("QUERY_INVALID")
            tokens = self.tokenizer.encode(value)
            ids, mask = np.array([tokens.ids], dtype=np.int64), np.array([tokens.attention_mask], dtype=np.int64)
            hidden = self.session.run(["last_hidden_state"], {"input_ids": ids, "attention_mask": mask})[0]
            output = ((hidden * mask[:, :, None]).sum(axis=1) / mask.sum(axis=1, keepdims=True)) @ self.dense.T
        return normalized(output)


def download(root, manifest):
    import urllib.request
    import urllib.parse
    def allowed(url):
        parsed = urllib.parse.urlsplit(url)
        return parsed.scheme == "https" and parsed.username is None and parsed.password is None and (
            parsed.hostname == "huggingface.co" or (parsed.hostname or "").endswith(".hf.co"))
    class Redirect(urllib.request.HTTPRedirectHandler):
        def redirect_request(self, request, fp, code, message, headers, newurl):
            if not allowed(newurl):
                raise ValueError("MODEL_INVALID")
            return super().redirect_request(request, fp, code, message, headers, newurl)
    opener = urllib.request.build_opener(Redirect())
    # This branch is reachable only after the desktop's explicit download confirmation.
    for item in manifest["files"]:
        path = root / item["name"]
        if path.exists():
            raise ValueError("MODEL_CONFLICT")
        if not item["url"].startswith("https://huggingface.co/"):
            raise ValueError("MODEL_INVALID")
        with opener.open(item["url"], timeout=45) as response, path.open("xb") as target:
            total = 0
            while chunk := response.read(1024*1024):
                total += len(chunk)
                if total > item["size"]:
                    raise ValueError("MODEL_INVALID")
                target.write(chunk)
    verify(root, manifest)


def main():
    kind, folder, raw_manifest = sys.argv[1:4]
    root, manifest = Path(folder), json.loads(raw_manifest)
    started = time.perf_counter()
    if kind == "download":
        download(root, manifest)
        return {"downloaded": True}
    verify(root, manifest)
    if kind == "verify":
        return {"verified": True}
    if kind not in ("image", "text"):
        raise ValueError("KIND_INVALID")
    encoder = Encoder(root, kind)
    loaded = time.perf_counter()
    source = Path(sys.argv[4])
    value = str(source) if kind == "image" else json.loads(source.read_text(encoding="utf-8"))["query"]
    vector = encoder.encode(value)
    return {"vector": vector, "model": manifest["id"], "version": manifest["version"],
            "loadMs": round((loaded-started)*1000, 2), "encodeMs": round((time.perf_counter()-loaded)*1000, 2)}


if __name__ == "__main__":
    try:
        print(json.dumps({"ok": True, "result": main()}, allow_nan=False))
    except Exception as error:
        code = str(error)
        print(json.dumps({"ok": False, "code": code if code in {
            "MODEL_MISSING", "MODEL_INVALID", "MODEL_CONFLICT", "QUERY_INVALID", "IMAGE_LIMIT", "VECTOR_INVALID"
        } else "EMBEDDING_FAILED"}))
