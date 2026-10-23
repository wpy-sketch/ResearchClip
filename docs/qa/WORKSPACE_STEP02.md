# Step02 验收记录

环境：Windows x64、Node22、React19、Tauri2/WebView2。所有运行只用新建artifacts测试目录，未重置真实AppData。

## 需求与实现

已实现多标签、水平/垂直拆分、指针调节比例、关闭/隐藏/重开、分项目工作区布局、独立文档和图片会话、类型注册、资源管理器/终端区域显示切换、脏检查及窗口变化重排。未打开Markdown时不显示空白Markdown分栏。

新增依赖：flexlayout-react 0.11.1，MIT，无运行依赖。未升级原有依赖。SQLite保持12，无本阶段迁移。

## 实际检查

- npm ci --ignore-scripts：退出0，固定锁文件干净安装；npm run check：退出0，66文件454测试通过，类型/lint/生产构建通过（check-final4.log）。
- npm run format:rust：退出0；npm run test:rust：退出0，230通过/7忽略；npm run check:rust：退出0。最终Rust运行使用已缓存锁定依赖CARGO_NET_OFFLINE=true。
- node scripts/guide-reader-smoke.mjs v411-reader1：退出0，13组浏览器检查，19章118示例31张离线图片，两主题/字号12/16/20/宽480–1440/内容缩放75–200%。不是Windows DPI证明。
- node scripts/workspace-native-smoke.mjs v411-acceptance：退出0，7组真实Windows WebView2检查：双文档文件读写、取消关闭/隐藏、拆分拖动与显示切换/字号/窗口变化、Win32最大化最小化还原、进程重启与按稳定ID重新打开、实际Konva矩形绘制/撤销/重做/保存以及19章离线指南。使用新的v11测试库升级到12。
- node scripts/resources-native-smoke.mjs v411-image-fixed：退出0，6组旧截图/Markdown/引用/原生Konva/重启/只读审计通过，原图与原文哈希不变。
- npm run check:docs：退出0，137文档/787本地链接（后续文档增加后需重跑）。

开发失败均保留：升版批量替换误改依赖字段已恢复原锁值；并行安装争用CLI文件已顺序重装成功；早期原生脚本状态选择器不唯一、将未访问标签视作已渲染面板、错误probe动作名与SW_RESTORE状态预期已按实际API修正。未修改业务断言绕过问题。原生发现隐藏终端残留空组已修复并增加单测；批量组关闭被拦截以保留逐份草稿检查。额外真实回归发现并修正图片选择器初始化顺序、重复导航未聚焦工作台；增加IO计数及脏面板关闭保护与作用域键盘检查。新增绘制脚本仅使用32×24测试PNG，不能代表物理截图。失败日志在artifacts/dock-evaluation，不作为通过证据。

- 最终NSIS构建与隔离安装/卸载检查退出0，4组通过；安装目录许可证原文SHA匹配，用户AppData哈希保持，未残留安装。见[发布记录](RELEASE-4.1.1.md)。

## Windows人工验收（未执行）

1. 在自己的练习资料中打开两份Markdown和图片，切换、拖标签、拆分、调比例，确认正文/撤销和图片标注独立。
2. 修改一份文档，分别取消关闭、取消系统保存、模拟文件只读保存失败，确认草稿保留；在隐藏/其他工作区有草稿时退出，确认逐份提示。
3. 保存文档与布局、重启；先确认没有空白Markdown分栏，再明确重新打开上次资源。移动文件后使用原关联流程重连。
4. 检查两个主题、12/16/20px、Windows100/125/150/200% DPI、跨屏、最小化/最大化/恢复，确认文本输入、指针坐标、图片画布和停靠布局。
5. 回归真实截图、OCR、WebClipper、MathOCR、离线搜索；浏览器或生成图片不能证明实际截图与硬件通过。

## 已知限制与失败回退

最多同时3份Markdown，包含隐藏与其他工作区。未保存草稿不在重启恢复范围；PDF/代码仅类型注册、终端仅布局区域。布局损坏或未知版本保留记录并停止写入；文件/数据库不改动。macOS、HarmonyOS、Zotero真实环境未验证。

历史advanced-native附加回归存在退出超时，见[4.1.0报告](RELEASE-4.1.0.md)；不能将该历史套件宣称为本阶段通过。本轮原截图/Markdown独立原生回归已6组通过，不能替代历史附加套件。

主要文件：src/features/workspace/*；document/store与作用域调用组件；editor画布和会话；desktop退出检查；projects按token关联缓存；App、指南、锁文件、测试脚本和本文档。
