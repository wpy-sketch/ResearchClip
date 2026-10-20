import {create} from 'zustand';
import type {Target} from './api';
interface State{open:boolean;target:Target|null;documentId:string|null;show:(target?:Target,documentId?:string)=>void;close:()=>void}
export const useReferenceStore=create<State>(set=>({open:false,target:null,documentId:null,show:(target,documentId)=>set({open:true,target:target??null,documentId:documentId??null}),close:()=>set({open:false,target:null,documentId:null})}));
