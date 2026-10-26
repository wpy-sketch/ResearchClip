// QA-only HTTP boundary fixture, not Zotero. Real API CRUD is tested separately.
import assert from 'node:assert/strict';
import {annotationFixture} from './zotero-annotations-fixture.mjs';
export function writeFixture(){
 const rows=new Map(),authorized=new Set();let version=40,committedUnknown=false;
 const list=library=>{const scope=library.type+library.id;if(!rows.has(scope)){const items=annotationFixture(library,0,'normal').slice(0,3);for(const a of items)if(a.data.annotationType==='note')delete a.data.annotationText;rows.set(scope,new Map(items.map(a=>[a.key,a])));}return rows.get(scope);};
 return async(req,res,library,mode,path)=>{
  if(mode!=='writeVerifyUnavailable')committedUnknown=false;
  if(!path.endsWith('/children')&&!path.endsWith('/local/authorize')&&!/\/items(?:\/[A-Z0-9]{8})?$/.test(path))return false;
  if(req.method==='GET'&&(path.endsWith('/items')||/\/(ATTACH01|PARENT01)$/.test(path)))return false;
  res.setHeader('X-Zotero-Version','10.0.6');res.setHeader('Last-Modified-Version',String(version));res.setHeader('Content-Type','application/json');
  const reply=(status,body)=>{res.writeHead(status);res.end(body===undefined?'':JSON.stringify(body));return true;};
  const store=list(library),key=path.split('/').at(-1);
  if(req.method==='GET'){
   if(path.endsWith('/children'))return reply(200,[...store.values()]);
   if(mode==='writeVerifyUnavailable'&&committedUnknown)return reply(503,{error:'fictional interrupted reread'});
   return store.has(key)?reply(200,store.get(key)):reply(404);
  }
  let raw='';for await(const chunk of req){raw+=chunk;assert.ok(raw.length<=512*1024);}const body=raw?JSON.parse(raw):null;
  if(path.endsWith('/local/authorize')){assert.equal(body.appName,'ResearchClip');if(mode==='writeAuthDenied')return reply(403);const token='a'.repeat(32);authorized.add(token);return reply(200,{key:token,remember:true});}
  if(!authorized.has(req.headers['zotero-api-key']))return reply(401);
  if(mode==='writeForbidden'||library.type==='group')return reply(403);
  if(req.method==='POST'){
   const data=body[0];assert.equal(Object.keys(data)[0],'annotationType');if(mode==='writeItemDenied')return reply(200,{successful:{},failed:{0:{code:403,message:'fictional item denial'}}});
   const item={key:data.key,version:++version,library,data};store.set(data.key,item);if(mode==='writeVerifyUnavailable')committedUnknown=true;return reply(200,{successful:{0:item},failed:{}});
  }
  const old=store.get(key);if(!old)return reply(404);
  if(Number(req.headers['if-unmodified-since-version'])!==old.version||mode==='writeConflict')return reply(412);
  if(req.method==='PATCH'){old.data={...old.data,...body};old.version=++version;}
  else if(req.method==='DELETE'){store.delete(key);version++;}else throw Error('Unexpected mutation');
  return reply(204);
 };
}
