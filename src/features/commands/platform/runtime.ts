import {commandNames,type CommandName,type CommandArgs} from './contracts';
import {ErrorReporter,CommandFailure} from './errors';
import {PermissionService} from './permissions';
import {ResourceResolver} from './resolver';
import {EventBus} from './events';
import {TaskManager} from './tasks';
import {CommandRegistry} from './registry';
import {workspaceManager} from '../../workspace/manager';
import {useDocumentStore} from '../../document/store';
import {openLibraryEditor} from '../../../app/library-actions';
export const commandErrors=new ErrorReporter();
const permissions=new PermissionService();
const user=permissions.issue('desktop-user',commandNames);
const publisher=permissions.issue('application-router',['event:resource.opened','event:markdown.imageInserted','event:command.completed']);
const statusReader=permissions.issue('command-status',['event:resource.opened','event:markdown.imageInserted']);
export const applicationEvents=new EventBus(permissions,commandErrors);
export const commandTasks=new TaskManager(commandErrors);
export const applicationCommands=new CommandRegistry(permissions,new ResourceResolver(permissions),commandTasks,commandErrors,applicationEvents,publisher);
applicationCommands.register('document.open',async(context,args)=>{
 const resource=context.resources[0];if(!resource.moduleAvailable)throw new CommandFailure('MODULE_UNAVAILABLE');if(!workspaceManager.state.getState().ready)throw new CommandFailure('MODULE_UNAVAILABLE');
 context.signal.throwIfAborted();context.commit();
 const editorKind=resource.editor==='pdf'?'pdf':resource.editor==='markdown'?'markdown':'image';
 const existing=[...workspaceManager.sessions.values()].find(s=>s.panel.resourceId===args.resource.id&&s.panel.kind===editorKind);
 if(existing)workspaceManager.focus(existing.panel.id);
 else if(resource.editor==='markdown'){
  await useDocumentStore.getState().openProject(args.resource.id);
  if(![...workspaceManager.sessions.values()].some(s=>s.panel.kind==='markdown'&&s.panel.resourceId===args.resource.id))throw new CommandFailure('OPERATION_FAILED');
 }else if(resource.editor==='pdf')await(await import('../../pdf/PdfViewerSession')).openPdf(args.resource.id);
 else if(resource.editor==='image')await openLibraryEditor(args.resource.id);
 else throw new CommandFailure('MODULE_UNAVAILABLE');
 const opened=[...workspaceManager.sessions.values()].find(s=>s.panel.resourceId===args.resource.id&&s.panel.kind===editorKind);
 if(!opened||opened.workspaceId!==workspaceManager.workspaceId||opened.panel.hidden||!workspaceManager.model.getNodeById(opened.panel.id))throw new CommandFailure('OPERATION_FAILED');
 applicationEvents.publish(publisher,'resource.opened',{requestId:context.request.requestId,resource:args.resource},context.cause);
});
applicationCommands.register('markdown.insertImage',async(context,args)=>{
 const session=[...workspaceManager.sessions.values()].find(s=>s.panel.kind==='markdown'&&s.panel.resourceId===args.document.id),binding=session?workspaceManager.documents.get(session.panel.id):null;
 if(!session||!binding)throw new CommandFailure('OPERATION_FAILED');const state=binding.store.getState();if(!state.document||state.phase!=='idle'||state.pending)throw new CommandFailure('BUSY');
 context.signal.throwIfAborted();workspaceManager.focus(session.panel.id);context.commit();
 if(!await binding.store.getState().addImages([args.image.id]))throw new CommandFailure('OPERATION_FAILED');
 applicationEvents.publish(publisher,'markdown.imageInserted',{requestId:context.request.requestId,document:args.document,image:args.image},context.cause);
});
for(const name of ['pdf.navigateTo','screenshot.capturePdfSelection','annotation.create','codeWorkspace.open','agent.requestEdit'] as const)applicationCommands.register(name,async()=>{throw new CommandFailure('MODULE_UNAVAILABLE');});
/** Only trusted, explicit UI actions call this entry. External/Agent transports have no handle or IPC bridge. */
export function executeUserCommand<N extends CommandName>(name:N,args:CommandArgs[N]){return applicationCommands.execute(user,name,args);}
export function runUserCommand<N extends CommandName>(name:N,args:CommandArgs[N]){void executeUserCommand(name,args).catch(()=>{/* CommandRegistry has already reported a safe, visible diagnostic. */});}
export function subscribeCommandNotices(listener:(message:string)=>void){
 const opened=applicationEvents.subscribe(statusReader,'resource.opened',()=>listener('资源已在独立编辑面板打开。'));
 try{const inserted=applicationEvents.subscribe(statusReader,'markdown.imageInserted',()=>listener('素材图已插入指定 Markdown 草稿；请另行保存。'));return()=>{opened();inserted();};}catch(e){opened();throw e;}
}
