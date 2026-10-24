# v4.1.3 发布检查

目标：在原有Markdown编辑器增量增加工具栏并保留此前功能和附件。基础检查见[工具栏报告](MARKDOWN_TOOLS_4.1.3.md)。数据库保持12，无新迁移；可选组件运行时仍按原指南配置。

正式构建npm run desktop:bundle退出0，CARGO_TARGET_DIR为src-tauri/target/v413-distribution，避开运行中的旧版本EXE。最终包为该目录release/bundle/nsis/ResearchClip_4.1.3_x64-setup.exe。

pwsh -NoProfile -File scripts/installer-smoke.ps1 -RunId v413-limit-distribution -TargetDir src-tauri/target/v413-distribution退出0，4组实际安装/运行中拒绝覆盖/卸载/元数据清理通过。FlexLayout和新增18包Markdown许可文件实际安装且哈希匹配。16个既有AppData文件只读哈希不变，测试安装不再存在，未对真实资料启动生产应用。安装包SHA-256：5474151b0c6daf77d81517a6c6c9ebed35c95d83a0ae88fdf8951fd74276daa6。

新增GitHub侧运输工作流已获用户明确授权；本地精确重建3.73GB历史归档退出0并匹配原SHA，配套4个重建/摘要拒绝/禁止覆盖测试通过。GitHub工作流40075692657（提交8bc5a89）已成功，用runner的zlib1.3精确重建相同归档并验证223卷；v4.1.2已正式公开；其匿名下载由只读GitHub任务逐项验证，首次因Windows CRLF来源文本解析失败，已补充LF/CRLF与精确身份拒绝测试后重跑。237个公开附件已在CI实际匿名校验通过，接收器与本地冻结清单核对通过，旧31版226附件最终审计通过。详情见[前版回执](PUBLICATION-4.1.2.md)。本版243项实际公开匿名下载已通过，来源为只读GitHub CI，完整凭据见[正式回执](PUBLICATION-4.1.3.md)。

源码d89ae4a、v4.1.3标签和正式Release均已推送；243项匿名校验及32版463附件前后终审通过。本文件保留构建检查，最终发布状态以[正式回执](PUBLICATION-4.1.3.md)为准。Windows未签名，物理输入法/多DPI、真实资料和外部平台验收仍未执行。

最终候选包含发布前发现的超大替换长度预判和可见错误修复；525项前端/31组原生回归与新安装检查均通过。先前候选尚未正式公开，其本轮标签更新受远端期望SHA保护，不改任何历史正式标签。
