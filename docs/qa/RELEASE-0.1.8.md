# v0.1.8 导入、双图对比与工作区验收

执行日期：2026-10-17，Asia/Hong_Kong。Windows 11 NT10.0.26200 x64、WebView2、Node22.22.2/npm10.9.7、Rust MSVC1.99、Tauri2.12.3/CLI2.12.2。合并此前v0.1.6/v0.1.7请求为单一v0.1.8，不创建未经交付的中间Release。规范沿用[TEST_PLAN](TEST_PLAN.md)。

## 功能与入口

| 功能 | 实现状态、关键文件与入口 | 本轮验收 |
| --- | --- | --- |
| 主动剪贴板导入 | 已实现；import/ImportControls、store、api；commands/import::import_clipboard_image，services/image_import::clipboard_frame | PASS：真实Windows图片、占用、文本、重复导入 |
| 外部批量导入 | 已实现；use-image-drop、LibraryView、MarkdownPanel；import_image_files、decode_file、unpack | PASS：真实OLE单图/多图、PNG/JPEG/WebP、混合失败、容量边界 |
| 资料持久化 | 复用LibraryService/create_screenshot和现有SQLite/缩略图/恢复，不改schema | PASS：9条记录重启、改名、标注、Markdown及assets，源图哈希未变 |
| 独立对比 | 已实现；compare/CompareView、store、model、api；choose_comparison_image | PASS：原生文件框/素材选择、交换、同/不同尺寸、失败保留旧图、多次退出 |
| 三种对比模式 | 已实现；共享editor/geometry、DOM transform/clip/opacity | PASS：真实像素、拖动分割/平移、滚轮缩放、透明度、资源URL释放 |
| 列表/网格 | 已实现；LibraryView、LibraryCard、styles | PASS：同一真实列表、操作可见、单缩略图失败占位，分页仍为24条 |
| 完整指南 | 已实现；GuideView、安全MarkdownPreview、USER_GUIDE.md | PASS：13节、真实目录滚动、480px、右侧文档保留 |
| 可选差异图 | NOT_IMPLEMENTED，本次未选择可选扩展 | 不影响三种基础模式；无配准/科学误差承诺 |

## 实际命令及结果

以下均实际执行，退出码0，未跳过断言或失败项。Mock测试与原生证据分开。

- PASS：`npm ci --offline`，294包干净安装。新增image为既有锁定0.25.10的直接依赖并启用PNG/JPEG/WebP；image-webp0.2.4已锁定，无通用fs插件或新前端框架。
- PASS：`npm run check`：TypeScript、ESLint、Vitest **38文件/261测试**、前端生产构建。
- PASS：`npm run format:rust`、`npm run test:rust`（**116测试**，0失败/忽略）、`npm run check:rust`（all-targets，-D warnings）。测试涵盖实际解码、EXIF方向、DIB透明度/行填充/边界、封包、真实数据库/文件重启；前端涵盖会话失败/过期加载/共享坐标。
- PASS：`node scripts/with-rust.mjs tauri build --debug --no-bundle`，Windows调试程序；`node scripts/with-rust.mjs cargo build --manifest-path src-tauri/Cargo.toml --locked --example workflow_input`，原生测试辅助器。
- PASS：`node scripts/import-native-smoke.mjs workspace-v018-final`，[9组真实Windows导入证据](evidence/v018-import-native.json)。WinForms/OLE实际系统拖拽进入WebView，非仅合成DOM事件；剪贴板导入与受控源逐RGBA像素比较。不是实时屏幕捕获验收。
- PASS：`node scripts/compare-native-smoke.mjs workspace-v018-initial`，[8组原生对比/列表/指南证据](evidence/v018-compare-native.json)。分割红/蓝像素、25%叠加实测、共享变换、真实文件框与SQLite、反复进入退出及源哈希。
- PASS：`node scripts/split-native-smoke.mjs workspace-v018`，[11组分栏回归](evidence/v018-split-native.json)，含缩放后Konva原像素坐标、PNG合成、SQLite和重启。
- PASS：`node scripts/document-native-smoke.mjs workspace-v018`，[8组图文保存回归](evidence/v018-document-native.json)。
- PASS：`node scripts/templates-native-smoke.mjs workspace-v018`，[10组模板/大纲回归](evidence/v018-templates-native.json)。
- PASS：`node scripts/editor-native-smoke.mjs workspace-v018`，[6组编辑/导出回归](evidence/v018-editor-native.json)，真实项目保存/打开、撤销、PNG、复制到独立图像查看器逐RGBA相同及重启。
- PASS：`node scripts/material-ui-smoke.mjs workspace-v018-final`，[8组浏览器界面检查](evidence/v018-material-ui.json)。深浅主题文本/按钮对比度、8种480–1440宽度、分栏和跟随系统；不代替原生截图验收。
- PASS：`npm run desktop:build`，最终生产EXE **17,254,400字节**，PE FileVersion/ProductVersion均0.1.8；SHA256 `f5475732e108856d1a9e6406160f81cfc2494037862f727e900d5f8f39c05c94`。

