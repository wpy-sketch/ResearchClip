# ResearchClip v1.0.5 / A05 实际验收记录

环境：2026-10-19，Windows 11 x64，Node22.22.2/npm10.9.7，Rust MSVC1.99，Tauri2.12.3。所有存储写入使用新建的隔离目录或 tempfile；未修改真实用户资料库。单元测试和隔离 CLI 进程不代表桌面点击验收。

## 功能与入口

| 功能 | 实现入口 | 实际验证状态 |
| --- | --- | --- |
| 原图不可变及首版本 | storage/library.rs commit_pending、005触发器 | PASS：同一原图字节、仅1份原图 |
| 显式保存、说明、编号与去重 | storage/revisions.rs save_annotation_revision / record；library-actions.ts | PASS：正常/相同内容/数值写法/CAS冲突 |
| 多版本和重启读取 | annotation_history / annotation_revision；revision_probe | PASS：两个独立进程 seed / verify |
| 旧版预览与对比 | RevisionHistory.tsx → renderComposite | PASS：读取不变更 store；原生界面/实际合成图点击 NOT_RUN |
| 恢复并继续编辑 | restore_annotation_revision；store.restoreRevision | PASS：追加v3、父版本、继续v4、会话撤销独立 |
| 损坏/未知格式隔离 | annotation_revision 校验 | PASS：拒绝损坏及未来版本、跨图访问、尺寸不符；其他版本保留 |
| 迁移与备份 | migrations.rs、005_annotation_revisions.sql | PASS：v4→v5、早期Annotation v1原字节保留、备份仍v4；既有迁移回滚测试通过 |
| 历史删除规则 | screenshot外键级联 | PASS：单张明确删除后历史级联；无自动清理入口 |
| 浏览器扩展保留 | browser-extension及原v1.0.2包 | PASS：独立构建与5测试；端到端工具栏交互 BLOCKED（既有环境限制） |

## 实际命令

| 命令 | 结果 |
| --- | --- |
| npm run check | PASS，退出0，48文件/325 Vitest测试，TS/ESLint/生产前端构建 |
| npm run test:rust | PASS，退出0，149测试，0 ignored |
| npm run format:rust | PASS，退出0 |
| npm run check:rust | PASS，退出0，all-targets、-D warnings |
| npm --prefix artifacts/qa-v105/npm-clean ci --offline | PASS，退出0，303依赖，锁文件干净安装，无新依赖 |
| node scripts/with-rust.mjs cargo build --manifest-path src-tauri/Cargo.toml --locked --example revision_probe | PASS，退出0 |
| src-tauri/target/debug/examples/revision_probe.exe seed artifacts/qa-v105/restart-1 | PASS，退出0，创建/修改/恢复v3 |
| src-tauri/target/debug/examples/revision_probe.exe verify artifacts/qa-v105/restart-1 | PASS，退出0，新进程读取3版本、当前JSON和原PNG相等 |
| npm --prefix browser-extension run build | PASS，退出0，独立MV3构建 |
| npm --prefix browser-extension test | PASS，退出0，5测试 |

最终 `npm run desktop:bundle` 退出0（artifacts/qa-v105/build-release.log），包含最终恢复流程和指南；交付 EXE及NSIS路径为 `src-tauri/target/release/researchclip.exe` 和 `src-tauri/target/release/bundle/nsis/ResearchClip_1.0.5_x64-setup.exe`。

`pwsh -NoProfile -File scripts/installer-smoke.ps1 -RunId v105-final -TargetDir src-tauri/target` 退出0：真实隔离安装、版本/图标/元数据及payload核对、占用时安装/卸载均拒绝（预期退出2），默认静默卸载通过，8份既有用户资料哈希保持，隔离程序/注册已清理、无生产资料库启动。见[安装证据](evidence/v105-installer.json)。历史QA-008仍未证明根因已修复。

[两个独立进程持久化证据](evidence/v105-restart.json)。恢复现在同步截图预览的CAS引用；新增等待期间草稿变化保护测试已通过。7个既有原生冒烟脚本的当前schema期望/指南图片数已更新，ESLint通过；这些脚本的实际桌面执行本轮仍为NOT_RUN，未绕过此前Computer Use环境限制。

`npm run check:docs` 退出0：75文档、449本地链接、main-only权限一致、001–004迁移未改。远程发布核验完成后补回执，不移动标签或替换已发布附件。

## 失败与修复记录

1. 首轮沙箱 Rust 测试：76通过、67失败，主要为禁止覆盖落盘/临时文件提交权限失败；未移除断言。相同单测经授权沙箱外先通过，再完整149测试通过。
2. 首轮 ESLint：effect cleanup直接访问ref，1警告导致退出1；捕获稳定ref后完整检查退出0。
3. 首轮 Clippy：新探针引用私有storage模块编译失败；改为仅debug可用的qa_revisions封装，未公开生产任意SQL/文件入口，随后all-targets退出0。
4. 前端生产构建保留既有大于500kB chunk提示（非失败），未通过提高告警阈值隐藏。

