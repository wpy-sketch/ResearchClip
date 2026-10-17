# ResearchClip 系统性质量验收规范

版本：QA v1；日期：2026-10-15，Asia/Hong_Kong。执行结果另见 [TEST_REPORT](TEST_REPORT.md)。本文件是检查规范，不是功能通过证明，也不授权开发下一阶段。

## 1. 范围与基线

依据 [AGENTS](../../AGENTS.md)、[README](../../README.md)、[PRD](../PRD.md)、[ARCHITECTURE](../ARCHITECTURE.md)、[ROADMAP](../ROADMAP.md) 和实际源码。先验收 Windows x64；macOS 单独建立证据，不用 Windows 结果替代。

本轮为独立 QA：盘点、复验上轮失败、修正已定位的测试设施问题、生成本地 release 构建。新发现的业务 Bug 只记录，等待单独修复指令。不得通过改业务契约、过滤 stderr、减少断言、跳过失败用例来获得绿色结果。既有未提交文件、旧失败报告、用户资料一律保留。

需求冲突按用户最新明确要求、当前实际契约、早期目标设计逐项记录，不默默选一个版本。本仓库 PRD/ROADMAP 含历史目标：回收站、NFKC、标签改名、Markdown 合成图、默认显示器策略等不能因文档中出现就当作实现。具体差异列入报告，未经确认不补做功能。

## 2. 状态、证据与优先级

实现状态单独使用“已实现 / 部分实现 / 未实现”；测试结果只用下列五种状态：

| 状态 | 定义 | 使用规则 |
| --- | --- | --- |
| PASS | 已执行测试并通过 | 必须有命令或操作、退出码/断言、环境与证据位置 |
| FAIL | 已执行测试且失败 | 保留原始失败；复验通过另记一条，不覆盖历史 |
| BLOCKED | 环境或依赖限制导致无法继续 | 写出缺失条件、已尝试动作和解除条件 |
| NOT_IMPLEMENTED | 对应功能尚未实现 | 写明需求来源；不自动当作新功能任务 |
| NOT_RUN | 本轮未实际执行 | 历史通过、代码审阅、Mock 均不能改写本轮状态 |

Mock 单测、纯函数单测、实际 SQLite/文件系统测试、浏览器渲染、原生 WebView2、真实屏幕像素、人工桌面操作、安装验收必须分别说明。原生应用载入明确的测试 PNG 可以验收插件和持久化，不能证明真实截图。合成按键或托盘消息也不等于人工操作系统托盘。

| 优先级 | 定义 |
| --- | --- |
| P0 | 数据丢失、严重安全问题或核心流程完全不可用 |
| P1 | 重要功能失效、稳定复现的崩溃 |
| P2 | 次要功能异常、明显交互或兼容性问题 |
| P3 | 轻微视觉或体验问题 |

执行失败与根因分开：可将用例记为 FAIL，同时将产品 Bug 定为“待归因”。环境故障不能直接判定产品坏了；产品缺陷也不能被笼统归为环境问题。未复现的高风险代码观察须标为静态发现。

## 3. 每个模块的固定流程

1. 找到用户入口、前端组件/store、typed IPC、Rust 命令/服务、存储或平台适配器。
2. 对照实际分支和版本，登记已实现/部分实现/未实现，区分产品目标与当前接口。
3. 检查正常路径、边界、失败提示、取消/重试、并发/过期响应、资源释放和数据保护。
4. 运行现有相关测试；缺少覆盖时提出具体用例。只在已授权的测试修正范围内改脚本。
5. 在可用环境执行集成验证，记录夹具、真实系统能力与人工动作的边界。
6. 列出尚未运行项和需要用户完成的 Windows 桌面操作。
7. 输出结果、可复现步骤、严重程度及是否建议进入下一阶段。模块内某些断言通过，不代表整个模块通过。

每个记录至少包括：用例 ID、需求来源、源码/关键函数、版本/构建哈希、环境、前置条件、输入、操作/命令、预期、实际、退出码、状态、证据、Bug ID、清理和人工项。

## 4. 隔离与资料保护

- 测试放在新的 `artifacts/<suite>-<unique-id>/`；脚本拒绝复用已有目录。数据库和图像夹具只在该目录或临时目录。
- debug 原生测试使用 `RESEARCHCLIP_TEST_SETTINGS_DIR`，同时隔离工作区、配置、单实例标识和 Tauri/WebView 目录。生产 EXE 不支持此调试后门。
- 不删除真实 AppData，不修改其配置，不以重建数据库修复迁移。安装测试对科研资料做只读前后哈希，排除项必须精确列明。
- 测试故障注入仅限自建 SQLite/PNG；原图 hash、旧目标文件 hash、记录数量和 revision 是数据断言。
- UI 自动化只作用于本次启动的 PID；不能为了让按键通过而取消前台 PID 校验、操作用户阅读器或强制结束用户应用。
- 安装/卸载只用新建测试目录；已有正式安装、未知注册信息或运行实例时停止。卸载“删除应用数据”选项不得在用户资料上试验。
- 运行真实截图前取得可控桌面，像素验证使用屏幕上显示的测试图。受遮挡时记录失败，不以预生成 PNG 替换原生捕获。
- 保留失败日志，避免日志记录截图像素、正文和完整科研路径。屏幕图片证据不得上传。

