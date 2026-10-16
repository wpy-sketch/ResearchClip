import {MAX_EXPORT_CARDS} from '../../shared/api/markdown';
export function toggleExportSelection(current:string[],id:string){return current.includes(id)?current.filter(value=>value!==id):addExportSelection(current,[id]);}
export function addExportSelection(current:string[],pageIds:string[]){const next=[...new Set([...current,...pageIds])];if(next.length>MAX_EXPORT_CARDS)throw new Error('每次最多选择100张卡片，请分批导出。');return next;}
