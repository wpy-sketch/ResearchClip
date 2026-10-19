# v2.0.0 实际质量验收

本轮范围：界面布局、深浅色、分级使用指南与普通稳定Release。环境：Windows11 x64、Node22.22.2/npm10.9.7、Rust MSVC1.99、Tauri2.12.3/CLI2.12.2。业务Rust、数据库schema5及已发布001–005迁移保持。无新依赖。

## 实现与文件

- src/features/guide/GuideView.tsx：任务入口、分层导航、搜索、展开收起、当前阅读、返回目录和离线图片白名单。
- src/features/guide/navigation.ts及测试：复用mdast解析，源码偏移、真实标题层级、祖先保留、可见活动节点。
- src/styles.css：主题角色、侧栏/工具栏/卡片尺寸、指南窄屏排布、键盘焦点和动效偏好。
- docs/USER_GUIDE.md及images/v200-guide-*.png：14章详细步骤、19幅图；新版图片明确是本地Edge前端渲染。
- scripts/guide-reader-smoke.mjs与guide-native-smoke.mjs/check-docs.mjs：真实前端交互测试与图数/迁移保护。
- package、npm/Cargo锁文件、Tauri元数据升级2.0.0；README、CHANGELOG、架构/PRD/ROADMAP更新。
- 保留之前未提交的A06-M1文档及隔离审计脚本；仅为设计，M2–M6 BLOCKED，等待专用服务器信息与模型迁移决策。

## 实际检查

以下均为本轮实际执行。最终检查退出0；初次失败及重跑原因另列，不隐藏。测试只用隔离前端及临时安装目录；不写真实用户资料。

| 命令 | 实际结果 |
| --- | --- |
| npm run check | PASS，TypeScript、ESLint、49文件329测试、前端生产构建；最终日志artifacts/v200-check-final-retry.log |
| npm run format:rust | PASS，退出0 |
| npm run check:rust | PASS，all-targets Clippy -D warnings，退出0 |
| npm run test:rust | PASS，149测试，0失败/忽略，退出0 |
| npm --prefix artifacts/qa-v200/npm-clean ci --registry=https://registry.npmjs.org --no-audit --no-fund | PASS，303包，退出0；锁文件无依赖变化 |
| npm --prefix browser-extension test / run build | PASS，5测试及两份独立Vite产物，退出0 |
| node scripts/material-ui-smoke.mjs v200-first | PASS，8组，本地Edge深浅色/系统主题、8种宽度480–1440、对比度、截图入口、Markdown状态；无pageerror |
| node scripts/guide-reader-smoke.mjs v200-images-retry | PASS，9组，本地Edge目录/键盘/搜索/当前祖先/19图解码/弹窗/7种宽度/Markdown草稿，退出0，无pageerror |
| npm run desktop:bundle | PASS，Windows x64 EXE和NSIS，退出0 |
| pwsh -NoProfile -File scripts/installer-smoke.ps1 -RunId v200-final -TargetDir src-tauri/target | PASS，实际安装/忙碌拒绝/默认卸载，9个既有资料文件摘要不变；退出0 |
| npm run check:docs | PASS，81文档496本地链接，版本/权限/迁移一致，退出0（发布回执新增后数量另计） |

### 初次失败及修正

- 沙箱内check：34个测试文件因临时SSR模块ENOENT失败，91项已通过；同一源码沙箱外重跑49文件329项全过。没有删除断言或跳过测试。
- 沙箱内指南smoke：localhost启动超时，未执行交互；沙箱外新ID重跑通过。
- 两次offline ci：缺少estree-jsx缓存元数据而ENOTCACHED；转为锁文件联网干净安装后通过，不改变锁依赖。
- 新增图片逐图滚动测试遇到重绘旧DOM节点分离；测试改为当前DOM的19张图片解码状态，重新完整9组通过。业务不以隐藏异常处理。
- 前端构建保留大于500kB chunk提示，非构建失败；不提高阈值隐藏提示。

### 证据与产物

本轮摘要：[v200-checks.json](evidence/v200-checks.json)。原始日志与截图保留artifacts/qa-v200、guide-reader-v200-images-retry、material-ui-v200-first和installer-v200-final，不上传真实资料快照。最终EXE：src-tauri/target/release/researchclip.exe；NSIS：src-tauri/target/release/bundle/nsis/ResearchClip_2.0.0_x64-setup.exe。安装测试比对Tauri允许的UNK→NSS标记差异，并验证版本、图标和发布者。

本轮没有执行新的Windows主界面物理交互测试；安装/卸载通过不代替截图、托盘、DPI或第三方应用验收。A06的8项隔离模型审计是此前阶段结果，本轮没有将其计入同步功能通过。

## Windows 人工验收（NOT_RUN）

1. 退出旧实例，启动新EXE，检查深/浅/系统主题、素材网格/列表、窗口缩放及键盘焦点。
2. 打开指南，切换三级目录、搜索正文、展开/收起、定位步骤、返回目录、Enter放大图、Esc关闭。
3. 将窗口缩至最小，检查内容与目录可读、无横向根布局溢出；在125%/150%及多屏重复。
4. 右侧保留Markdown草稿，左侧指南/素材/编辑器之间切换；关闭分栏取消后草稿应保留。
5. 回归实际全屏/区域截图、快捷键、剪贴板、标注坐标/导出、OCR、SQLite重启、笔记与搜索、托盘退出。
6. 按扩展说明加载原v1.0.2包，验证本机Chrome/Edge工具栏剪藏；前端测试不能替代此项。

以上均未在本轮通过人工桌面验收，不因正式版标识记PASS。macOS NOT_RUN。历史QA-001/002/008未在本轮修复，公式识别错误仍需人工校对；未签名与WebView2依赖保留。


## 本轮源码与文档文件清单

- CHANGELOG.md
- README.md
- docs/ARCHITECTURE.md
- docs/PRD.md
- docs/RELEASE-NOTES-2.0.0.md
- docs/ROADMAP.md
- docs/USER_GUIDE.md
- docs/images/v200-guide-dark.png
- docs/images/v200-guide-light.png
- docs/qa/A06-M1.md
- docs/qa/RELEASE-2.0.0.md
- docs/qa/TEST_PLAN.md
- docs/qa/TEST_REPORT.md
- docs/qa/evidence/a06-m1-model-audit.json
- docs/sync/MIGRATION.md
- docs/sync/PROTOCOL.md
- package-lock.json
- package.json
- scripts/check-docs.mjs
- scripts/guide-native-smoke.mjs
- scripts/guide-reader-smoke.mjs
- scripts/qa-webdav-model-audit.py
- src-tauri/Cargo.lock
- src-tauri/Cargo.toml
- src-tauri/tauri.conf.json
- src/features/guide/GuideView.tsx
- src/features/guide/navigation.test.ts
- src/features/guide/navigation.ts
- src/styles.css
- docs/qa/evidence/v200-checks.json

建议进入正式版本发布；Windows人工矩阵仍为NOT_RUN，A06仍BLOCKED，不宣称所有设备通过。


## 正式发布

普通Latest发布、六附件匿名下载及16个旧发布保留均PASS，见[发布回执](PUBLICATION-2.0.0.md)。发布后仅修改README/本报告并新增回执和公开证据；源码标签与程序附件不变。
