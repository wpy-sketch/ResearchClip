# ResearchClip Research IDE：Step00基线报告

记录边界：本文件保留Step00改动前的schema11基线，测试结果不被后续重跑覆盖。Step01的schema12改动和独立验收见[RESOURCE_MODEL_STEP01](RESOURCE_MODEL_STEP01.md)。

日期：2026-10-23，Asia/Hong_Kong。目标：核实真实现状、完成升级规划并执行既有可运行检查。范围仅新增[现状](../architecture/CURRENT_STATE.md)、[目标架构](../architecture/TARGET_ARCHITECTURE.md)、[技术决策](../architecture/TECHNOLOGY_DECISIONS.md)、[路线图](../ROADMAP_RESEARCH_IDE.md)及本报告；无业务/依赖/迁移/权限变更。

## 1. 环境和资料隔离

基点53e4bfa994d4c648b5b4d0545cb1b87a164e033f，main，4.0.0，schema11；开始时无未提交工作。Windows NT10.0.26200 x64；Node22.22.2，npm10.9.7，Rust1.99.0 MSVC、cargo1.99.0，Python3.14.6。锁定rusqlite0.37.0/libsqlite3-sys0.35.0的bundled sqlite3.h为SQLite3.50.2，不是系统sqlite命令行版本。

日志放仓库忽略目录artifacts/research-ide-step00-20261023，首轮和重试分别保存，不覆盖历史证据。原生脚本使用新的settings/workspace/WebView2 profile及测试PNG，真实资料根未启动/未重置/未移动。未安装应用或扩展、未改Zotero库、未登录Agent、未下载模型、未联系真实WebDAV账户。Rust凭据单测仅创建随机测试目标并清理。

## 2. 实际检查结果

| 命令/条件 | 退出码 | 数量/结果 | 日志或说明 |
| --- | --- | --- | --- |
| npm run check，默认沙箱临时目录 | 1 | typecheck/lint已执行通过；测试18文件/109测试通过，45文件加载失败；build未执行 | frontend-check.log；ENOENT指向沙箱SSR缓存 |
| npm run check，TEMP/TMP改仓库隔离目录 | 0 | 63文件、428测试；typecheck/lint/build通过 | frontend-check-workspace-temp.log |
| npm run format:rust | 0 | 格式检查通过 | format-rust.log；命令为fmt --check，不重写源码 |
| npm run test:rust，默认沙箱临时目录 | 101 | 111通过、101失败、7忽略 | test-rust.log；大量文件提交失败，不能算通过 |
| npm run test:rust，仓库TEMP但仍在沙箱 | 101 | 210通过、2失败、7忽略 | test-rust-workspace-temp.log；Windows凭据和模板目录提交失败 |
| npm run test:rust，正常Windows权限及隔离TEMP | 0 | 212通过、0失败、7忽略；main/doc tests各0 | test-rust-unsandboxed.log |
| npm run check:rust | 0 | Clippy all-targets -D warnings通过 | check-rust.log |
| npm --prefix browser-extension run check | 0 | 5测试通过，TypeScript及两项Vite构建通过 | extension-check.log |
| node scripts/with-rust.mjs cargo test --manifest-path browser-extension/Cargo.toml --locked --workspace | 0 | protocol3通过；Host/doc tests0 | clipper-rust.log |
| python -m unittest discover -s semantic-search -p test_worker.py -v | 0 | 3通过 | semantic-worker-tests.log；边界单测，不是实际ONNX模型准确率 |
| node scripts/with-rust.mjs cargo build --manifest-path src-tauri/Cargo.toml --locked --bins --examples | 0 | debug二进制及探针构建 | debug-native-build.log；这次构建本身未嵌入生产前端 |
| npm run desktop:build | 0 | Windows release EXE构建，无安装包 | desktop-build.log；Rust优化编译约1m25s |
| npm run tauri -- build --debug --no-bundle | 0 | 打包前端的Windows debug EXE | debug-tauri-build.log；后续原生脚本使用此产物 |
| node scripts/split-native-smoke.mjs research-ide-step00-20261023 | 1 | 0组；等待主界面超时 | 初次cargo构建使用devUrl，未运行dev server；随后改用正式Tauri debug构建重试 |
| 同脚本，后缀-r2（沙箱）和-r3（正常权限） | 各1 | 各6组完成，整体FAIL | split-native-r2/r3.log；等待已不存在的旧成功文案超时 |
| node scripts/smoke.mjs --desktop --run-id research-ide-step00-20261023 | 1 | 整体FAIL，截图步骤未到达 | capture-native.log；等待旧指南标题“从图片到研究记录”，当前标题为“从截图到研究成果” |
| node scripts/advanced-native-smoke.mjs research-ide-step00-20261023 | 0 | 当前v4原生编辑器5组通过 | advanced-native.log及独立report.json；测试PNG，不是物理屏幕捕获 |
| src-tauri/target/debug/examples/capture_probe.exe 3 | 0 | 3次实际GDI捕获，2560x1440；无效显示器拒绝；GDI对象1→1 | capture-probe.log；只读尺寸/计数、不存屏幕像素 |

