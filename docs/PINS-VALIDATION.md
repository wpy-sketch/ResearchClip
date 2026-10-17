# 桌面截图贴图：实现与验收

日期：2026-10-17；Windows 优先。此功能为当前会话贴图，不增加数据库、云服务或重启恢复。

## 使用与行为

1. 在截图完成区域点击“贴图”；图片编辑器的“贴图”使用原图分辨率的标注合成图，不含编辑手柄。
2. 每张贴图是独立的无系统标题栏、置顶 Tauri WebView 窗口。左键拖动移动，滚轮等比缩放；右上角关闭按钮在悬停或键盘聚焦时显示。
3. 右键打开菜单：复制图片、另存为 PNG、不透明度、恢复位置和关闭。透明度范围 20%–100%；不改变原 PNG 的像素或剪贴板输出。
4. 主工具栏图钉按钮“恢复贴图位置”把所有贴图恢复到主显示器工作区，并恢复到原图大小（超出工作区时等比缩小）。可以先通过托盘打开主窗口，再恢复屏幕外贴图。
5. 关闭主窗口：默认走现有退出确认流程；启用“关闭到托盘”时隐藏主窗口并保留贴图。明确退出成功后关闭全部贴图。取消退出保留贴图；未保存编辑仍使用既有确认，不因贴图绕过。
6. 最多 8 张，按 PNG 字节及原始 RGBA 解码尺寸估算的图像预算总计 256 MiB。不是整个进程内存上限；WebView/图形驱动还有额外开销。缩放范围以 5%–400% 为基础，并保证较长边至少 48 物理像素、最大不超当前工作区；极端宽高比/小屏按工作区优先收紧边界。
7. 另存为使用系统对话框和已有同目录安全暂存、禁止覆盖服务。同名目标保留，请选择新名称。复制使用真正的 RGBA 图片，不复制路径。文件读取和创建不开放给 WebView。

## 实现映射

| 入口/模块 | 责任 |
| --- | --- |
| [CapturePanel](../src/features/capture/CapturePanel.tsx) / [EditorView](../src/features/editor/EditorView.tsx) | 截图/标注合成图贴图入口，处理忙状态和错误 |
| [pins/api.ts](../src/features/pins/api.ts) | PNG 二进制 IPC；延迟复用现有 renderComposite，不增加渲染系统 |
| [PinView](../src/features/pins/PinView.tsx) / [RecoverPinsButton](../src/features/pins/RecoverPinsButton.tsx) | 独立贴图窗口内容、右键菜单和全局恢复入口 |
| [services/pins.rs](../src-tauri/src/services/pins.rs) | 唯一标签、图片快照、资源上限、原生创建/位置/尺寸/透明度/关闭；单个超时巡检线程 |
| [commands/pins.rs](../src-tauri/src/commands/pins.rs) | 主窗口创建/恢复/计数；贴图仅能读取和操作自己的资源；PNG 完整解码校验，耗时任务离开 UI 线程 |
| [desktop-pins capability](../src-tauri/capabilities/desktop-pins.json) | 仅 pin_image 与 pin_action；不允许通用窗口创建、任意路径、SQLite、主截图读取或剪贴板读取 |
| [lib.rs](../src-tauri/src/lib.rs) / [desktop.rs](../src-tauri/src/services/desktop.rs) | 注册管理器、窗口 Destroyed 清理、退出时关闭贴图；主窗口异常销毁也清理贴图 |

