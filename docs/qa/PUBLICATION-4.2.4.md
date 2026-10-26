# v4.2.4 正式发布回执

[ResearchClip v4.2.4](https://github.com/wpy-sketch/ResearchClip/releases/tag/v4.2.4)已正式发布，Release ID423950946，非草稿/非预发布。应用标签固定149f8d8970b7192e028296f3f2e7aee70c562db7；不重写标签、安装包或历史附件。

完成Step13双向定位：点击文献截图返回稳定PDF页/区域，保留草稿、光标、视图及当前旋转；8秒临时高亮，PDF选区反查已打开/已保存笔记，真实文字层引用及同一笔记多文献关联。受管副本逐次摘要核对，内容替换明确拒绝跳转，原始PDF不修改。SQLite18只追加派生索引，复用原链接CAS事务、资源ID、Markdown文件/撤销和原有OCR规则。指南30章217练习59图，图标增加双向箭头；无新增依赖。

实际检查退出0：干净安装327包；前端89文件710项测试及类型/ESLint/生产构建；Rust280通过7忽略、格式与Clippy通过。Windows综合摘录/双向25组、既有原生68组、指南浏览器14组、发布基础设施20项通过。NSIS安装/占用保护/PDF候选与默认保留/卸载及清理5组通过，四份许可一致。安装器未签名，SHA256：ca5f719536a427296e78a2296fed377bb814bf9db9898ed1a9b6ee7fb7c38f0c。

资料长区间原审计FAIL保留：原26文件无改写或缺失，15:13:29新增一份schema16升级前快照，晚于15:11:20原生回归及15:12:59安装结束。用户明确确认当时手动启动过ResearchClip，新增快照与该启动相符；没有删除/恢复/重置资料，不把整段区间说成无变化。安装区间26文件核对通过。此前v4.2.2两项来源未明新增仍未归属，本次确认不覆盖它们。见[审计说明](evidence/v424-data-audit-explanation.json)、[实际测试及修改文件](RELEASE-4.2.4.md)和[机器证据](evidence/v424-editor-checks.json)。

309附件均实际匿名下载，大小/SHA256匹配冻结清单：[公开验证40287564948](https://github.com/wpy-sketch/ResearchClip/actions/runs/40287564948)，job121454738322，见[完整证据](evidence/v424-public-verification.json)。[历史继承40286261126](https://github.com/wpy-sketch/ResearchClip/actions/runs/40286261126)复制301项既有附件，本地上传8项；此前43版3466项原附件前后审计PASS。223历史分卷保持原字节，原伴侣组件和各版附件均可定位。

第一次公开验证因脚本遗漏v4.2.4数量映射而在下载前退出；[157365c发布工具修复](https://github.com/wpy-sketch/ResearchClip/commit/157365c3c83c969eae9afc22ba3d261fc3920ffa)只补映射，重新执行全部匿名校验通过，应用标签与附件未改变。部分本地网络上传/清单读取失败按已有身份与摘要续传，未覆盖异名内容。

物理多屏DPI、真实Zotero窗口设置/双击及专用文献人工验收继续保留；模拟DPR和隔离API不冒充真实GUI通过。本版不包含Zotero批注适配器或写回、Markdown导出PDF、独立备份/恢复产品。用户已授权的Step14在下一版本v4.2.5继续实施。
