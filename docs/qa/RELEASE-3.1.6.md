# ResearchClip v3.1.6 验收记录

环境：2026-10-21，Windows x64、Node22、Rust/Tauri2/WebView2。仅使用隔离测试资料，不重置用户数据库。

## 变更与架构

- 修改GuideView与guide/navigation：操作/示例目录共享同一份Markdown源文，按解析器位置跳转；84个示例覆盖全部16章，新增筛选与空结果回归。
- styles.css：侧栏独立滚动、工具按钮形状、深浅色背景层次、正文行宽与标题层级；保留原有业务入口与分栏。
- USER_GUIDE、examples、组件清单、发布脚本与版本清单：随Release补齐兼容插件、公式OCR脚本、模型索引及离线指南。
- 无新增第三方依赖，无SQLite/schema/IPC/原图存储格式变更，沿用schema9与现有安全边界。

## 检查记录

| 实际执行 | 退出码/结果 | 覆盖 |
| --- | --- | --- |
| npm run check | 0 / PASS | TypeScript、ESLint、396测试/58文件、Vite生产构建；保留既有大chunk警告 |
| npm run format:rust / check:rust | 各0 / PASS | fmt及all-targets严格Clippy |
| npm run test:rust（正常Windows权限、串行） | 0 / PASS | 195通过、0失败、7原有WebDAV独立集成用例忽略 |
| 隔离npm ci --ignore-scripts --offline | 0 / PASS | 303包；锁文件只改应用根版本，依赖版本/下载地址/摘要不变 |
| guide-reader-smoke v316-final | 0 / PASS | 11组，16章84示例、搜索/空结果/定位、29图解码、键盘放大、深浅色7宽度、草稿保护、600高窗口侧栏 |
| guide-native-smoke v316-final --verify-only | 0 / PASS | 6组，真实WebView2/SQLite/系统保存、素材/标注/卡片/对比/Markdown、示例目录及图片阅读 |
| links-worker-smoke | 0 / PASS | 2组，生产Worker无DOM解析与超限失败 |
| links-native-smoke v316-regression | 0 / PASS | 5组，稳定ID/双向链接、中文emoji位置、重命名、导出、循环、图谱筛选/平移缩放、删除、重启与原图摘要 |
| references-native-smoke v316-regression | 0 / PASS | 7组，文献/BibTeX/关联/引用、真实选择器与重启；实际Crossref返回Measured measurement / Markus Aspelmeyer / 2009 |
| similarity-native-smoke v316-regression | 0 / PASS | 6组，38张测试图、重复组/近似检索/分页/范围、取消/损坏图片隔离/重启；未启用--ai |
| npm --prefix browser-extension run check | 0 / PASS | 5测试、类型与生产构建 |
| npm run test:clipper:browser | 0 / PASS | 5组，真实Edge DOM及生产提取脚本；不声称工具栏activeTab授权链 |
| clipper-native-smoke v316-packaged --packaged-host | 0 / PASS | 7组，发布ZIP内原版Host到本版桌面、实际SQLite/assets、幂等、脏文档确认与重启；仅测试进程APPDATA指向隔离目录，不注册真实Host |
| installer-smoke -RunId v316-pwsh -TargetDir src-tauri/target（PowerShell7） | 0 / PASS | 4组，真实NSIS安装/默认卸载、忙碌拒绝各exit2、版本3.1.6、13个既有资料文件摘要不变；已卸载；NotSigned |
| npm run desktop:bundle / tauri build --debug --no-bundle | 各0 / PASS | Windows优化EXE/NSIS及打包debug静态前端；生产程序未启动读写真实用户库 |
| python scripts/test-release-companions.py | 0 / PASS | 3项，摘要/缺失/ZIP成员安全边界；配套包独立校验CRC与历史SHA256 |
| 配套源码一致性检查 | 0 / PASS | MathOCR ZIP五份运行脚本/锁定文件与当前源码相同；WebClipper ZIP五份dist生产文件与本次构建逐字节相同 |
| npm run check:docs | 0 / PASS | 当次111文档623本地链接，版本/权限/schema9及已发布迁移不可变性 |

[机器可读汇总](evidence/v316-checks.json)。原生测试使用打包debug程序和隔离资料，不把生成的练习曲线当作实际屏幕捕获或真实实验数据。安装测试核对优化载荷，未将正式应用启动到真实资料库。

## 中间失败与修复

- 浏览器目录回归初次因测试定位器对“功能示例”匹配33个按钮而失败；限定到目录内容切换组后通过，未移除断言。
- 沙箱原生指南启动时WebView2未创建主页面，超时；正常Windows权限复验及最终6组均通过。
- 配套ZIP测试最初假设Windows zipfile保留反斜杠，实际会规范化；改用跨平台ZIP路径规则并单独测试原始成员名，保留路径穿越、绝对路径及反斜杠拒绝断言，3项通过。
- 文档检查最初发现写入工具添加多余文件尾空行；规范后重跑通过。
- 安装检查首轮误用Windows PowerShell5.1，缺少`$IsWindows`，在安装前置检查停止，没有发生安装；改用项目当前PowerShell7执行后完整4组通过。

## 附件与发布

本版准备10个附件：EXE、NSIS、WebClipper ZIP、MathOCR Setup ZIP、离线指南ZIP、Qwen模型清单、模型说明、离线资源索引、源码提交说明、SHA256。配套版本和许可证保留；Qwen大资源沿用v3.1.2，索引明确25卷和32运输片的原始URL/大小/摘要。发布脚本要求组件齐全、安装器摘要与实际验收一致，先创建草稿，全部上传核对后才正式发布，再逐件匿名下载核对。发布前后对[历史快照](evidence/v316-prior-releases.json)中的24个Release及附件元数据做严格比较；不覆盖旧版本。

正式发布及匿名下载结果将在完成上传后另行记录，不把本段准备流程作为已发布证明。

## Windows人工验收与边界

自动化原生测试和安装测试结果单独记录，不称为人工硬件验收。完整物理高DPI/多显示器、macOS、所有网页工具栏activeTab点击链、真实第三方WebDAV两设备矩阵未在本版完成人工验证。旧图仍按原版本标记；新指南截图标明实际来源。Qwen可选大资源沿用v3.1.2；发布元数据复核不代替重新下载所有旧数GB文件。
