# ResearchClip v3.1.3 · DOI 与 BibTeX 文献引用

## 新增与调整

- 本机文献库：Crossref DOI 查询预览、确认后保存、手工录入与编辑，保留信息来源和初始来源记录。
- DOI 规范化去重、稳定 citation key、冲突提示及乐观锁；已用引用键不会因元数据更新被自动重写。
- 系统对话框导入 UTF-8 .bib，单条/多条选择预览；事务保存，冲突整批回滚；未修改字段和额外 BibTeX 字段保留。
- 截图详情、Markdown 分栏与网页剪藏通知可关联文献；插入 `[@key]`、复制 BibTeX、打开 DOI；不替换原笔记或公式。
- 相似检索入口移至左侧；候选和重复组支持项目、未分类、仅收藏组合，新增参考图选择。
- 统一深浅色、柔和边框与窄窗口布局；指南增加第15章，包含详细三级/四级操作目录和两幅本版实际 Windows 测试界面图。

## 安装与兼容

下载本版 Windows x64 EXE 或 NSIS，先保存并退出旧实例再安装。需要 WebView2；程序未数字签名。SQLite 从 schema7 升至8，沿用迁移前校验和不可覆盖快照；不删除原始图片，不上传数据库。

本版附带兼容的 **WebClipper v1.0.2** 浏览器扩展，扩展 ID 与 Host 协议不变；不是把旧扩展代码冒充新开发版本。

Qwen 模型和 CPU runtime 与 v3.1.2 字节相同，已安装的 `similarity-ai` 可继续使用，**不需要重复下载数GB权重**。首次使用可从 [v3.1.2完整离线资源](https://github.com/wpy-sketch/ResearchClip/releases/tag/v3.1.2) 获取全部25卷，校验解压后仅使用其中的 `similarity-ai` 文件夹，并运行/安装本页的 v3.1.3 应用；旧包中的应用版本仍为3.1.2，不冒充新版。轻量 NSIS 自身不包含模型。模型与普通重复检索相互独立。

## 实测与边界

实际结果见 [v3.1.3验收](qa/RELEASE-3.1.3.md)，操作见[完整使用指南](USER_GUIDE.md)，数据协议见[文献设计](REFERENCES.md)。

已在真实 Windows WebView2/SQLite/本地文件对话框验证文献写入、BibTeX、关联、引用保存重开和原图完整性；实际 Crossref 返回 `Measured measurement`、Markus Aspelmeyer、2009，并在确认后落盘。早先网络/TLS失败与重跑结果均保留。

文献表及关联目前仅本机，不随 WebDAV 同步；Markdown 源文中的 `[@key]` 仍正常保存和同步。引用自动排版取决于外部工具自身支持，不承诺所有 Markdown 软件自动生成参考文献。BibTeX 宏和拼接须先展开，不执行 TeX 或网页脚本。

完整物理高DPI/多显示器、macOS、真实坚果云凭据与所有外部文献软件矩阵仍需分别验收。现有未签名、托盘等历史限制不因正式 Release 标识消失。不新增 PDF、独立备份系统或云端服务。