窗口创建/操作采用已安装 Tauri 2.12.3 的 WebviewWindowBuilder / WebviewWindow API，核对 [Tauri 文档](https://docs.rs/tauri/2.12.3/tauri/webview/struct.WebviewWindow.html) 与锁定版本源代码。Windows 透明度使用已安装 windows-rs 0.62.2 的 [SetLayeredWindowAttributes](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setlayeredwindowattributes)，只增加 WS_EX_LAYERED，不增加鼠标穿透标记。非 Windows 透明度明确返回不支持，macOS 尚未验收。

标签从应用内递增序列分配，不接受前端指定标签。图片由 Arc 快照持有，不依赖主截图缓存或临时文件。关闭销毁 WebView 后撤销其会话资源；主截图替换/丢弃不影响已有贴图。前端释放 Blob URL 和滚轮监听；原生销毁回调还覆盖 Alt+F4。加载超时 15 秒自动清理并通知主窗口，失败不会无限创建后台线程。管理器锁不跨 GUI 操作持有。

尺寸和位置使用 Tauri 物理坐标及显示器 work_area。负原点和恢复边界有纯逻辑测试；混合 DPI 实际跨屏行为仍需人工验收，不能把单屏结果外推到所有显示器。

## 实际检查记录

最终源码检查如下；命令退出码均为 0。环境沿用 Windows 11 x64、WebView2、Node 22.22.2 / npm 10.9.7、Rust MSVC 1.99、Tauri 2.12.3。所有原生测试只在唯一 artifacts 目录下创建设置、工作区、图片；未删除用户数据。

| 命令 | 状态与实测范围 |
| --- | --- |
| `npm run check` | PASS：TypeScript、ESLint、Vitest 30 文件 / 220 测试、Vite 生产构建；包含新增贴图 IPC 单测 6 项 |
| `npm run format:rust` | PASS：cargo fmt --check |
| `npm run check:rust` | PASS：cargo clippy --locked --all-targets -- -D warnings |
| `npm run test:rust` | PASS：102 测试，其中新增管理器 5 项；资源数量/预算/重复释放、唯一 ID、授权、负坐标恢复、缩放边界 |
| `node scripts/with-rust.mjs tauri build --debug --no-bundle` | PASS：构建后实际启动 Windows 应用做原生验收 |
| `npm run test:smoke:pins:native` | PASS：13 组；最终同一脚本重跑命令 `node scripts/pins-native-smoke.mjs final-20261017` 亦 PASS |
| `node scripts/editor-native-smoke.mjs pins-regression-20261017` | PASS：既有编辑器 6 组；五类标注、保存/重开、撤销、原分辨率导出、剪贴板全像素对比、重启恢复 |
| `npm run desktop:build` | PASS：Windows 生产 EXE，未打包 NSIS、未发布远程 |
| `npm run check:docs` | PASS：文档链接/围栏、命令注册、三个 capability 范围、版本及数据库迁移一致性 |

贴图原生证据：[最终报告](qa/evidence/pins-development-20261017.json)、本地窗口图像 `artifacts/pins-final-20261017/pin.png`。编辑器回归：[报告](qa/evidence/pins-editor-regression-20261017.json)。公开的 JSON 仅含检查摘要和窗口属性；原始 artifacts 仍按仓库规则忽略，不上传测试图像、浏览器会话或工作区。

实际执行了：两个独立 HWND、无非客户区标题栏、置顶标记、真实 SendInput 拖动、滚轮和重复缩放上下限、Windows Alpha 51/255（20%/100%）、无鼠标穿透、跨窗口 capability 拒绝、坏 PNG/非法参数拒绝、系统 PNG 保存及取消、另一个 Edge 应用读取全部 RGBA、真实剪贴板锁定失败及重试、屏幕外恢复、主窗口确实隐藏到托盘/恢复、10 次关闭后 HWND/WebView 消失且资源计数归零、编辑器矩形合成与原图不变、主截图丢弃后贴图仍存活、取消退出保留及批准退出清空。

最终没有未解决的本模块自动检查失败。历史 QA-001/QA-002 的完整素材库关闭/托盘稳定性矩阵本轮未重新验收，不凭本轮成功关闭旧问题。超时故障、OS 创建/销毁失败、真实跨 DPI 和长期堆/显存检查 NOT_RUN；不能把资源计数归零说成全部系统资源均无泄漏。生产 EXE 本轮仅构建，原生交互运行的是隔离 debug 版本；未将生产启动或安装测试标为 PASS。

生产文件：`src-tauri/target/release/researchclip.exe`，15,969,280 字节；SHA-256：`ef9c081f3f4da32adac95d0e671f92daa3a1333bce39ac56564f2a6220185584`。应用版本保持 0.1.0，没有修改或发布已有 Release。


初次检查失败记录保留：沙箱内 Vitest 的临时模块路径 ENOENT；新增 TypeScript 括号/测试数据构造问题；Rust 已弃用原子 API。修正后完整重跑。原生探针最初把 WS_CAPTION 样式位误当成可见系统标题栏；Tao 0.37.1 的 WM_NCCALCSIZE 返回零实现无边框，已改为核对实际客户区和窗口矩形完全相同，并保留原始样式证据。探针坐标改为每显示器 DPI 感知，屏幕外测试验证不可见位置及恢复结果，不依赖 Windows 重定位后的精确输入值。

## Windows 人工验收（NOT_RUN，待用户执行）

- 使用真实全屏及区域截图各创建贴图，对比原屏幕；标注箭头、文字、矩形后从编辑器创建，确认字体、透明 PNG 及图层显示正确。
- 在画图或常用科研图像软件粘贴贴图；验证其他程序占用剪贴板时提示及重试。
- 在其他普通程序前切换焦点，确认贴图持续置顶；多张叠放、右键菜单、20% 透明度、极小/极大尺寸的操作可用性。
- 125% / 150% / 200% DPI、混合 DPI、负坐标副屏、跨屏拖动、显示器拔插后，用工具栏恢复位置；比较图片比例与实际窗口边界。
- 主窗口最小化、隐藏到托盘、恢复、多次关闭；确认真正退出时所有贴图消失，取消退出仍保留。
- 真实高分辨率图片及长期使用的进程/显存稳定性；达到 8 张或预算上限后提示，关闭部分贴图后可再创建。
- Windows 10、独占全屏程序、UAC 安全桌面、macOS 未验收；普通置顶窗口不承诺覆盖安全桌面或所有独占全屏模式。

## 修改文件清单

29 个源码、配置、测试与文档文件；无依赖版本、锁文件、数据库迁移或用户原图变更。

```text
README.md
docs/ARCHITECTURE.md
docs/qa/TEST_PLAN.md
docs/qa/TEST_REPORT.md
docs/PINS-VALIDATION.md
package.json
scripts/check-docs.mjs
scripts/pins-native-smoke.mjs
src-tauri/build.rs
src-tauri/tauri.conf.json
src-tauri/capabilities/main.json
src-tauri/capabilities/desktop-pins.json
src-tauri/examples/workflow_input.rs
src-tauri/examples/pin_probe.rs
src-tauri/src/commands/mod.rs
src-tauri/src/commands/pins.rs
src-tauri/src/services/mod.rs
src-tauri/src/services/pins.rs
src-tauri/src/services/desktop.rs
src-tauri/src/lib.rs
src/main.tsx
src/styles.css
src/app/layout/Toolbar.tsx
src/features/capture/CapturePanel.tsx
src/features/editor/EditorView.tsx
src/features/pins/api.ts
src/features/pins/api.test.ts
src/features/pins/PinView.tsx
src/features/pins/RecoverPinsButton.tsx
```