以上PASS是本轮真实命令结果；历史v4.0.0报告未替代本轮执行。首轮失败不能被最终成功覆盖。测试数量按各套件单独列示，不把重复重跑累计成新增测试。

原生编辑器通过涵盖真实WebView2/Konva编辑、效果与像素、原始PNG字节、项目/PNG磁盘操作、SQLite标注保存及进程重启。分栏前六组涵盖未开文档不挂载、空白不落盘、中文/公式/撤销、左侧导航保留右文档、光标插图、真对话框保存和拖动/键盘比例持久化；后续断言没有执行完，不能称整个分栏验收通过。

Rust七项忽略依赖显示器/系统剪贴板/真实本机识别或外部WebDAV等原生条件，忽略不计PASS。真实GDI探针仅补充当前主屏捕获和资源计数，不补齐区域交互、混合DPI、像素颜色、所有显示器或其他忽略项。

## 3. 失败分类与已尝试动作

1. 环境限制：默认沙箱TEMP的Vite缓存/文件安全提交失败，先改仓库隔离TEMP，Rust剩余凭据/目录提交仍受限；正常Windows权限重跑212通过。未改断言、源码或依赖。
2. 测试启动方式：普通cargo build的debug应用未带打包前端；真实Tauri debug构建后主界面正常。初次失败保留，不归为应用功能修复。
3. 既有脚本失配：分栏脚本等待旧保存文案；截图脚本等待旧指南标题。源码与实际失败截图可核对当前文本。未修改脚本、未删断言以制造通过；改用既有已适配v4的advanced-native补充，原脚本继续FAIL。需下一阶段单独修复测试后重跑。
4. 工具环境：后续沙箱通道连简单只读命令也无输出，Node工具亦超时；普通权限通道正常。切换后只在授权仓库写文档/执行隔离检查，没有以环境故障掩盖应用失败。
5. Vite仍提示主chunk约780kB（gzip约236kB），debug构建有增量缓存硬链接失败转复制警告；构建退出0，不等于警告消失。

## 4. 已知问题与未执行项

历史报告保留的偶发Markdown保存I/O与WebView退出句柄告警，本轮未复现根因，不宣称已修复。文档仍有早期schema/MVP状态混杂，现状报告已指出冲突。

| 验收 | 状态/原因 | 所需条件 |
| --- | --- | --- |
| 完整区域/窗口/滚动截图UI链 | 未完成；旧主冒烟在采集前失败 | 修复旧脚本断言后重跑，不能用GDI尺寸探针替代 |
| 混合DPI/多屏/HDR/远程桌面 | NOT_RUN | 人工Windows硬件矩阵，100/125/150/200%及双屏 |
| 安装器/升级/卸载/签名 | NOT_RUN | 本轮非发布；隔离安装和旧资料副本另验 |
| macOS | NOT_RUN | 对应系统和原生后端 |
| Zotero读取/定位/写回 | NOT_IMPLEMENTED/NOT_RUN | 实际版本、隔离profile、测试文献 |
| HarmonyOS压感/倾斜/手掌抑制 | NOT_IMPLEMENTED/NOT_RUN | 固定SDK和真机/笔 |
| Monaco/Theia/Code-OSS/PDF.js集成 | NOT_IMPLEMENTED/NOT_RUN | 固定版平台实验 |
| Codex/Claude协议、权限、Diff | NOT_IMPLEMENTED/NOT_RUN | 固定运行时、安全隔离、用户认证及选中上下文授权 |
| 真实WebDAV服务商认证/并发 | NOT_RUN | 用户测试账户及双端隔离环境 |
| 模型下载/真实准确率/长期性能 | NOT_RUN | 用户主动选择运行时与样本；不新增下载 |

所有人工验收均未由本轮自动脚本代签。建议人工验证截图硬件、Markdown连续保存/退出、旧库升级副本、Zotero标准批注与Harmony压感；后两项仅在对应功能实现后进行。

## 5. 文件、架构和阶段结论

修改文件即上述五份文档；新依赖无，业务/数据库/权限变更无。方案采用原栈增量、同ID资源适配、单一Markdown服务、Zotero进程内插件、只读PDF及私有ink、受限代码/Agent进程。

Step00完成代表审查、方案与实际基线记录完成，不代表全绿验收或Research IDE已实现。用户已追加授权：完成本步骤后实施Step01统一资源模型；先隔离迁移演练再提供真实应用升级方案。Step02及后续仍未授权。

文档最终检查：npm run check:docs退出0，128份文档、756个本地链接；围栏/命令权限/schema11及001–011一致性检查通过。此结果仅为文档检查，不是运行验收。
