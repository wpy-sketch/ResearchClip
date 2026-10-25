# v4.2.1 正式发布回执

2026-10-26，[ResearchClip v4.2.1](https://github.com/wpy-sketch/ResearchClip/releases/tag/v4.2.1)已正式发布，非草稿/非预发布，Release ID423609505。应用标签固定cc20a25ce0e58c24e44ddee63df4b812bc80df5a；[61文件完整变更](https://github.com/wpy-sketch/ResearchClip/commit/cc20a25ce0e58c24e44ddee63df4b812bc80df5a)。本回执仅追加文档，不重写应用标签或安装包。

## 修复与使用变化

PDF模糊原因是显示端整页100万像素限制及较低的图像重采样阈值，原始PDF没有被重新压缩。本版按可见区域、DPR和实际CSS显示比例重新绘制，单块1024×1024像素，单会话/全局画布预算24M/72M像素。解码重采样阈值16MiB改为128MiB，与既有32Mi像素图像支持上限对应。主视图和缩略图共享页资源租约，最后使用者离开才清理。DPR事件遗漏有轻量检测补偿，关闭清理监听与缓存；文字层、选择复制、原阅读状态和原件字节保留。

Zotero配置助手进入“设置→Zotero与PDF”或PDF来源侧栏：复制当前researchclip.exe路径，手动在Zotero选择自定义外部阅读器；说明高级设置中的本机API授权及只读无需API key。可独立测试/取消连接并报告API/schema，按钮实际跳到指南第28章。正式版固定127.0.0.1:23119，不接收任意远程URL，不申请写入key、不写Zotero数据库；来源预览/确认继续复用稳定资源与文献模型。API不可用仍可阅读PDF。

界面使用面板容器与rem断点：工具条换行、窄PDF导航转上方、主导航收为图标轨道；保持原用户工作区、Markdown草稿与撤销，没有Markdown时没有空白分栏。图标重设计为原创四色论文页＋回形夹，SVG与五份原生图标同步。指南28章、191练习、51幅实际加载图片，新增4幅原生练习视图；截图中的32120测试端口明确区别于正式23119。

SQLite保持16，001–016迁移不改，无新增应用依赖，没有第二套Markdown/资源ID服务。没有Markdown导出PDF或独立数据备份/恢复系统。

[设计](../architecture/PDF_QUALITY_ZOTERO_421.md)、[修改文件/实际验收](RELEASE-4.2.1.md)、[机器证据](evidence/v421-editor-checks.json)、[使用指南](../USER_GUIDE.md)、[下载与历史配套](../RELEASE-COMPONENTS.md)。

## 实际测试

- npm ci --ignore-scripts：327包干净安装。npm run check：84文件、671项前端测试、类型/ESLint/生产构建通过。
- npm run format:rust、npm run test:rust、npm run check:rust：格式与Clippy通过，Rust272通过、7忽略；忽略不是通过。
- Windows原生63组：工作区7、命令8、资源6、引用11、PDF13、清晰度5、Zotero配置/来源13；指南浏览器14组，发布基础设施20项通过。
- 720万像素无损练习扫描图，在250%与模拟DPR2下与独立源像素比较：26576采样、平均绝对误差0；画布4194304像素，每块最多1048576。DPR变化、旋转、离屏释放与75–200% CSS缩放/窄屏重排通过。模拟DPR不能代替物理显示器验收。
- 最终NSIS安装、占用保护、PDF候选注册与默认/UserChoice保留、卸载清理5组通过；四份许可一致。最终原生回归与安装记录区间内23个默认资料文件摘要保持，未重置或恢复真实资料库。

NSIS SHA-256：621149919c5d35dd2618ff6a34dab08c58eb425cab335902fb9c9e0cf89d6c4b。安装器未签名。

## 附件与公开下载

291个附件全部实际匿名下载，大小和SHA-256与冻结清单一致，下载不携带Authorization。[只读验证40226277989](https://github.com/wpy-sketch/ResearchClip/actions/runs/40226277989)，job121268233648，见[完整下载证据](evidence/v421-public-verification.json)。接收器核对工作流身份、源提交及全部文件集合。

[历史继承40225746431](https://github.com/wpy-sketch/ResearchClip/actions/runs/40225746431)，job121266614805，复制283个非冲突既有附件，本地上传8个新增附件。此前40个版本2575项原附件前后审计均PASS。原223运输卷不重新压缩，同名SOURCE/SHA按原字节保存在版本Metadata.zip；V421清单逐项定位，不以外部链接替代附件。所有旧功能需要的旧配套文件继续保留。

## 未执行与人工验收

本机Zotero10.0.3的独立配置健康GET实际200、API3/schema44/实例标识。真实空库界面可读取，但激活输入返回GetCursorPos拒绝访问0x80070005；用户明确选择保留真实Zotero设置/双击附件链路为人工验收，本轮停止窗口操作。独立HTTP练习测试、原生argv交接和实际API健康读取不能冒称完整真实Zotero UI通过。没有修改真实Zotero偏好或直接访问其SQLite。

人工按指南27.8和28章核对自己有权限的论文：高倍小字/扫描细节、拖动窗口和字体重排，设置正确程序后双击个人与群组附件，核对来源、API关闭回退及重启。物理多屏DPI/Windows放大镜、旧WebView2、长期大文件内存及其他操作系统仍未验证。低清源图不能恢复不存在的细节；超过图像保护预算可能不完整，PDF.js整体解析堆不承诺OS硬内存上限。Zotero批注读写、批注页码传入、自定义API端口/远程URL仍不支持。

开发、最终检查、v4.2.1正式发布和全部附件验证完成。本轮停止；后续阶段需要用户新授权。
