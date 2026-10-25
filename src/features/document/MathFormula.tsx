import {renderMath} from './math-render';
import 'katex/dist/katex.min.css';

// Only engine-generated markup reaches this boundary. User HTML stays disabled
// in react-markdown; each expression receives fresh, bounded macro state.
export function MathFormula({source,display,error}:{source:string;display:boolean;error?:string}){
  const {html,error:message}=renderMath(source,display,error);
  if(message)return <span role="alert" className="math-error"><strong>公式错误</strong><span>{message}</span><code>{source}</code></span>;
  return <span className={display?'math-render math-block':'math-render'} dangerouslySetInnerHTML={{__html:html}}/>;
}
