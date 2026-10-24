# 已授权的 GitHub 发布运输

2026-10-24用户明确授权新增仅用于发布的GitHub Actions，以加速历史附件上传和继承。它不是应用功能、遥测、同步或独立科研数据备份/恢复系统。

workflow仅支持workflow_dispatch，Ubuntu24.04任务，只在该任务授予contents:write。没有第三方Action，使用runner内置git/Python标准库；公共克隆固定为本仓库与GitHub提供的40位提交SHA，不持久化Git凭据。目标Release必须是明确传入的本仓库草稿、指定标签和精确提交。它只能补传附件，不发布Release、不修改历史Release或标签。

v4.1.2使用冻结ZIP头/目录字节、公开附件大小/摘要及每个压缩载荷摘要，重建原定3728704762字节归档。源文件先校验、每个压缩载荷再校验、完整ZIP最终必须匹配8faa561b4282aeb7343bc7e7ebb8c4ae59f8ca07d73f1840c1953cacd331786f，才可拆成原223卷上传。压缩库差异导致任一摘要不同就停止，不冒充相同归档。

v4.1.3只继承前版235个不冲突的附件，按冻结237附件快照验证；重名SOURCE/SHA已由本地Metadata.zip保留。上传前已有同名附件必须完全匹配；只允许清理已知目标草稿中无摘要的starter残片，不覆盖完整附件。终态逐项核验GitHub大小与摘要；实际公开发布和匿名下载仍由本地发布器完成，且v4.1.2完成后才允许v4.1.3。

工具脚本：scripts/github-release-transfer.py，配套本地重建/摘要拒绝/禁止覆盖测试。冻结配方和上传清单位于本目录evidence。原附件下载URL和ID均为公开Release记录；不包含用户资料或凭据。CI实际运行结果在后续发布凭据记录，本文本身不表示CI已经通过。

官方依据：[Release二进制下载](https://docs.github.com/en/rest/releases/assets)、[Actions最小权限与不可信输入](https://docs.github.com/en/actions/reference/security/secure-use)。
