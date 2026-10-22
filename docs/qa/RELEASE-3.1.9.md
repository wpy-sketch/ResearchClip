# ResearchClip v3.1.9 / A13 验收记录

本记录区分实际执行与待验收项目。开发与核心验证已完成；GitHub公开发布状态以文末回执为准。

## 范围与文件

本地实验图文语义检索、普通/语义切换、受限索引/取消/重建/删除、固定模型校验/导入/确认下载、指南示例。主要文件：`src/features/semantic/*`、`src-tauri/src/{services,commands,storage}/embeddings.rs`、`semantic-search/*`、migration011、素材库及指南。架构/许可/数据边界见 [语义搜索设计](../SEMANTIC-SEARCH.md)。没有新增依赖；五条main专用IPC权限，没有通用fs、HTTP、shell权限。

## 实际执行（2026-10-21，Windows 11 x64）

| 命令或检查 | 状态 | 实际结果 |
| --- | --- | --- |
| 官方固定权重下载与全量SHA256核对 | PASS | 八份228,030,489字节；固定提交，未发送用户图片 |
| 真实ONNX图文检索探针 | PASS | 六类、12查询，保存完整排名；Top1仅8/12，详见下表 |
| `npm run check`（正常Windows权限） | PASS | tsc、eslint、63文件424测试、Vite构建退出0；主JS大于500KB警告保留 |
| `npm run check:rust` | PASS | 退出0；此后改动需最终复核 |
| Python `unittest discover -s semantic-search` | PASS | 3项，退出0；正常Windows临时目录 |
| 沙箱内 `npm run check` | FAIL | 43套件无法加载沙箱SSR缓存，124测试执行通过；正常权限完整重跑通过，未删断言 |
| 沙箱内 Rust测试 | FAIL | 110通过99失败7忽略，多项临时原子文件/权限失败；正常Windows重跑另行记录 |
| 沙箱内Python测试 | FAIL | 1通过2临时文件权限错误；正常Windows3项通过 |
| 首次Clippy | FAIL | 使用不存在的Result方法及两项Lint，已更正后通过；未引入外部API |
| 首次性能探针 | FAIL | Windows ctypes句柄类型声明错误；修正探针后得到真实测量，不涉及业务代码 |
| 完整Rust/原生/安装/发布 | 见最终记录 | 本表保留早期过程，最终结果与已知FAIL/BLOCKED见下文 |

## 真实检索结果与性能

硬件：Intel Core Ultra 7 356H（16核/16逻辑处理器），内存33,879,642,112字节；存在RTX5060 Laptop/Intel集显，但没有进行GPU推理测试。

图模型加载634.62ms、文本297.25ms、校验164.49ms；热图像24.09–36.14ms/张，文本3.62–6.53ms/条；峰值工作集301,867,008字节（约288MiB），2个推理线程、CPUExecutionProvider、Python3.12.14、ORT1.31.0。不能把探针热编码当作应用端到端时间。便利样本，非通用基准；风景为含建筑的scikit-learn CC-BY示例。

| 目标 | 中文目标排名 | 英文目标排名 | 观察 |
| --- | --- | --- | --- |
| 数学公式 | 1 | 1 | 仅图形语义，不验证公式内容 |
| 神经网络图 | 1 | 1 | 自主受控图示 |
| 追逃轨迹 | 2 | 1 | 中文首位错误返回公式图 |
| 流程图 | 1 | 3 | 英文首位错误返回公式图 |
| 软件截图 | 3 | 2 | 两种语言均未排第一 |
| 自然风景 | 1 | 1 | 小样本命中不能外推 |

## Windows人工验收（NOT_RUN，需逐项填写）

1. 正常安装包首次启动，无Python/模型时切换普通搜索，截图、导入、编辑、OCR、Markdown保存仍可使用；未确认不联网。
2. 以原生文件选择器选专用Python；断网导入八份模型，错误摘要/大小/已有不同文件应报错且不覆盖。
3. 在12线程以下及高负载机器更新20/200/2000张索引，观察CPU、内存、取消响应；先确认后台任务完成再关闭。
4. 六类真实科研截图，中英文描述，记录排名与误匹配；精细数学图、长图、非中心内容单独核对。
5. 新增、重命名、删除图片，增量/重建、取消、重启，检查SQLite记录一致；删除索引后原图和文档均仍存在。
6. 索引目录只读/磁盘满、损坏模型、运行时缺失、并发点击、取消后重试，应有明确失败且普通搜索可用。
7. 搜索结果连续打开图片，右侧Markdown未保存草稿及分栏宽度保持；窗口缩放、100/125/150/200%DPI、双主题检查无控件遮挡。
8. 对旧schema10副本升级，核对升级前备份、原图SHA256/笔记/文档，未知schema12拒绝写入；勿对唯一用户资料做故障测试。

