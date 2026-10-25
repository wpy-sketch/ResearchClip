import {create} from 'zustand';
import type {Target} from './api';
import type {DocumentStore} from '../document/store';
interface State{open:boolean;target:Target|null;documentId:string|null;owner:DocumentStore|null;show:(target?:Target,documentId?:string,owner?:DocumentStore)=>void;close:()=>void}
export const useReferenceStore=create<State>(set=>({open:false,target:null,documentId:null,owner:null,show:(target,documentId,owner)=>set({open:true,target:target??null,documentId:documentId??null,owner:owner??null}),close:()=>set({open:false,target:null,documentId:null,owner:null})}));
