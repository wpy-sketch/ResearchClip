# v0.2.1 项目分类与离线公式验收

执行：2026-10-17–18，Asia/Hong_Kong；最终检查2026-10-18。环境：Windows 11 NT10.0.26200 x64 / WebView2、Node22.22.2/npm10.9.7、Rust MSVC1.99、Tauri2.12.3/CLI2.12.2。沿用 [测试规范](TEST_PLAN.md)，不把 Mock 或代码存在作为原生通过证明。

结论：本轮增量功能及相关回归最终 PASS，可按用户授权发布普通 v0.2.1。完整首发硬件、第三方软件与安装矩阵仍 NOT_RUN；历史 QA-001/002 未宣称修复。测试只使用 artifacts 下的专用数据与受控 PNG，未重置真实 AppData、删除用户原图或运行破坏性迁移。

## 功能、入口与实际状态

| 功能 | 涉及文件/关键函数 | 实现与验收 |
| --- | --- | --- |
| 素材管理侧栏项目 | Sidebar、ProjectNavigation、ProjectStore.refresh、LibraryStore.refresh | 已实现；PASS，项目A2图/B1图/未分类29图/全部32图；窄窗选择器保持相同筛选语义 |
| 实时与完整公式预览 | MarkdownPanel、MarkdownPreview、MathFormula | 已实现；PASS，离线字体、编辑后更新、行内/独立、分式、上下标、希腊字母、范数、矩阵、多行、求和积分 |
| 安全与错误 | remark-math/mathBudget、KaTeX.renderToString | 已实现；PASS，HTML、危险TeX链接/外部图片、宏循环/超限、跨公式宏隔离、错误源码保留 |
| 大纲/分栏兼容 | outline、MarkdownPanel、既有WorkspaceSplit | 已实现；PASS，公式内伪标题不生成目录，实际标题偏移正确，右侧仅活动文档时挂载 |
| 原始LaTeX保存与重开 | model.parseMarkdown/serializeDocument、DocumentFiles | 已实现；PASS，UTF-8、CRLF无修改保存字节不变、实际进程重启重新打开；图文块/Annotation版本不变 |
| Windows其他应用入口 | ExternalOpenButton、open_markdown_external、DocumentFiles::external_path/shell_path | 已实现；PASS：真实Windows选择器显示和取消、未授权token与外部修改拒绝；第三方应用逐个打开 NOT_RUN |
| 图文指南与布局 | GuideView、USER_GUIDE、styles | 已实现；PASS，14节任务目录/搜索、两张受控实际截图可加载、代码示例可复制；深浅主题和480–1440窗口 |

数据库保持 **schema v3**，001/002/003迁移均不修改；没有复制分类数据库或移动图片。现有项目图像/文档计数和批量事务继续使用同一 SQLite 查询。已有截图ID、笔记、标注、来源和路径保留。权限仅 main 增加 allow-open-markdown-external，overlay/pin不增权限；无通用fs、SQL、shell或opener授权。

## 依赖、安全和兼容性

复用 react-markdown10.1.0，同一次 MDAST 解析接入 remark-math6.0.0；KaTeX0.19.0与字体随程序打包，均 MIT。outline 同步使用 micromark-extension-math3.1.0、mdast-util-math3.0.0，不引入第二套预览管线。已核对官方示例、安装版本类型/API、许可证、registry版本；本轮无需MathJax的额外能力，选择较轻的KaTeX。API依据见 [架构说明](../ARCHITECTURE.md)。

用户HTML继续skipHtml，链接惰性，远程/任意本地图片不加载。KaTeX trust拒绝、strict=error、throwOnError=true，maxExpand500、maxSize20，每式独立宏字典；每式8192字符、预览256式/131072公式字符预算，超限明确错误且不改源码。只有引擎生成的HTML进入专用注入边界。指南图片仅由GuideView提供两个已打包静态映射，普通文档不获得此路径能力。公式不是图片，不接入OCR/AI识别或上传。

用户给出的矩阵 `\begin{bmatrix}1&2\\\3&4\end{bmatrix}` 在行分隔后多一个反斜杠：实际验证其错误状态和源码不变；正常示例 `\begin{bmatrix}1&2\\3&4\end{bmatrix}` 已通过。范数正确LaTeX写法为 `\|x-p\|=\lambda\|x-e\|`。不自动猜测/替换用户公式。

“使用其他应用打开”仅解析活跃授权token、检查文件快照，并先保存脏文档；保存取消/失败不进入系统选择器。Windows用UTF-16与Shell兼容路径，实际调用SHOpenWithDialog/OAIF_EXEC；无任意可执行文件参数，不替用户修改默认关联。外部应用修改后会触发既有拒绝覆盖机制。macOS分支尚未整机验证。

## 实际命令与结果

以下最终命令退出码均 **0**；无skip/忽略失败断言。

