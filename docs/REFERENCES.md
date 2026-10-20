# A09 文献引用管理

## 范围和数据契约

本机 SQLite schema 8 新增 `literature_references`、`screenshot_references`、`document_references`。001–007 不改；旧库升级前沿用一致性校验、只增不覆写的迁移快照和事务迁移。文献表不存图片，解除关联不删除截图、Markdown、文献或笔记。

Reference 包含 ID、标题、作者数组、可空整数年份、期刊/会议、规范化 DOI、URL、引用键、BibTeX 原文、来源、来源快照、修改时间与乐观锁 revision。来源值为 `manual / crossref / bibtex / edited`，**表示录入来源，不表示论文或公式已被核实正确**。编辑自动获取/导入的数据会保留初始来源快照并标记人工修订。

截图关联采用已有 screenshot ID；文档关联采用现有 `markdown_documents.id`，不使用前端任意路径。未保存的 Markdown 可以浏览文献、插入普通引用文本；首次保存并登记文件后才能持久关联。移动或删除应用之外的文件不能保证关联自动发现；通过已有应用重命名/保存服务保持其稳定 ID。

## 调用关系

`ReferenceButton / References → references/api → main 限定 Rust 命令 → LibraryService → storage/references`。界面复用 Modal、语义色和 Zustand 轻状态；正文仍由现有 MarkdownDocumentSession 管理，未打开文档时不增加分栏。

只有 DOI 查询联网；Rust 固定访问 Crossref HTTPS 域名，拒绝重定向、验证 TLS、不发送截图或笔记。网络不持有 SQLite 锁。每次最多15秒，连接5秒、响应256 KiB、单并发、至少2秒间隔；HTTP429 后60秒冷却。取消等待使旧返回失效；底层有界请求在超时前结束，不会保存结果。

Crossref REST 的预期接口为 `GET https://api.crossref.org/works/{percent-encoded-doi}`，使用 `message.title / author / published(date-parts) / container-title / DOI / URL`。未知字段不补造；错误日期/结构明确失败。自动信息只有用户确认后才写入。当前环境早先 CLI HTTPS 请求曾 TLS 失败，后续实际 Windows 应用查询已获取 Measured measurement（2009，Markus Aspelmeyer），经明确确认保存；具体证据见 QA 报告。

参考接口资料：[Crossref REST 文档](https://www.crossref.org/documentation/retrieve-metadata/rest-api/)、[Crossref REST API 仓库](https://github.com/CrossRef/rest-api-doc)。2026-10-20 已通过 Chrome 阅读官方文档并核实 /works/{doi} 与实际 JSON；Windows 应用也完成同一 DOI 的真实查询。未新增依赖或升级已安装 API。

## DOI 与引用键

- DOI 去 `doi:` / DOI resolver URL 前缀、首尾空白，并 ASCII 小写；长度最多512，`10.` 后4–9位数字，斜杠后非空、无空白或控制字符。
- 非空 DOI 唯一；引用键大小写敏感唯一。空 DOI 为 SQL NULL，多个手工记录可以没有 DOI。
- 新录入建议键 `ref-<8位随机串>`；只在创建时产生，可手改。导入保留原键，编辑标题/作者不会重新生成键。
- 键仅允许字母、数字、`_ : -`，1–100字符。改已有键要显式确认；不批量改写用户现有 `.md`。
- Markdown 插入 `[@key]`，这是合法的普通 Markdown 文本/Pandoc 风格标记。外部软件是否自动排版取决于它的引用插件；应用不声称原生 Markdown 都会自动生成参考文献列表。

## BibTeX 互操作边界

系统对话框选择 `.bib`，UTF-8、最多1 MiB、200条、单条64 KiB、花括号最多64层。支持普通 `@article / @misc / @inproceedings / …`、花括号/引号字段、数字年份、嵌套花括号、反斜杠转义、百分号行注释与多条记录。作者只在花括号外按 ` and ` 分割，保留机构作者。

这是有明确边界的本机导入器，不执行 TeX。`@string / @preamble / @comment`、裸宏与 `#` 拼接必须先由来源工具展开/移除；语法不支持时整次预览失败，不悄悄跳过。未知年份如 `forthcoming` 须手工改为空或整数。

导入先预览和选择，再单事务保存。DOI/引用键冲突整批回滚，可取消冲突记录、查找并关联现有记录；不覆盖已有文献。未修改的记录保留原始 BibTeX 字符串和额外字段；修改后的输出更新已知字段并保留额外字段与条目类型。新手工记录生成 `@misc`，已提供字段才写入，正确转义普通文字。复制通过已有受限本机剪贴板命令，不开放通用文件系统权限。

## 检索与资源限制

文献本地搜索使用参数化 `instr(lower(...))`，中文子串与英文 ASCII 大小写可检索，百分号等按普通字符处理；不执行用户 FTS 语法。每页25条，最多1001页；每目标最多200关联。写入 optimistic revision 检查，失败不替换草稿。当前 WebDAV 协议未包含新增文献表，界面和指南明确提示本机范围。

相似检索主入口移动到左侧；增加全部、未分类、单项目和收藏组合过滤，作用于候选和重复组。索引仍按全库有界更新并复用旧特征，参考图片可来自其他项目；卡片快捷检索入口保留。SHA-256 与感知距离含义沿用 [SIMILARITY.md](SIMILARITY.md)，不自动合并或删除。

## 验收

自动测试覆盖 DOI/缺失元数据/错误作者和年份、BibTeX 多条与拒绝语法、事务冲突、关联/解除、重启重读、迁移快照、乐观锁及来源保留；Windows 原生脚本覆盖真实 WebView2、SQLite、文件对话框、Markdown 插入/保存/重开与范围过滤。执行结果见 [A09报告](qa/RELEASE-3.1.3.md)。
