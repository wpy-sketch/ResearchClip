import type {ReferenceInput} from './api';
export function referenceProvenance(record:Pick<ReferenceInput,'verifiedSource'|'sourceMetadata'>):string{
 const metadata=record.sourceMetadata;
 if(metadata&&typeof metadata==='object'&&!Array.isArray(metadata)&&'provider'in metadata&&metadata.provider==='zoteroLocal')return 'Zotero Local API导入 · 待人工核对';
 return {manual:'手工录入 · 待人工核对',crossref:'Crossref获取 · 待人工核对',bibtex:'BibTeX导入 · 待人工核对',edited:'人工修订 · 原来源保留，待核对'}[record.verifiedSource];
}