## 5. 分层检查及可执行命令

从项目根目录运行。示例 ID 每次改为新的值；不要删除旧输出以复用 ID。

```powershell
npm run typecheck
npm run lint
npm test
npm run build
# 等价顺序集合；前一步失败会停止，报告要明确后续是否运行
npm run check

npm run format:rust
npm run test:rust
npm run check:rust
npm run check:docs

# 原生测试依赖 debug 应用和验收辅助程序
npm run tauri -- build --debug --no-bundle
node scripts/with-rust.mjs cargo build --manifest-path src-tauri/Cargo.toml --locked --examples

npm run test:smoke:desktop -- --run-id qa-new-capture
npm run test:smoke:editor -- qa-new-renderer
node scripts/workflow-smoke.mjs qa-new-workflow --fixture --native-save-controls
node scripts/editor-native-smoke.mjs qa-new-editor
node scripts/storage-native-smoke.mjs qa-new-storage
node scripts/library-native-smoke.mjs qa-new-library
node scripts/knowledge-native-smoke.mjs qa-new-knowledge
node scripts/markdown-native-smoke.mjs qa-new-markdown
node scripts/desktop-native-smoke.mjs qa-new-desktop

# 本地 release 构建，不发布
$env:CARGO_TARGET_DIR = Join-Path (Get-Location) 'src-tauri/target/release-audit'
npm run desktop:bundle
pwsh -NoProfile -File scripts/installer-smoke.ps1 -RunId qa-new-installer
```

原生/UI/剪贴板/快捷键用例必须串行执行，避免测试争抢焦点和全局资源。Rust 命令使用 `--locked`，Clippy 保持 `--all-targets -- -D warnings`，不修改锁文件来绕过失败。受限沙箱的临时文件/权限故障可以在授权的正常进程环境重试，但必须保留初次结果和区别。

依赖检查：读取 npm/Cargo 锁定版本，核对直接依赖用途，记录 `cargo tree --locked --duplicates` 的重复来源。`npm audit --json --registry=https://registry.npmjs.org` 只证明其当时覆盖的 npm 公告；不能代替 Rust 公告扫描。未执行的 scanner 写 NOT_RUN，不推测零漏洞。不为了最新版本无必要升级。

## 6. 模块用例目录

| ID | 模块与实际入口 | 必测场景与客观预期 | Windows 人工补充 |
| --- | --- | --- | --- |
| CAP-01 | 全屏 / CaptureControls → capture_fullscreen | 枚举、主屏/指定屏、原生九点色块、物理尺寸、自身窗口隐藏、拒绝/BUSY 后恢复、取消不落盘 | 普通桌面与应用窗口内容一致；受保护画面明确限制 |
| CAP-02 | 区域 / CaptureOverlay → capture_region | 冻结后再建层；四向拖动、0面积、越界、Esc、失焦、20轮生命周期、裁切逐像素比对 | 任务栏边界、桌面四角、连续触发、显示器拔插 |
| DPI-01 | capture/geometry + editor/geometry | 100/125/150/200%，混合 DPI、负原点、旋转；边缘误差不超过1物理像素 | 每个真实显示器分别截图；CDP DPR不能替代 |
| KEY-01 | SettingsView → SettingsService | 区域/全屏独立、等价组合冲突、外部占用、旧配置保留、释放旧键、重启/退出注销 | 其他应用前台、主窗口隐藏时用真实键盘触发 |
| CLIP-01 | CapturePanel / EditorView → copy_* | 实际 PNG/RGBA；独立应用读取全部像素；占用报错且保留预览；原图与合成区别 | Paint/Word/其他图像应用 Ctrl+V；透明度 |
| EDIT-01 | EditorView / EditorCanvas / AnnotationNode | 五种工具；唯一ID；选择/移动/缩放/删除/取消；文字编辑；坐标往返；每图独立状态 | 中英文IME、多行文字、不同字体和高DPI手柄 |
| HIST-01 | editor/store + history | 创建/删/改/拖/属性均可撤销；一次手势一次操作；redo失效；100步/8MiB有界 | Ctrl+Z/Y/Shift+Z、文字输入时不误删对象 |
| FILE-01 | editor/serialization + commands/editor | JSON版本/尺寸校验；重开仍可编辑；原图hash不变；未知版本/缺图不覆盖草稿；拒绝同名 | 中文/空格路径、长路径、只读目录、磁盘不足 |
| PNG-01 | renderComposite → export_editor_png | 原分辨率；透明背景/文字/箭头/层级；缩放不改变像素；无手柄/选框；保存取消 | Paint/其他查看器打开导出图 |
| DB-01 | Storage / migrations / files | v1→v2验证备份；未来/坏库拒写；事务回滚；WAL一致；意图恢复/删除队列；重启原图与向量一致 | 独立测试副本上断电/磁盘满/只读/重解析目录矩阵 |
| LIB-01 | LibraryView → screenshot_page | 24条分页、49条基线、1000条性能集；排序/收藏/改名/删除；损坏缩略图单卡占位；退出无错误 | 窗口缩放、清晰度、快速翻页/删除确认 |
| NOTE-01 | KnowledgeCardView → save_knowledge_card | 700ms防抖；切换保留草稿；串行保存/版本冲突；错误可见；来源仅http(s)；标签解除不删图 | 快速编辑切卡后退出/重启；恶意HTML保持纯文本 |
| SEARCH-01 | QuickSearch → search_screenshot_page | 中英、%、_、反斜杠、空查询、多标签AND、收藏、排序组合、清空条件；不加载全量原图 | IME组合输入、响应及时性、大库p95 |
| MD-01 | MarkdownExport → export_knowledge_markdown | 单/多卡、notes.md+assets、相对路径、重名新目录、特殊标题、缺图部分成功、无自动绝对路径 | 移动整个导出目录到Obsidian/VS Code后查看图片 |
| DESK-01 | SettingsView / DesktopService | 三态主题、标注默认值、目录、版本；单托盘/单实例；关闭/最小化/恢复；脏数据阻止退出；释放键 | 实际OS主题、可见及隐藏托盘、Explorer重启、物理点击菜单 |
| SEC-01 | capabilities / commands / CSP | main/overlay隔离；受管ID/路径；无任意SQL/fs；禁远程脚本；失败不打印科研数据 | 结合生产包确认无调试端口及意外联网 |
| PKG-01 | tauri.conf / NSIS hook | 版本/图标/元数据；中文空格路径；运行中拒绝安装卸载且不杀进程；默认卸载保留数据/无关文件 | Windows10/11、交互安装/升级/卸载、缺WebView2、签名/SmartScreen |

