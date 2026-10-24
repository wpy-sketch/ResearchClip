import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const read = (file) => readFileSync(join(root, file), 'utf8');
const collectDocs = (directory) => readdirSync(join(root, directory), { withFileTypes: true })
  .flatMap((entry) => entry.isDirectory() ? collectDocs(join(directory, entry.name))
    : entry.name.endsWith('.md') ? [join(directory, entry.name)] : []);
const docs = ['math-ocr/README.md','browser-extension/README.md','browser-extension/PROTOCOL.md','README.md', 'AGENTS.md', ...(existsSync(join(root, 'CHANGELOG.md')) ? ['CHANGELOG.md'] : []), ...collectDocs('docs')];
let links = 0;
for (const file of docs) {
  const content = read(file);
  assert.equal((content.match(/^\s*```/gm) ?? []).length % 2, 0, `Unclosed Markdown fence: ${file}`);
  for (const match of content.matchAll(/\[[^\]]*\]\(([^)]+)\)/g)) {
    const target = match[1].split('#')[0];
    if (!target || /^[a-z][\w+.-]*:/i.test(target)) continue;
    assert.ok(existsSync(resolve(root, dirname(file), decodeURIComponent(target))), `Missing local link: ${file}: ${target}`);
    links += 1;
  }
}
const config = JSON.parse(read('src-tauri/tauri.conf.json'));
const main = JSON.parse(read('src-tauri/capabilities/main.json'));
const overlay = JSON.parse(read('src-tauri/capabilities/capture-overlay.json'));
const pins = JSON.parse(read('src-tauri/capabilities/desktop-pins.json'));
const mainPinCommands=['create_pin','recover_pins','pin_status'];
const pinCommands=['pin_image','pin_action'];
assert.deepEqual(pins.windows,['pin-*']);
assert.equal(JSON.parse(read('package.json')).version, config.version);
const lock = JSON.parse(read('package-lock.json'));
assert.equal(lock.version, config.version);
assert.equal(lock.packages[''].version, config.version);
assert.equal(read('src-tauri/Cargo.toml').match(/^version\s*=\s*"([^"]+)"/m)?.[1], config.version);
assert.deepEqual(config.app.security.capabilities, [main.identifier, overlay.identifier, pins.identifier, "scroll-control", "window-picker"]);
assert.deepEqual(main.windows, ['main']);
assert.deepEqual(overlay.windows, ['capture-overlay-*']);
const syncCommands=['sync_status','sync_configure','sync_start','sync_stop','sync_resolve','sync_authorize_documents'];for(const command of syncCommands){assert.ok(main.permissions.includes('allow-'+command.replaceAll('_','-')));assert.ok(read('src-tauri/src/lib.rs').includes('commands::sync::'+command));assert.ok(read('src-tauri/build.rs').includes('"'+command+'"'));}
const aiCommands=['similarity_ai_status','choose_similarity_ai_bundle','start_similarity_ai','cancel_similarity_ai'];for(const command of aiCommands){assert.ok(main.permissions.includes('allow-'+command.replaceAll('_','-')));assert.ok(read('src-tauri/src/lib.rs').includes('commands::similarity_ai::'+command));assert.ok(read('src-tauri/build.rs').includes('"'+command+'"'));}
const mathCommands=["math_ocr_status","cancel_math_ocr","download_math_models","choose_math_runtime","import_math_models","recognize_math_ocr"];for(const command of mathCommands){assert.ok(main.permissions.includes('allow-'+command.replaceAll('_','-')));assert.ok(read('src-tauri/src/lib.rs').includes('commands::math_ocr::'+command));assert.ok(read('src-tauri/build.rs').includes('"'+command+'"'));}
const indexCommands=["get_ocr_content","correct_ocr_content","start_ocr_index","ocr_index_status","ocr_index_action","rebuild_search_index"];for(const command of indexCommands){assert.ok(main.permissions.includes('allow-'+command.replaceAll('_','-')));assert.ok(read('src-tauri/src/lib.rs').includes('commands::ocr_index::'+command));assert.ok(read('src-tauri/build.rs').includes('"'+command+'"'));}
const clipperCommands=['clipper_status','clipper_connection']; for(const command of clipperCommands){assert.ok(main.permissions.includes('allow-'+command.replaceAll('_','-')));assert.ok(read('src-tauri/src/lib.rs').includes('commands::clipper::'+command));}
const extension=JSON.parse(read('browser-extension/manifest.json'));assert.deepEqual(extension.permissions,['activeTab','scripting','nativeMessaging','storage']);assert.equal(extension.host_permissions,undefined);assert.equal(extension.content_scripts,undefined);assert.equal(extension.background,undefined);
const scrollingCommands=['scroll_targets','scroll_prepare','scroll_start','scroll_status','scroll_action'];
const scrolling=JSON.parse(read('src-tauri/capabilities/scroll-control.json'));assert.deepEqual(scrolling.windows,['scroll-control-*']);assert.deepEqual(scrolling.permissions,['allow-scroll-status','allow-scroll-action']);
const picker=JSON.parse(read('src-tauri/capabilities/window-picker.json'));assert.deepEqual(picker.windows,['window-picker-*']);assert.deepEqual(picker.permissions,['allow-window-picker-status','allow-window-picker-action']);for(const name of ['capture_window','window_picker_status','window_picker_action']){assert.ok(read('src-tauri/build.rs').includes('"'+name+'"'));assert.ok(read('src-tauri/src/lib.rs').includes('commands::window_capture::'+name));}
  const resourceCommands=['resolve_research_resource','prepare_research_command'];for(const command of resourceCommands){assert.ok(main.permissions.includes('allow-'+command.replaceAll('_','-')));assert.ok(read('src-tauri/src/lib.rs').includes('commands::research_commands::'+command));assert.ok(read('src-tauri/build.rs').includes('"'+command+'"'));assert.ok(!overlay.permissions.includes('allow-'+command.replaceAll('_','-'))&&!pins.permissions.includes('allow-'+command.replaceAll('_','-')));}
  const mainCaptureCommands = ['capture_window','list_displays', 'capture_fullscreen', 'capture_region', 'region_result_png'];
const overlayCommands = ['region_overlay_info', 'region_overlay_image', 'ready_region_overlay', 'commit_region_selection', 'cancel_region_selection'];
const workflowCommands = ['get_preferences','update_preferences','choose_save_directory','capture_result_png','current_capture','discard_capture','copy_capture_image','save_capture_png'];
const projectCommands=['list_projects','save_project','delete_empty_project','select_library_screenshots','batch_library_screenshots','list_project_documents','assign_project_document','sync_project_document','open_project_document'];
const importCommands=['choose_comparison_image','import_clipboard_image','import_image_files'];
assert.equal(config.app.windows.find(w=>w.label==='main').dragDropEnabled,false);
const ocrCommands=['ocr_status','recognize_ocr','copy_ocr_text'];
const editorCommands = ['save_editor_project','open_editor_project','export_editor_png','copy_editor_png'];
const documentCommands = ['open_markdown_external','save_markdown_document','open_markdown_document','release_markdown_document','rename_markdown_document','list_markdown_templates','load_markdown_template','create_markdown_template','rename_markdown_template','delete_markdown_template'];
const markdownCommands = ['export_knowledge_markdown'];
const desktopCommands = ['desktop_status','desktop_action','complete_desktop_exit'];
const libraryCommands=['create_screenshot','library_status','reconcile_library','list_screenshots','screenshot_page','get_knowledge_card','save_knowledge_card','open_card_source','get_screenshot','screenshot_image','update_screenshot','delete_screenshot','list_notes','save_note','delete_note','list_tags','save_tag','delete_tag','get_screenshot_tags','set_screenshot_tags'];
const permission = (command) => `allow-${command.replaceAll('_', '-')}`;
for (const command of [...scrollingCommands,...mainCaptureCommands,...workflowCommands,...editorCommands,...ocrCommands,...projectCommands,...importCommands,...libraryCommands,...markdownCommands,...desktopCommands,...documentCommands,...mainPinCommands]) assert.ok(main.permissions.includes(permission(command)));
assert.deepEqual([...pins.permissions].sort(),pinCommands.map(permission).sort());
assert.deepEqual([...overlay.permissions].sort(), overlayCommands.map(permission).sort());
assert.equal([...main.permissions, ...overlay.permissions, ...pins.permissions].some((name) => name.startsWith('fs:') || name.startsWith('core:window:allow-create')), false);
for (const command of [...scrollingCommands,...mainCaptureCommands, ...overlayCommands,...workflowCommands,...editorCommands,...ocrCommands,...projectCommands,...importCommands,...libraryCommands,...markdownCommands,...desktopCommands,...documentCommands,...mainPinCommands,...pinCommands]) {
  assert.ok(read('src-tauri/build.rs').includes(`"${command}"`), `Missing command permission metadata: ${command}`);
  const module = command==='capture_window'?'window_capture':scrollingCommands.includes(command)?'scrolling':projectCommands.includes(command)?'projects':importCommands.includes(command)?'import':ocrCommands.includes(command)?'ocr':[...mainPinCommands,...pinCommands].includes(command)?'pins':documentCommands.includes(command)?'document':desktopCommands.includes(command)?'desktop':markdownCommands.includes(command)?'markdown':libraryCommands.includes(command)?'library':editorCommands.includes(command)?'editor':workflowCommands.includes(command)?'workflow':['list_displays', 'capture_fullscreen'].includes(command) ? 'capture' : 'region';
  assert.ok(read('src-tauri/src/lib.rs').includes(`commands::${module}::${command}`), `Missing command registration: ${command}`);
}
for(const plugin of ['clipboard-manager','dialog','global-shortcut','opener']) assert.equal(main.permissions.some((p)=>p.startsWith(`${plugin}:`)),false,'Only bounded Rust wrappers are allowed');
assert.ok(main.permissions.includes('core:event:allow-listen'));assert.ok(main.permissions.includes('core:event:allow-unlisten'));
execFileSync('git', ['diff', '--exit-code', '--', 'src-tauri/migrations/001_workspace.sql'], { cwd: root, stdio: 'pipe' });
execFileSync('git', ['diff', '--exit-code', '--', 'src-tauri/migrations/002_library.sql'], { cwd: root, stdio: 'pipe' });
execFileSync('git', ['diff', '--exit-code', '--', 'src-tauri/migrations/003_projects.sql'], { cwd: root, stdio: 'pipe' });
execFileSync('git', ['diff', '--exit-code', '--', 'src-tauri/migrations/004_ocr_index.sql'], { cwd: root, stdio: 'pipe' });
execFileSync('git', ['diff', '--exit-code', '--', 'src-tauri/migrations/005_annotation_revisions.sql'], { cwd: root, stdio: 'pipe' });
const currentSchema = Number(read('src-tauri/src/storage/migrations.rs').match(/pub const SCHEMA_VERSION: u32 = (\d+);/)?.[1]);
assert.equal(currentSchema, 12);
assert.ok(existsSync(join(root, 'src-tauri/migrations/012_research_resources.sql')));
execFileSync('git', ['diff', '--exit-code', '--', 'src-tauri/migrations/006_sync.sql'], { cwd: root, stdio: 'pipe' });
const similarityCommands=['similarity_status','start_similarity_index','cancel_similarity_index','find_similar_images','duplicate_image_groups'];for(const command of similarityCommands){assert.ok(main.permissions.includes('allow-'+command.replaceAll('_','-')));assert.ok(read('src-tauri/src/lib.rs').includes('commands::similarity::'+command));assert.ok(read('src-tauri/build.rs').includes('"'+command+'"'));}
const semanticCommands=['semantic_status','semantic_start','semantic_cancel','semantic_clear','semantic_import'];for(const command of semanticCommands){assert.ok(main.permissions.includes('allow-'+command.replaceAll('_','-')));assert.ok(read('src-tauri/src/lib.rs').includes('commands::embeddings::'+command));assert.ok(read('src-tauri/build.rs').includes('"'+command+'"'));for(const other of [pins,overlay])assert.ok(!other.permissions.includes('allow-'+command.replaceAll('_','-')));}
execFileSync('git',['diff','--exit-code','--','src-tauri/migrations/010_annotation_anchors.sql'],{cwd:root,stdio:'pipe'});
execFileSync('git', ['diff', '--exit-code', '--', 'src-tauri/migrations/009_internal_links.sql'], { cwd: root, stdio: 'pipe' });
const anchorCommands=['annotation_anchors','resolve_annotation_anchor','save_annotation_anchor','delete_annotation_anchor'];for(const command of anchorCommands){assert.ok(main.permissions.includes('allow-'+command.replaceAll('_','-')));assert.ok(read('src-tauri/src/lib.rs').includes('commands::anchors::'+command));assert.ok(read('src-tauri/build.rs').includes('"'+command+'"'));}
assert.ok(read('src-tauri/migrations/002_library.sql').includes('CREATE TABLE pending_file_deletions'));
execFileSync('git', ['diff', '--check'], { cwd: root, stdio: 'pipe' });
console.log(JSON.stringify({ result: 'passed', documents: docs.length, localLinks: links, commandPermissions: 'consistent; no fs or window-create grants', storageAndMigrations: `schema v${currentSchema}; published migrations001–011 unchanged` }, null, 2));

for(const command of ["annotation_history","annotation_revision","save_annotation_revision","restore_annotation_revision"]){assert.ok(main.permissions.includes("allow-"+command.replaceAll("_","-")));assert.ok(read("src-tauri/src/lib.rs").includes("commands::revisions::"+command));assert.ok(read("src-tauri/build.rs").includes("\""+command+"\""));}

execFileSync('git', ['diff', '--exit-code', '--', 'src-tauri/migrations/007_image_features.sql'], { cwd: root, stdio: 'pipe' });
const referenceCommands=['lookup_reference_doi','reference_page','save_references','associate_reference','import_bibtex','open_reference_doi'];for(const command of referenceCommands){assert.ok(main.permissions.includes('allow-'+command.replaceAll('_','-')));assert.ok(read('src-tauri/src/lib.rs').includes('commands::references::'+command));assert.ok(read('src-tauri/build.rs').includes('"'+command+'"'));}

const linkCommands=['link_targets','link_document_source','replace_document_links','link_details','link_graph','set_document_link_tags','export_linked_document'];
for(const command of linkCommands){assert.ok(main.permissions.includes('allow-'+command.replaceAll('_','-')));assert.ok(read('src-tauri/src/lib.rs').includes('commands::links::'+command));assert.ok(read('src-tauri/build.rs').includes('"'+command+'"'));}
execFileSync('git',['diff','--exit-code','--','src-tauri/migrations/008_references.sql'],{cwd:root,stdio:'pipe'});

const publishedMigrations = readdirSync(join(root, 'src-tauri/migrations')).filter(name => /^0(?:0[1-9]|1[01])_/.test(name)).map(name => `src-tauri/migrations/${name}`);
assert.equal(publishedMigrations.length, 11);
execFileSync('git',['diff','--exit-code','v3.1.9','--',...publishedMigrations],{cwd:root,stdio:'pipe'});
