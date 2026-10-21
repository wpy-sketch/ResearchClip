# A10 内部链接与知识图谱

## 身份与文件映射

复用 `markdown_documents.id` 与 `screenshots.id`，均为32位小写十六进制随机ID。文档ID与工作区数据库中的规范化文件路径绑定，文件名仅用于候选显示。应用内重命名通过既有 `index_paths/previous` 更新原记录，保持ID不变；另存为与外部复制产生新记录。本功能不向正文新增身份头；既有块编排文件中的 `ResearchClip:document.id` 是编辑模型ID，不作为LinkIndex的文档身份，复制它不会合并数据库记录。单独拷贝Markdown到另一工作区不会自动带走映射；当前不宣称跨设备ID映射同步。

用户输入 `[[名称` 触发候选；选中插入 `[显示文字](researchclip://document/<id>)` 或 screenshot。只有严格类型和ID组成的URI可以在预览触发受管导航；路径、查询、片段与未知scheme不获得文件访问权限。没有选择目标的wiki文字仍是文字。同名目标按类型和短ID区分。

## 保存与链接索引

schema9新增 `link_index`、`link_index_state`、`document_tags`；001–008迁移不改，沿用升级前完整性检查、不可覆盖一致性快照、版本拒写与事务迁移。

LinkIndex存储sourceId、targetId、linkType、sourceLocation（UTF16起止偏移）、sourceLine、label、updatedAt。sourceId外键随登记删除清理；targetId故意无外键，保留失效目标。索引基于已保存文件，Web Worker复用既有CommonMark/math解析器，忽略代码、HTML、图片和公式；遍历语法树不沿引用递归。构建使用既有实体解码依赖的纯实现，避免其浏览器DOM入口在Worker中失败；已增加生产Worker无DOM回归。后台Rust任务只按已登记ID读取有界UTF8源文，返回SHA256摘要。提交时重新读取并比较摘要、校验位置、类型、ID与实际源文；单事务替换，旧任务不能覆写已变化文件的索引。UTF16到字节位置一次扫描转换，避免对每个链接重复扫描全文。

每份文档上限2MiB/5000条引用。当前文档保存或打开后更新；全量更新逐篇处理，支持取消、逐项失败清单与重试。取消保留已提交结果。外部文件修改须重新更新索引；未安装目录扫描或任意路径读取能力。

## 导航、反向链接与图谱

文档导航复用单活动文档状态和未保存确认，来源引用位置可延续到确认后的打开操作。卡片复用既有草稿与保存服务。反向区域给出来源文件、行号、显示文字和跳转入口，重复引用保留各位置；每区显示上限500并提示。

图谱为本机SQL节点/连线快照，使用SVG与简单确定性环形布局，无新全局状态框架。支持缩放、平移、节点键盘入口、项目和标签组合筛选、当前文档的直接邻居。截图沿用标签，文档使用document_tags。循环与自引不递归展开；缺失文件或删除截图显示失效节点。最多1000节点/5000连线，限制明确显示，不承诺大规模自动布局。

## 标准Markdown导出

仅允许当前有效、未被外部修改的文档授权，经原生受限保存选择器选择新路径。以已校验的索引转换文档URI为编码后的相对文件路径，跨磁盘/缺失目标/截图卡片转换为文字提示并返回警告。原文档和引用目标不修改；目标文档不自动复制，用户须保留相对目录结构。正文受管assets复用现有不可覆盖落盘管线；同名目标文件拒绝覆盖。

## 权限与边界

7个main-only有界IPC：link_targets、link_document_source、replace_document_links、link_details、link_graph、set_document_link_tags、export_linked_document。未授予通用fs、SQL或任意路径打开权限；保留skipHtml和原CSP。React转义候选文字，不执行OCR、Markdown、网页或同步数据中的脚本。无新第三方依赖、上传、账号、PDF或独立备份恢复功能。索引及文档标签当前仅本机。

实际验收见 [v3.1.5](qa/RELEASE-3.1.5.md)，操作见 [使用指南](USER_GUIDE.md)。
