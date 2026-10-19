# v3.0.0 质量与发布验收

本轮WebDAV、设置布局和图文指南增量。状态统一PASS/FAIL/BLOCKED/NOT_RUN；原生与前端/隔离服务器证据分别记录。测试只使用临时资料和专用服务器，不修改用户原图或重置真实数据库。

## 实现与入口

- 已实现：`services/sync`协议/transport/engine/credentials/runtime，`storage/sync`及006迁移，`commands/sync`6个main命令，`features/sync`设置/范围/状态/确认，设置分类导航、指南21图。
- 当前限制：手动任务；受管PNG Markdown资源；共享标签名称差异需先人工统一；历史/缓存不自动回收；没有PDF/独立全库恢复。
- 服务器：指定 `https://dav.jianguoyun.com/dav/ResearchClip/`，旧Zotero目录未访问。缺少用户在本机安全配置的应用凭据，认证验收BLOCKED。

## 实际命令和结果

本轮环境：Windows NT10.0.26200 x64、Node22.22.2/npm10.9.7、Rust1.99 MSVC、Tauri2.12.3/CLI2.12.2。下列最终命令均退出0；最初失败保留，不将历史结果混入新通过。机器可读摘要见[evidence/v300-checks.json](evidence/v300-checks.json)。

| 实际命令 | 结果与证据 |
| --- | --- |
| npm run check | PASS，51文件338测试、TypeScript/ESLint/前端生产构建；artifacts/v300-visibility-check.log。最终bundle再次类型检查/构建 |
| npm run test:rust | PASS，158测试；7个需要服务器的用例单独运行，不当作跳过通过；v300-rust-approved.log |
| RESEARCHCLIP_QA_DAV_BASE=http://127.0.0.1:1461/；node scripts/with-rust.mjs cargo test --manifest-path src-tauri/Cargo.toml --locked -- --ignored --test-threads=1 | PASS，全部7集成测试；独立1461原子条件写入/内容ETag HTTP与1462不可信TLS服务；v300-webdav-approved.log |
| npm run format:rust / npm run check:rust | PASS，格式与all-targets Clippy -D warnings；v300-fmt-approved.log / v300-clippy-approved.log |
| npm --prefix artifacts/qa-v300/npm-clean ci --registry=https://registry.npmjs.org --no-audit --no-fund | PASS，干净锁安装303包；v300-clean-install.log |
| npm --prefix browser-extension test / run build | PASS，5测试及独立扩展/内容脚本产物；v300-extension-test.log / v300-extension-build.log |
| node scripts/sync-ui-smoke.mjs v300-polished --update-guide | PASS，5组真实Edge布局/禁止浏览器原生操作/草稿保持；预览不等于原生同步；v300-sync-ui-polished.log |
| node scripts/guide-reader-smoke.mjs v300-final-layout | PASS，9组：14章层级、搜索、焦点、21图实际解码、深浅色7宽度、Markdown草稿；v300-guide-final-layout.log |
| node scripts/material-ui-smoke.mjs v300-final | PASS，8组主题/对比度/8宽度/原入口/Markdown状态；v300-material-ui.log |
| npm run tauri -- build --debug --no-bundle | PASS，实际Windows调试EXE；v300-debug-build.log与布局修正后v300-visibility-debug.log |
| node scripts/sync-native-smoke.mjs v300-first | PASS，5组真实WebView2/IPC/SQLite/Credential Manager：初始无自动同步、表单保存和清空密码、拒绝不可信TLS、真正关闭/重启恢复、DB不含测试凭据。测试系统凭据已清理；v300-native-sync.log |
| node scripts/guide-native-smoke.mjs v300-visibility-final --verify-only | PASS，5组实际Windows：6张SQLite示例网格/列表、既有箭头文字及卡片标签保存、双图对比、真实系统对话框保存.md/合成assets、完整关闭分栏、21图与指南键盘/布局；v300-visibility-native.log。图是明确标注的测试图，不冒充屏幕捕获 |
| npm run desktop:bundle | PASS，最终Windows3.0.0 EXE及NSIS；v300-visibility-bundle.log |
| pwsh -NoProfile -File scripts/installer-smoke.ps1 -RunId v300-visibility-final -TargetDir src-tauri/target | PASS，真实隔离安装、运行中安装/卸载拒绝、默认卸载。10个既有资料文件hash不变（首轮为9个；每轮仅对照本轮前后，不跨轮推断外部变化），无生产程序访问真实资料；v300-visibility-installer.log |
| npm run check:docs | PASS，版本/权限/已发布迁移不变、本地链接和围栏；最终文档数量以提交前日志为准 |

