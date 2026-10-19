import {describe,it,expect} from 'vitest';
import {parseOCRContent} from './api';
import {highlightParts} from './highlight';
const record={screenshotId:'a'.repeat(32),recognizedText:'中文 OCR $x^2$ <script>',language:'zh-Hans-CN',recognitionStatus:'completed',ocrEngineVersion:'Windows.Media.Ocr',updatedAt:1,sourceImageRevision:`original-v1:${'a'.repeat(32)}`,revision:2,error:null,corrected:true};
describe('persisted OCR boundary and safe highlights',()=>{
 it('preserves corrected plain Unicode/LaTeX text independently from notes',()=>{expect(parseOCRContent(record)).toEqual(record);expect(parseOCRContent(null)).toBeNull();});
 it('rejects malformed identity, future source and unbounded data',()=>{for(const patch of [{screenshotId:'../x'},{sourceImageRevision:'future:1'},{revision:0},{recognitionStatus:'future'},{recognizedText:'a\0b'},{recognizedText:'中'.repeat(400000)}])expect(()=>parseOCRContent({...record,...patch})).toThrow();});
 it('matches Chinese short words, English case and literal special symbols without HTML',()=>{for(const [text,keyword]of [['科研研究科研','科研'],['Local OCR local','LOCAL'],['<script> x%_ \\','<script>'],['中文LaTeX $x^2$','LaTeX']]){const parts=highlightParts(text,keyword);expect(parts.map(p=>p.text).join('')).toBe(text);expect(parts.some(p=>p.match)).toBe(true);}expect(highlightParts('empty','')).toEqual([{text:'empty',match:false}]);});
});