## Windows 人工验收（全部 NOT_RUN）

1. 打开已有素材，核对编辑基准v1；新截图首次保存后可查看v1。
2. 增加箭头、文字、马赛克/模糊，填写说明并保存为v2；同内容重复保存不增加版本。重命名/收藏/笔记保存不增加标注版本。
3. 缩放和平移后打开历史，逐一预览v1/v2，再与当前含未保存草稿的版本并排比较；退出历史后草稿/视口/选择仍正确。
4. 有草稿时恢复v1，先取消核对草稿保留；先保存后再恢复，核对新增版本及父关系；确认原v1/v2还在。
5. 恢复后修改颜色/拖动/缩放/撤销/重做，保存、导出PNG，退出应用再打开，核对标注全部可编辑。
6. 深浅色、窄窗口、125%/150%DPI、长说明及超过50历史版本分页；连续打开关闭检查焦点和资源。
7. 使用复制的专用测试库破坏一条历史JSON，验证错误提示及其他版本；不要破坏真实用户库。
8. 运行兼容v1.0.2插件，通过真实Chrome/Edge工具栏剪藏并在新版打开Markdown（历史环境限制未解除）。

## 已知边界与发布判断

- 初版比较为同原图的静态合成图并排显示，不新增同步差异算法。只保存Annotation schema1可编辑JSON，旧的基本/高级图元都继续使用同一格式；不虚构schema0转换。
- 第一次保存说明固定「首次保存」；后续显式保存可自定义。更改说明而不更改内容不会创建版本。导出的独立编辑项目不含整套SQLite历史。
- 版本保留会增长数据库，单次JSON受4MiB/500对象既有限额约束，列表50条分页。不自动清理。系统磁盘满/权限失败返回错误并事务回滚；不保证任意外部篡改/物理损坏可恢复。
- 未执行新历史面板的Windows鼠标/键盘、实际PNG视觉对比、长时资源测试；不将Mock当作其通过。macOS NOT_RUN。
- QA-001/P2退出警告、QA-002托盘间歇问题、QA-008/P2历史卸载间歇失败仍保留，不声称本轮修复。A04公式误识别限制继续保留。
- 当前自动检查无未解决FAIL；可以继续本轮打包与发布核验，但不能据此声明人工矩阵通过。

## 修改文件（发布源码提交前清单）

- `CHANGELOG.md`
- `README.md`
- `docs/ARCHITECTURE.md`
- `docs/PRD.md`
- `docs/RELEASE-NOTES-1.0.5.md`
- `docs/ROADMAP.md`
- `docs/USER_GUIDE.md`
- `docs/images/v105-revisions.svg`
- `docs/qa/RELEASE-1.0.5.md`
- `docs/qa/TEST_PLAN.md`
- `docs/qa/TEST_REPORT.md`
- `docs/qa/evidence/v105-installer.json`
- `docs/qa/evidence/v105-restart.json`
- `package-lock.json`
- `package.json`
- `scripts/check-docs.mjs`
- `scripts/guide-native-smoke.mjs`
- `scripts/knowledge-native-smoke.mjs`
- `scripts/library-native-smoke.mjs`
- `scripts/markdown-open-with-native-smoke.mjs`
- `scripts/math-native-smoke.mjs`
- `scripts/projects-native-smoke.mjs`
- `scripts/storage-native-smoke.mjs`
- `src-tauri/Cargo.lock`
- `src-tauri/Cargo.toml`
- `src-tauri/build.rs`
- `src-tauri/capabilities/main.json`
- `src-tauri/examples/revision_probe.rs`
- `src-tauri/migrations/005_annotation_revisions.sql`
- `src-tauri/src/commands/mod.rs`
- `src-tauri/src/commands/revisions.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/qa_revisions.rs`
- `src-tauri/src/storage/library.rs`
- `src-tauri/src/storage/migrations.rs`
- `src-tauri/src/storage/mod.rs`
- `src-tauri/src/storage/ocr.rs`
- `src-tauri/src/storage/projects.rs`
- `src-tauri/src/storage/revisions.rs`
- `src-tauri/tauri.conf.json`
- `src/app/library-actions.test.ts`
- `src/app/library-actions.ts`
- `src/features/editor/EditorView.tsx`
- `src/features/editor/RevisionHistory.tsx`
- `src/features/editor/revisions-api.ts`
- `src/features/editor/revisions.test.ts`
- `src/features/editor/store.ts`
- `src/features/editor/types.ts`
- `src/features/guide/GuideView.tsx`
- `src/features/library/store.test.ts`
- `src/shared/api/library.test.ts`
- `src/shared/api/library.ts`
- `src/styles.css`
