# 科研知识卡片Markdown导出验收

2026-10-15，Windows / PowerShell。范围为当前用户要求的单张/多张知识卡片Markdown导出，沿用SQLite v2与原图不可变规则，不增加数据库迁移或依赖。契约见 [架构第12节](ARCHITECTURE.md#12-当前markdown导出2026-10-15)，使用说明见 [README](../README.md#markdown-导出)。

## 修改文件与行为

新增src/features/export下的MarkdownExport、store、selection及测试，shared/api/markdown及测试；修改卡片和素材库的导出入口、选择框、App全局结果提示及styles。Rust新增storage/markdown和commands/markdown，复用knowledge_card_from快照读取；注册main-only导出命令及权限。新增scripts/markdown-native-smoke和测试辅助示例examples/markdown_dialog，package.json增加原生检查命令，check-docs校验新权限。更新README、ARCHITECTURE、ROADMAP和本报告。保留既有未提交工作。

单张入口位于知识卡片；多张入口位于素材库，可跨页勾选、选择本页和清空选择。最多100张，每批正文16 MiB/原图512 MiB。先保存所选卡片脏草稿；失败不弹目录选择框，草稿保留。使用系统文件夹对话框选择目录，客户端只发送ID，不能指定任意输出路径。

新建ResearchClip-Export或递增后缀目录，输出notes.md、assets/screenshot-序号-合法名称.png；有失败/提醒时另写export-report.txt。标题、原Markdown正文、来源、标签和UTC创建时间导出；中文标题保留，图名仅URL安全ASCII。图片链接为相对路径，复制原始PNG字节及透明度，不合成标注或收集正文中外部附件；额外既有笔记保留并提醒未导出。内部存储/目标绝对路径不写进生成Markdown，用户正文不擅自重写。

快照元数据同一读取事务，原图File句柄打开后释放库锁，后续逐张读取、解码、暂存和提交。图源异常仅跳过该卡，不留下断链图片引用；全失败不发布空目录。目标写入异常中止并明确报错。先完成独立暂存，再create_dir预留新目录、sync/禁止覆盖提交完整文件，notes.md最后，成功移除EXPORT-INCOMPLETE.txt。目录整体不是原子事务，失败或中断可能保留带标记目录；不覆盖/删除用户已有文件或重置资料库。

## 实际检查

Node22.22.2/npm10.9.7、Rust MSVC1.99、Tauri2.12.3、已安装官方dialog2.8.2/tempfile3。新API使用前读取安装的dialog源码，并复用现有PNG导出的NamedTempFile/persist_noclobber；没有编造插件API或授予通用FS/对话框权限。

以下均为本轮实际执行结果，退出码0：

| 命令 | 结果 |
| --- | --- |
| npm run check | 最终21个文件157项通过；类型、ESLint、Vite生产构建通过 |
| npm run test:rust | 最终80项通过，含新增10项导出目录、引用、重名、缺图、原图句柄、元数据与失败测试 |
| npm run format:rust | 格式通过 |
| npm run check:rust | 所有目标及测试辅助示例Clippy通过，禁止警告 |
| npm run tauri -- build --debug --no-bundle | Windows debug构建通过，原生验收使用该程序 |
| CARGO_TARGET_DIR=src-tauri/target/markdown-release；npm run desktop:build | 新目录完整Windows release构建通过，Rust release编译1分29秒；不是安装包或签名验收 |
| npm run test:smoke:markdown:native -- markdown-20261015-7 | 实际系统目录对话框/SQLite/文件写入7组通过，errors为空 |
| npm run test:smoke:markdown:native -- markdown-20261015-final | npm命令自动构建必要助手，最终7组通过，errors为空 |
| npm run test:smoke:knowledge:native -- markdown-regression-20261015 | 原有笔记、标签、组合搜索、编辑器往返、保存失败恢复和实际重启9组通过 |
| npm run test:smoke -- --run-id markdown-ui-20261015 | 6组及32种界面/主题/内容视口通过；4种overlay mock DPR不代表真实屏幕捕获 |
| npm run check:docs | 最终21份文档、61个本地链接及命令权限/迁移一致性通过 |

最终证据目录artifacts/markdown-markdown-20261015-final保留report.json、完整Win32控件清单、export-result.png和实际生成的导出包。测试使用3条真实SQLite卡片与明确的320×200受控PNG；在含中文、空格、方括号、#、%的父目录中通过真实系统对话框落盘，逐个解析Markdown相对图片路径、读取PNG、比对SHA-256及实际尺寸。随后移动整个两卡导出目录，所有引用仍可解析；已有单卡导出内容哈希不变。

原生还验证：导出前保存未完成的正文/来源/标签；取消不创建目录；人为删除一张测试原图后部分成功、没有断链且问题报告明确，原SQLite记录仍在；全缺图失败且不发布空目录；客户端额外directory参数无法绕过真实目录选择（取消后没有写入该参数位置），非法ID和通用dialog IPC被拒绝。所有自启动测试应用均已正常关闭，没有重置真实AppData、结束用户应用或删除用户资料。

发布产物为src-tauri/target/markdown-release/release/researchclip.exe。可以直接启动，或运行npm run desktop:dev；先保存并关闭旧版，避免多个实例争用同一工作区。完整功能原生回归在debug程序执行；release在本轮验证了构建，未使用真实资料库进行自动化写入。

## 已发现并处理的问题

早期两次新增Rust测试均79通过/1失败：测试试图经已有限制的save_note写入file来源，被正常拒绝。修正为仅在独立测试数据库中模拟历史异常记录，并固定主/额外笔记时间顺序；不放宽生产URL校验，完整重跑80项通过。

原生1/2的UIAutomation只能枚举对话框顶部控件，不能读取底部确认控件。Win32测试辅助程序首次编译缺少IsWindowEnabled所在模块导入，按安装的windows0.62.2源码修正后编译及Clippy通过。原生3/4未读回有效输入，5发现隐藏兼容控件，因此拒绝确认未验证目录。6显示并聚焦测试拥有的对话框、读取实际Edit1152/确认按钮1、填写并验证路径后，真实导出已完成，但脚本错误按complementary角色寻找已被role=status覆盖的提示，导致超时。改用实际带标签的状态元素，7及final完整通过；未移除业务断言、绕过对话框或添加假的导出响应。所有失败报告保留在artifacts/markdown-markdown-20261015-*。

## 边界与下一步

本轮不实现托盘、合成标注的Markdown模式、AI/OCR或其他导出格式。用户正文原文保留；没有新增HTML预览。macOS、真实磁盘满/ACL/网络盘、断电耐久性、实际Obsidian/VS Code打开、最大100张/512 MiB负载尚未验收。下一步建议用实际科研卡片在目标Markdown编辑器中验收正文和图片，等待新的开发请求后再扩展。
