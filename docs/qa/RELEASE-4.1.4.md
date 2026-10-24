# v4.1.4 发布检查

目标：完成用户Step05、原文保真及Windows验证，正式发布后才进入已另行授权Step06。实现与失败记录见[Step05验收](MARKDOWN_STEP05.md)。SQLite12，原迁移001–011不变，用户文件未移动、删除或重置。

本文件先记录发布门槛，实际构建、回归、安装包哈希及公开下载校验将在取得结果后追加。尚未有成功回执前不能视为已正式交付。

最终npm run check退出0：557项、70文件；Rust fmt/test/clippy退出0：237通过、7忽略。Windows新功能7组及旧功能31组实际原生回归通过；指南浏览器14组通过。无新增运行依赖。源文/未知扩展/CRLF混合稿与带title图片本轮已实际保存并重启验证。

npm run desktop:bundle退出0，复用src-tauri/target/v413-distribution编译缓存，生成4.1.4 NSIS；不触及默认target/release旧实例路径。pwsh -NoProfile -File scripts/installer-smoke.ps1 -RunId v414-distribution -TargetDir src-tauri/target/v413-distribution退出0，四组真实安装、运行中保护、卸载和注册清理通过，许可实际安装且哈希一致。16个真实AppData文件只读哈希不变，隔离测试安装已移除；未用生产实例打开真实资料。

安装器SHA-256：19c1778dcb9f085889cf438f040e78f0ae41a82fb78128eef704812d5c0d1848。未签名。物理IME、多DPI、真实资料与外部平台未验收；已有额外物理截图前台保护失败未冒充通过。

附件准备已核对此前33版706原附件不变，244个继承伴侣文件通过原名/字节映射；本版预期249附件。尚待正式上传及匿名校验，结果完成后补正式回执。
