# Step12：PDF选区摘录（设计与实现）

范围：v4.2.3。只截取ResearchClip已授权、已打开的PDF页，不调用Windows截图，不修改PDF原件，不进入下一阶段。复用PDF.js6.4.299（Apache-2.0）、PdfViewerSession渲染队列/页租约/缓存、现有Screenshot原图/标注/笔记、DocumentStore光标与相对图片保存、原文投影和图文块拖动。无新依赖。

核对依据：[官方示例](https://mozilla.github.io/pdf.js/examples/)、[PDFPageProxy](https://mozilla.github.io/pdf.js/api/draft/module-pdfjsLib-PDFPageProxy.html)及安装版本types/src/display/page_viewport.d.ts和api.d.ts（6.4使用convertToViewportPoint，未假定已移除的矩形转换方法）。PDF原生坐标与Canvas坐标不同，使用viewport的逆变换；不从显示图块或屏幕复制像素。

## 坐标、质量和生命周期

选区只在单页透明覆盖层内建立。client坐标按该页getBoundingClientRect归一化，再通过当前总旋转viewport转换到PDF原生坐标。持久化复用PdfLocation既有契约：未旋转CropBox左上角原点、物理PDF点（72点/英寸），考虑非零CropBox、UserUnit与页面内置旋转。页码零基。显示比例、CSS缩放和DPR不进入持久坐标。

输出选择144/216/288DPI，按所选DPI新建裁切画布，经PDF.js page.render平移变换生成PNG；限制边长8192、面积16Mi像素、原有全局画布预算，超限提示降低DPI或缩小选区。保存本次输出总旋转以便解释图片朝向。渲染可取消，关闭取消并清理画布/页租约；开始提交后显示保存状态，完成回执前不重复提交。受限PDF不提供摘录以免绕过既有内容复制权限。

## 数据和故障一致性

schema17只追加迁移；001–016原样保留。沿用一致快照、结构/完整性检查、逐版本事务、未来版本拒写。新增pending_pdf_excerpts扩展现有pending_screenshots意图，新pdf_excerpts表仅保存screenshot到既有PdfLocation的关联、可空literature ID、DPI和总旋转。位置仍由统一research_entities管理，不新增全局ID系统。

截图保存复用create_screenshot落盘流程：截图意图与摘录草稿同一事务先提交；原图、缩略图仍唯一命名且禁止覆盖；最后事务同时提交Screenshot、PdfLocation、来源关系、摘录记录及包含回跳链接的可编辑笔记，再清除意图。断电恢复复用现有pending恢复，保留未完成文件，不能留下成功截图却丢来源的状态。

查询返回screenshotId、literatureId（未关联时null，不伪造）、pdfAttachmentId、pdfContentHash、pageIndex、pageRectangle、imageAssetPath、createdAt，另含locationId、documentId、DPI和旋转。Rust核对主窗口、当前PDF授权、附件/摘要、坐标/尺寸/PNG预算、已有文献ID，不接受客户端文件路径或SQL。后端不新建PDF解析器；实际页面几何由受控PDF.js会话校验，不能将IPC校验声称为独立验证PNG内容与PDF完全一致。

## Markdown和回跳

捕获完成后可打开现有标注工具，保存标注再插入；原始PNG不可变。插入锁定目标DocumentStore，复用addImages的光标、撤销、相对assets、图片在上/注释在下和块排序；无Markdown时显式新建。摘录笔记含可读来源信息和researchclip://pdf-location/稳定位置ID，预览点击走受权限校验的pdf.navigateTo命令，核对摘要并打开受管PDF定位/标记选区。外部普通编辑器保留可读链接与源码，不执行脚本。

Windows原生回归观察到旧HTML dragTo没有重排，因此在现有VisualEditor手柄补局部Pointer捕获/插入线/边缘滚动，仍调用同一个moveSourceBlock和editSource；HTML事件路径保留，不改应用文件拖入配置，不新增拖动框架。Tauri官方配置说明见[dragDropEnabled](https://v2.tauri.app/reference/config/#windowconfig)，当前主窗口已是false，不能将失败简单归因为默认值或冒称浏览器拖动已通过原生验收。

## 验收和边界

单元覆盖旋转0/90/180/270、DPR/CSS缩放无关性、反向框选、CropBox/UserUnit、裁切预算、过期/取消；隔离SQLite覆盖16→17旧数据摘要/关联、重启、失败恢复和未知版本拒写。Windows真实PDF.js覆盖连续/跨页摘录、不同缩放/旋转/模拟DPR、插入/移动图注、保存重开与稳定来源回跳，区分模拟DPR与物理多屏。默认资料只读前后核对，所有首/次进程指定同一隔离设置目录；此前v4.2.2长区间两项新增来源未明的事实不抹除、不自动清理。

真实Zotero UI仍按用户选择人工验收；本阶段没有Zotero标准批注写回、Markdown导出PDF或独立备份/恢复系统。

新插入源码用顶层HTML注释`rc-pdf-excerpt:v1 <locationId>`与`/rc-pdf-excerpt`界定已有SourceBlock范围，整块包含图片、公式/列表/标题注释和来源链接，后续正文独立。解析只识别AST顶层HTML节点，围栏中的字面标记不生效；未知版本、嵌套或未闭合标记保护原文并停用拖动。普通编辑器仍可读，不使用AST重序列化。

实际原生测试还暴露了隐藏标签零尺寸、过期滚动帧与旧页码输入值之间的竞态。恢复先标记restoring，只有可测量可见面板才恢复；滚动帧核对导航代际与可见尺寸，页面禁用浏览器自动滚动锚定，页码草稿随导航序号失效。未改变其他工作区的业务状态。