生产文件：`src-tauri/target/release/researchclip.exe`。本轮没有运行生产EXE访问真实用户资料；原生验收为同源debug程序/独立工作区，生产构建通过不等于生产完整人工通过。本轮未构建新NSIS，安装/卸载复验为NOT_RUN。

## 失败历史与修复

首次Cargo离线缺image-webp、直连TLS失败，通过本机代理取得锁定包后完整重跑成功。首轮Rust Clippy指出手工div_ceil，改为标准API；前端hooks导出边界调整后全部检查通过。未降低检查标准。

原生导入测试首轮未等待“复制图片”完成而遇到任务忙；入口现按captureBusy禁用并修正测试等待。OLE辅助器曾因根窗口判定、WinForms同线程计时器与多文件JSON嵌套失败，改为归属进程/子窗口验证、独立输入线程及直接JSON数组解析；真实单/多图OLE均已复验。

重启测试曾引用旧WebView的locator导致失败：[原始失败证据](evidence/v018-import-native-test-initial-fail.json)，8组已执行但整套FAIL。重新启动后重新定位当前窗口，9组完整复验PASS。未将该测试设施问题错误归因为产品退出，也未忽略失败断言。

另修正模板新建名称恰为“空白文档”时被改为untitled的P2边界，新增明确单元测试；默认空白模板仍由对话框提供untitled。

## 风险与安全

不修改外部或受管原图、不删除真实AppData。导入使用既有唯一落盘和SQLite一致性方案；逐图失败可重试，重复导入创建独立副本。图片归一为RGBA8 PNG，JPEG方向应用，不保证色彩/元数据和动画完整保留；解码预算为尽力限制而非进程内存保证。Markdown插入失败保留素材库资料，临时blob不写长期文件。

主窗口限定的3个业务命令，无任意路径参数/通用fs/读取剪贴板插件授权。Windows主WebView设dragDropEnabled=false以使用HTML5原生文件drop；其它覆盖/贴图窗口不变。对比本地文件只经系统对话框选择，不入库，不显示完整磁盘路径。URL、监听器及过期请求按会话清理。

