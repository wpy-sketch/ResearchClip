# ResearchClip Pro 高级功能独立验收 / v4.0.0

日期：2026-10-22。范围：在A13交付后重新阅读AGENTS、PRD、ARCHITECTURE、各功能需求和实际源码，独立执行验收、修复审查发现的问题、改进缩放阅读布局和指南。未开发其他新业务功能。此处“独立”指新的验收批次，不表示第三方认证。

## 1. 状态与环境

| 状态 | 定义 |
| --- | --- |
| PASS | 本轮实际执行并通过；源码审查、单元、无头浏览器、原生自动化各自注明 |
| FAIL | 实际执行失败，保留复现与后续结果，不覆盖历史失败 |
| BLOCKED | 环境或外部依赖阻止执行 |
| NOT_RUN | 未实际执行，不能计为通过 |
| NOT_IMPLEMENTED | 功能没有实现，不以设计、Mock或可编译骨架冒充 |

环境：Windows 11 x64 build26200；Intel Core Ultra7 356H，约32GiB内存；Node22.22.2、Rust1.99、Python3.12.14/ONNX Runtime1.31.0。GPU存在但只执行CPU推理。所有原生测试使用单独的测试设置目录、生成/许可明确的练习图片和临时SQLite；没有重置真实AppData。WebView2/CDP自动化是实际Windows桌面程序测试，但不等于人工操作、高DPI多显示器或macOS验收。

## 2. 功能逐项对照

| 编号与名称 | 主要源码与架构 | 本轮验收层级、边界 |
| --- | --- | --- |
| A01 滚动长截图 | services/scrolling、capture/stitching、features/scrolling；有限帧垂直拼接，复用CaptureWorkflow | Rust拼接边界、取消/歧义/预算测试；物理滚轮、固定页脚、多DPI矩阵NOT_RUN |
| A02 网页剪藏 | browser-extension、Native Host、services/clipper；固定origin、限长帧、会话令牌，主动接收 | 独立真实Host恶意帧/来源/十秒超时测试；真实浏览器扩展安装与网页跨域矩阵NOT_RUN |
| A03 OCR全文索引 | services/ocr_index、storage/ocr；schema4/FTS5，单并发逐条落盘 | Rust转义、取消、恢复；A12原生调用实际Windows OCR。批量5000图和全部语言包NOT_RUN |
| A04 本地公式识别 | math-ocr、services/math_ocr；受限CPU子进程、人工确认 | A12复用实际本地ONNX并确认；既有公式识别错误保留，不能称数学正确性全通过 |
| A05 标注历史 | storage/revisions、RevisionHistory；schema5，CAS和追加式恢复 | Rust版本冲突、损坏快照、迁移/持久化；A11原生建立新标注版本并保持原图 |
| A06 WebDAV | services/sync、storage/sync；schema6、队列、ETag、逐项冲突与系统凭据 | 本机单元/故障测试；七项外部WebDAV/TLS测试忽略，独立服务商/坚果云认证BLOCKED，不能引用历史设计M1作为当前实现结论 |
| A07 窗口捕获 | services/window_capture、window-picker、WGC worker | 源码/构建检查；真实窗口遮挡、最小化、HDR/多屏本轮NOT_RUN |
| A08 相似与重复 | storage/similarity、features/similarity、可选similarity_ai；schema7 | Rust哈希/范围/持久化；Qwen人工质量、完整物理回归NOT_RUN；不自动删图 |
| A09 文献引用 | storage/references、features/references；schema8，主动DOI查询与BibTeX预览确认 | Rust导入/转义/事务/关联；真实Crossref在线服务本轮NOT_RUN |
| A10 双向链接与图谱 | features/links、commands/links、storage/links；schema9，后台Worker | 真实Windows：候选、稳定ID、截图/自引用、A↔B循环、重命名、引用位置、项目/标签/邻居筛选、平移缩放、取消索引、重启和删除失效5组PASS |
| A11 图片局部锚点 | features/anchors、storage/anchors、编辑器和links/navigation；schema10 | 点/矩形、原图像素、移动/显示/缩放、原生保存、左右分栏、连续跳转/短暂高亮、草稿保留、版本变化、删除失效、可读导出与重启；最终结果见第5节 |
| A12 命令与工作流 | features/commands model→ActionRegistry→finite runner→既有服务；设置保存组合 | 9命令、7动作、3预设；输入输出检查、用户确认、取消、失败步骤重试、幂等检查点、最近文档、无文档/文件写失败/重启；实际结果见第5节。3项物理捕获受前台条件限制 |
| A13 离线图文语义检索 | EmbeddingService、semantic-search/worker、storage/embeddings；schema11 | 固定共享512维空间、主动确认、真实ONNX/SQLite、增量/取消/重建/删除/故障恢复、12条双语排名；本轮13组PASS。GPU NOT_IMPLEMENTED，2000张规模性能NOT_RUN |