| 命令 | 实际结果 |
| --- | --- |
| npm ci --proxy http://127.0.0.1:7897 | PASS，303包，干净安装锁定依赖 |
| npm run check | PASS，TypeScript、ESLint、43文件/299项Vitest、前端生产构建 |
| npm run format:rust | PASS，cargo fmt --check |
| npm run test:rust | PASS，125项Rust测试，0失败/忽略；真实临时文件、SQLite与路径授权 |
| npm run check:rust | PASS，Clippy --all-targets -- -D warnings |
| node scripts/with-rust.mjs tauri build --debug --no-bundle | PASS，嵌入生产前端资源的原生验收程序 |
| node scripts/with-rust.mjs cargo build --manifest-path src-tauri/Cargo.toml --locked --example workflow_input --example markdown_dialog --example markdown_open_with_probe | PASS，原生窗口输入/选择器辅助器 |
| node scripts/math-native-smoke.mjs v021-accepted | PASS，[8组真实项目/公式/安全/保存/重启/指南](evidence/v021-math-native.json) |
| node scripts/markdown-open-with-native-smoke.mjs v021-accepted | PASS，[2组真实系统打开方式与取消](evidence/v021-openwith-native.json) |
| node scripts/projects-native-smoke.mjs v021-accepted | PASS，[11组项目/批量/迁移/文档索引](evidence/v021-projects-native.json) |
| node scripts/split-native-smoke.mjs v021-accepted | PASS，[11组分栏/坐标/历史/资源/重启](evidence/v021-split-native.json) |
| node scripts/document-native-smoke.mjs v021-accepted | PASS，[8组图文配对/合成像素/文件保存重开](evidence/v021-document-native.json) |
| node scripts/templates-native-smoke.mjs v021-accepted | PASS，[10组模板/大纲/持久化/删除归档](evidence/v021-templates-native.json) |
| node scripts/import-native-smoke.mjs v021-stable | PASS，[9组真实剪贴板/OLE/大小边界/SQLite/重启](evidence/v021-import-native.json) |
| node scripts/compare-native-smoke.mjs v021-final | PASS，[8组并排/滑块/透明像素/同步变换/列表](evidence/v021-compare-native.json) |
| node scripts/material-ui-smoke.mjs v021-final | PASS，[8组浏览器深浅主题/对比度/480–1440宽度](evidence/v021-material-ui.json)；不代替原生捕获 |
| npm run desktop:build | PASS，Tauri Windows x64生产EXE，FileVersion/ProductVersion=0.2.1 |
| npm run lint、npm run check:docs、git diff --check | PASS，最终脚本/本地链接/围栏/版本/权限/不可变迁移检查 |

本轮原生共 **67组**，浏览器8组。公式新增19项单元测试；核心既有测试全部继续执行。原生图片均为受控测试资料，不证明本轮真实屏幕读取。最终脚本日志留在本地 artifacts/qa-v021，公开证据不包含用户资料或认证信息。

生产文件：`src-tauri/target/release/researchclip.exe`，**19,106,816字节**；SHA256 `e23cbfa89b887d47c45d606ce91585e04c9f13eeb3e8dce35e7a625f2c393029`。未签名，需要WebView2；本轮未新建NSIS安装包。为避免生产EXE访问真实用户资料，原生运行使用同源debug隔离目录；生产EXE只做构建、PE版本和摘要验证。

## 初始失败、修正与复现

[原始失败摘要](evidence/v021-initial-failures.json)保留当时结果；最终PASS不抹除初始FAIL。

| 项目/程度 | 原始实际结果与复现 | 修正/最终结果 |
| --- | --- | --- |
| P2 外部源文换行被规范化 | 新增CRLF源文序列化测试FAIL，输入含CRLF后与输出不相等 | 普通source分支保存原始input；299单元及实际CRLF打开/保存PASS，不改结构化块格式 |
| Rust测试路径断言 | 授权路径为Windows规范路径，直接与非规范路径比较FAIL | 测试比较canonicalize；新增Shell UTF-16/UNC边界测试，125项PASS |
| 初次沙箱Vitest | 缓存模块写入ENOENT，进程失败 | 保留错误，环境外执行完整43文件299项PASS；未删测试 |
| 原生构建入口错误 | 普通cargo build生成dev-URL程序，没有1420服务，首次启动FAIL | 改用Tauri --debug --no-bundle嵌入dist；完整原生67组PASS |
| 测试定位/异步加载 | 用region定位普通预览div、指南加载前数目录为0、将供复制代码块当公式渲染，测试分别FAIL | 定位实际容器、等待懒加载目录；指南代码保持纯文本并核验可复制内容/两图实际解码，8组完整PASS |
| Windows选择器归属 | 最初仅枚举ResearchClip所属窗口超时；一次新主机可见窗口等待也超时 | 核对本轮新启动的System32/OpenWith.exe PID，再等待唯一可见窗口；只对其发WM_CLOSE，2组完整PASS；不把逐个第三方应用标为PASS |
| 原生拖放测试读取旧完成状态 | 5条记录时混合批次后立即断言6，FAIL | 每次原生拖放前关闭旧结果并等待本轮新结果；数量断言原样保留，最终9组PASS |
| 原生OLE桌面交互不稳定 | 另一次多文件OLE返回None，辅助器拒绝成功，FAIL | 不更改导入业务或伪造Drop事件；在生产编译结束后完整复验9组PASS。不能断言根因已永久消失，列人工拖放复验 |

