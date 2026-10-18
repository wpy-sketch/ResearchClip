import katex from 'katex';
import 'katex/dist/katex.min.css';

// Only engine-generated markup reaches this boundary. User HTML stays disabled
// in react-markdown; each expression receives fresh, bounded macro state.
export function MathFormula({source,display,error}:{source:string;display:boolean;error?:string}){
  let html='',message=error;
  if(!message)try{
    if(source.length>8192)throw new Error('单个公式超过 8192 字符限制。');
    let blocked=false;
    html=katex.renderToString(source,{displayMode:display,throwOnError:true,
      trust:()=>{blocked=true;return false;},strict:'error',maxExpand:500,maxSize:20,
      macros:{},output:'htmlAndMathml'});
    if(blocked)throw new Error('公式包含不允许的链接、HTML 或外部资源命令。');
  }catch(e){message=e instanceof Error?e.message:'无法解析此公式。';}
  if(message)return <span role="alert" className="math-error"><strong>公式错误</strong><span>{message}</span><code>{source}</code></span>;
  return <span className={display?'math-render math-block':'math-render'} dangerouslySetInnerHTML={{__html:html}}/>;
}
