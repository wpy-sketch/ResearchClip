import importlib.util,unittest
from pathlib import Path
spec=importlib.util.spec_from_file_location('inheritance',Path(__file__).with_name('inherit-release-assets.py'));module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
class InheritanceTests(unittest.TestCase):
 def test_preserves_colliding_names_by_version_and_keeps_large_parts_direct(self):
  old=[{'tag':'v1.0.0','name':'SOURCE.txt','sha256':'a'*64,'size':1}];previous=[{'name':n,'sha256':'b'*64,'size':2} for n in ('SOURCE.txt','SHA256SUMS.txt','history.zip.001')]
  items=module.records(old,previous);self.assertEqual(len(items),4);self.assertEqual(items[0]['representation']['member'],'v1.0.0/SOURCE.txt');self.assertEqual(items[1]['representation']['member'],'v4.1.2/SOURCE.txt');self.assertEqual(items[3]['representation'],{'asset':'history.zip.001'})
 def test_next_generation_keeps_previous_representation_without_recursive_archive(self):
  old=module.records([{'tag':'v1','name':'a'}],[{'name':'SOURCE.txt'}]);new=module.records(old,[{'name':'SOURCE.txt'},{'name':'history.zip.001'}],'4.1.3');self.assertEqual(new[0]['representation'],old[0]['representation']);self.assertEqual(new[1]['representation']['member'],'v4.1.2/SOURCE.txt');self.assertEqual(new[2]['representation']['member'],'v4.1.3/SOURCE.txt');self.assertEqual(new[3]['representation'],{'asset':'history.zip.001'})
 def test_rejects_duplicate_version_name(self):
  with self.assertRaises(AssertionError):module.records([], [{'name':'a'},{'name':'a'}])
 def test_rejects_traversal_in_names(self):
  with self.assertRaises(AssertionError):module.records([{'tag':'v1','name':'../private'}],[])
if __name__=='__main__':unittest.main()
