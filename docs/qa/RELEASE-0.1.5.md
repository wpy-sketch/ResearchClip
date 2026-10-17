# v0.1.5 Markdown 模板与大纲验收

范围：本轮只在既有 Markdown 编辑器/动态分栏上增加模板、自定义模板管理和大纲，不改截图、标注逻辑或数据库迁移。环境：Windows NT 10.0.26200 x64，Node22.22.2/npm10.9.7，Rust MSVC1.99，Tauri2.12.3/CLI2.12.2，WebView2。测试文件均位于隔离 artifacts 目录，不操作用户 AppData 或原始资料。

## 实现与关键入口

| 模块 | 实现状态及文件/关键入口 |
| --- | --- |
| 模板种子 | 已实现；document/templates.ts 的 builtInTemplates/templateDocument/templateMarkdown，源文模式与普通 Markdown |
| 会话保护 | 已实现；store.ts 的 createFromTemplate/request/resolvePending，复用未保存确认和现有文件服务 |
| 模板管理 | 已实现；TemplateDialog/template-api、Rust commands/document 与 services/templates::TemplateFiles；5个 main-only 命令，无通用 fs 权限 |
| 持久化 | 已实现；配置目录 templates-v1/<id>/名称.md + assets，使用现有 DocumentFiles 校验/暂存/禁止覆盖；最多100个，创建原文档不变 |
| 删除保护 | 已实现；两次确认，独立目录原子改名为 deleted-ID，从活动列表移除；不递归删除用户文件，不自动清理归档 |
| 大纲 | 已实现；outline.ts::documentOutline/revealSourceHeading，MarkdownPanel 和 MarkdownPreview；自动更新 H1–H6/Setext，源偏移跳转与真实滚动 |
| 原有分栏 | 保留；WorkspaceSplit 仍只在 document.id 存在时挂载面板；模板选择/取消不打开文档 |

模板包含图片快照，不与素材库标注建立第二个可编辑来源；导入时复用既有合成图与受管 assets。保存模板不保存当前 `.md`。未知、损坏、重解析目录、同名模板与不合法封包明确拒绝，不通过删除文件修复。列表扫描最多1024条目（含归档），超过时需用户检查或移出归档，不自动删除。