新增依赖核对：[image 0.25.10官方API](https://docs.rs/image/0.25.10/image/)、[Windows HTML5拖拽说明](https://docs.rs/tauri/2.12.3/tauri/webview/struct.WebviewWindowBuilder.html#method.disable_drag_drop_handler)、[Win32剪贴板](https://learn.microsoft.com/en-us/windows/win32/dataxchg/using-the-clipboard)。image及解码依赖MIT/Apache-2.0，复用锁定版本而非无必要升级；替代方案插件read_image会在限额前分配，故采用受控原生快照。

## 需要Windows人工验收（全部NOT_RUN）

1. 退出旧实例，运行生产EXE；分别从画图、浏览器、Office和第三方截图工具复制含透明度图片，显式导入、文本剪贴板和占用错误。确认没有自动采集。
2. 从资源管理器拖入中文/空格路径的单图、多图、损坏/不支持/大图；分别拖到素材库和活动Markdown，改名、写注释、标注、保存并重启；外部源文件不变。
3. 本地与素材对比、不同长宽/透明图、三种模式、A/B交换、反复进出、分栏开关，检查缩放/平移及键盘分割；Windows125/150/200%与混合DPI、多屏尚需覆盖。
4. 列表/网格、搜索/收藏/分页、删除确认，深浅主题、中文输入法、窄窗、长标题及长时间滚动；指南13节和定位。
5. 运行生产版截图/快捷键、笔记、Markdown在Obsidian/VS Code打开、OCR、贴图、托盘恢复和真正退出；重新验证历史QA-001退出句柄/P2、QA-002托盘间歇超时。
6. 磁盘满/权限不足/资源占用、长时间多次导入和资源稳定性、Windows10、macOS，以及签名/安装/卸载矩阵均NOT_RUN；不据此关闭历史问题。

## 阶段结论

本轮已执行检查无未解决的新FAIL，无已证实的新P0/P1。可以交付本功能与用户授权的普通GitHub Release；不把“非预发布”标识视为全产品/全硬件验收通过。可选差异图NOT_IMPLEMENTED；上述人工/生产全流程NOT_RUN。源码与发布结果在发布回执另记。


## 交付文件与文档复验

PASS / 0：`npm run check:docs`（45文档/269本地链接、权限与迁移契约一致）、`npm run lint`、`git diff --check`。生产EXE与GitHub附件哈希在发布回执记录。

本次修改/新增文件（不包含被忽略的构建产物、浏览器缓存、工具链或测试用户资料）：

```text
CHANGELOG.md
README.md
docs/ARCHITECTURE.md
docs/RELEASE-NOTES-0.1.8.md
docs/USER_GUIDE.md
docs/qa/RELEASE-0.1.8.md
docs/qa/TEST_REPORT.md
docs/qa/evidence/v018-compare-native.json
docs/qa/evidence/v018-document-native.json
docs/qa/evidence/v018-editor-native.json
docs/qa/evidence/v018-import-native-test-initial-fail.json
docs/qa/evidence/v018-import-native.json
docs/qa/evidence/v018-material-ui.json
docs/qa/evidence/v018-split-native.json
docs/qa/evidence/v018-templates-native.json
package-lock.json
package.json
scripts/check-docs.mjs
scripts/compare-native-smoke.mjs
scripts/image-drop-input.ps1
scripts/import-native-smoke.mjs
src-tauri/Cargo.lock
src-tauri/Cargo.toml
src-tauri/build.rs
src-tauri/capabilities/main.json
src-tauri/examples/workflow_input.rs
src-tauri/src/commands/import.rs
src-tauri/src/commands/mod.rs
src-tauri/src/lib.rs
src-tauri/src/services/image_import.rs
src-tauri/src/services/mod.rs
src-tauri/tauri.conf.json
src/app/App.tsx
src/app/layout/Sidebar.tsx
src/app/layout/Toolbar.tsx
src/app/ui-store.test.ts
src/app/ui-store.ts
src/features/compare/CompareView.tsx
src/features/compare/api.ts
src/features/compare/model.test.ts
src/features/compare/model.ts
src/features/compare/store.test.ts
src/features/compare/store.ts
src/features/desktop/store.test.ts
src/features/desktop/store.ts
src/features/document/LibraryPicker.tsx
src/features/document/MarkdownPanel.tsx
src/features/document/store.test.ts
src/features/document/store.ts
src/features/guide/GuideView.tsx
src/features/import/ImportControls.tsx
src/features/import/api.test.ts
src/features/import/api.ts
src/features/import/store.test.ts
src/features/import/store.ts
src/features/import/use-image-drop.ts
src/features/library/LibraryView.tsx
src/styles.css
```
