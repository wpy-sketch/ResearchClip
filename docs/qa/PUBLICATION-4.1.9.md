# v4.1.9 正式发布回执

2026-10-25，[ResearchClip v4.1.9](https://github.com/wpy-sketch/ResearchClip/releases/tag/v4.1.9)已正式发布，非草稿/非预发布，Release ID423381500。应用标签固定5358b2203bb29f35df9416c0a127a0c398d063e5；[75文件完整变更](https://github.com/wpy-sketch/ResearchClip/commit/5358b2203bb29f35df9416c0a127a0c398d063e5)。本回执仅追加文档，不重写应用标签。

Step10完成本地只读PDF标签、连续单双列、缩放旋转、目录/缩略图、文字选择复制、逐页全文搜索、页码和阅读位置、多个PDF及Markdown并排。按视口渲染、缓存/输入预算、进度/取消、密码/受限/损坏处理及关闭释放已落地。复用原稳定资源ID、文件服务、工作区与关系层，schema15仅存阅读位置，001–014不改。

[设计](../architecture/PDF_READER_STEP10.md)、[验收与修改范围](PDF_READER_STEP10.md)、[机器证据](evidence/v419-editor-checks.json)与[下载/附件说明](../RELEASE-COMPONENTS.md)。固定pdfjs-dist6.4.299，本地匹配Worker与必要解码/许可，不执行PDF脚本或附件动作，不开放通用fs/shell。

279附件全部实际匿名下载，大小/SHA-256与冻结清单一致。[只读验证40193074936](https://github.com/wpy-sketch/ResearchClip/actions/runs/40193074936)，job121168040662；不携带下载Authorization。接收器核对工作流身份、源提交和所有摘要，见[完整下载证据](evidence/v419-public-verification.json)。

[继承40192505614](https://github.com/wpy-sketch/ResearchClip/actions/runs/40192505614)，job121166318338，核对并补入271个历史附件；8份本版新文件由本机上传。此前38版2011个原附件前后审计通过，名称/ID/大小/摘要/标签未变。223运输卷原字节保留，历版安装器/便携包/指南完整承接；冲突SOURCE/SHA按原名放入各版Metadata.zip，V419-HISTORY-INVENTORY.json逐项定位，不以旧链接替代附件。

实际检查退出0：npm干净安装，前端657项/79文件、类型/lint/构建；Rust259通过/7忽略、格式和Clippy；45组Windows WebView2原生、14组指南浏览器、20项发布基础设施检查。NSIS安装/占用保护/卸载/清理4组通过，四份许可摘要匹配，既有16个AppData文件只读摘要不变，隔离安装已移除。

NSIS SHA-256：4c4c8032f78755d86dea95671f61e82b6f4ff6623efb7e1bc44a4120d90c1e06。安装器未签名；Vite原有大块提示保留。指南27章177练习44图，包括3幅本轮实际Windows阅读界面。沿用原创多色烧瓶图标，PDF工具条复用明暗主题与圆角/焦点样式。

已修复并复验：前端schema15契约遗漏曾拦下旧素材；布局恢复曾选回第一个标签；空Canvas默认尺寸占用；PDF.js6.4接口差异；Node递归复制异常。保留失败证据并重跑；指南测试1437被Windows排除，改27437通过，没有更改系统规则或删除断言。

未验证边界：7个忽略项不计通过；物理多屏DPI/系统放大镜、长期内存、稀有字体与超大复杂PDF仍需人工。只测当前WebView2 155，不冒称旧运行时通过。缓存有预算，PDF.js内部堆不承诺绝对OS硬隔离；图像超过保护尺寸可能不完整，位置恢复按页内比例，不保证旋转后同一文字行固定。真实用户库未用来迁移测试。

Windows人工验收按指南27.8，以独立练习PDF核对视图/选择复制/搜索/分栏/隐藏/重启/密码与错误，核对原件摘要；安装前保存退出旧实例。旧版拒写新库，不能重置真实资料绕过错误。没有Markdown导出PDF、独立备份恢复、PDF批注写回或手写业务。

Step10开发、安装验收、v4.1.9发布及279附件验证完成。按用户随后明确授权继续Step11，目标v4.2.0；Step11尚未发布，不把其预审或后续未提交代码计入本应用标签。
