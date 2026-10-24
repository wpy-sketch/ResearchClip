import type {Root,RootContent,Text,Emphasis} from 'mdast';
// A rendering-only extension: never interprets HTML, attributes, or executable input.
export function remarkHighlight(){return (tree:Root)=>{
 const walk=(node:Root|RootContent)=>{
  if(!('children'in node))return;
  const children:RootContent[]=[];
  for(const child of node.children){
   if(child.type!=='text'){walk(child as RootContent);children.push(child as RootContent);continue;}
   let at=0;for(const match of child.value.matchAll(/==(?=\S)([^=\n]+?)==/g)){const index=match.index!;if(index>at)children.push({type:'text',value:child.value.slice(at,index)} as Text);children.push({type:'emphasis',data:{hName:'mark'},children:[{type:'text',value:match[1]}]} as Emphasis);at=index+match[0].length;}
   if(at<child.value.length)children.push({...child,value:child.value.slice(at)});else if(!at)children.push(child);
  }
  // Parent types retain their existing schema; only text children are split.
  node.children=children as typeof node.children;
 };walk(tree);
};}
