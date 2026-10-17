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
const docs = ['README.md', 'AGENTS.md', ...(existsSync(join(root, 'CHANGELOG.md')) ? ['CHANGELOG.md'] : []), ...collectDocs('docs')];
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
assert.equal(JSON.parse(read('package.json')).version, config.version);
const lock = JSON.parse(read('package-lock.json'));
assert.equal(lock.version, config.version);
assert.equal(lock.packages[''].version, config.version);
assert.equal(read('src-tauri/Cargo.toml').match(/^version\s*=\s*"([^"]+)"/m)?.[1], config.version);
assert.deepEqual(config.app.security.capabilities, [main.identifier, overlay.identifier]);
assert.deepEqual(main.windows, ['main']);
assert.deepEqual(overlay.windows, ['capture-overlay-*']);
const mainCaptureCommands = ['list_displays', 'capture_fullscreen', 'capture_region', 'region_result_png'];
const overlayCommands = ['region_overlay_info', 'region_overlay_image', 'ready_region_overlay', 'commit_region_selection', 'cancel_region_selection'];
const workflowCommands = ['get_preferences','update_preferences','choose_save_directory','capture_result_png','current_capture','discard_capture','copy_capture_image','save_capture_png'];
const editorCommands = ['save_editor_project','open_editor_project','export_editor_png','copy_editor_png'];
const documentCommands = ['save_markdown_document','open_markdown_document','release_markdown_document','rename_markdown_document'];
const markdownCommands = ['export_knowledge_markdown'];
const desktopCommands = ['desktop_status','desktop_action','complete_desktop_exit'];
const libraryCommands=['create_screenshot','library_status','reconcile_library','list_screenshots','screenshot_page','get_knowledge_card','save_knowledge_card','open_card_source','get_screenshot','screenshot_image','update_screenshot','delete_screenshot','list_notes','save_note','delete_note','list_tags','save_tag','delete_tag','get_screenshot_tags','set_screenshot_tags'];
const permission = (command) => `allow-${command.replaceAll('_', '-')}`;
for (const command of [...mainCaptureCommands,...workflowCommands,...editorCommands,...libraryCommands,...markdownCommands,...desktopCommands,...documentCommands]) assert.ok(main.permissions.includes(permission(command)));
assert.deepEqual([...overlay.permissions].sort(), overlayCommands.map(permission).sort());
assert.equal([...main.permissions, ...overlay.permissions].some((name) => name.startsWith('fs:') || name.startsWith('core:window:allow-create')), false);
for (const command of [...mainCaptureCommands, ...overlayCommands,...workflowCommands,...editorCommands,...libraryCommands,...markdownCommands,...desktopCommands,...documentCommands]) {
  assert.ok(read('src-tauri/build.rs').includes(`"${command}"`), `Missing command permission metadata: ${command}`);
  const module = documentCommands.includes(command)?'document':desktopCommands.includes(command)?'desktop':markdownCommands.includes(command)?'markdown':libraryCommands.includes(command)?'library':editorCommands.includes(command)?'editor':workflowCommands.includes(command)?'workflow':['list_displays', 'capture_fullscreen'].includes(command) ? 'capture' : 'region';
  assert.ok(read('src-tauri/src/lib.rs').includes(`commands::${module}::${command}`), `Missing command registration: ${command}`);
}
for(const plugin of ['clipboard-manager','dialog','global-shortcut','opener']) assert.equal(main.permissions.some((p)=>p.startsWith(`${plugin}:`)),false,'Only bounded Rust wrappers are allowed');
assert.ok(main.permissions.includes('core:event:allow-listen'));assert.ok(main.permissions.includes('core:event:allow-unlisten'));
execFileSync('git', ['diff', '--exit-code', '--', 'src-tauri/migrations/001_workspace.sql'], { cwd: root, stdio: 'pipe' });
assert.equal(read('src-tauri/src/storage/migrations.rs').includes('pub const SCHEMA_VERSION: u32 = 2;'),true);
assert.ok(read('src-tauri/migrations/002_library.sql').includes('CREATE TABLE pending_file_deletions'));
execFileSync('git', ['diff', '--check'], { cwd: root, stdio: 'pipe' });
console.log(JSON.stringify({ result: 'passed', documents: docs.length, localLinks: links, commandPermissions: 'consistent; no fs or window-create grants', storageAndMigrations: 'schema v2; published v1 migration unchanged' }, null, 2));
