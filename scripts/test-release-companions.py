import hashlib
import importlib.util
import tempfile
import unittest
import zipfile
from pathlib import Path

spec = importlib.util.spec_from_file_location('companions', Path(__file__).with_name('package-release-companions.py'))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class CompanionTests(unittest.TestCase):
    def test_verified_archive_and_hash_mismatch(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'component.zip'
            with zipfile.ZipFile(path, 'w') as archive:
                archive.writestr('dist/manifest.json', '{}')
            expected = {'name':path.name, 'size':path.stat().st_size, 'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}
            module.validate_component(path, expected)
            with self.assertRaises(ValueError):
                module.validate_component(path, {**expected, 'sha256':'0'*64})

    def test_missing_and_traversal_member(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'missing.zip'
            with self.assertRaises(FileNotFoundError):
                module.validate_component(path, {'name':path.name})
            for member in ['../secret', '/absolute', 'C:/secret', 'dist/../../secret']:
                with zipfile.ZipFile(path, 'w') as archive:
                    info = zipfile.ZipInfo('safe')
                    info.filename = member  # Avoid Windows ZipInfo constructor normalizing backslashes.
                    archive.writestr(info, 'bad')
                expected = {'name':path.name, 'size':path.stat().st_size, 'sha256':module.sha(path)}
                with self.assertRaises(ValueError, msg=repr(member)):
                    module.validate_component(path, expected)

    def test_member_names_are_checked_without_platform_normalization(self):
        module.validate_member('dist/manifest.json')
        for name in ['dist\\file', '../secret', '/absolute', 'C:/secret']:
            with self.assertRaises(ValueError):
                module.validate_member(name)


if __name__ == '__main__':
    unittest.main()
