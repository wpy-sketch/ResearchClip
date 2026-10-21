import {parseLinks} from './model';
const scope=globalThis as unknown as {onmessage:((e:MessageEvent<string>)=>void)|null;postMessage:(v:unknown)=>void};
scope.onmessage=e=>{try{scope.postMessage({links:parseLinks(e.data)});}catch(error){scope.postMessage({error:error instanceof Error?error.message:'链接解析失败。'});}};
