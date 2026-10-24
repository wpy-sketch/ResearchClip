# v4.1.5 发布检查

范围为用户Step06，先完成v4.1.4正式发布与249项匿名校验才开始。基点e9654db，SQLite12，无新迁移/权限；只增加两包代码高亮依赖，版本/API/许可真实核对。实现与分阶段实际结果见[Step06报告](MARKDOWN_STEP06.md)。

本文件保留构建/安装/发布门槛。当前准备继承34版955原附件，250个伴侣文件已逐项校验；本版预期255附件。完成正式安装、发布及实际匿名下载后追加成功回执，不能用计划代替结果。锁屏导致真实OLE拖放未通过，保持人工验收项。

最终npm run check退出0：583项/71文件，类型、lint及构建均通过；干净npm ci --ignore-scripts退出0。Rust fmt/test/clippy退出0：237通过、7忽略。见[实测证据](evidence/v415-editor-checks.json)与[官方依赖摘要核对](evidence/v415-dependencies.json)。

受影响检查在最后修复后重跑：图文/GFM边界、异步导入原会话归属与跨标签不串写的原生7组，工作区原生7组；同轮其余Step05/工具栏/命令/资源回归共45组，浏览器38组，普通记事本2组。真实OLE仍因锁屏未通过，未冒充验收。

npm run desktop:bundle退出0，复用src-tauri/target/v413-distribution缓存生成最终4.1.5 NSIS；不覆盖默认release旧实例路径。pwsh -NoProfile -File scripts/installer-smoke.ps1 -RunId v415-owner-distribution -TargetDir src-tauri/target/v413-distribution退出0：四组真实安装、运行中拒绝覆盖、卸载及注册清理通过，三份许可文件实际安装且SHA匹配；16个真实AppData文件只读哈希不变，隔离安装已移除。安装器未签名。

最终安装器SHA-256：b21a4cc22da7f2f4619571c3001aab0d0ce8c7b8fa88fbdc2a867be6a5448acf。之前两个未发布安装候选的成功检查不作为本次最终包回执；使用上述唯一最终哈希。

尚待源码/标签推送、255附件发布与真实匿名下载校验。旧34版955附件快照和250继承伴侣文件已核对。
