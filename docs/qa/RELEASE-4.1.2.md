# v4.1.2 发布检查

目标：Step03框架与少量真实适配、响应布局/指南，以及完整历史发布附件。不推进Step04。无新依赖、SQLite保持12。

## 本轮已执行

实际自动与Windows结果汇总在[Step03报告](COMMAND_SYSTEM_STEP03.md)。npm run desktop:bundle退出0；正式NSIS路径src-tauri/target/release/bundle/nsis/ResearchClip_4.1.2_x64-setup.exe。pwsh -NoProfile -File scripts/installer-smoke.ps1 -RunId v412-distribution -TargetDir src-tauri/target退出0，4组实际安装/忙碌拒绝/卸载/元数据清理检查通过。16个真实AppData文件只读哈希保持，未启动生产应用访问真实库，测试安装已移除。安装包SHA-256：84cbbff11ad083412a4804ae20906ca0ef8db300b6af85c8e4623503b0fe8809，签名NotSigned。

最终npm run check:docs退出0，142份文档805本地链接；git diff --check退出0。公开传输与匿名下载凭据在发布后记录，未记录不代表通过。

历史31版226附件已逐个校验GitHub摘要后打包；归档内每个成员再校验。Windows已从223卷实际重组并匹配3728704762字节ZIP，已有相同目标仅验证，已有不同目标/路径越界元数据均拒绝，测试哨兵内容保留。原始公开附件合计4117989169字节。

## 交付范围和限制

当前Release计划237附件：当前安装包/便携ZIP/指南/SOURCE/SHA校验，加232个伴侣/历史运输文件。仅包括源码、程序、许可、指南和此前公开发布文件，不包含用户科研资料。原历史Release/附件保留并在发布前后逐项核对。

Windows应用未签名，安装需要WebView2；可选Python/模型运行时仍按对应组件说明配置。原版本附件不代表本轮已重新安装/验证其全部运行时。

用户真实库、多显示器物理DPI、Zotero、HarmonyOS及未接入的PDF/代码/Agent未验证；CSS缩放测试不代替物理DPI。历史advanced-native退出超时及本轮额外workflow-smoke前台窗口保护失败不宣称通过。人工步骤与回退前提见Step03报告；先保存并退出，不重置数据库/原图。
