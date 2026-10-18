# ResearchClip v1.0.0 实际验收报告

日期：2026-10-18。范围：素材页布局、详细图文指南、版本及Windows发布，不增加其他业务功能。

**结论：构建、单元测试、11套原生专项、最终安装包隔离安装/卸载通过；素材库专项因退出时两条无效窗口句柄告警仍为FAIL（QA-001/P2）。不能写成全套验收通过。** 未观察到本轮新增P0/P1。按用户要求提供正式Release，保留已知问题和未验证矩阵；正式标识不等于完整稳定性认证。远程结果在发布后另附回执。

## 环境与数据边界

- Windows11 x64（NT10.0.26200）；Node22.22.2、npm10.9.7、Rust MSVC1.99、Tauri2.12.3、CLI2.12.2，使用仓库锁文件。
- 原生专项使用真实Windows WebView2/SQLite、系统对话框和artifacts下独立资料目录。受控曲线是界面演示，不能当作实验结果或真实屏幕捕获证明。
- schema v3、既有001/002/003迁移、标注JSON及权限不变；无新依赖，没有重置真实AppData、原图或用户数据库。
- 安装器只在独立路径执行，只读比较7份既有资料摘要；没有启动生产应用写入真实资料。用户主动勾选删除数据的破坏性卸载分支未执行。

## 修改文件及架构

| 文件 | 用途 |
| --- | --- |
| src/features/capture/CaptureControls.tsx、CapturePanel.tsx | 紧凑空状态，保留加载/错误/原有截图操作，解释临时资料 |
| src/features/library/LibraryView.tsx、src/features/projects/BulkActions.tsx、src/styles.css | 首屏图片优先、网格/列表留白、零选择批量栏、窄窗和指南排版 |
| src/features/guide/GuideView.tsx | 保留14章，增加三条阅读路线、搜索清除、高亮、10图白名单和原尺寸对话框 |
| src/features/document/MarkdownPreview.tsx、math.test.tsx | 复用安全解析器，仅指南传入图片放大回调；普通文件权限不变 |
| docs/USER_GUIDE.md、docs/images/v100-*.png | 详细步骤/安全提醒及8张受控应用图，加上既有2张共10张 |
| scripts/guide-native-smoke.mjs、scripts/math-native-smoke.mjs | 指南原生检查；常规公式检查不再改写旧指南图，更新须显式参数 |
| scripts/installer-smoke.ps1、src-tauri/examples/knowledge_fault.rs | 修正测试target边界和故障夹具schema3；业务后端不变 |
| package.json、package-lock.json、src-tauri/Cargo.toml、Cargo.lock、tauri.conf.json | 同步版本1.0.0和指南测试入口，依赖版本不变 |
| README.md、CHANGELOG.md、docs/PRD.md、ARCHITECTURE.md、ROADMAP.md、RELEASE-NOTES-1.0.0.md及QA文档 | 范围、用法、发布、实际检查记录 |

## 自动检查与实际命令

最终一轮计数，不累加重跑。[可公开结构化证据](evidence/v100-acceptance.json)；原始日志保留在本地`artifacts/qa-v100/`。

| 命令 | 实际结果 |
| --- | --- |
| npm ci --proxy http://127.0.0.1:7897 | PASS/0，干净安装303包，无依赖升级 |
| npm run check | PASS/0，类型检查、ESLint零警告、Vitest43文件300测试、Vite生产构建 |
| npm run format:rust | PASS/0，cargo fmt检查 |
| npm run test:rust | PASS/0，125测试，0失败、0忽略 |
| npm run check:rust | PASS/0，Clippy all-targets、-D warnings |
| npm run tauri -- build --debug --no-bundle | PASS/0，原生专项使用包含生产前端资源的debug应用，仅隔离目录启用测试夹具 |
| npm run desktop:bundle | PASS/0，最终release EXE与NSIS，包含修正后的完整截图操作图 |

### 实际原生及浏览器专项

直接Node命令均实际执行；其Rust探针先经仓库with-rust.mjs构建。前11行合计**88组PASS**，每组含多个断言，不等于88项独立人工测试。安装、浏览器与捕获探针分列。