## 3. 架构、数据契约与安全审查

沿用React/Zustand + Tauri2 + Rust模块化单体 + SQLite。v4.0.0没有新依赖、capability、数据库表或迁移；schema保持11，001–011应与v3.1.9逐字节一致。此前schema9/10/11分别增加链接、锚点和派生向量，不改变不可变原图或Annotation schema1。

- 文档内部目标使用稳定documentId/screenshotId/anchorId，数据库映射授权Markdown源文件；标题仅作标签。LinkIndex保存sourceId、targetId、linkType、UTF16位置/行号、时间和锚点ID。重命名保留ID，删除保留可核对的失效引用。标准导出使用相对文档链接；无法携带的内部截图/锚点引用保留文字并提示。
- AnnotationAnchor保存原图SHA256版本、原图像素区域、标题/说明及CAS revision；标注历史变化不改变原图身份，原图版本不符拒绝跳转。异步连续跳转以序号丢弃过期请求，右侧未保存文档仍保留。
- ActionRegistry只包含固定动作。2–8步、最多16项组合，不接受OS命令、任意脚本或循环。检查点只在本进程保留，重启不自动重放写入；不可逆失败要求检查结果，不盲目重试。
- EmbeddingService单任务、固定模型ID/版本、512维归一化向量；表含screenshotId、embeddingModelId、embeddingVersion、imageHash、embeddingData、sourceStamp、updatedAt。向量只从固定兼容编码器产生；普通搜索不依赖模型。最多2000图、60候选、2推理线程、2GiB进程限制，取消保留已完成行。
- 迁移先quick_check和既有schema验证，升级前唯一快照、逐版本事务、失败回滚，未来版本拒绝写入。原图和新导出禁止覆盖；正常已授权Markdown保存用原子替换并校验外部变化。已有文件/数据库故障测试只对隔离副本执行。
- Markdown使用skipHtml、固定图片白名单/受管assets、受限内部URL解析；不执行HTML/脚本，不从内部ID读取任意路径。不新增外部deep-link注册。
- Tauri权限仍为窗口限定固定IPC；main语义5命令不授予overlay/pin；无通用fs/http/shell权限。模型下载、DOI和WebDAV都需既有明确操作，未新增启动下载、遥测或图片上传。
- 清理审查：链接Worker在成功/失败/取消时terminate并移除abort；图谱wheel、指南scroll/rAF、命令键盘监听均卸载清理；缩略图URL释放；识别子进程复用Windows Job Object/超时，退出请求取消。代码审查未发现新增无界线程/监听器，但不等于长期资源泄漏压力测试通过。

## 4. 本轮修正与主要修改文件

1. `src/features/semantic/api.ts`、`api.test.ts`、`SemanticPanel.tsx`：修复轮询与动作响应乱序可能倒退任务/取消状态的问题。按generation、终态和已完成进度保留新状态；旧effect轮询有独立取消标志，卸载后不续订。新增4项顺序/取消/新代次测试。
2. `src-tauri/src/services/markdown_document.rs`：文件读取、暂存创建/写入、原子替换失败报告具体操作和OS错误码；不记录正文或完整路径。不猜测偶发I/O根因、不加入绕过外部变化保护的覆盖重试。新增100次原子保存/重开、Windows占用失败保留原文件并解除占用后成功重试两项测试。
3. `src/features/guide/GuideView.tsx`、`src/styles.css`、`docs/USER_GUIDE.md`：六个任务阅读入口，18章从上至下连续编号，114项练习，31幅离线图片；文字和目录随基础字号缩放，柔和背景/圆角，阅读栏可换行，窄窗和短窗保留滚动入口。编辑器画布最小高度从110提高到240 CSS像素，外层滚动，避免顶部控件增加后把图片压得难以拖动。
4. `scripts/guide-reader-smoke.mjs`增加双主题12/16/20字号与75–200%内容缩放验证；`scripts/anchors-native-smoke.mjs`保留失败alert诊断；`scripts/package-release-companions.py`将本报告放入v4离线指南。版本元数据和发布说明更新4.0.0。

