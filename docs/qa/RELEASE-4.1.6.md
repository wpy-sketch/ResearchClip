# v4.1.6 发布检查

范围仅Step07数学论文Markdown。实际实现与运行结果见[验收记录](MARKDOWN_STEP07.md)，文件与语法见[技术契约](../architecture/MATH_PAPER_STEP07.md)。SQLite12，无迁移、权限扩张、Markdown导出PDF或独立备份恢复产品。

当前已冻结此前35版1210个附件快照，核对256个继承伴侣文件。预计本版261个附件：历史同名字节保留，冲突的SOURCE/SHA置于相应版本Metadata.zip；223个原归档运输卷保持原字节，不重复嵌套。

最终npm run check退出0：612项/73文件；干净npm ci、Rust fmt/test/clippy退出0（237通过、7忽略）。53原生组、45浏览器组、2组真实记事本读取通过。见[实际证据](evidence/v416-editor-checks.json)。

npm run desktop:bundle退出0，使用src-tauri/target/v413-distribution缓存；pwsh -NoProfile -File scripts/installer-smoke.ps1 -RunId v416-licenses -TargetDir src-tauri/target/v413-distribution退出0：实际安装、占用保护、卸载、清理4组通过，三份许可哈希一致，16个真实AppData文件只读摘要不变，隔离安装已移除。未启动正式程序访问真实资料。

最终NSIS SHA-256：825f557ca52537b3cdadb047d5f7a67b511e5633fbbdc36fc6c9df966e3d13c5。安装包未签名。物理IME、多DPI/跨屏及先前锁屏OLE问题仍需人工验证。

源码/标签推送、261附件发布与实际匿名下载及旧发布前后终审尚待完成，不将准备状态记为发布成功。
