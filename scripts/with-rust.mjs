import { spawn } from 'node:child_process';
import { existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

// A project-local toolchain is optional. Normal installations use Rust from PATH.
const root = dirname(dirname(fileURLToPath(import.meta.url)));
const localCargo = join(root, '.tools', 'cargo');
const localRustup = join(root, '.tools', 'rustup');
const suffix = process.platform === 'win32' ? '.exe' : '';
const env = { ...process.env };
const pathKey = Object.keys(env).find((key) => key.toUpperCase() === 'PATH') ?? 'PATH';
if (existsSync(join(localCargo, 'bin', `cargo${suffix}`))) {
  env.CARGO_HOME = localCargo;
  env.RUSTUP_HOME = localRustup;
  // Windows commonly exposes "Path". Do not add a duplicate uppercase PATH key.
  env[pathKey] = `${join(localCargo, 'bin')}${process.platform === 'win32' ? ';' : ':'}${env[pathKey] ?? ''}`;
}
const [tool, ...args] = process.argv.slice(2);
if (!['tauri', 'cargo'].includes(tool)) throw new Error('Expected tauri or cargo');
const command = tool === 'tauri' ? process.execPath : `cargo${suffix}`;
const parameters = tool === 'tauri'
  ? [join(root, 'node_modules', '@tauri-apps', 'cli', 'tauri.js'), ...args]
  : args;
const child = spawn(command, parameters, { cwd: root, env, stdio: 'inherit', shell: false });
child.on('error', (error) => {
  console.error(`无法启动 ${tool}: ${error.message}。请参考 README 安装 Rust MSVC 工具链。`);
  process.exitCode = 1;
});
child.on('exit', (code) => { process.exitCode = code ?? 1; });
