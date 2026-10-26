# v4.2.5 正式发布回执

[正式Release](https://github.com/wpy-sketch/ResearchClip/releases/tag/v4.2.5)，ID424019110；源码与标签冻结在8771dc7615aa2455ee897680e49b4b564a2e460f。正式非预发布，附件315项。

GitHub Actions [继承任务](https://github.com/wpy-sketch/ResearchClip/actions/runs/40295233165)（job121479575180）成功，将上版307项无冲突附件原字节继承；当前8项新附件另行上传。当前清单逐项比较名称、大小、SHA256后发布。

[匿名公开下载验证](https://github.com/wpy-sketch/ResearchClip/actions/runs/40326695337)（run40326695337，job121585134151）成功。315附件均从公开下载URL实际读取字节并核对SHA256和大小，下载不携带Authorization。验证工作流提交8771dc7615aa2455ee897680e49b4b564a2e460f。完整机器回执见[公开校验](evidence/v425-public-verification.json)。

此前44个Release、3775项原附件的ID、名称、大小和摘要前后保持一致，历史核对PASS；原标签/资产未覆盖。重名SOURCE/SHA按原名称和字节放入各版本metadata ZIP，历史清单保留对应归档成员，不以旧链接代替附件。便携包、当前NSIS、离线指南及历代功能所需组件均在本版附件中。

安装器SHA256：3d94810a6313999b1c31f4f51f7e10c23b0ae53d524ec541052d7839dcc8741d；未签名。实际安装/占用保护/PDF默认保留/卸载清理5组通过，没有残留安装。前端724、Rust291（7忽略）、Windows原生108、指南14、发布基础设施23项通过；完整修改和测试边界见[开发验收](RELEASE-4.2.5.md)及[机器证据](evidence/v425-editor-checks.json)。

默认资料27文件0变化。此前v4.2.4升级快照已有用户手动启动确认，原审计FAIL和文件保留；此确认不覆盖v4.2.2来源未明新增。真实Zotero10.0.6只读API探测与虚构专用HTTP夹具已区分，真实窗口多类型文献和物理混合DPI仍需人工验收。没有Zotero写入、私有笔迹转换、原PDF修改、Markdown导出PDF或独立备份/恢复产品。

本回执在源码标签之后以独立文档提交保存，源码标签及已发布附件不改写。后续Step15/v4.2.6需在独立测试Zotero资料库中验证写入能力，不将本版只读测试冒称写入通过。