## 5. 实际执行命令与结果

本节区分首轮与修正后结果；尚未返回成功结果的项目不得计PASS。日志保存在隔离artifacts，公开证据仅包含检查结果与受控练习数据。

| 命令 | 状态 | 结果 |
| --- | --- | --- |
| `npm ci --no-audit --no-fund --cache .tools/npm-cache` | PASS | 303包，退出0，未升级依赖 |
| `npm run check` | PASS | TypeScript、ESLint、63文件428测试、Vite退出0；大chunk提示保留 |
| `python -m unittest discover -s semantic-search` | PASS | 3项退出0 |
| `python scripts/test-release-companions.py` | PASS | 3项退出0 |
| `npm --prefix browser-extension run check` | PASS | 实际构建及测试退出0 |
| `node scripts/links-native-smoke.mjs v400-independent` | PASS | 5组，退出0；脚本实际断言schema11，首条旧结果文字残留schema10属说明笔误；脚本已更正，最终独立重跑另记 |
| `node scripts/semantic-native-smoke.mjs v400-independent` | PASS | 13组，退出0；真实模型、SQLite、取消、故障恢复和重启 |
| `node scripts/clipper-host-smoke.mjs v400-independent` | PASS | 5组，退出0；真实Host进程，不冒充浏览器扩展完整安装 |
| `node scripts/guide-reader-smoke.mjs v400-zoom` | PASS | 13组，退出0；含字号和内容缩放，明确非物理DPI |
| `npm run test:rust`（修正后） | PASS | 212通过、0失败、7忽略，退出0；含迁移回滚、未来版本拒写、持久化及新增2项文件测试 |
| `npm run format:rust`、`npm run check:rust` | PASS | 格式与严格all-targets Clippy退出0 |
| `node scripts/anchors-native-smoke.mjs v400-final` | PASS | 最终Windows构建11组退出0，包括再次保存、连续跳转和真实进程重启 |
| `node scripts/commands-native-smoke.mjs v400-final --skip-physical` | PASS | 最终Windows构建13组退出0；3项物理捕获明确NOT_RUN |
| `node scripts/advanced-renderer-smoke.mjs v400-independent` | PASS | 4组退出0，实际Edge像素合成；不是原生物理捕获 |
| `node scripts/semantic-native-smoke.mjs v400-final` | PASS | 最终状态修复构建13组退出0；6图索引5440ms，12查询完整排名保留 |
| `node scripts/links-native-smoke.mjs v400-final` | PASS | 5组退出0，schema11说明同步 |
| `node scripts/anchors-native-smoke.mjs v400-canvas-final` | PASS | 画布修复后11组退出0 |
| `node scripts/guide-native-smoke.mjs v400-canvas-final --verify-only` | PASS | 画布修复后6组退出0，18章114示例31图 |
| `node scripts/advanced-renderer-smoke.mjs v400-canvas-layout` | PASS | 5组退出0，新增240px可操作画布与短窗滚动检查 |
| `node scripts/guide-reader-smoke.mjs v400-delivery` | PASS | 最终指南13组退出0 |
| `npm run check`（画布修复后） | PASS | 428测试、类型/Lint/前端构建退出0 |
| `npm run check:docs` | PASS | 123文档672本地链接及权限/001–011迁移检查退出0；后续仅报告链接更新会再复核 |
| `node scripts/advanced-native-smoke.mjs v400-verified` | PASS | 原坐标/像素断言保留，5组退出0；实际SQLite保存及进程重启通过 |
| `npm run desktop:bundle`（最终画布修复） | PASS | 优化EXE和NSIS退出0 |
| `installer-smoke.ps1 -RunId v400-final -TargetDir src-tauri/target`（PowerShell7.5.4） | PASS | 4组，退出0；完整载荷摘要/版本、Unicode路径、忙碌安装和卸载exit2、默认卸载保留14份既有AppData文件摘要及无关哨兵；测试安装已清理 |

