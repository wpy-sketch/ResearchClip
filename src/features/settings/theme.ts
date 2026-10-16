export type ThemePreference='dark'|'light'|'system';
export const resolveTheme=(preference:ThemePreference,systemDark:boolean):'dark'|'light'=>preference==='system'?(systemDark?'dark':'light'):preference;
export function observeTheme(preference:ThemePreference,media:Pick<MediaQueryList,'matches'|'addEventListener'|'removeEventListener'>,apply:(theme:'dark'|'light')=>void){
  const update=()=>apply(resolveTheme(preference,media.matches));update();
  if(preference!=='system')return()=>{};
  media.addEventListener('change',update);return()=>media.removeEventListener('change',update);
}
