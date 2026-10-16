# ResearchClip 主界面与导航验收

日期：2026-10-12，Asia/Hong_Kong。本轮交付**应用主界面与导航**，没有接入真实截图、素材数据库或编辑功能。工程初始化结果另见 [历史初始化验收](INITIALIZATION-VALIDATION.md)。

## 修改内容

- app/App组合侧栏、工具栏、内容区和底部状态栏；app/layout包含Sidebar、Toolbar、WorkspaceDialogs。
- features/home欢迎引导支持收起与通过“使用指南”恢复；library提供素材/收藏空状态与网格/列表；tags提供空状态；settings提供主题、会话视图和真实桌面连接检查。
- app/ui-store采用简单Zustand会话状态，没有persist、localStorage或数据库调用。导航、主题、视图、欢迎引导和搜索输入重启后恢复默认。
- styles.css使用Tailwind 4与主题CSS变量：默认石墨深色，支持浅色，柔和边框与少量紫灰强调色；仅120ms颜色反馈，尊重减少动态效果设置。
- shared/ui/Modal使用原生dialog，支持Esc、背景点击、焦点约束、搜索输入初始焦点和关闭后恢复焦点。Ctrl/⌘ K为应用窗口内搜索入口，未注册系统全局快捷键。
- 新建截图按钮打开说明，区域/全屏动作禁用并标明即将开放；搜索输入明确仅用于界面预览，不显示虚构搜索结果。没有假卡片、保存成功提示或持久化设置。
- 新增统一图标依赖lucide-react 1.55.0（ISC，兼容React 19），更新npm锁文件，未升级其他依赖版本。[Lucide React官方文档](https://lucide.dev/guide/react)
- Tauri主窗口默认1180×800，最低480×400，保留原生拖动/缩放；增加core:window:allow-set-theme，用官方Window.setTheme同步标题栏主题。[Tauri Window API](https://v2.tauri.app/reference/javascript/api/namespacewindow/)
- 更新README、ARCHITECTURE、ROADMAP当前状态与冒烟脚本。Rust业务源码、Cargo.lock、SQL迁移均未修改，schema仍为既有v1；当前界面不调用get_workspace_status。

## 实际环境与检查

Windows x64 10.0.26200，Node 22.22.2，npm 10.9.7，项目内Rust MSVC 1.99.0，WebView2 154.0.4258.62。沿用初始化时的Tauri 2 / React 19 / Vite 8 / Tailwind 4配置。

| 命令 | 实际结果 | 退出码 |
| --- | --- | --- |
| npm ci --fetch-retries=1 --fetch-timeout=30000 | 根据锁文件干净安装214 packages | 0 |
| npm ls --depth=0 | 依赖树无peer错误 | 0 |
| npm run check | TypeScript、ESLint、33项Vitest与生产构建通过 | 0 |
| npm run format:rust | rustfmt检查通过 | 0 |
| npm run test:rust | 9项Rust测试通过 | 0 |
| npm run check:rust | clippy all-targets、-D warnings通过 | 0 |
| npm run desktop:build | Windows release exe构建通过，no-bundle | 0 |
| npm run test:smoke | Edge生产预览6组检查通过，含32组响应式布局组合 | 0 |
| npm run test:smoke -- --desktop | 常规桌面环境真实启动/重启，14组检查通过 | 0 |
| node .tools/check-main-ui.mjs | 10份文档、22个本地链接、围栏、配置/锁文件一致性与git diff --check通过；Rust业务/迁移未改动 | 0 |

33项前端测试包括新增4项会话状态测试与原有29项版本/IPC/错误路径测试；Rust保留9项已有契约、SQLite和迁移保护测试。没有把旧验收报告当作本轮通过证据。

前端生产输出：JS 263.20 kB（gzip 82.00 kB），CSS 28.35 kB（gzip 5.51 kB）。未测量内存、启动性能或截图性能。

Windows可执行文件：src-tauri/target/release/researchclip.exe；10,313,216 bytes；SHA-256 297C7D5372DBE62679BF1B20D86F10B650518544A8D53ECFDD888B728570E0B1。

## 交互与布局证据

- 四个导航页面、选中项、网格/列表切换、欢迎引导收起/恢复、设置与工具栏状态同步通过。
- 截图说明、禁用动作、搜索输入/清空、Ctrl+K、Esc、搜索初始焦点、弹窗焦点约束、关闭后恢复触发按钮通过。
- 浏览器预览禁用后端检查；Windows实际按钮两次调用test_connection，均返回“ResearchClip：Rust 后端连接成功。”，响应版本与getVersion一致。
- 深/浅主题与设置页选择通过；原生标题栏主题API无拒绝错误。重启后恢复默认深色、网格与欢迎引导。
- 480×400、640×500、960×680、1440×900 × 两种主题 × 四个页面，共32种组合无横向溢出；侧栏、工具栏、内容区与底部栏均在窗口内。小窗口内容区单独滚动。
- 原生检查通过只读网络观察记录实际IPC：plugin:window|set_theme与test_connection；同时断言确实捕获这两个命令，交互期间未调用get_workspace_status。启动不建库另外由未修改的Rust组装与当前前端调用链核对，网络观察未覆盖连接测试工具之前的启动瞬间。
- 全部交互未写localStorage/sessionStorage，无canvas。WebView2测试资料在artifacts/main-ui-webview-profile，没有清空真实AppData或数据库；测试结束关闭测试子进程。
- 无renderer console/page错误。已查看深色、浅色、窄窗口与Windows截图，确认字体、留白、边框及内容滚动状态。

证据文件（不纳入Git）：artifacts/main-ui-preview-smoke.json、main-ui-desktop-smoke.json、main-ui-desktop-dark.png、main-ui-desktop-light.png、对应restart截图，以及main-ui-preview-480x400.png等四个窗口尺寸截图。历史init/M1证据保留。

## 已修复问题与限制

- 初次交互冒烟发现弹窗卸载后焦点未回到触发按钮；补上显式恢复。随后发现React autoFocus在dialog打开前执行，改为showModal后设置初始焦点。保留断言并重跑后通过。
- 沙箱内原生冒烟首次退出码1，主WebView连接超时；诊断记录显示WebView2 GPU进程STATUS_ACCESS_DENIED。没有关闭GPU/浏览器安全机制或修改应用代码；在常规桌面环境重跑成功。沙箱内运行失败不能视为通过。
- 最初尝试替换只读Tauri内部invoke来观察命令，未捕获任何请求，不能作为证据；已移除，改为只读IPC网络观察，并要求捕获真实调用后才通过。
- macOS、安装包/签名、真实截图、多显示器/DPI截图矩阵与业务持久化未验证；这些不属于本轮界面验收。
- 当前无未解决的主界面阻塞。主题与视图仅在本次会话生效，所有素材相关功能仍未开放。
- 下一候选步骤：按独立请求开展M2A Windows原生捕获实验。本轮完成后停止，不自动进入后续阶段。
