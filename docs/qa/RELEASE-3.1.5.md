# ResearchClip v3.1.5 / A10 验收记录

环境：2026-10-21，Windows x64、Node22、本机Rust/Tauri2/WebView2。测试仅使用tempdir与artifacts隔离数据。

## 修改与架构

新增features/links（解析、worker、候选、反向链接、SVG图谱、IPC契约）；storage/links与commands/links；迁移009。复用Markdown状态、DocumentFiles、项目和截图标签服务。现有单体及状态框架保持，无新第三方依赖。修改Sidebar、Toolbar、App、MarkdownPanel/Preview、KnowledgeCardView、版本清单、指南及验收脚本。完整契约见[内部链接](../INTERNAL-LINKS.md)。

## 本轮实际中间结果

- 前端第一轮check：57文件389测试、类型、ESLint、Vite构建通过，退出0，保留大chunk警告。
- Rust第一轮：190通过、3失败、7忽略；2个旧库测试夹具/版本断言需随schema9更新，1个系统凭据测试在沙箱失败。未删除断言。
- 正常Windows权限、串行Rust复验：195通过、0失败、7原有忽略，退出0。
- 第一轮原生A10：FAIL，超时未产生后台链接索引，正在定位；不能以单元测试或可编译替代原生成功。

## 最终检查

| 检查 | 结果 | 本轮实际值 |
| --- | --- | --- |
| npm run check | PASS / 0 | TypeScript、ESLint、58文件394测试、Vite；保留既有大chunk警告 |
| npm run test:rust（正常Windows权限、串行） | PASS / 0 | 195通过、0失败、7原有忽略 |
| npm run check:rust | PASS / 0 | all-targets严格Clippy |
| npm run format:rust | PASS / 0 | 格式检查 |
| 隔离npm ci --ignore-scripts --offline | PASS / 0 | 303包；与v3.1.4的依赖版本、下载地址、integrity完全相同 |
| links-worker-smoke | PASS / 0 | 生产Worker无DOM解析与超限失败，2组 |
| links-native-smoke a10-final | PASS / 0 | 最终schema9/闭合wiki输入/真实Worker/选择器/SQLite/重启，5组 |
| guide-native-smoke v315-final --verify-only | PASS / 0 | 原有素材、标注、卡片、对比、Markdown和16章29图，5组 |
| guide-reader-smoke v315-final-reader | PASS / 0 | 9组；深浅色7种宽度、目录、键盘与图片放大 |
| npm run check:docs | PASS / 0 | 当前106文档、612本地链接，版本与权限、001–008不可变性检查 |

[机器可读证据](evidence/v315-checks.json)。实际Windows是打包debug程序的自动化回归；没有用浏览器或Mock替代原生结果。

优化构建首轮FAIL：旧researchclip.exe属于此前CodexSandboxOffline账户，当前沙箱替换被拒绝（os error5）。正常Windows权限检查无运行中的researchclip进程，不终止用户应用、不改ACL；正常权限重建已通过，退出0；优化EXE与8.33MiB NSIS已生成。


## Windows人工验收与已知边界

本轮自动化原生回归单独记录，不代替人工科研资料审阅或物理硬件矩阵。完整高DPI、多显示器、macOS、跨设备ID映射和所有第三方Markdown软件均未验证。文档映射/索引/文档标签仅本机；外部重命名不自动跟踪。v3.1.4已恢复网络并完成源码、标签、4附件及匿名哈希验证；v3.1.5待最终构建后上传。用户已允许网络恢复前先开发A10。无PDF及独立备份恢复系统。


## 原生问题修复与回归

- Worker首次失败：依赖包浏览器导出使用document，生产Worker中不可用；改为已安装包的纯实现，生产Worker无DOM回归2组通过，无新依赖。
- 点击首次失败：源文失焦更新导致Markdown内联渲染组件替换，鼠标按下和抬起间目标消失；稳定渲染组件和回调身份后，真实预览点击与反向跳转通过。
- 导出初次原生测试失败：新选择器缺少窗口parent且旧测试辅助程序只识别保存对话框。已补父窗口和既有操作门控，测试辅助程序增加专用标题且保持artifacts路径限制。
- `node scripts/links-native-smoke.mjs a10-export-fixed --guide`：退出0，5组实际Windows/WebView2/SQLite/原生选择器、Worker与重启通过；验证中英文/emoji位置、截图引用、A↔B与自引、重命名、导出、失效、筛选、平移缩放、取消/重建及原图SHA256。
- 最终契约审查发现前端素材库状态仍限定schema8；已同步为schema9并保留未来版本拒绝断言，继续执行原有素材库/指南回归。

中间lint发现指南图片回调的手工memo依赖未被React Compiler认可；已补齐稳定setFigure依赖，未禁用检查，最终重跑。


## 安装包与旧功能复验

`installer-smoke.ps1 -RunId v315-final -TargetDir src-tauri/target` 正常权限退出0：真实NSIS隔离安装/卸载4组通过；版本/发布者/图标、载荷哈希、忙碌安装和卸载拒绝均已核对。13个真实资料文件仅做只读摘要，前后不变；没有启动正式程序读写用户库；临时安装已完整卸载。签名状态NotSigned。

同步了早期原生脚本针对当前应用启动的schema断言（5/7/8→9），没有删减断言或把历史迁移夹具改成新版本。A10、指南及下述文献/相似回归单独执行；其余早期脚本未在本轮执行，不据此声称完整原生矩阵通过。


- `references-native-smoke a10-references-regression`：退出0，7组通过；真实Crossref返回Measured measurement / Markus Aspelmeyer / 2009，BibTeX、文献关联和引用保存/重启通过。
- `similarity-native-smoke a10-similarity-regression`：退出0，6组通过；38张真实测试图索引、重复/相似分页、取消重建、损坏隔离、主题/窄窗、比较与重启通过。没有执行AI模型。原报告末尾的schema8是旧说明文字，代码已断言schema9，本轮同步修正该说明；未修改历史报告来制造结果。

## 需求完成情况

内部候选搜索（完整和未闭合wiki输入）、稳定ID文档/截图引用、预览跳转、来源位置反向链接、重命名、失效提示、标准Markdown相对链接与警告、SVG图谱缩放/平移/打开/邻居/项目/标签、后台索引及取消、安全边界、重启一致性、界面与指南均已实现并按上述范围验证。无新增第三方依赖；没有实现本轮排除的PDF和独立备份系统。人工完整硬件矩阵仍未验证，不能视为本版已完成的验收。

新增业务集中于 `src/features/links/`、`storage/links.rs`、`commands/links.rs`、迁移009；导航、文档/卡片UI和共享素材库schema契约增量接入。文件服务仅新增标准Markdown副本，复用受限授权与不可覆盖落盘。旧存储文件改动为schema9状态/测试夹具同步。构建配置只切换既有实体解码包的纯实现。其余改动为版本、指南、测试和发布证据。原有两个未跟踪发布运输脚本保留，未纳入本版提交。
