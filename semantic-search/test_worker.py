import importlib.util
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("worker", Path(__file__).with_name("worker.py"))
worker = importlib.util.module_from_spec(spec)
spec.loader.exec_module(worker)


class WorkerBoundaryTests(unittest.TestCase):
    def test_vector_validation(self):
        for value in ([0]*512, [1]*511, [float("nan")]*512, [float("inf")]*512):
            with self.assertRaises(ValueError):
                worker.normalized(value)
        self.assertEqual(worker.normalized([2]+[0]*511), [1]+[0]*511)

    def test_missing_and_corrupt_models_fail_closed(self):
        with tempfile.TemporaryDirectory() as root:
            root = Path(root)
            manifest = {"files": [{"name": "model.onnx", "size": 3, "sha256": "0"*64}]}
            with self.assertRaisesRegex(ValueError, "MODEL_MISSING"):
                worker.verify(root, manifest)
            (root / "model.onnx").write_bytes(b"abc")
            with self.assertRaisesRegex(ValueError, "MODEL_INVALID"):
                worker.verify(root, manifest)

    def test_download_refuses_existing_files_and_unapproved_host(self):
        with tempfile.TemporaryDirectory() as root:
            root = Path(root)
            file = {"name": "model.onnx", "size": 3, "sha256": "0"*64, "url": "http://invalid/"}
            with self.assertRaisesRegex(ValueError, "MODEL_INVALID"):
                worker.download(root, {"files": [file]})
            (root / "model.onnx").write_bytes(b"keep")
            with self.assertRaisesRegex(ValueError, "MODEL_CONFLICT"):
                worker.download(root, {"files": [file]})
            self.assertEqual((root / "model.onnx").read_bytes(), b"keep")


if __name__ == "__main__":
    unittest.main()