## 已知限制

仅CPU，GPU及macOS NOT_RUN；老CPU指令兼容未形成完整矩阵。每图独立进程释放内存，端到端慢于热推理；索引最多2000张，前60结果。重启任务需手动更新；不自动监听新图。误匹配已实测，不能宣称理解所有数学图像。语义派生索引不随WebDAV同步。极端长宽比安全失败。已有A11一次原生Markdown保存I/O错误历史尚无根因结论，不能称已修复；既有隔离复测通过不等于全硬件验收。


## 最终核心与原生结果

- PASS：最终 `npm run check`，63文件424测试，类型/ESLint/Vite退出0；`npm run format:rust`、严格 `check:rust` 退出0；全部Rust210通过0失败7忽略，退出0（忽略项需独立WebDAV/TLS环境，不计通过）。
- PASS：`semantic-native-smoke.mjs a13-verified`，真实Windows Tauri/WebView2/ONNX/SQLite的13组，退出0；12条完整排名见[原生证据](evidence/v319-semantic-native.json)，模型探针见[热推理证据](evidence/v319-retrieval.json)。应用索引6图6281ms，查询933–4242ms、中位1218.5ms。实测Top1仍8/12，未调整测试来追求全命中。
- PASS：`guide-reader-smoke.mjs a13-regression`，11组，退出0；18章114示例31图，双主题、480–1440宽、600高；[证据](evidence/v319-guide-reader.json)。这是无头Edge布局测试，不是Windows硬件人工验收。
- FAIL→已修复：第一次普通素材库原生回退暴露前端schema10校验与后端schema11不一致，已同步DTO/校验/测试并重建真实应用；最终同流程通过。第一次测试还使用旧结果未等待新查询结束，补充了等待并在执行期间隐藏旧结果。
- FAIL→已修复：正常Windows第一次Rust完整重跑200通过9失败7忽略，包括旧schema预期和模拟旧库夹具未移除新表；修正测试夹具/预期后全量210通过，保留原有断言目的。
- FAIL（既有问题未解决）：A11锚点回归前8组成功，第二次保存脏Markdown失败，界面保留草稿、原文件仍在。此前v3.1.7已记录同类偶发I/O问题；本轮已再次复现，不能称已修复。正式独立验收需诊断具体操作系统错误。
- FAIL（物理测试前台条件）：首次命令工作流回归在物理截图OCR断言失败，捕获画面未匹配受控文字；不作为受控物理截图通过。辅助程序已增加固定测试进程/唯一标题的前台校验，后续重跑另记结果。

独立完整功能验收与既有保存问题将按用户后续授权在A13交付后开展；本记录不把上述FAIL改写为PASS。全部测试为隔离资料，无用户原图库重置或文件覆盖。


- PASS：`commands-native-smoke.mjs a13-logical-final --skip-physical`，13组真实Windows快捷键、OCR/ONNX确认、取消、失败步骤重试、配置写入失败回滚、Markdown原生保存/最近文档/重启及双主题布局，退出0；[完整证据](evidence/v319-commands-native.json)。物理截图3项明确NOT_RUN。
- BLOCKED：受控Edge前台身份检查被Windows拒绝，新的物理回归在截图前停止；此后不继续采集不匹配窗口。物理捕获/OCR/取消需专用前台环境或人工验收，不能宣称本轮16组全通过。
- PASS：正式`desktop:bundle`优化EXE和NSIS构建，退出0；Python附件安全测试3项退出0。安装器隔离验收另记最终回执。


- FAIL→已修复：原生指南回归首排素材y位置超出680px，原因是新搜索模式另占整行；将切换放入页标题并保留窄窗换行，原断言不变。`guide-native-smoke.mjs a13-layout-final --verify-only`全部6组退出0：[证据](evidence/v319-guide-native.json)，包含真实编辑/对比/Markdown保存、31图解码与114示例导航。
- FAIL（退出告警，保留原断言）：`semantic-native-smoke.mjs a13-delivery`完成13组功能检查，但最终收集到一次WebView `PostMessage ... 0x80070578` 无效窗口句柄日志，整体退出1；没有改为静默忽略。独立原样重跑另记；不能据单次干净重跑宣称所有退出时序问题已修复。
- PASS：最终完整Rust210通过7忽略（51.57s），格式与Clippy退出0；最终布局的前端完整检查424项退出0。硬件与模型性能见上文，不含GPU推理。


