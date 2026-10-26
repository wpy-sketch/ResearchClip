# v4.2.5 只读Zotero批注验收

目标：Step14，在现有PDF.js、已确认来源、受限Local API和SQLite中增量完成只读适配。设计先行记录见[架构](../architecture/ZOTERO_ANNOTATIONS_STEP14.md)，操作见[指南第31章](../USER_GUIDE.md)。未执行项目不视为通过，正式发布与公开下载另有发布回执。

## 修改和技术设计

- Rust：services/zotero_annotations模型与协议校验、zotero_local只读children分页、commands/zotero_annotations两个main-only IPC、storage/zotero_annotations及019事务迁移。原件授权、完整来源、Server ID和PDF哈希必须同时匹配，版本不一致、取消、超限、网络错误不提交部分缓存。
- 前端：ZoteroAnnotationIdentity、ZoteroAnnotationRepository、AnnotationCoordinateAdapter、AnnotationReadCache集中在pdf/zotero-annotations；现有PdfViewerSession持有并释放仓库，PdfViewer提供导航，PdfPage仅为可见页叠加。没有新ID系统、状态框架或第二PDF解析器。
- 缓存：自然复合身份含实例/库类型与ID/附件/PDF哈希，原annotation key保留；20项分页、最多1000条，8MiB每快照、64MiB总量。版本和缓存时间明确显示，ink/未知位置保留诊断，不创建私有笔迹。
- 界面与文档：只读来源标识、独立滚动与换行；assets/app-icon.svg及Tauri生成图标；指南、导航、README、架构、发布说明和既有发布脚本追加425。没有第三方新增依赖，锁文件仅应用根版本变化。

## 实际检查结果

最终图标/指南后，npm ci --ignore-scripts干净安装327包；npm run check完成90文件724测试、类型/ESLint/生产构建，退出0。Rust npm run test:rust在RUST_TEST_THREADS=4下291通过、7忽略、0失败，退出0；npm run format:rust和npm run check:rust退出0。逐项比较npm和Cargo锁文件，只有应用根版本改变，依赖内容完全相同。

Windows内嵌资源debug构建和正式NSIS构建退出0。最终原生28组批注用例通过（含13组既有Zotero与15组新增）：分页22→21去重、五类型与未支持诊断、文字/颜色、四方向下划线、跨页/CropBox/UserUnit2、窄窗缩放、两资料库相同key隔离、缓存/重读/403重连/取消/分页版本冲突/实例及库冲突、IPC授权和真实进程重启。84次HTTP记录全部GET；不是实际Zotero窗口验收。关闭进程后只读检查隔离数据库，两个来源缓存各21条、共42身份，没有新增私有手写实体。

另有工作区7、命令8、旧资源迁移6、引用11、PDF13、清晰度5、混合布局5，以及Step12/13综合25组通过；连同批注28共108组Windows原生用例，退出0。指南浏览器14组通过，31章229练习62图实际解码；23项发布基础设施测试通过，退出0。一次unittest discover因文件名含连字符未找到测试，保留零测试结果；之后逐文件实际执行，未把零测试称为通过。

NSIS实际安装、占用保护、PDF候选及默认保留、卸载/清理5组通过，退出0，四份许可核对一致，安装残留为false。安装器未签名，SHA256：3d94810a6313999b1c31f4f51f7e10c23b0ae53d524ec541052d7839dcc8741d。默认资料目录只读核对27文件、0变化、PASS；安装区间也独立核对通过。详见[机器证据](evidence/v425-editor-checks.json)。

## 失败修复记录

1. HTTP测试夹具误把/api/?判为非根路由，触发断言且析构二次panic；修正路由判断及panic保留方式。空TCP探测连接不含HTTP请求时跳过，GET/实例头断言保留。完整Rust重跑通过。
2. Clippy指出嵌套if可合并及测试模块后有项目，修正测试代码结构后重跑通过。
3. 实际WebView百分比CSSOM只保留六位有效数字，原1e-7归一化容差过严；按小于0.002物理像素级误差改为1e-6，保留四旋转独立校准值。来源文字跨行匹配也保留页码断言。
4. 虚构群组夹具第二页较新高亮未加群组评论，导致库隔离预期失败；补全夹具第二页，未改变产品隔离逻辑和断言。
5. 最终全并发Rust出现一次500ms本机连接超时；夹具从逐字节改为块读取，完整套件限制4线程重跑通过，产品网络预算保持不变。原失败日志保留。
6. 指南服务器端口1437被Windows拒绝，改用显式测试端口14377，完整14组重跑通过。工作区旧断言仍要求30章，更新为当前31章/62图后完整回归通过。
7. 首次NSIS验收与隔离原生测试进程重叠，安装测试按原规则拒绝。未关闭用户进程或放宽保护；待原生测试正常退出后，使用新证据目录顺序重跑，5组通过。

## 未完成、已知问题与人工验收

真实只读GET探测确认Zotero10.0.6/API3/schema44；20样本均为ink，另一次100项排序读取5秒超时保留为失败。没有从真实用户文献创建/改动批注；真实Zotero窗口此前因桌面输入访问拒绝，用户明确选择保留人工验收，不重试窗口控制。7项Rust历史忽略不算通过。

在专用测试文献中分别创建高亮、下划线、便签、区域及文本批注，再通过Zotero外部阅读器打开ResearchClip，确认来源并读取，核对原key、库、页码、颜色、评论、四旋转和跨页。关闭Zotero后核对缓存时间，重连/取消/重复读取，重启ResearchClip检查缓存；个人与群组相同key需独立核对。物理混合DPI/多屏、触控、旧WebView2、其他系统及长期大文件内存另行验证。

此前v4.2.4新增schema16快照已有用户手动启动确认，原FAIL/快照保留；此确认不覆盖v4.2.2两项来源未明新增。没有删除、恢复或重置真实资料。本版本独立27文件审计通过，不改写旧版失败证据。

完成后停止；下一阶段须用户另行指定，标准批注写回与私有手写没有本轮授权。
