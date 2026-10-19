# ResearchClip v1.0.3 · OCR全文索引

新增单张/主动批量OCR、持久化全文与人工纠错、暂停继续取消和失败重试。标题/笔记/标签/OCR统一搜索、来源筛选、安全摘要高亮及索引重建。FTS5 trigram处理较长查询，短中文使用转义LIKE。

数据库升级schema4前校验并备份，事务迁移，旧截图/标注/笔记保持。原图不可变，OCR不混入笔记。修复已实际复现的重复Windows OCR能力查询访问冲突；更新索引界面与操作路线图。

实际312项前端/140项Rust、20次原生能力查询、30张中英图OCR/关键词/队列/重开验证通过；30关键词命中不等于逐字准确率。新增WindowsUI、其他语言/硬件及长期资源矩阵未运行，历史退出/托盘/卸载间歇问题保留。

[使用指南](USER_GUIDE.md) · [实际验收及失败记录](qa/RELEASE-1.0.3.md)。网页剪藏扩展继续使用兼容的[v1.0.2独立ZIP](https://github.com/wpy-sketch/ResearchClip/releases/tag/v1.0.2)。不含公式识别、PDF或云服务。未签名EXE/NSIS需WebView2，先保存并退出旧实例。
