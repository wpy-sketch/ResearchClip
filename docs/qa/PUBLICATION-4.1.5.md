# v4.1.5 正式发布回执

日期2026-10-24。已按顺序完成v4.1.4后发布[ResearchClip v4.1.5](https://github.com/wpy-sketch/ResearchClip/releases/tag/v4.1.5)；非草稿、非预发布，Release ID 422763684。应用源码和v4.1.5标签固定caa4eefe109186c27cbc647b84a1942b5aabcdd5。后续回执提交只改文档，不改变该应用标签。

255个附件全部实际匿名下载并与本地冻结清单逐项核对大小、SHA-256；不是用元数据代替下载。执行于只读GitHub Ubuntu24.04任务，附件请求不携带Authorization。运行[40101028660](https://github.com/wpy-sketch/ResearchClip/actions/runs/40101028660)，job120893023211；[完整255项回执](evidence/v415-public-verification.json)。本地接收器验证工作流身份、源提交、成功状态和每一项本地摘要。继承任务40100677746实际核对并填入247个无冲突历史附件，只修改本版精确授权草稿。

旧34版955个附件前后终审通过，原文件ID、名称、大小、摘要及发布标签未改。250个继承伴侣文件逐项表示全部原附件；重名SOURCE/SHA保留在对应版本Metadata.zip，历史223分卷不重复嵌套，不以链接代替内容。当前255附件包括4.1.5便携ZIP、NSIS、离线指南、SOURCE和SHA清单。见[下载说明](../RELEASE-COMPONENTS.md)。

最终测试：583项前端/71文件，237项Rust（7忽略），同轮45组原生、38组浏览器、2组普通记事本读取，以及4组最终安装均通过。最近修复后的跨标签导入/图文边界和工作区已针对性重跑。详见[Step06验收](MARKDOWN_STEP06.md)、[最终构建检查](RELEASE-4.1.5.md)、[实测证据](evidence/v415-editor-checks.json)。

最终NSIS SHA-256：b21a4cc22da7f2f4619571c3001aab0d0ce8c7b8fa88fbdc2a867be6a5448acf。三份许可文件实际安装且哈希一致，16个既有AppData文件只读哈希不变，隔离测试安装已移除。安装器未签名。SQLite12，无新迁移或权限扩张。

未通过/未验证仍明确保留：真实系统OLE拖放因Windows锁屏前台归属保护失败，受控WebView File→真实Rust/SQLite/PNG/保存路径通过；物理IME、多DPI/跨屏、真实资料及外部平台未人工验收。PDF原生阅读/位置选区接口仍未接入；已入库PDF截图沿通用插图路径自动获得注释区。没有Markdown导出PDF或独立备份恢复产品。

Step05、Step06及这两版的开发/正式发布工作完成后停止。下一步仅需解锁环境中的人工拖放、IME和多DPI验收，其他阶段须用户另行授权。
