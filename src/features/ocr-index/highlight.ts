export function highlightParts(text:string,keyword:string):{text:string;match:boolean}[]{
 if(!keyword)return[{text,match:false}];
 // ASCII fold preserves offsets; Unicode math and Chinese remain literal text.
 const fold=(s:string)=>s.replace(/[A-Z]/g,c=>c.toLowerCase());const needle=fold(keyword),haystack=fold(text),parts:{text:string;match:boolean}[]=[];let start=0,index=haystack.indexOf(needle);
 while(index>=0){if(index>start)parts.push({text:text.slice(start,index),match:false});parts.push({text:text.slice(index,index+keyword.length),match:true});start=index+keyword.length;index=haystack.indexOf(needle,start);}if(start<text.length)parts.push({text:text.slice(start),match:false});return parts;
}