两个直接依赖为已锁定且原 react-markdown 解析链已使用的 mdast-util-from-markdown 2.1.0 与 mdast-util-to-string 4.0.0（MIT、ESM、TypeScript），未引入新的编辑器框架或升级这些包。已核对[官方解析 API](https://github.com/syntax-tree/mdast-util-from-markdown)及[文字提取 API](https://github.com/syntax-tree/mdast-util-to-string)、安装版本声明和许可证；对比手工正则方案，选择相同 CommonMark 语义以正确处理围栏/缩进代码、Setext、容器及实体。

## 实际执行与结果

- PASS / 0：`npm ci --offline`，294包干净安装，锁文件重现。首次沙盒安装缓存缺失 ENOTCACHED，未改依赖版本；在本机已有缓存环境重跑成功。
- PASS / 0：`npm run check`，34文件/244测试、TypeScript、ESLint、前端生产构建。模板编辑/独立会话/取消与失败保护/图片关联/Unicode与LaTeX/H1–H6/Setext/伪标题及预览偏移均覆盖。
- PASS / 0：`npm run format:rust`、`npm run test:rust`（110测试）、`npm run check:rust`（all-targets，-D warnings）。新增3个文件服务测试覆盖重启、重命名、归档删除、PNG字节复制、同名/路径/封包/容量/目录失败与原文件保留。
- PASS / 0：`node scripts/with-rust.mjs tauri build --debug --no-bundle`，真实 Windows 调试程序。
- PASS / 0：`node scripts/templates-native-smoke.mjs v015-initial`，[10组原生证据](evidence/v015-templates-native-initial.json)。真实系统保存框、UTF-8及PNG落盘、模板保存/重启/副本编辑、重复名拒绝、重命名、确认归档删除、源光标/软换行滚动、预览可见位置、取消/放弃切换、图文模式保留与损坏目录错误均实际执行。
- PASS / 0：`node scripts/split-native-smoke.mjs templates-v015`，[11组既有分栏回归](evidence/v015-split-native.json)。真实左右拖动、最小宽度、480px、Konva原像素坐标、PNG合成/原图哈希、SQLite、文件冲突、另存为和重启均通过。

首轮失败如实保留：Rust 缺少 Mutex 类型引用导致编译失败，已修复后重跑全部 Rust 检查；沙盒 Vitest worker 无法共享临时模块缓存，21套件 ENOENT，仅92测试执行通过，未当作代码通过，改在正常临时目录环境重跑全部；既有预览测试硬编码 `<h1>` 与新定位属性冲突，更新为验证真实标题和 source-offset，全部244项复验通过，没有移除安全断言或跳过测试。

生产构建、最终原生复验、文档检查及 GitHub 发布完成后在下方补记。代码存在不作为 PASS；Mock 状态测试不作为 Windows 原生通过证据。

## Windows 人工验收（以下均 NOT_RUN）

1. 运行本次生产 EXE，分别从4个内置模板创建文档，输入中文/英文/LaTeX，保存并用 Obsidian/VS Code 打开整个文档目录。
2. 保存含多张不同标注图片的文档为模板，重启加载，修改副本并另存；对照原文档、原图和模板未变。
3. 同名/非法名称/权限不足/磁盘满/被外部软件修改时，确认提示明确、草稿保留；模板重命名与两步删除。
4. 长标题、重复标题、H1–H6、Setext、列表/引用标题、代码块和HTML；点击大纲检查源文与预览，打开/关闭大纲后继续输入及撤销。
5. 深浅主题、中文输入法、125/150/200%及混合DPI、多屏、小窗口/分栏调整与长文性能；左侧截图/编辑/笔记继续可用。
6. 无文件时全宽；模板选择取消不出现分栏；未保存切换选择保存/放弃/取消，退出/托盘流程、长时间资源稳定性。

本轮尚无已确认的新 P0/P1。完整生产人工流程、安装/卸载/签名、长稳及 macOS 为 NOT_RUN。历史 QA-001 关闭句柄错误与 QA-002 托盘间歇超时未据此关闭。删除模板保留归档占用磁盘，需要用户自行管理；不提供自动恢复UI。建议按本功能自动化结果交付，不能宣称完整产品质量矩阵已全部通过。


## 图文回归发现及修复

`node scripts/document-native-smoke.mjs templates-v015` 首轮 FAIL/1：[原始证据](evidence/v015-document-native-initial-fail.json)。P2：窄窗口保存按钮实际宽28px，小于原测试要求30px（复现：新建图文文档、添加/重排图片、将窗口缩小至脚本配置）。保留断言，将紧凑工具栏按钮调整为32px；后续必须重跑该套件及480px分栏检查，不把首轮2组通过当作完整套件通过。


复验：`node scripts/document-native-smoke.mjs templates-v015-fixed` PASS/0，[8组原生证据](evidence/v015-document-native.json)。原有30px断言保留；系统保存取消、双图片合成像素、图文顺序、相对资源、覆盖保护、重新加载及真实退出均通过。`node scripts/templates-native-smoke.mjs v015-final` PASS/0，[10组最终模板证据](evidence/v015-templates-native.json)，新增名称校验和提示复验通过。


## 最终交付复验

- PASS / 0：`node scripts/templates-native-smoke.mjs v015-release`，最终32px工具栏版本的[10组模板/大纲原生验收](evidence/v015-templates-native.json)。
- PASS / 0：`node scripts/split-native-smoke.mjs templates-v015-fixed`，[11组分栏复验](evidence/v015-split-native.json)，480px源文编辑高度仍满足80px下限，左右工作流及坐标正常。
- PASS / 0：`npm run desktop:build`，最终 Windows x64 生产 EXE 16,545,280字节；PE FileVersion/ProductVersion 均0.1.5。
- EXE SHA256：`2ccd38838cf4756a856236147ca8d01be8bf4d669ce8dcdac414de47a7a02416`。
- PASS / 0：最后一次 `npm run lint`、`git diff --check`；32px为单纯CSS修复，前端核心仍为上述34文件/244项，Rust核心为110项，均无跳过。
- `npm run check:docs` 已执行 PASS/0（42文档/245本地链接），补充最终证据与清单后复验。

本轮失败均已定位并复验，无尚未解决的新自动化失败。全套硬件/人工矩阵仍为NOT_RUN，发布非预发布标识不代表这些项目已通过。源码推送及GitHub Release公开结果另以发布回执记录。

## 修改文件

```text
CHANGELOG.md
README.md
docs/ARCHITECTURE.md
docs/RELEASE-NOTES-0.1.5.md
docs/qa/RELEASE-0.1.5.md
docs/qa/TEST_REPORT.md
docs/qa/evidence/v015-document-native-initial-fail.json
docs/qa/evidence/v015-document-native.json
docs/qa/evidence/v015-split-native.json
docs/qa/evidence/v015-templates-native-initial.json
docs/qa/evidence/v015-templates-native.json
package-lock.json
package.json
scripts/check-docs.mjs
scripts/templates-native-smoke.mjs
src-tauri/Cargo.lock
src-tauri/Cargo.toml
src-tauri/build.rs
src-tauri/capabilities/main.json
src-tauri/src/commands/document.rs
src-tauri/src/lib.rs
src-tauri/src/services/mod.rs
src-tauri/src/services/templates.rs
src-tauri/tauri.conf.json
src/app/layout/Toolbar.tsx
src/features/document/MarkdownPanel.tsx
src/features/document/MarkdownPreview.tsx
src/features/document/TemplateDialog.tsx
src/features/document/WorkspaceSplit.tsx
src/features/document/api.ts
src/features/document/outline.ts
src/features/document/preview.test.tsx
src/features/document/store.test.ts
src/features/document/store.ts
src/features/document/template-api.ts
src/features/document/templates.test.ts
src/features/document/templates.ts
src/styles.css
```
