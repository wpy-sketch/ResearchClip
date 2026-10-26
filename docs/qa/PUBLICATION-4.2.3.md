# v4.2.3 正式发布回执

[ResearchClip v4.2.3](https://github.com/wpy-sketch/ResearchClip/releases/tag/v4.2.3)已正式发布，Release ID423908813，非草稿/非预发布。应用标签固定1b12a487cdb3df33015680e280c94baed1c99266；源码、安装包和历史标签不重写。本回执只追加发布证据。

本轮完成Step12：PDF.js页面选区144/216/288 DPI渲染，稳定附件/哈希/页码/物理矩形，现有图片标注、一键Markdown光标插入、可编辑图注、整组拖动/撤销、保存重开和来源返回。SQLite17事务迁移扩展现有写入意图和统一位置，无新增依赖、原PDF写入或第二Markdown存储。指南29章205练习57图，新增四张原生练习图，应用图标重新设计。

实际检查：干净安装327包；前端88文件698项测试/类型/ESLint/生产构建通过；Rust276通过7忽略，格式与Clippy通过。Windows旧功能原生68组、新摘录15组、指南浏览器14组、发布基础设施20项通过。真实三页PDF校准包括非零CropBox、UserUnit2、内置旋转、四向视图旋转、四种缩放、三档DPI、模拟DPR及相对图片保存/重启。模拟不冒充物理显示器验收。NSIS实际安装与卸载5组通过，四份许可一致，安装器未签名，SHA256：5b1e234b53e0b17e30a3f07cda887d3350169c3d8a6de3ee3abd132f69917004。

默认资料本版独立只读核对26文件，changed=0/PASS；安装区间26文件无变化。此前v4.2.2长区间发现的两项来源未明新增资料仍保留原FAIL及文件，不自动认作测试资料、不清理或恢复。真实Zotero UI按用户选择保留人工验收，物理多屏DPI等边界仍见[验收与修改文件](RELEASE-4.2.3.md)及[机器证据](evidence/v423-editor-checks.json)。

303附件全部实际匿名下载，大小与SHA256匹配冻结清单：[公开下载验证40278981129](https://github.com/wpy-sketch/ResearchClip/actions/runs/40278981129)，job121427817635，见[完整证据](evidence/v423-public-verification.json)。[历史继承40278444645](https://github.com/wpy-sketch/ResearchClip/actions/runs/40278444645)由GitHub侧复制295项，本地上传8项；此前42版3163项原附件前后审计PASS。223历史运输卷原样保留，同名源码/摘要文件使用版本Metadata.zip保存，原有伴侣组件与旧附件均可定位。

Step12开发、受控检查、正式发布和附件验证完成。下一阶段双向定位已由用户另行授权为v4.2.4；本回执不把尚未完成的下一版本说成已发布。没有Markdown导出PDF、独立备份/恢复产品或Zotero标准批注写回。