未发现本轮已证实、未解决的新P0/P1；这不等于完全无风险。既有QA-001素材库关闭句柄错误、QA-002托盘恢复超时仍保持跟踪，本轮相关套件正常退出不构成这两个专项关闭验证。新功能检查建议进入发布；全面稳定版质量认证仍需下述人工矩阵。

## 尚未运行与Windows人工验收

以下 **NOT_RUN**，不能按自动测试或普通Release标识当作通过：

1. 保存并退出旧版本，备份完整科研资料；启动新EXE确认版本0.2.1，旧截图/标注/笔记/项目不变。真实用户生产资料目录升级本轮未运行。
2. 素材管理选择不同项目、全部素材和未分类，核对实际计数；组合关键词、标签、收藏，切换项目确认选择清空。滚动很多项目及高DPI字体人工检查。
3. 新建Markdown，复制指南中的行内、矩阵、多行公式；修改后检查实时预览，切换源码/预览并确认文字保留。首次保存到中文路径，关闭、重启并重新打开。
4. 输入错误矩阵或未知命令，确认显示“公式错误”，源码仍可编辑；离线环境检查字体与指南截图，不依赖网络。
5. 使用其他应用打开，分别选择已安装Obsidian/Typora/VS Code并核对标准.md/相对assets。第三方是否支持公式由其设置决定。外部修改后回到ResearchClip继续保存应拒绝覆盖；保留草稿再重新打开磁盘内容。
6. 多次从资源管理器拖PNG/JPEG/WebP进入素材库和右侧文档，包含损坏/混合文件；检查每批新增数及错误，尤其复验本轮OLE自动化失败记录。没有用Mock替代系统拖放。
7. 100/125/150/200%显示缩放、多屏负坐标、真实全屏/区域截图、快捷键、画图粘贴、贴图、OCR、托盘/单实例专项及长稳内存仍需人工/硬件矩阵；本轮不以受控图代替实时截图。
8. Windows10、macOS、NSIS新安装/升级/卸载、签名/SmartScreen及第三方杀毒、万卡性能均未运行。不要直接用旧版写入schema v3工作区。

下一步仅为用户人工验收及已授权的源码/普通GitHub Release发布；不扩展新功能。

## 修改文件

完整开发文件列表见下方，发布回执另提交。运行缓存、工具链、测试数据库/浏览器目录与用户资料不上传。

```text
CHANGELOG.md
README.md
docs/ARCHITECTURE.md
docs/PRD.md
docs/RELEASE-NOTES-0.2.1.md
docs/ROADMAP.md
docs/USER_GUIDE.md
docs/images/v021-math.png
docs/images/v021-projects.png
docs/qa/RELEASE-0.2.1.md
docs/qa/TEST_PLAN.md
docs/qa/TEST_REPORT.md
docs/qa/evidence/v021-compare-native.json
docs/qa/evidence/v021-document-native.json
docs/qa/evidence/v021-import-native.json
docs/qa/evidence/v021-initial-failures.json
docs/qa/evidence/v021-material-ui.json
docs/qa/evidence/v021-math-native.json
docs/qa/evidence/v021-openwith-native.json
docs/qa/evidence/v021-projects-native.json
docs/qa/evidence/v021-split-native.json
docs/qa/evidence/v021-templates-native.json
package-lock.json
package.json
scripts/check-docs.mjs
scripts/import-native-smoke.mjs
scripts/markdown-open-with-native-smoke.mjs
scripts/math-native-smoke.mjs
src-tauri/Cargo.lock
src-tauri/Cargo.toml
src-tauri/build.rs
src-tauri/capabilities/main.json
src-tauri/examples/markdown_open_with_probe.rs
src-tauri/src/commands/document.rs
src-tauri/src/lib.rs
src-tauri/src/services/markdown_document.rs
src-tauri/tauri.conf.json
src/app/layout/Sidebar.tsx
src/features/document/DocumentView.tsx
src/features/document/ExternalOpenButton.tsx
src/features/document/MarkdownPanel.tsx
src/features/document/MarkdownPreview.tsx
src/features/document/MathFormula.tsx
src/features/document/math.test.tsx
src/features/document/model.ts
src/features/document/outline.ts
src/features/guide/GuideView.tsx
src/features/projects/ProjectNavigation.tsx
src/styles.css
```


## GitHub发布结果

PASS：v0.2.1源码与标签已上传，普通Latest发布成功；3附件服务器摘要与匿名公开下载大小/SHA256一致，旧9个Release保留。详情见[发布回执](PUBLICATION-0.2.1.md)。该回执单独提交，不移动构建源码标签。