| 命令 | 结果 |
| --- | --- |
| node scripts/guide-native-smoke.mjs v100-release --verify-only | PASS/0，5组；真实资料/首屏图片、10图解码、Enter放大/Esc关闭、导航及480/760/1180布局 |
| node scripts/projects-native-smoke.mjs v100-release | PASS/0，11组；有资料v2→v3迁移/备份、项目/事务批量、重启 |
| node scripts/split-native-smoke.mjs v100-release | PASS/0，11组；无文档无分栏、光标插图、分栏变换坐标、保存/失败/关闭 |
| node scripts/document-native-smoke.mjs v100-release | PASS/0，8组；图注关联、合成图、UTF-8/assets、外部改动拒绝覆盖、重开 |
| node scripts/templates-native-smoke.mjs v100-release | PASS/0，10组；持久化、重名保护、大纲及安全边界 |
| node scripts/import-native-smoke.mjs v100-release | PASS/0，9组；真实剪贴板/占用、OLE单多图、损坏/大图、编辑/Markdown/重启 |
| node scripts/compare-native-smoke.mjs v100-release | PASS/0，8组；分割/叠加真实像素、共享变换、异尺寸、交换/释放 |
| node scripts/math-native-smoke.mjs v100-release | PASS/0，8组；离线公式、错误/恶意输入、实时预览、源文和CRLF保存重开 |
| node scripts/editor-native-smoke.mjs v100-release | PASS/0，6组；原有5类矢量、原图不变、全分辨率PNG、独立Edge剪贴板逐像素比较、重启 |
| node scripts/desktop-native-smoke.mjs v100-recheck | PASS/0，10组；真实托盘/快捷键冲突回滚、最小化/单实例、失败阻止退出和资源释放；区域流程CANCELLED不计截图成功 |
| node scripts/markdown-open-with-native-smoke.mjs v100-recheck | PASS/0，2组；新System32/OpenWith.exe真实窗口及取消，未改默认关联 |
| node scripts/library-native-smoke.mjs v100-release | **FAIL/1**；10组资料断言完成，但退出stderr两条0x80070578，最终零错误断言失败 |
| node scripts/material-ui-smoke.mjs v100-release | PASS/0，8组；Edge渲染器480–1440布局、深浅色对比度/分栏，非硬件DPI验收 |
| node scripts/guide-native-smoke.mjs v100-capture-actions --capture-only | PASS/0，1组；实际截图完成操作图等待控件就绪，复制/保存/素材库/编辑器完整可见 |
| .\src-tauri\target\debug\examples\capture_probe.exe 3 | PASS/0；枚举主屏2560×1440及负坐标副屏2560×1600，实际主屏3帧；GDI对象前后均1，无效显示器被拒绝。未捕获副屏；不输出像素 |
| pwsh -NoProfile -File scripts/installer-smoke.ps1 -RunId v100-final -TargetDir src-tauri/target | PASS/0，4组；最终包中文空格路径安装/默认卸载、图标/元数据/载荷、运行中拒绝安装卸载（退出2）、7份资料摘要及无关文件保留、注册清理 |

最终NSIS：`src-tauri/target/release/bundle/nsis/ResearchClip_1.0.0_x64-setup.exe`，6,222,645字节，SHA256 `88bb12d721874b00271919fa51c1767e96e4704a102f5785a8a973bf630d0502`。

独立EXE：`src-tauri/target/release/researchclip.exe`，19,842,560字节，SHA256 `d32064b0ae05d3d4b3fc575dec41beb8dd56d877198737014ceb4c8e0a901716`。均未签名。安装载荷按Tauri预期UNK→NSS标记变化单独核对，不要求与独立EXE字节相同。

## 失败、修正及未解决问题

1. **QA-001/P2/FAIL保留。** 素材库49卡分页/重命名/收藏/损坏缩略图/删除取消与确认/重启10组断言完成，进程正常退出0，但出现两条`PostMessage failed ...0x80070578`。复现：构建debug，运行上述library专项，完成重启和最后正常关闭，检查report.errors及最终断言。出现时脚本退出1；本轮确实出现，不声称每次稳定必现。只读检查Wry/Tao异步PostMessage路径，尚未证明根因，未过滤日志或移除断言。未观察到资料损坏，退出竞态和长稳仍待专项。
2. 指南首轮用textbox定位带datalist输入失败；改getByLabel后完整复验PASS。早期截图操作图裁切不完整，等待控件就绪并正常滚动后重拍、目视核对，最终包包含修正版。
3. desktop首轮7组后因故障夹具误断言schema2失败；只校正测试示例为实际schema3，重跑10组PASS。历史QA-002托盘超时不因本次通过自动关闭。
4. 打开方式首轮系统宿主等待超时FAIL，独立复测真实窗口/取消2组PASS；启动时序及第三方软件仍需人工检查。
5. 安装脚本首轮拒绝标准target根，核对绝对路径后只允许该根或子目录；两次隔离安装/卸载实际PASS。不同执行上下文的预检查曾误判安装受阻，真实安装结果已更正，未绕过已有安装拒绝保护。

初次批量脚本总退出1如实保留。[证据](evidence/v100-acceptance.json)含初始失败与最终结果，本地原始日志未清除。没有通过删除资料、删断言或隐藏异常制造成功。

文档复查：`npm run check:docs` PASS/0，54份Markdown、342个本地链接，权限/schema一致；`git diff --check` PASS/0。此结果仅为文档检查。

## NOT_RUN / Windows人工验收

下列未实际运行；缺硬件/平台时为BLOCKED，历史结果不计本轮通过。

1. 生产EXE完整交互：先保存退出旧实例；查看素材首屏/网格/列表、14章指南及10图放大，缩窗切主题，检查操作行无遮挡。
2. 真正全屏/区域像素矩阵：100/125/150/200%缩放、负坐标副屏/混合DPI、四角/越界/零面积/Esc/连续截图。主屏3帧探针不覆盖这些场景。
3. 物理快捷键、托盘鼠标操作、真实OS主题切换和Explorer重启；自动化部分使用限定PID输入和CDP媒体模拟。
4. 本轮未重跑OCR中文/英文/混合/低清/符号专项、马赛克/模糊/编号专项、贴图专项、长期堆/显存及大库性能；不据单元测试标为原生PASS。OCR既有准确性/语言包限制保留，复杂公式不保证正确。
5. 卡片→标注→无损保存→重开→Markdown图文/公式保存→移动文档文件夹，在Obsidian/Typora/VS Code逐个检查；系统选择器通过不等于各应用打开通过。
6. 交互安装/旧版升级/缺WebView2/SmartScreen、Windows10、其他架构/macOS；不得为验收覆盖既有安装。默认静默安装卸载已测，不代表所有升级场景通过。
7. 更长时间重复退出/素材库开关，复测QA-001/002及CPU/内存/句柄，不在真实资料上执行故障夹具。

本阶段优化和发布准备完成；后续仅按用户指令推进专项。Pro阶段另行只记录约束，不在这里声称高级功能已经实现。
