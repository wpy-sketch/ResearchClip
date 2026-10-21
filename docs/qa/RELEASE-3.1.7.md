# ResearchClip v3.1.7 / A11 验收

本轮范围：图片点/区域锚点、Markdown稳定ID引用与原像素区域定位，优化编辑器/指南目录，并发布含必要配套附件的v3.1.7。沿用模块化单体和已有依赖，无新第三方依赖或系统深度链接注册。数据契约见[图片锚点](../ANNOTATION-ANCHORS.md)。

## 已实现与架构

新增storage/anchors、commands/anchors、迁移010和features/anchors；复用编辑器会话、Konva viewport、Markdown insertText与A10链接索引/导出。截图删除级联锚点，链接保留失效目标。原图SHA绑定独立于标注版本，SQLite升至10，旧版应用须按未来schema保护拒绝写入。

指南17章按目录/正文从上到下连续编号，94个功能示例；侧栏分组顺序与章节物理顺序一致。编辑器窄分栏的导出工具水平滚动，锚点列表可展开，留出画布区域。

## 实际检查（Windows，Node 22.22.2 / Rust 1.99）

| 检查 | 本轮结果 |
| --- | --- |
| `npm run check` | 退出0；60文件403测试，TypeScript、ESLint、生产构建通过 |
| `npm run format:rust` / `npm run check:rust` | 均退出0；严格Clippy全部targets |
| `npm run test:rust`（单线程） | 退出0；200通过、0失败、7忽略；忽略项不算运行验收 |
| 隔离离线 `npm ci` | 退出0；303包，依赖版本/完整性未变，无新依赖 |
| `python scripts/test-release-companions.py` | 退出0；3项，附件哈希/ZIP安全校验 |
| `node scripts/links-worker-smoke.mjs` | 退出0；2组生产Worker检查 |
| `node scripts/anchors-native-smoke.mjs a11-confirm` | 退出0；11组，原生画布/保存/导出/跳转/失效/重启 |
| `node scripts/links-native-smoke.mjs v317-regression` | 退出0；5组，A10循环/重命名/反向链接/图谱/导出/重开 |
| `node scripts/references-native-smoke.mjs v317-regression` | 退出0；7组，含实际Crossref查询 |
| `node scripts/guide-native-smoke.mjs v317-final --verify-only` | 退出0；6组，94示例、17章、30图真实解码及原生保存 |
| `node scripts/guide-reader-smoke.mjs v317-final` | 退出0；11组，明暗主题、480–1440七种宽度和600px高度 |
| `node scripts/clipper-native-smoke.mjs v317-packaged --packaged-host` | 退出0；7组，实际已发布Host配合本版桌面端，未修改真实浏览器注册 |
| `npm run tauri -- build --debug --no-bundle` / `npm run desktop:bundle` | 均退出0；最终静态前端和Windows NSIS |
| `& .\scripts\installer-smoke.ps1 -RunId v317-final -TargetDir src-tauri/target` | 退出0；4组，安装/忙碌拦截/卸载，13个既有用户数据文件哈希未变 |

最终安装包SHA256：`99ffa9faa269cb83ddc841b4d800ed30dd63f2f53fed54851f3a21f475fa0794`。机器可读摘要见[检查记录](evidence/v317-checks.json)。文档链接/围栏与权限、schema一致性由`npm run check:docs`检查；该检查不替代原生运行。

### 失败记录与复验

- 首轮完整Rust有3个历史迁移夹具失败：模拟旧schema时未移除新表、旧版本断言未更新；修正夹具后完整200项通过。新增锚点存储4项和锚点索引/可读导出测试已纳入最终检查。
- 初次编译使用了未安装uuid名字空间；改用已有SQLite randomblob生成ID，无新增依赖。测试夹具调用签名与已有存储API不符已修正，未修改正式接口。
- 原生首轮通过真实画布点/区域创建、拖动与缩放；测试调用系统保存辅助程序过早而失败。改为等待匹配的原生对话框出现，保留失败证据；第二轮实际插入与保存通过，后续因测试把普通带aria-label的div误认为region而未找到预览按钮，已修正定位器。均未删除业务断言或伪造运行结果。
- 最后自查修正锚点选择器在React StrictMode effect重放时的取消标记初始化，并重建最终程序。`a11-last`在二次Markdown保存时出现一次文件操作错误，磁盘旧文件和界面草稿均保留；根因未确定。未改断言的独立`a11-confirm`全部11组通过，先前`a11-portable`/`a11-final`也各11组通过。此偶发文件错误保留为已知风险，不能称为已修复；遇到时应保留草稿重试或另存新文件。

## Windows人工验收与已知边界

原生自动化仅使用隔离PNG、SQLite、文件选择器和WebView2，不等于物理屏幕/DPI或人工科研材料验收。完整高DPI/多显示器、macOS、全部第三方编辑器/浏览器、真实第三方WebDAV两设备矩阵未在本版完成人工验证。锚点仅本机，不随WebDAV、旧标注JSON或PNG迁移。现有未签名及大chunk警告继续记录，不因版本发布消失。

## 发布结果

[v3.1.7 Release](https://github.com/wpy-sketch/ResearchClip/releases/tag/v3.1.7)已公开，10个附件全部经匿名下载SHA256核验；25个历史Release和176个历史附件元数据/哈希未变。独立GitHub读取确认latest=v3.1.7，draft=false。见[发布回执](evidence/v317-publication.json)。离线指南ZIP的42个成员CRC与相对链接校验通过。
