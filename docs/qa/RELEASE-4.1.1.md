# v4.1.1 发布检查

目标：Step02科研工作区、响应布局和指南。代码延续原SQLite schema12、文档服务、图片编辑及权限边界。

## 实际结果

- 干净npm ci --ignore-scripts退出0，原有依赖未升级；新增固定flexlayout-react 0.11.1（MIT）。
- npm run check退出0，66文件454测试、类型/lint/生产构建通过；Rust格式/Clippy退出0，Rust测试230通过/7忽略。
- 工作区Windows原生7组通过（workspace-native-v411-acceptance），旧资源Windows原生6组通过（resources-native-v411-image-fixed），浏览器指南13组通过。对应原生用例均实际启动独立进程并正常退出。
- npm run desktop:bundle退出0；pwsh scripts/installer-smoke.ps1 -RunId v411-distribution -TargetDir src-tauri/target退出0，4组实际安装/运行阻止卸载/正常卸载/资料哈希保留检查通过，额外核验MIT许可证原文已安装；未残留测试安装。安装包SHA-256：45e897f1029e1094732638a801dba9aef62f4a20635717bf1febd6e6db83fd6a。此前构建/审计的独立记录不混用。

环境Windows x64、Node22.22.2、React19、Tauri2/WebView2；Rust用缓存锁定依赖离线执行。证据位于artifacts/dock-evaluation和各独立测试目录，日志不随发布上传。

## 未通过与未执行

开发失败均记录在[Step02报告](WORKSPACE_STEP02.md)，包括旧资源回归曾发现的图片加载失败（已修复后重跑通过）。历史advanced-native附加套件退出超时仍不宣称通过。未执行用户真实资料验收、多显示器物理DPI、Zotero、HarmonyOS、macOS及所有外部OCR/同步运行时的完整重装。

未签名；Windows可能显示SmartScreen。PDF/代码仅类型注册、终端仅布局区域。最多3份Markdown，未保存草稿不跨进程恢复；自动布局不保存文件授权。

用户数据、截图原图和原迁移保持不变。发布只含源码、程序、离线指南与校验信息，不包含科研资料。不提供Markdown导出PDF或独立备份/恢复。

Windows人工步骤与回退前置条件见[工作区验收](WORKSPACE_STEP02.md)。本版发布后仅继续用户明确授权Step03，不推进其他阶段。
