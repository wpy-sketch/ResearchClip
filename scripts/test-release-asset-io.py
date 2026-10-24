import threading,time,unittest
from release_asset_io import read_assets,transfers
def expected(n):return {'name':f'file{n}.zip','size':10,'sha256':'a'*64}
def actual(item):return {'name':item['name'],'state':'uploaded','size':item['size'],'digest':'sha256:'+item['sha256']}
class ReleaseTransferTests(unittest.TestCase):
 def test_resumes_exact_existing_and_uploads_each_missing_once(self):
  calls=[];items=[expected(i) for i in range(8)]
  def upload(item):calls.append(item['name']);return actual(item)
  results=list(transfers(items,[actual(items[0])],upload));self.assertEqual(len(results),7);self.assertEqual(len(set(calls)),7)
 def test_never_overwrites_different_or_unexpected_remote_assets(self):
  item=expected(1);wrong=actual(item);wrong['digest']='sha256:'+'b'*64
  with self.assertRaises(AssertionError):list(transfers([item],[wrong],lambda _:self.fail('upload started')))
  with self.assertRaises(AssertionError):list(transfers([item],[actual(expected(2))],lambda _:self.fail('upload started')))
 def test_rejects_duplicate_or_unsafe_local_names(self):
  for items in [[expected(1),expected(1)],[{**expected(1),'name':'../private'}],[{**expected(1),'name':'C:\\private'}]]:
   with self.assertRaises(AssertionError):list(transfers(items,[],lambda _:self.fail('upload started')))
 def test_checks_upload_result_and_propagates_failure(self):
  with self.assertRaises(AssertionError):list(transfers([expected(1)],[],lambda x:{**actual(x),'size':11}))
  def failed(_):raise TimeoutError('owned test transfer')
  with self.assertRaises(TimeoutError):list(transfers([expected(1)],[],failed))
 def test_bounds_concurrency(self):
  lock=threading.Lock();state={'active':0,'peak':0}
  def upload(item):
   with lock:state['active']+=1;state['peak']=max(state['peak'],state['active'])
   time.sleep(.03)
   with lock:state['active']-=1
   return actual(item)
  self.assertEqual(len(list(transfers([expected(i) for i in range(12)],[],upload,workers=4))),12);self.assertLessEqual(state['peak'],4);self.assertGreater(state['peak'],1)
 def test_pages_release_assets_beyond_first_hundred(self):
  pages=[]
  def request(url):
   pages.append(url);return [{'name':f'f{i}'} for i in (range(100) if url.endswith('&page=1') else range(100,200) if url.endswith('&page=2') else range(200,226))]
  self.assertEqual(len(read_assets(request,'https://api.github.com/repos/example/repo',1)),226);self.assertEqual(len(pages),3)
if __name__=='__main__':unittest.main()
