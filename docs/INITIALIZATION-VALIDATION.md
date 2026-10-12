# ResearchClip 工程初始化验收

日期：2026-10-12，Asia/Hong_Kong。**初始化通过**。本轮仅交付欢迎主窗口、版本与Rust通信验证；没有实现截图、图片编辑或其他MVP业务。

## 模板与环境

- 使用官方稳定create-tauri-app npm包4.7.5生成react-ts / npm / Tauri 2模板到忽略的.tools/researchclip-template作对照；生成器帮助显示内部版本4.8.0。未覆盖既有仓库与规划文档。
- 保留官方Tauri lib/main/build/config布局，采用Rust edition 2024；删除默认greet/logo/opener演示用途，实际应用不安装opener插件。
- 实际依赖：Tauri Rust/API 2.12.3、CLI 2.12.2、React 19.3.0、Vite 8.3.4、React plugin 6.1.2、TypeScript 6.0.3、Tailwind 4.3.3、Zustand 5.0.15、Vitest 5.0.3、ESLint 9.39.4。TypeScript按模板采用兼容的6.0系列，不升级到ESLint当前peer范围之外的7.x。
- 本机Windows x64（10.0.26200）、Node 22.22.2、npm 10.9.7、项目内Rust MSVC 1.99.0、C++ Build Tools/Windows SDK、WebView2 154.0.4258.62。
- npm package-lock.json与Cargo.lock为受版本控制文件；本轮干净npm ci成功，依赖树无peer错误。

模板来源：[Tauri create-project](https://v2.tauri.app/start/create-project/)。工具链版本以锁文件与实际命令为准，不要求不同Tauri子包的小版本号完全一致。

## 修改内容

- App/HomeView改为顶部ResearchClip、中央欢迎文字和按钮、底部版本；移除旧library/editor/settings占位视图及PhaseBadge，原文件备份在忽略的.tools/initialization-view-backup。
- Zustand管理版本读取、通信等待/成功/失败；API验证Rust响应，阻止预览模式调用，去重并发请求，拒绝前后端版本不一致。
- Rust增加test_connection及camelCase响应DTO；main窗口仅拥有版本读取、通信测试和既有诊断权限。
- 保留SQLite v1与既有诊断，取消启动时自动打开数据库；未新增迁移，未删除或重置用户资料。
- 对齐Vite 8 / React plugin 6 / TypeScript 6 / Vitest 5；Tailwind与ESLint继续使用兼容配置；移除未使用的lucide-react。
- 更新冒烟脚本、README和架构当前状态；Git忽略依赖/生成文件/工具链/测试资料/本地SQLite/环境配置，保留锁文件。

## 实际检查

| 命令 | 实际结果 | 退出码 |
| --- | --- | --- |
| npm ci | 干净安装213 packages成功 | 0 |
| npm ls --depth=0 | 依赖树无错误 | 0 |
| npm run typecheck | TypeScript严格检查通过 | 0 |
| npm run lint | ESLint通过，无warning | 0 |
| npm test | 3个测试文件、29项通过，0失败 | 0 |
| npm run build | Vite 8生产构建通过，27 modules transformed | 0 |
| npm run test:rust | 9项Rust测试通过，0失败 | 0 |
| npm run format:rust | rustfmt检查通过 | 0 |
| npm run check:rust | clippy all-targets、-D warnings通过 | 0 |
| npm run desktop:build | release Windows exe编译通过，no-bundle | 0 |
| npm run test:smoke | 浏览器预览2组检查通过，无renderer错误 | 0 |
| npm run test:smoke -- --desktop | Windows exe真实启动两次、6组检查通过，无renderer错误 | 0 |

前端测试覆盖实际命令名称、版本读取、响应校验、浏览器防误调用、重复初始化/点击、失败重试、版本冲突和旧消息清理；保留10项已有工作区边界测试。Rust新增通信DTO契约测试，保留8项SQLite/迁移/用户数据保护测试。

生产前端输出：JS 224.67 kB（gzip 70.86 kB），CSS 9.54 kB（gzip 2.96 kB）。这些是构建输出，未测量运行内存或截图性能。

## 原生运行检查

使用Playwright连接测试子进程的WebView2，不mock Rust命令。实际确认：

- 顶部ResearchClip、中央欢迎使用 ResearchClip、底部版本0.1.0可见。
- 按钮调用test_connection返回“ResearchClip：Rust 后端连接成功。”；响应版本与Tauri getVersion一致。
- 重复点击、退出后再启动均成功；初始化视图没有canvas/截图/编辑页面。
- WebView2测试profile位于artifacts/init-webview-profile，不重置用户AppData或数据库；退出测试后原生进程已结束。
- 浏览器版明确显示预览，按钮禁用且没有伪造成功消息；480×400与960×680无横向溢出。

profile隔离使用已核对的WEBVIEW2_USER_DATA_FOLDER环境变量：[Microsoft WebView2环境文档](https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/webview2-idl?view=webview2-1.0.3967.48)。仅测试子进程启用本地CDP调试端口。

本轮证据为artifacts/init-desktop-smoke.json、init-preview-smoke.json、init-desktop.png、init-desktop-restart.png、init-preview.png。历史M1的desktop-smoke.json及截图保留原样。测试产物不纳入Git。

Windows可执行文件：src-tauri/target/release/researchclip.exe；10,297,856 bytes；SHA-256 D45F024C2371E36EAE04DB0386B830C91BD14C839804E99A34BA3BCF54BCC65F。

## 问题与限制

- 初次升级因旧node_modules/plugin-react/Vite peer关系发生ERESOLVE；备份旧安装树和锁文件后干净安装解决，后续npm ci与检查通过，没有使用force或legacy-peer-deps。
- Vite首次提示配置JSON导入缺少attributes；改为default import with { type: 'json' }后构建/测试无该warning。
- 生成器因系统PATH不含Rust而提示缺失，但项目内工具链由包装脚本识别，实际Rust测试/编译已通过。
- macOS、安装包、签名、截图/编辑功能和硬件矩阵未执行，不属于本次初始化验收。
- 提交前git diff --cached --check发现部分文件EOF空行；统一LF与单个末尾换行、增加.gitattributes后通过，文本内容没有业务改动。
- 文档链接/围栏/版本一致性检查：9份文档、18个本地链接通过，退出码0；两个锁文件可被Git跟踪，工具链/编译产物/测试profile被忽略。
- 当前无未解决的初始化阻塞；后续应在新请求中推进M2A，不由本轮自动开始。