### 测试具体覆盖

- 7组真实服务器：初始双客户端/单端修改/项目范围/重开；两端修改/双副本/删除修改冲突；同号历史分叉/head/继续编辑/6次无变化同步0上传；Markdown相对PNG/双端修改/外部原文件保留；ACK丢失重启幂等/取消/损坏或短资源；生产TLS拒绝自签名；GET/PUT/MKCOL/PROPFIND/强ETag条件并发/catalog CAS。
- 新常规测试还真实断开Content-Length未完成的HTTP连接，及在途下载触发取消；都要求错误返回而非保存部分资源。schema5→6精确PNG与ID/历史保留、迁移失败回滚和备份、明确删除后恢复清除旧清理任务、损坏hash/未来Annotation/路径越界/未授权文档均验证。
- 第三方真实服务器、物理两台机器、完整125%/150%截图与多屏、托盘长期稳定性和插件工具栏实际点击仍未运行。没有因为上述自动/原生专项通过就把它们标PASS。

### 阶段结论

M1资源协议与迁移、M2安全连接/目录、M3内容寻址上传下载、M4资源双向、M5冲突双版本、M6持久队列/手动重试/停止/管理UI均有隔离执行证据。可发布有明确边界的Windows手动同步版本；**不宣称坚果云兼容已通过**，该服务必须经本机账号实际能力探针。当前无已复现的P0/P1阻断项；旧P2见总表。


## 初次失败与修正

1. 沙箱临时文件/原子落盘权限失败，保留日志，沙箱外同源重跑；首轮schema版本断言5→6已修正。
2. reqwest原TLS特性离线缺少依赖，选择已核验Windows系统TLS并锁版本；quick-xml安装版本API差异按源码修正后编译。
3. 默认WsgiDAV并发条件测试FAIL；串行版本仍因Windows整数秒mtime/大小ETag未变FAIL。保留原结果，不降低客户端门槛。独立参考测试配置明确增加SHA256 ETag与原子条件写入，不能据此宣称默认WsgiDAV/坚果云兼容。
4. 集成首次断连：旧隔离服务器生命周期结束导致5组连接失败；重新启动独立服务器后重跑。项目范围测试原先期望3次apply，实际文档先创建相同项目依赖，只需2次apply+1次基线ACK；保留总资源3、ID、数量与内容断言并复跑，不跳过用例。
5. OpenSSL工具缺少默认配置，改为测试目录内明确配置生成自签名证书；不安装任何系统信任证书。
6. 格式/Clippy初轮失败（含Socket测试未处理read数量）、新测试模块宏/类型未导入均修正，重跑158常规及7集成/严格Clippy通过；保留编译失败记录，不删除断言。
7. 原生指南首轮FAIL：非空素材库截图面板挤占首行。P2布局修正为默认收起仍可展开，修正后实际Windows首行断言通过；同时将旧指南按钮选择器更新为当前分级目录。完整5组重跑PASS，原失败截图/报告保留。发布前再复核发现活动截图也被收起：在隔离原生窗口稳定复现FAIL后，改为保留同一details组件、以新预览/错误自动展开、取消后收起；加入活动预览可见断言并完整5组复跑PASS。同步setState的lint失败已改用DOM展开同步，没有禁用规则。指南重绘导致旧图节点分离的脚本失败，改为当前DOM中21图全量解码断言后通过，保留失败记录。
8. 文档检查首次FAIL：schema常量仍期待5、Cargo.toml尾部空行；分别更新到6/移除多余空行后通过。构建大chunk/插件时序提示保留，不提高阈值隐藏警告。

## Windows 人工清单（NOT_RUN，除另有实际记录）

1. 安全退出旧版本，安装新版本并核对原项目、原图、标注历史、笔记、OCR与Markdown。
2. 本机设置输入坚果云账号和应用授权密码（不要发送聊天）；使用ResearchClip专用目录，运行安全探针。失败应停止，不能强制覆盖。
3. 两台实际设备/小项目验证初始传输、单端修改、同时修改、双副本、删除确认、断网中断/恢复、重启队列和限流提示。
4. Markdown目录只授权已登记文件，核对相对assets、中文/LaTeX原文和新受管版本；关闭/保存失败草稿保持。
5. 125%/150%、多屏、窗口缩小、实际截图/拖拽/托盘/快捷键及浏览器插件工具栏回归。前端测试不替代本项。
6. 大文件/超过历史上限明确失败，停止等待最多约30秒；退出后再次手动同步，原图hash不变。

