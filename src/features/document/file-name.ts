// Shared UX rules mirror the native boundary; only a filename, never a path.
export function normalizeMarkdownName(input:string):string {
  const raw=input.trim();
  if(!raw||/[<>:"/\\|?*]/.test(raw)||Array.from(raw).some(c=>c.charCodeAt(0)<32||c.charCodeAt(0)===127)||/[. ]$/.test(raw))throw new Error('文件名不能为空，也不能包含路径、控制字符或 Windows 非法字符。');
  const name=/\.md$/i.test(raw)?raw:`${raw}.md`,stem=name.split('.')[0].trimEnd().toUpperCase();
  if(name.length>255||/^(CON|PRN|AUX|NUL|CLOCK\$|COM[1-9¹²³]|LPT[1-9¹²³])$/.test(stem))throw new Error('文件名过长或属于 Windows 保留名称。');
  return name;
}
