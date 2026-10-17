# v0.1.1 发布核对

2026-10-17，Asia/Hong_Kong。用户授权上传 ResearchClip v0.1.1 至既有公开仓库 wpy-sketch/ResearchClip。此版本包括桌面贴图与此前全部已跟踪源码，程序内版本同步为 0.1.1。按现有 QA 状态发布为 Windows x64 预发布；不覆盖旧标签或 Release，不公开本地缓存与资料。

## 检查范围

发布只更新版本元数据、说明与测试记录，包含上一阶段未提交的贴图功能。依赖版本没有升级，SQLite 迁移、原图和用户工作区不改。先完成检查及生产构建，再提交、快进推送和创建新标签；Release 先以草稿上传附件，核对大小/校验值后公开。

| 实际执行命令 | 结果 |
| --- | --- |
| `npm ci --offline` | PASS/0：按锁文件干净安装 294 包，无依赖升级 |
| `npm run check`（干净安装后） | PASS/0：TypeScript、ESLint、30 测试文件 / 220 项、Vite 生产构建 |
| `npm run format:rust` | PASS/0 |
| `npm run check:rust` | PASS/0：all-targets，-D warnings |
| `npm run test:rust` | PASS/0：102 项，无失败或忽略 |
| `node scripts/with-rust.mjs tauri build --debug --no-bundle` | PASS/0：0.1.1 debug 应用 |
| `npm run test:smoke:pins:native` | PASS/0：13 组，errors=[]，真实 Windows 窗口/拖动/透明度/保存/剪贴板和退出流程；[证据](evidence/v011-pins-prepublish.json) |
| `npm run desktop:build` | PASS/0：0.1.1 Windows x64 生产 EXE，无 NSIS |
| Windows `FileVersionInfo` 检查 | PASS/0：FileVersion / ProductVersion 均为 0.1.1 |

`npm run check:docs` PASS/0：34 文档、194 条本地链接及权限/版本/迁移一致性；`git diff --cached --check` PASS/0，待发布变更共 38 个文件。原有编辑器 6 组本轮不重跑，沿用开发阶段记录并明确区分。

EXE：ResearchClip-v0.1.1-windows-x64.exe，15,969,280 字节；SHA256=`be18bbe376dc79749a43e8c33c96bbb137addad36cbf5f320d2d3453e1332a0b`。SOURCE.txt 将记录最终源码提交，SHA256SUMS.txt 校验 EXE 和 SOURCE.txt。

## 既有证据与限制

贴图专项的 220 项前端、102 项 Rust、13 组原生贴图及 6 组编辑器回归属于开发阶段证据，见[贴图验收](../PINS-VALIDATION.md)。发布版本改为 0.1.1 后的重跑单独记录，不把历史结果冒充本轮结果。

Windows 高/混合 DPI、多屏完整矩阵、真实资料端到端、长稳、macOS、生产版完整交互和 NSIS 安装/卸载本轮未验收。QA-001/QA-002 历史退出句柄/托盘问题未全面关闭。当前只发布直接运行 EXE，无新安装器，未签名且依赖 WebView2。具体功能和使用见[发布说明](../RELEASE-NOTES-0.1.1.md)。

## 网络与提交边界

GitHub API 身份/仓库核验 PASS：wpy-sketch、公开 wpy-sketch/ResearchClip；main 起点 de5f53fee2809e805031b1c90945c15f04cb2532，旧 rc.1/rc.2 均存在，尚无 v0.1.1。Git 直连两次连接重置（FAIL/1），临时读取系统现有代理后只读 ls-remote PASS；不改全局 Git 配置。发布使用普通快进推送，无 force；凭据由系统 Credential Manager 仅在内存使用，不写入文件或输出。

常见私钥/令牌格式、超大文件与误纳缓存/数据库预检查无发现；这不是完整安全审计。测试原始截图、浏览器状态、构建产物和工具链不提交；GitHub Release 单独提供编译程序。