- PASS：`semantic-native-smoke.mjs a13-delivery-recheck`，原断言不变，13组退出0、stderr为空：[独立重跑](evidence/v319-semantic-recheck.json)。保留此前退出告警，未声称已根治。
- PASS：`semantic-native-smoke.mjs a13-model-download --download-models`，14组退出0，实际点击下载确认后下载全部八份固定模型、核对摘要和已有文件保护，耗时78,775ms；完整索引/12查询/取消/故障/重启也通过：[下载与原生证据](evidence/v319-model-download.json)。测试代理只用于此明确下载测试，应用默认离线行为未更改。
- 环境调整（2026-10-22）：恢复工作时PATH中不再有PowerShell7，首次安装验收命令未执行；使用官方ZIP校验后在隔离工具目录准备PowerShell7.5.4，不改应用依赖、用户系统PATH或全局安装。最终安装器验收另记。


## 交付前核对（2026-10-22）

最终优化NSIS安装器通过`installer-smoke.ps1 -RunId v319-verified -TargetDir src-tauri/target`真实隔离4组验收，退出0：Unicode/空格路径安装、完整载荷及版本核对、忙碌安装/卸载均exit2、默认卸载保留既有AppData摘要和无关文件，测试安装及登记已移除；NotSigned。摘要与待发布payload一致。汇总见[机器可读检查](evidence/v319-checks.json)。

| A13需求 | 实际状态与依据 |
| --- | --- |
| 主动开启、默认离线 | PASS：首次无运行时/模型/任务；不确认下载被拒绝，重启不自动索引 |
| 图像向量/文本向量/共享空间 | PASS：实际固定两编码器、投影与预处理；六类图片12查询，不同模型ID/版本拒绝混用 |
| 按相似度排序、点击打开 | PASS：真实SQLite余弦降序，UI缩略图打开原卡片，右侧脏稿保留 |
| 取消、删除、重建、增量 | PASS：已完成记录保留，删除仅向量，更新跳过不变图，取消后重试 |
| 普通/语义切换、模型不可用回退 | PASS：损坏模型失败且普通素材仍正常；schema11契约回归已修复 |
| 固定模型确认下载/离线导入 | PASS：原生目录选择器导入及实际228MB确认下载，全量摘要；不同目标不覆盖 |
| 资源和安全边界 | PASS：固定脚本和文件、2线程、2GiB进程上限、超时/输出限制、main限定IPC；无上传、任意命令或云向量库 |
| 重启、索引更新与错误恢复 | PASS：SQLite写失败不丢旧向量；恢复后仅补新图，退出重开仍可检索 |
| 真实排名与性能 | PASS：已披露12条排名、8/12 Top1及加载/RSS/端到端时间；不声称全类数学理解 |
| 可选GPU、完整人工/硬件矩阵 | NOT_IMPLEMENTED / NOT_RUN：本版只CPU，GPU只是评估替代；人工步骤保留上文 |

发布准备包含10个附件，插件/MathOCR/旧Qwen资源索引全部保留；发布前历史快照27个Release、196个资产。公开发布与匿名下载核验回执完成后补记，不以本节替代上传完成。


## 公开发布回执（2026-10-22）

[v3.1.9 Release](https://github.com/wpy-sketch/ResearchClip/releases/tag/v3.1.9)已公开，源码标签固定`8b2e4ab0c7283a6210886ccdc759b5d04ff62e97`。独立GitHub读取确认latest=v3.1.9、draft=false、10个附件；全部匿名下载大小/SHA256通过。发布前后27个旧Release、196个旧附件ID/版本/提交/名称/大小/摘要一致：[完整回执](evidence/v319-publication.json)。

大EXE沿用已验证的压缩Git对象传输，[一次性任务](https://github.com/wpy-sketch/ResearchClip/actions/runs/39703346030)不重新编译，只在固定源码/Release/摘要守卫下还原测试字节；原EXE与服务端及匿名下载SHA256均为`1ae637e26d0d79589d40095f61108bf7f44e612fa52bd233dcd88cd6ac4cc6d5`。只删除精确租约匹配的本任务临时分支，并移除一次性任务文件：[运输记录](evidence/v319-transfer.json)。正式安装器SHA256为`704a362bdadb7cc508268103e80b42f2e0aee7103099a7fc228a26a18c92c65b`。

指南ZIP50个文件，CRC与全部内置相对链接通过；浏览器插件和MathOCR仍与既有正式附件逐字节一致，旧Qwen资源URL/摘要保留。v3.1.8的001–010迁移逐文件未变。上述已知Markdown二次保存失败、偶发退出告警及人工/硬件未验项仍保留，后续独立验收继续追查。
