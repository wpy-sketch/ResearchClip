# v0.1.3 OCR 实现与验收

2026-10-17。本轮先完成 v0.1.2 发布回执推送，再增量实现离线 OCR 和 v0.1.3 发布；不开发其他新功能，不迁移或重置 SQLite，不操作真实 AppData。

## 方案及实际环境

Windows NT 10.0.26200 x64、Node 22.22.2、npm 10.9.7、Rust MSVC 1.99、Tauri 2.12.3、windows-rs 0.62.2。直接复用已锁定的 windows crate，仅启用 WinRT Globalization、Graphics.Imaging、Media.Ocr、Storage.Streams、Foundation 和 RoInitialize 绑定；不升级库、不增加 OCR 依赖或打包模型，沿用 MIT/Apache-2.0 许可。

核对 [Microsoft OcrEngine](https://learn.microsoft.com/en-us/uwp/api/windows.media.ocr.ocrengine)、[Microsoft windows-rs 绑定](https://microsoft.github.io/windows-docs-rs/doc/windows/Media/Ocr/struct.OcrEngine.html)、[语言可选组件](https://learn.microsoft.com/en-us/windows-hardware/manufacture/desktop/features-on-demand-language-fod?view=windows-11) 和安装的 0.62.2 源码，再编译原生探针；没有根据文档存在就判定运行通过。

实际未打包的 Rust EXE 可调用引擎，探针输出 `zh-Hans-CN 简体中文(中国大陆)`、`maxDimension=10000`。未发现 en-US 语言包；没有更改或安装系统语言组件。API 声明 Windows 10 起可用，本轮实际环境仅上述 Windows 11，不能承诺其他构建/语言配置均支持。识别已有图片不需要屏幕录制、磁盘全局访问或网络权限；原生屏幕捕获权限沿用原模块。

## 入口、状态及安全

| 部分 | 实际文件与关键入口 |
| --- | --- |
| UI 入口 | `CapturePanel.tsx`、`EditorView.tsx` → `OcrButton`；编辑器复用 `renderComposite` |
| 面板与选区 | `features/ocr/OcrPanel.tsx`、`model.ts::regionFromDrag`，显示图像矩形转换为原图整数像素，反向拖动/边界夹取/零面积处理 |
| 状态 | `features/ocr/store.ts`：preparing/recognizing/copying/idle；单活动图片、临时结果，关闭释放 URL；忙时不替换来源或关闭 |
| IPC | `commands/ocr.rs`：ocr_status、recognize_ocr、copy_ocr_text；仅 main，PNG 二进制与小 JSON 头，无用户路径或任意 SQL |
| 服务 | `services/ocr.rs::OcrService`：跨平台边界，Windows 后端；macOS 返回明确未实现错误 |
| 生命周期 | spawn_blocking 前取得既有 CapturePermit，限制为一项系统图片工作；退出沿用已有有界等待，任务结束释放门控；WinRT apartment RAII，关闭 SoftwareBitmap/DataWriter |
| Markdown | `document/store.ts::insertText` 使用现有会话与光标；源文支持撤销/重做，图文编排保留图片块关联并追加文本；失败保留原文 |

复用受控 PNG 解码和 Rust 原像素裁切。上传受现有约 65 MiB、16,777,216 像素上限约束；选区受引擎实际单边上限约束，不悄悄降采样。透明像素在临时 BGRA8 缓冲中合成白底，原图不变。结果上限 1 MiB，无日志正文/像素输出。没有通用剪贴板读取权限，复制由官方插件的受限 Rust wrapper 完成。overlay/pin 窗口未获得 OCR capability。

OCR 结果必须人工触发复制/插入，绝不自动写回笔记。重新识别覆盖手动修改前再次确认；面板说明关闭前须复制或插入。OS 识别调用运行于后台，忙时不能关闭该面板；没有声称已中止仍运行的本机任务。若系统引擎长时间无响应，需等待完成后重试退出；本轮未验证 OS OCR 永久挂起故障。

## 已执行检查

| 实际命令 | 结果与边界 |
| --- | --- |
| `cargo run ... --locked --example ocr_probe`（经 with-rust） | PASS/0：真实引擎枚举，简体中文包、单边 10000 |
| `npm run check` | PASS/0：32 文件、229 Vitest 测试，TypeScript、ESLint、Vite；最后改动后复验另记 |
| `npm run test:rust` | PASS/0：106 项，0 失败/忽略；包含 4 项新增 OCR 核心测试 |
| `npm run check:rust` | 首次 FAIL/101：Clippy 要求 as_chunks_mut；修正后 PASS/0，全 targets、-D warnings |
| `npm run format:rust` | PASS/0 |
| `node scripts/ocr-engine-smoke.mjs v013-clean` | PASS/0：6 类真实图片、明确关键词断言及空图断言；另有 5 次连续探针调用。不是完美转录通过，详见准确性记录 |
| `node scripts/with-rust.mjs tauri build --debug --no-bundle` | PASS/0：真实 Windows debug EXE，修复 UI 后重建 |
| `node scripts/ocr-native-smoke.mjs v013-initial ocr-engine-v013-clean` | 首次 FAIL/1，前 3 组通过；提示导致面板重新居中、图像坐标移动，后续选区未建立 |
| `node scripts/ocr-native-smoke.mjs v013-fixed ocr-engine-v013-clean` | 修正后 PASS/0：8 组，真实 Windows 引擎、鼠标选区、SQLite 原图、独立 Edge 剪贴板、系统 Markdown 保存 |
| `npm run check:docs` | PASS/0（新增本报告前 36 文档/206 链接；最终数量另记） |

测试状态/IPC 单测使用 Mock，专门标为单元级；Windows 验收使用真实 WinRT 和隔离 SQLite/文件/剪贴板，不 Mock OCR 输出。控制图片由本机 Edge canvas 生成，不含用户资料，不证明真实屏幕读取。

初次 UI 复现：识别完成 → 框选模式 → 单击零面积选区 → 再从原图左上角拖动。提示消失改变居中弹窗高度，图片随之移动，选区起点可能落到说明文字。P2，本轮修正：固定 OCR 面板外框高度、内容内部滚动；保留原失败，回归新增“提示变化前后图片位置不变”断言并通过。

## 识别质量与未通过项

实际中文样例及中英混合的关键词/换行通过；空白图返回空串。英文样例关键词通过，但 `Optical` 识别为 `OpticaI`，`for` 出现 `f0 r`，因此**逐字准确性不通过（FAIL）**，没有包装为无错英文识别。中文引擎替代专用英文包仅是可尝试路径；独立英文引擎验收 **BLOCKED：本机没有该语言包**。

10px 小字样例只能识别部分字符，准确性 **FAIL**；特殊符号样例中 α、β、∑、√、∞ 识别错误，± 成功，符号完整性 **FAIL**。它们是已知 OCR 限制，无静默入库：必须人工校对、主动复制/插入。复杂公式、表格结构、多栏阅读顺序 **NOT_RUN/不承诺支持**，不生成 LaTeX 公式。

没有语言包的完整系统环境、Windows 10、额外语言包、125/150/200% 真实 DPI、多屏/混合 DPI、生产 EXE 全套手工操作、macOS、NSIS 安装卸载、长时间内存稳定性 **NOT_RUN**。既有 QA-001/QA-002 仍见历史 [测试报告](TEST_REPORT.md)，没有在本轮宣布修复。

## Windows 人工验收清单（本表均 NOT_RUN）

1. 用真实全屏/区域截图识别论文中的中英文自然语言；确认屏幕捕获和 OCR 各自正常，不用自动化控制图替代。
2. 在素材库打开旧截图，调整编辑器缩放/平移、添加箭头/文字，再识别整图及局部；检查选区与原图位置一致。
3. 在 125%、150%、200% 及混合 DPI 显示器拖选；检查零面积、反向拖动、越界、窄窗口滚动和 Esc。
4. 在 Windows 语言选项安装英文 OCR 后重开面板，确认列表及真实英文识别；在没有 OCR 包的测试机器核验安装提示，不卸载工作机组件来测试。
5. 编辑结果，在记事本/Obsidian 实际粘贴；在 Markdown 任意光标/选择处插入、撤销、保存、关闭重开，确认中文和换行。
6. 用低清、小字、数学公式测试，确认不自动写笔记；对错字和公式人工校对。每次关闭结果前先复制或插入。
7. 连续换多张图片识别，忙时重复点击/尝试退出，观察资源和错误；长稳/OS 引擎挂起仍需专项验收。

## 最终构建与发布

干净安装、既有原生回归、生产构建与 GitHub 发布结果：执行后补记，不将待执行项写为通过。用户已授权创建 v0.1.3 非预发布 Release；保留旧版本和附件，禁止覆盖已有发布或用户资料。


## 最终实际结果补记

- `npm ci --offline` PASS/0：294 包，锁文件干净安装，npm 报告 0 已知漏洞（不等同完整安全审计）。
- `npm run check` PASS/0：32 文件、229 项，类型、ESLint、生产前端均通过；此轮包含面板固定及插入失败提示的最终源码。
- `npm run desktop:build` PASS/0：Windows release EXE，16,224,768 字节，FileVersion/ProductVersion 均 0.1.3，SHA256 `76c82c8b6b1bbe41e41c30deafdecc89157dfd5ed99bcfd62472c31ddd7ed85f`。没有捆绑 OCR 模型；相对 0.1.2 EXE 增加 254,976 字节。
- `node scripts/editor-native-smoke.mjs ocr-v013` PASS/0：[6 组证据](evidence/v013-editor-native.json)，5 种标注、原图不变、实际文件保存/重开/重启、合成 PNG 和跨应用图片剪贴板。
- `node scripts/split-native-smoke.mjs ocr-v013` PASS/0：[11 组证据](evidence/v013-split-native.json)，包含左右缩放坐标、文件安全、SQLite、480×400、重启和面板生命周期。
- `npm run test:smoke:pins:native` PASS/0：[13 组证据](evidence/v013-pins-native.json)，真实多窗口、拖动、置顶、透明度、保存、剪贴板、10 次开关与退出。
- OCR [6 类真实引擎结果](evidence/v013-ocr-engine.json)、[初始 UI 失败](evidence/v013-ocr-initial-failure.json)、[8 组修复复验](evidence/v013-ocr-fixed.json) 均保留。
- 扩展同进程多图片/ACL 检查时，测试夹具直接等待销毁中 WebView 的 IPC 回执而收到 page closed：[夹具失败](evidence/v013-ocr-harness-failure.json)。改用实际关闭按钮、等待窗口销毁，并明确断言 ACL 拒绝消息；未改业务代码或放宽产品断言。
- `node scripts/ocr-native-smoke.mjs v013-final-fixed ocr-engine-v013-clean` PASS/0：[最终 10 组证据](evidence/v013-ocr-final.json)，新增**同一原生进程内七次不同图片识别**、空图后重识别及贴图窗口 OCR 权限实际拒绝。原 UI、原图和保存断言全部保留。
- 最终 `npm run lint` PASS/0；`npm run check:docs` PASS/0：38 文档、212 本地链接（发布补记若新增链接，将再次检查）。

本轮新增业务文件：`src/features/ocr/{model,api,store,OcrButton,OcrPanel}`（TS/TSX）、`src-tauri/src/services/ocr.rs`、`src-tauri/src/commands/ocr.rs`；接入 App、CapturePanel、EditorView、document/store、CSS、lib/build/mod/capability。新增 OCR model/store 测试、document 插入测试、原生探针和两项 OCR 专项脚本。版本/锁文件、README、CHANGELOG、架构和本轮发布说明同步更新；本报告与八份结果证据提交源码。迁移文件、数据库 schema、原始截图均未改变。

阶段建议：本轮 OCR 工作流的可执行检查已通过，可按用户授权发布 0.1.3；识别质量限制与 NOT_RUN 项必须随发布保留，不表示完整软件/硬件验收完成。