所有人工项初始 NOT_RUN。没有对应硬件时列 BLOCKED 并记录硬件缺口；不以口头预计代替测量。

## 7. 缺失覆盖与发布门槛

需补充或执行：混合 DPI/负坐标屏像素矩阵；真正物理快捷键和托盘；1000卡/4K预热5次后50次p95；生产包端到端；升级与干净Windows10/11；运行中WAL备份恢复；所有文件/SQL提交节点崩溃恢复；系统主题切换；Explorer重启。按实际缺口单独安排，不在本轮扩展业务。

进入下一轮模块检查：规范、入口映射和隔离条件齐全即可；进入发布验收：类型/编译/测试/构建通过且已知失败有处置；对外正式发布：未解决P0/P1为零，核心原生用例通过，P2有明确处置，必要人工矩阵与安装/数据保留证据齐全。构建成功或拿到EXE不等于发布通过。

新 Bug 修复需单独指令；修复后先重跑原复现，再做受影响回归。当前请求未授权远程push、创建Release、覆盖已发布内容或实现ROADMAP下一阶段。

## 8. 统一检查记录模板

```text
模块 / 用例 ID：
需求来源 / 实现状态：
涉及文件 / 关键函数：
测试版本 / 环境 / 夹具：
实际执行的命令或动作 / 退出码：
PASS 项及证据：
FAIL 项 / 预期与实际 / 最短复现步骤：
BLOCKED / NOT_IMPLEMENTED / NOT_RUN 项：
需要 Windows 人工验收的内容：
Bug ID / P0-P3 / 已确认或待归因：
用户数据与清理结果：
是否建议下一阶段 / 条件：
```

## 桌面贴图补充用例（2026-10-17）

按相同五态记录。入口和最短执行步骤见[贴图验收](../PINS-VALIDATION.md)。自动检查包括真实独立 HWND/全客户区、置顶/透明度样式、单张和多张、滚轮上下限、物理拖动、屏幕外恢复、二进制 PNG、跨应用剪贴板及占用失败、十次开关归零、主窗口隐藏/取消退出/真正退出。不得以窗口样式位单独判断系统标题栏，应核对实际客户区。固定 PNG 只证明窗口/导出/剪贴板流程，不证明真实屏幕采集；图像预算计数归零不等于长期 OS 堆/显存泄漏检查通过。真实截图、高 DPI/多屏和人工图像应用矩阵分别验收。


## OCR 增量测试要求（v0.1.3）

对照 `services/ocr.rs`、三个受限 OCR 命令、`features/ocr` 和 Markdown insertText：分别运行纯坐标/裁切/协议/状态测试、真实 Windows.Media.Ocr 控制图片测试、原生 UI/剪贴板/Markdown 文件联动。覆盖中文、英文、混合、空图、低清、符号、连续图片和语言缺失；关键词通过与逐字准确性分开报告。不能用 Mock 识别结果或控制 PNG 代替真实屏幕捕获及硬件 DPI。全部使用隔离目录，不卸载工作机语言包、不重置 AppData。实际命令和人工矩阵见 [OCR 验收](RELEASE-0.1.3.md)。