## 6. 失败复现、未解决问题与性能边界

- FAIL（已修正测试编译）：新Windows锁文件测试最初使用unwrap_err要求成功DTO实现Debug，Rust编译失败；改用err().unwrap()保持原断言，没有修改生产DTO来迎合测试，重跑结果在第5节记录。
- FAIL（本轮原生标注回归）：默认800高窗口的画布被顶部工具/锚点区压至110像素，适应缩放34.5%；原样两次坐标回归得到x48.179而预期50（阈值1.8），随后区域宽高也偏离，不能忽略。保留失败记录，改进最小画布高度并让页面滚动；扩大画布后，旧鼠标脚本未把画布滚进视野，首轮高度断言再次失败；补真实scrollIntoView，不改变坐标阈值。随后全部移动/缩放/像素/项目重开断言通过，暴露了旧脚本等待过时“截图与标注已保存”提示的超时；按实际源码更新为“标注已保存；内容未改变时保留现有版本。”，数据库与重启断言保留。最终`v400-verified`完整5组退出0，真实SQLite保存和进程重启均通过。复现命令：`node scripts/advanced-native-smoke.mjs v400-final`，受控马赛克移动/Transformer调整步骤。
- 历史FAIL，根因未解决：v3.1.7/A13锚点流程第一次保存后，点击引用、调整分栏、再Ctrl+S，偶发通用Markdown I/O失败。原文件及草稿均保留。本轮诊断版11组通过不能证明根治；新增操作/OS码用于下次复现。复现：两锚点引用→首次另存→修改Markdown→连续跳转和分栏调整→Ctrl+S，记录错误码并核对原文件，不关闭脏稿。
- 历史FAIL，根因未解决：A13一次退出出现WebView PostMessage无效窗口句柄0x80070578；原断言整体判FAIL，之后独立重跑干净。未全局吞掉stderr或声称修复框架退出时序。长期启动/退出资源压力测试NOT_RUN。
- BLOCKED：Windows拒绝将受控Edge置前时，物理截图测试在采集前终止；不放松PID/标题守卫。命令逻辑、原生键盘/OCR/ONNX测试可独立执行，3项物理捕获另列NOT_RUN。
- BLOCKED：外部WebDAV/TLS认证环境缺失；七项忽略不计PASS。第三方编辑器、坚果云、多设备真实服务器一致性需专门验收。
- A13实际六类12查询仅8/12 Top1（中文4/6、英文4/6）：公式/神经网络/风景中英文首位；追逃中文第2；流程图英文第3；软件截图中文第3/英文第2。详见[真实检索记录](RELEASE-3.1.9.md)。数学语义检索不能证明公式内容理解。
- 热推理图像24.09–36.14ms、文字3.62–6.53ms，峰值约288MiB为A13探针历史实测；应用每项独立进程，端到端更慢。v4本轮端到端另见机器证据，不能混称。2000张规模、旧CPU、GPU、macOS和完整硬件矩阵NOT_RUN；GPU推理NOT_IMPLEMENTED。
- EXE/安装器未签名，前端大chunk提示存在。本轮未开发PDF导出或独立备份恢复系统，相关未来规划不算已实现功能。

## 7. Windows原生人工验收清单（均NOT_RUN，待人工填写）

