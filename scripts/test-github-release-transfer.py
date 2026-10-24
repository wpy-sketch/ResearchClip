import base64,hashlib,importlib.util,tempfile,unittest,zlib
from pathlib import Path
spec=importlib.util.spec_from_file_location('transport',Path(__file__).with_name('github-release-transfer.py'));module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
class ReconstructionTests(unittest.TestCase):
 def recipe(self,path):
  raw='中文 stable bytes'.encode();path.write_bytes(raw);c=zlib.compressobj(6,zlib.DEFLATED,-15);encoded=c.compress(raw)+c.flush();whole=b'HEAD'+encoded+b'TAIL'
  return {'schemaVersion':1,'archive':{'size':len(whole),'sha256':hashlib.sha256(whole).hexdigest()},'segments':[{'kind':'literal','size':4,'data':base64.b64encode(b'HEAD').decode()},{'kind':'asset','size':len(encoded),'compression':8,'encodedSha256':hashlib.sha256(encoded).hexdigest(),'source':{'tag':'v1','name':'public.exe','size':len(raw),'sha256':hashlib.sha256(raw).hexdigest()}},{'kind':'literal','size':4,'data':base64.b64encode(b'TAIL').decode()}]}
 def test_exact_reconstruction_and_no_overwrite(self):
  with tempfile.TemporaryDirectory() as folder:
   p=Path(folder);recipe=self.recipe(p/'source');module.reconstruct(recipe,p/'result',lambda _:p/'source');before=(p/'result').read_bytes()
   with self.assertRaises(FileExistsError):module.reconstruct(recipe,p/'result',lambda _:p/'source')
   self.assertEqual((p/'result').read_bytes(),before)
 def test_encoded_or_source_digest_failure_is_never_accepted(self):
  with tempfile.TemporaryDirectory() as folder:
   p=Path(folder);recipe=self.recipe(p/'source');recipe['segments'][1]['encodedSha256']='a'*64
   with self.assertRaises(AssertionError):module.reconstruct(recipe,p/'result',lambda _:p/'source')
 def test_unknown_recipe_or_inconsistent_total_is_rejected(self):
  with tempfile.TemporaryDirectory() as folder:
   p=Path(folder);recipe=self.recipe(p/'source');recipe['schemaVersion']=9
   with self.assertRaises(AssertionError):module.reconstruct(recipe,p/'result',lambda _:p/'source')
if __name__=='__main__':unittest.main()
