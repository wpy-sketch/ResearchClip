# A11 图片区域锚点契约

## 身份与坐标

`AnnotationAnchor`包含anchorId、screenshotId、imageRevision、x/y/width/height、title、description，另有独立乐观锁revision。anchorId由后端生成32位小写十六进制随机ID，新增接口不接受指定旧ID；删除后不复用。screenshotId沿用素材库ID，imageRevision为受管原始PNG字节的SHA256。点宽高均为0；矩形宽高均大于0；有限浮点坐标均在原图内，尺寸上限沿用16777216像素。每图200锚点、标题200字、说明4000字。

原图不可变；普通Annotation v1版本变更不改变imageRevision，因此锚点仍对应原图区域。若受管原图被外部替换或损坏，锚点停止跳转而不猜测映射位置。锚点独立保存，不改变标注草稿的revision，不修改PNG、旧标注对象或标注历史。

## SQLite与受限接口

迁移010在现有验证、升级前安全快照、事务和回滚管线中创建annotation_anchors，并给link_index增加可空anchor_id；已发布迁移001–009保持原字节。截图删除级联删锚点，链接索引不外键删除目标，保留失效引用。

四个main-only命令：annotation_anchors（列表及原图版本）、resolve_annotation_anchor（精确匹配截图/锚点/原图版本）、save_annotation_anchor（创建或乐观锁更新）、delete_annotation_anchor（版本匹配删除）。复用LibraryService串行锁和spawn_blocking，只接受ID和有界业务数据，不接受SQL、文件路径或系统URL。读图沿用受管目录与PNG验证。

## 引用与跳转

格式：`[图片区域：标题](researchclip://screenshot/<32hex>/anchor/<32hex>)`。严格匹配大小写/层级；拒绝document锚点后缀、任意路径、百分号路径、查询参数、尾斜杠和脚本协议。只在应用Markdown预览中操作，不注册OS协议或增加deep-link插件。

选择器复用A10搜索分页，选截图后读取其锚点；插入前再解析目标并核对文档会话及光标。沿用Markdown insertText和撤销栈。源码模式插入光标，图文模式遵循现有追加文字块规则。

导航先解析精确目标，再通过现有openLibraryEditor打开图片；重复编辑会话保留标注草稿。新旧会话都核对实际Blob摘要，避免旧缓存定位到新原图。请求序号阻止旧异步结果抢占连续跳转。视口围绕原像素区域居中，缩放范围沿用编辑器上限；分栏改变重算视口，选中区域高亮3秒。右侧Markdown store不重开，不清空脏状态、光标或撤销历史。

## 普通Markdown、反查及兼容

A10后台CommonMark索引同时保存parent screenshot ID和anchor ID，源码UTF16范围仍经后端摘要/片段检查；图谱保留截图节点，反向链接指向来源行。锚点失效时本文引用区显示不可用，点击原预览给出明确失效错误。

标准Markdown副本导出将有效锚点写成标题、原图像素区域和说明；失效目标转为文字并列出警告。增强原稿不覆盖，不把专有URL作为唯一可读信息。现有卡片导出和PNG导出不改变。

本版锚点仅本机，不随WebDAV、可编辑项目JSON或标注历史快照复制；这些格式仍保持原契约，不伪称已同步锚点。内部URL在别的资料库不自动具有同一目标。未增加新依赖、外部账号、PDF或独立备份系统。

## 验证

单元测试覆盖原坐标/视口、点矩形边界/非有限值、ID与版本冲突、删除不复用、原图替换、schema9升级、索引/可读导出/失效及重开。Windows原生矩阵和实际结果记录于[本版验收](qa/RELEASE-3.1.7.md)；目标用例不冒充已执行结果。
