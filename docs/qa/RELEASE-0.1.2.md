# v0.1.2 界面与发布验收

2026-10-17，Windows 11 x64。用户明确要求参考 Google 应用优化布局与配色，并上传 v0.1.2，取消该新版本的预发布标识。只调整本轮界面、版本、测试和发布材料；不改数据库或截图/标注算法。

## 设计及实现

参考 Material 3 的 surface/container/primary 角色与应用栏，沿用 Tailwind、Lucide、Zustand；不新增依赖、远程字体或 Google 标识。默认深色保持，已有用户设置不重置。浅色蓝灰外框、白色工作区、蓝色强调；深色使用炭灰和浅蓝。顶部独立搜索行、第二行文档工具，侧栏突出“新建截图”，窄屏改为导航轨及带 tooltip 的文档图标。

涉及 `src/styles.css`、`Sidebar.tsx`、`Toolbar.tsx`、`use-appearance.ts`。`scripts/material-ui-smoke.mjs` 执行真实 Edge 深浅布局/对比度/文档状态检查；旧 smoke 的截图按钮定位由 toolbar 改为唯一按钮入口，保留原有行为和焦点断言。

## 检查记录

首轮浏览器 UI 检查 PASS：8 组，包括深浅主题、480/600/760/761/820/1000/1180/1440 八种宽度、截图入口、Markdown 切换/脏关闭和系统主题变化。截图取证等待主题颜色过渡结束，避免把过渡中间颜色当作最终视觉。

| 实际命令 | 最终结果 |
| --- | --- |
| `npm ci --offline` | PASS/0：294 包干净安装，无依赖升级 |
| `npm run check` | PASS/0：TypeScript、ESLint、30 文件 / 220 测试及生产前端构建；窄屏修正后再次执行 |
| `npm run format:rust` / `npm run check:rust` | PASS/0：fmt 与 Clippy --all-targets -D warnings |
| `npm run test:rust` | PASS/0：102 项，无失败或忽略 |
| `node scripts/material-ui-smoke.mjs v012-fixed` | PASS/0：8 组；深浅主题各八宽度、480×400 分栏高度、系统主题、截图弹窗、文档脏关闭；[证据](evidence/v012-material-ui.json) |
| `node scripts/smoke.mjs --run-id material-v012` | PASS/0：6 组，含 32 个页面/主题/尺寸组合；另有 4 组明确标记的覆盖层 Mock 渲染，不能代替截图；[证据](evidence/v012-preview-smoke.json) |
| `node scripts/with-rust.mjs tauri build --debug --no-bundle` | PASS/0：0.1.2 Windows debug 程序 |
| `npm run test:smoke:pins:native` | PASS/0：13 组真实窗口/拖动/透明度/PNG/剪贴板/生命周期；[证据](evidence/v012-pins-native.json) |
| `node scripts/editor-native-smoke.mjs material-v012` | PASS/0：6 组，五类标注、原分辨率导出、系统对话框、跨应用剪贴板、重启重开；[证据](evidence/v012-editor-native.json) |
| `node scripts/split-native-smoke.mjs material-v012` | 首次 FAIL/1，P2 窄屏高度；[保留失败](evidence/v012-split-initial-failure.json) |
| `node scripts/split-native-smoke.mjs material-v012-fixed` | 修正后 PASS/0：11 组，真实 SQLite、原图不变、系统保存/打开、分隔线持久化、左右撤销隔离、480×400 编辑高度；[证据](evidence/v012-split-native.json) |

上述原生用例均使用独立测试目录和受控 PNG。没有操作用户资料；原始截图/浏览器缓存保留本地不上传。对比度检查仅覆盖所测正文/次要正文/主按钮配色对，不宣称完整 WCAG 认证。

`npm run desktop:build` PASS/0：生产 EXE 15,969,792 字节，SHA256=`135453f3381e9008e51648e543042809dfcc03c29a90623c5c0e786369d2b559`；Windows PE 的 FileVersion/ProductVersion 均为 0.1.2。`npm run lint` 与 `npm run check:docs` 最终复验 PASS/0。公开发布结果在执行后补记。发布先普通快进推送、创建新标签和草稿；核验附件后公开，`prerelease=false`、作为 Latest，不改旧 Release。

## 未执行及限制

完整高/混合 DPI 和多屏、真实屏幕像素矩阵、真实资料长稳、macOS、生产 EXE 完整人工交互及 NSIS 安装/卸载 NOT_RUN。历史 QA-001/QA-002 仍按原报告保留。浏览器只验证渲染及 UI，会明确区分隔离 Windows 原生 SQLite、PNG 和系统对话框证据。

## 修复记录

首次 Windows 分栏回归 FAIL：前 10 组通过，480×400 时右侧工具按钮换行占用高度，Markdown 文本区低于既有 80px 可用性断言。复现：打开文档并保留左右分栏，将主窗口缩为 480×400。属于本轮布局回归 P2。修正窄屏 Markdown 工具条为不换行、可水平滚动的紧凑按钮，保持所有操作及原断言；随后重建并复验，不降低测试标准。

## 变更文件

界面：`src/styles.css`、`src/app/layout/Sidebar.tsx`、`src/app/layout/Toolbar.tsx`、`src/app/use-appearance.ts`。

版本/文档：`package.json`、`package-lock.json`、`src-tauri/Cargo.toml`、`src-tauri/Cargo.lock`、`src-tauri/tauri.conf.json`、`README.md`、`CHANGELOG.md`、`docs/ARCHITECTURE.md`、本报告、`docs/RELEASE-NOTES-0.1.2.md`。

测试：新增 `scripts/material-ui-smoke.mjs`，更新 `scripts/smoke.mjs` 的截图按钮位置；六份 `docs/qa/evidence/v012-*.json` 保留渲染、原生、初次失败与最终通过。不提交测试图片、AppData 或缓存。