## 风险与结论

坚果云账号认证与其真实条件写入兼容性BLOCKED；macOS NOT_RUN。默认WsgiDAV配置实际不兼容会拒绝。历史QA-001/002/008保留；离线公式结果仍需人工校对。正式版本标识仅描述发布渠道，不把上述未验收项改为PASS。

## 文件、依赖与产物

新增Rust `services/sync/{model,transport,credentials,engine,runtime,tests}.rs`、`storage/sync.rs`、`commands/sync.rs`、006迁移，前端`features/sync`及三份QA脚本。现有存储/revisions/桌面退出、设置/指南/素材库仅必要接入。新增直接依赖与许可见[迁移说明](../sync/MIGRATION.md)，无新前端依赖。

EXE：`src-tauri/target/release/researchclip.exe`；NSIS：`src-tauri/target/release/bundle/nsis/ResearchClip_3.0.0_x64-setup.exe`，未签名。Release保留原版WebClipper1.0.2及可选MathOCR Setup1.0.4；不附带模型/测试目录/凭据。

### 本轮修改文件清单

- CHANGELOG.md
- README.md
- docs/ARCHITECTURE.md
- docs/PRD.md
- docs/RELEASE-NOTES-3.0.0.md
- docs/ROADMAP.md
- docs/USER_GUIDE.md
- docs/images/v300-sync-flow.svg
- docs/images/v300-sync-settings.png
- docs/qa/RELEASE-3.0.0.md
- docs/qa/TEST_PLAN.md
- docs/qa/TEST_REPORT.md
- docs/qa/evidence/v300-checks.json
- docs/sync/MIGRATION-M1-DRAFT.md
- docs/sync/MIGRATION.md
- docs/sync/PROTOCOL-M1-DRAFT.md
- docs/sync/PROTOCOL.md
- package-lock.json
- package.json
- scripts/check-docs.mjs
- scripts/guide-native-smoke.mjs
- scripts/guide-reader-smoke.mjs
- scripts/markdown-open-with-native-smoke.mjs
- scripts/math-native-smoke.mjs
- scripts/projects-native-smoke.mjs
- scripts/qa-webdav-server.py
- scripts/sync-native-smoke.mjs
- scripts/sync-ui-smoke.mjs
- src-tauri/Cargo.lock
- src-tauri/Cargo.toml
- src-tauri/build.rs
- src-tauri/capabilities/main.json
- src-tauri/migrations/006_sync.sql
- src-tauri/src/commands/library.rs
- src-tauri/src/commands/mod.rs
- src-tauri/src/commands/sync.rs
- src-tauri/src/lib.rs
- src-tauri/src/services/desktop.rs
- src-tauri/src/services/markdown_document.rs
- src-tauri/src/services/mod.rs
- src-tauri/src/services/sync/credentials.rs
- src-tauri/src/services/sync/engine.rs
- src-tauri/src/services/sync/mod.rs
- src-tauri/src/services/sync/model.rs
- src-tauri/src/services/sync/runtime.rs
- src-tauri/src/services/sync/tests.rs
- src-tauri/src/services/sync/transport.rs
- src-tauri/src/storage/library.rs
- src-tauri/src/storage/migrations.rs
- src-tauri/src/storage/mod.rs
- src-tauri/src/storage/ocr.rs
- src-tauri/src/storage/projects.rs
- src-tauri/src/storage/revisions.rs
- src-tauri/src/storage/sync.rs
- src-tauri/tauri.conf.json
- src/features/guide/GuideView.tsx
- src/features/library/LibraryView.tsx
- src/features/library/store.test.ts
- src/features/settings/SettingsView.tsx
- src/features/sync/SyncSettings.tsx
- src/features/sync/api.test.ts
- src/features/sync/api.ts
- src/features/sync/store.test.ts
- src/features/sync/store.ts
- src/shared/api/library.test.ts
- src/shared/api/library.ts
- src/styles.css

### 发布前候选修正

最初源码4c5dcbf与本轮尚未公开的v3.0.0空草稿已创建；发现截图预览回归后停止公开，完成复现、修正、338前端检查、原生5组和最终EXE/NSIS/隔离安装复验。最终源码与标签以发布回执为准，不能把旧候选包当最终附件。17个既有发布不修改；更新本轮未公开候选标签时限定旧标签SHA，避免覆盖并发修改。
