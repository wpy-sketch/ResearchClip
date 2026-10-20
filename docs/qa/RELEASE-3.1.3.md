# ResearchClip v3.1.3 / A09 验收与发布记录

环境：Windows x64；Node22.22.2、本机 Rust1.99、Tauri2/WebView2。2026-10-20。仅隔离工作区，不写真实用户资料。

状态使用 PASS / FAIL / BLOCKED / NOT_RUN；解析夹具、Mock、浏览器测试与 Windows 原生分别记录。

## 范围与实际入口

- 文献：`services/references.rs` 规范化/解析/生成；`storage/references.rs` 事务/查询/关联；`commands/references.rs` main IPC、Crossref 和系统选择器；`features/references` 管理面板。
- 截图知识卡片 / Markdown 分栏 / 剪藏通知：关联入口，沿用既有保存服务与稳定ID。
- 相似检索：左侧入口与 SearchScope，候选/重复组使用相同参数化项目和收藏范围。
- 新迁移008，001–007不改；无新增第三方依赖、账号、PDF或独立备份功能。

## 已执行的中间检查

- 初次 Rust 测试 FAIL：180通过、4失败、7忽略。新测试夹具错误、旧版本断言和构造旧库夹具已修复；Windows凭据测试在沙箱失败。
- 第二次沙箱测试 FAIL：183通过、1失败、7忽略，仅系统凭据存储失败；没有删改断言。
- 正常 Windows 权限复验 PASS：187通过、0失败、7忽略（版本调整前）；隔离凭据往返测试通过。
- `npm run check` PASS：TypeScript、ESLint、56文件380测试、Vite生产构建（版本调整前）。保留既有 >500kB chunk 警告。
- 首轮 strict Clippy FAIL：新增 scope 后 IPC 参数超过 lint门槛；保留兼容契约，并在该命令添加局部说明，不关闭全局检查。
- Crossref 实际请求 BLOCKED：普通网络拒绝连接，正常权限请求 Schannel TLS握手失败；不能将响应夹具当成联网成功。

## 最终检查

| 检查 | 状态 | 实际结果 |
| --- | --- | --- |
| `npm run check` | PASS | 类型、ESLint、56文件381测试、Vite；退出0，保留既有大chunk警告 |
| `npm run format:rust` | PASS | 退出0 |
| `npm run test:rust`（正常权限，串行） | PASS | 188通过、0失败、7原有集成用例忽略；退出0 |
| `npm run check:rust` | PASS | all-targets严格Clippy，退出0 |
| 隔离 `npm ci --ignore-scripts` | PASS | 官方npm、303包、版本和integrity完全不变；退出0 |
| 浏览器扩展测试/构建 | PASS | 5测试，双Vite构建，退出0；兼容1.0.2 ZIP保留 |
| `references-native-smoke a09-verify --guide` | PASS | 7组真实Windows/SQLite/选择器/重启，Crossref真实查询并确认保存 |
| `guide-native-smoke a09-current --verify-only` | PASS | 5组，真实素材/标注/笔记/对比/Markdown，15章27图，1180/760/480 |
| A09最终增补编辑回归 | PASS | a09-corrected-final 7组全部通过，已有记录修改/机构作者/数学源码保真、引用与重启；初次失败a09-final保留 |
| `npm run desktop:bundle` | PASS | 最后修复已重建优化EXE与NSIS，退出0 |
| 隔离NSIS安装/卸载 | PASS | v313-ship最终4组，12个真实用户资料文件只读摘要不变；默认卸载退出0、忙碌安装/卸载均退出2 |
| GitHub v3.1.3发布 | NOT_RUN | 本地检查已完成，正在准备源码标签与附件 |

**失败与恢复记录**：首次原生脚本错误对象读取和按钮选择器失败，修正后重跑；.bib测试辅助程序先错误限制.md，修正为仍限定artifacts的.bib。最终编辑回归暴露真实P1问题（只读字段回传被严格Rust边界拒绝），没有关闭后端校验来绕过；新增单测并重新跑原生测试。另有一次并行Rust测试链接workflow_input.exe被占用失败，待原生测试退出后串行188项通过。离线npm安装先ENOTCACHED、旧镜像网络重置/证书不匹配，保持锁文件版本/integrity，换官方地址并通过隔离干净安装；未关闭TLS。

Crossref：早先CLI TLS失败；后续浏览器读官方文档和API、真实Windows应用均得到 Measured measurement / Markus Aspelmeyer / 2009，确认前0条、确认后1条入库；这些结果独立于Mock解析测试。


## Windows 人工矩阵

1. 在可连接 Crossref 的桌面，查询 `10.1038/nphys1170`；核对自动元数据，修改后确认保存；重复 DOI 不新建。无结果/断网/取消不得影响笔记。
2. 手工录入中文作者、未知年份，关联截图；取消关联，确认截图/笔记/文献均保留；重启再看。
3. 选择自己的 UTF-8 .bib，多条预览；冲突键整批不写入；排除冲突后再导入，复制到外部软件检查。
4. 打开剪藏文档，关联文献并插入 `[@key]`，保存关闭重开；原始正文、图片和公式仍在。
5. 左侧相似与重复，切项目/未分类/收藏；从其他项目选参考图；人工对比，不自动删图。
6. 深浅色、窄窗口、中文输入、真实剪贴板跨软件粘贴和原有完整物理DPI/多显示器矩阵分别验收。

## 已知边界

文献当前仅本机，不随 WebDAV 同步；未保存 Markdown 先插入引用文本，保存后再建立持久关联。BibTeX 宏须展开；不提供自动文献排版。跨服务网络、GitHub附件回读和完整物理DPI矩阵只按实测结果认定。


## 离线模型回归

`node scripts/similarity-native-smoke.mjs a09-model-regression --ai` 退出0，7组PASS：38张真实隔离SQLite测试图、分页/阈值/主题/窄窗、损坏图/取消/复用、实际本地Qwen CPU推理、进程取消与临时图释放、应用重启；不代表物理屏幕截图或AI数学正确性。模型/运行时版本及哈希未变，本版复用v3.1.2离线资源。


## 最终产物与证据

安装包：`src-tauri/target/release/bundle/nsis/ResearchClip_3.1.3_x64-setup.exe`。固定发布副本：`artifacts/v313-final/`。验收摘要见[evidence/v313-checks.json](evidence/v313-checks.json)。安装包未签名；没有声称未执行的物理DPI、多显示器、macOS、所有浏览器/文献软件或真实WebDAV认证矩阵通过。
