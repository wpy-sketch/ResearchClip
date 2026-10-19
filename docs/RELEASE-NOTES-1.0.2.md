# ResearchClip v1.0.2 · A02 网页剪藏

新增独立Chrome/Edge Manifest V3扩展：主动剪藏链接与来源、选中文字、主要正文及选定图片；可预览、改标题、选项目、保存和重试。扩展独立构建/本地加载，没有后台全站扫描或Cookie权限。

独立Rust Native Messaging Host验证来源和消息，再经带256位令牌的本机长度帧IPC交给桌面。没有HTTP写入接口或扩展SQL能力。桌面沿用受管Markdown/assets、动态分栏、未保存确认和SQLite项目索引，schema v3与原图保留；相同请求去重，提交失败保留文件可恢复。

图片默认保留来源，明确允许后才尝试同源HTTPS匿名复制；跨域、登录限制、坏图/超限有状态说明，不绕过访问控制。设置增加按会话启用的剪藏连接卡片，完善分步安装指南、失败处理表和两张实际界面图。

[扩展安装说明](../browser-extension/README.md) · [完整使用指南](USER_GUIDE.md) · [实际测试与人工验收](qa/RELEASE-1.0.2.md)

实际Edge DOM提取5组、Host到Tauri/SQLite/Markdown7组、浏览器实际Native Messaging项目读取完成。完整浏览器工具栏/activeTab交互测试未完成：自动popup等待失败，Windows Computer Use因无法可靠识别浏览器URL停止。该项需人工验收，不能由上述分层测试代替。Chrome/其他系统、同源图片真实下载及长期资源矩阵未验证。

桌面EXE和NSIS均未签名、需要WebView2；扩展ZIP含独立Host和注册脚本，不需Node/Rust即可本地加载。注册只修改本目录所属的HKCU项，不覆盖其他安装。历史QA-001/002及QA-008卸载间歇问题继续跟踪。
