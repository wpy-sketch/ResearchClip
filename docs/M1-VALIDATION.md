# M1 实际验收记录

验收日期：2026-10-12（Asia/Hong_Kong）。**M1 通过；交付到此停止。** 未开始 M2 截图、M3 编辑或后续卡片功能。

## 环境与依赖

- 本机：Microsoft Windows 10.0.26200，x64；Node v22.22.2、npm 10.9.7。
- Visual Studio 2022 Build Tools 的 C++ 工具链已存在；WebView2 Runtime 154.0.4258.62 已存在。
- 原先没有可用 Rust。项目内 `.tools/` 安装隔离 stable MSVC 工具链：rustc 1.99.0、cargo 1.99.0，含 rustfmt/clippy；没有修改系统 PATH。
- 实际锁定：Tauri Rust/API 2.12.3，Tauri CLI 2.12.2，React/React DOM/react-konva 19.3.0，Konva 10.7.1，Zustand 5.0.15，Tailwind 4.3.3，Vite 7.3.7，TypeScript 5.9.3，Vitest 4.0.18，rusqlite 0.37.0。
- 应用 SQLite 实测版本 3.50.2，schema version 1。完整依赖以两个锁文件为准。

## 执行结果

| 命令 / 检查 | 实际结果 | 退出码 |
| --- | --- | --- |
| npm ci | 从锁文件重新安装 230 packages 成功 | 0 |
| npm run typecheck | TypeScript 严格检查通过 | 0 |
| npm run lint | ESLint 通过，max-warnings=0 | 0 |
| npm test | 2 个测试文件，13 项通过，0 失败 | 0 |
| npm run build | Vite 生产构建成功，1831 modules transformed | 0 |
| npm run test:rust | 8 项 Rust 测试通过，0 失败 | 0 |
| npm run format:rust | rustfmt 检查通过 | 0 |
| npm run check:rust | clippy all-targets，通过，-D warnings | 0 |
| npm run desktop:build | release Windows exe 构建成功（no-bundle） | 0 |
| npm run test:smoke | 本机 Edge 运行生产前端，3 组检查通过，renderer errors=[] | 0 |
| npm run test:smoke -- --desktop | 真实 exe + WebView2 启动两次，7 组检查通过，renderer errors=[] | 0 |

前端生产输出：主 JS 238.18 kB（gzip 75.23 kB）；按需加载 EditorView JS 337.20 kB（gzip 103.88 kB）；CSS 13.54 kB（gzip 4.12 kB）。这些为本次 Vite 输出，不是内存占用或安装包体积。

Rust 测试覆盖：Unicode 工作区目录与真实 SQLite 创建；外键/WAL；重复打开保留元数据；拒绝未来 schema 且不降级；拒绝损坏文件且不覆盖；不兼容无版本 schema 拒绝迁移；执行到一半的 DDL 事务回滚；根路径为文件时保留原文件；camelCase IPC 序列化。其中相关断言集中在 8 个测试用例中。

前端测试覆盖：预览分支不调用 Rust；真实命令名称与返回值校验；后端拒绝/错误结构；无效和不兼容字段（含空路径、非整数/字符串版本）；重复初始化去重；失败重试；刷新失败清除旧状态。

## 原生运行证据

Playwright 连接测试子进程 WebView2 的本地 CDP 端口；未 mock get_workspace_status。验证链路为：Windows exe → WebView2 → React → 注册的 Rust 命令 → 真实 SQLite。

- 三页可导航；区域截图、搜索、标注工具保持禁用。
- 编辑页存在真实有尺寸的 Konva canvas。
- 工作区设置展示 `Tauri 桌面端 · windows`、`Schema 1 · SQLite 3.50.2`，刷新成功。
- 数据目录实际为 `C:\Users\wang1\AppData\Roaming\io.researchclip.desktop`。
- 实际数据库文件存在，originals/thumbnails/exports 目录存在；独立只读连接 `PRAGMA integrity_check` 返回 `ok`。
- 两次启动读取的 workspace_initialized_at 均为 `2026-10-12T07:30:12.840Z`（香港时间 15:30:12.840），证明重启没有覆盖首次初始化元数据。
- 测试结束已退出启动的原生进程。

证据文件位于忽略的 `artifacts/`，不纳入源码仓库：

- `desktop-smoke.json`、`preview-smoke.json`：检查结果。
- `desktop-library.png`、`desktop-editor.png`、`desktop-settings.png`：原生三个页面截图。
- `desktop-*-restart.png`：第二次启动截图。
- `preview-*.png`：浏览器预览截图；预览不出现本地 DB 路径，900×650 与1366×900 视口没有横向溢出。

## 可运行产物

```text
src-tauri/target/release/researchclip.exe
大小：10,393,600 bytes（约 9.91 MiB）
SHA-256：D5B1A85938933B056A5212C245850E39F513A638736989F9F01BDA917832003E
```

该产物已在本机 Windows x64 上启动验证，依赖 WebView2；不是已签名安装包。目标目录不入库，可用 README 中的命令重建。

## 本轮遇到并已解决的问题

1. 沙箱网络直接下载被限制；使用授权的工具执行下载。官方 npm registry 发生 ECONNRESET，使用本机已有 npm mirror 配置后安装成功。锁文件记录此次下载来源。
2. npm 10 的依赖解析在浮动 Vitest 4.x 的可选 peer 解析中失败；固定 Vitest 4.0.18 后 install 与 clean npm ci 均成功，npm ls --depth=0 无依赖错误。
3. 首次原生构建 beforeBuildCommand 找不到 npm。原因是包装脚本在 Windows 环境同时处理 Path/PATH 时丢失原 PATH。修正为保留现有键和值后 release 构建通过。
4. 首次浏览器冒烟中的按钮 accessible name 选择器未匹配空格，且默认 favicon 返回404；调整测试 locator 并加入应用图标链接。最终浏览器与原生冒烟均无渲染错误。
5. Node 22 的 node:sqlite 在原生测试中输出 ExperimentalWarning，测试退出码为0。该模块仅用于验收脚本的只读验证，应用持久化由 Rust/rusqlite 完成。

## 未执行与后续范围

macOS 编译/运行、Windows 安装程序/签名、真实全局热键/区域截图、混合 DPI 截图矩阵、图像标注无损回归、卡片搜索和 Markdown 导出均不属于本次 M1 验收，未实现也未宣称通过。

M1 初始化错误会导致原生应用退出，需从开发终端读取诊断；后续可以增加启动错误界面。科研图片、笔记和卡片尚未被写入；当前仅建库元数据与空目录。M1 验收完成后停止。