1. 保存退出旧实例后，用v4.0.0正式安装器升级独立旧数据副本。核对schema11、升级前快照、原图SHA/笔记/标签/文档；未来schema拒写。不要对唯一资料做故障实验。
2. 100/125/150/200%DPI、双屏不同缩放、HDR/SDR：全屏、区域、窗口、遮挡窗口、滚动长截图、Esc取消；核对原图像素和跨屏坐标。600高/窄窗、12–20字号、双主题，确认设置/指南/文档保存等入口可达。
3. 原图→标注→撤销/重做→素材库→历史恢复→原分辨率PNG；模拟只读输出/同名导出，不覆盖已有文件。验证OCR语言、区域、批量取消与失败恢复。
4. 在全新Chrome/Edge配置中安装随包WebClipper与Host，核对固定ID/注册路径；主动接收网页、超长正文/恶意HTML、失败图片、取消与重复请求，关闭会话后连接拒绝。
5. A10：中文/emoji源文多处引用、A↔B循环、改名/删除目标、重新打开；验证反向位置/行号、图谱项目/标签/邻居、滚轮/平移、失效提示和相对链接导出。
6. A11：点和矩形创建/移动/隐藏，图像缩放后核对像素；插入两个锚点→连续点击→分栏拖动→脏稿保留；删除锚点和更换原图版本必须拒绝误跳；反复保存/关闭重启，记录任何OS错误码。
7. A12：实际Ctrl+Shift+P与第三方软件抢占，9命令、三个预设、自定义类型不兼容拒绝；各步骤取消、识别结果确认、失败重试、双击触发、无Markdown、只读设置和重启不自动执行。需保持受控测试窗口在前台。
8. A13：未确认不下载不上传；断网导入正确/错误模型，确认下载/取消、更新/删除/重建索引、损坏/磁盘满恢复、项目/收藏范围、六类真实中英文查询并记录完整排名。测试20/200/2000图耗时/RSS，不以便利6图推断规模性能。
9. 使用本人测试WebDAV账号，两台隔离客户端，认证/TLS/强ETag、离线重试、并发修改、删除与冲突逐项确认；未授权不传输，失败不覆盖唯一版本。
10. 反复开关素材、指南、图谱、锚点、命令和语义面板，任务运行时退出，观察进程/线程/句柄/内存及端口释放。默认卸载保留用户资料；不同用户权限与安全软件单独登记。

## 8. 发布与是否继续

v3.1.9已公开，10附件匿名摘要通过；其历史失败与未验项在此保留。v4.0.0发布只使用本轮最终构建字节，保留WebClipper、MathOCR、模型清单/既有大模型资源入口和完整指南；不覆盖旧Release。正式发布和历史附件比较结果另记[本版发布记录](RELEASE-4.0.0.md)。

建议：本轮修正和可执行检查通过后交付，并在扩大使用前优先完成上述人工/硬件及外部服务器验收。不建议把当前自动化结果表述为全平台、全模型准确率或长期稳定性认证；不自动开始下一业务功能。

机器证据：[锚点](evidence/v400-anchors.json)、[命令](evidence/v400-commands.json)、[语义含完整排名](evidence/v400-semantic.json)、[双向链接图谱](evidence/v400-links.json)、[指南原生](evidence/v400-guide-native.json)、[Host](evidence/v400-clipper-host.json)、[指南字号/缩放](evidence/v400-guide-reader.json)、[标注首次失败](evidence/v400-advanced-failure.json)、[原样失败复现](evidence/v400-advanced-failure-recheck.json)。

画布修正后的证据：[原生标注5组](evidence/v400-advanced-native.json)、[短窗像素与布局5组](evidence/v400-renderer-canvas.json)、[锚点11组](evidence/v400-anchors-canvas.json)、[原生指南6组](evidence/v400-guide-canvas.json)。无新依赖；编译与实际安装版API核对未发现本轮引入未经确认的API。

最终机器摘要：[检查汇总](evidence/v400-checks.json)、[确切安装器](evidence/v400-installer.json)。NSIS为未签名版本；安装器SHA256 `f4234e72d46f183b88c88ad9230b799c51ea8497fb47c6fc1ec5aaddacef3001`，原始EXE SHA256 `b09249e72b7b246570b3419873968b35445af8b61c33b251ee4a7574eda67591`。全量53组Windows GUI、Host5组及无头布局/像素18组分别记录，不混为人工硬件验收。

公开交付已完成：[v4.0.0 Release](https://github.com/wpy-sketch/ResearchClip/releases/tag/v4.0.0)，10个附件匿名下载SHA256通过，28个历史Release/206附件元数据保持；完整回执见[本版发布记录](RELEASE-4.0.0.md)。没有新增下一业务功能，人工与未解决项状态保持上文。
