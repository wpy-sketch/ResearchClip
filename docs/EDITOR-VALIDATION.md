# 图片编辑器：历史、无损保存与导出验收

2026-10-13。本轮完成区域截图快捷键设置名称、有界撤销/重做、独立项目保存/重载、标注 PNG 导出及合成图片复制，没有接数据库或 Markdown。

## 修改与契约

- editor/types、store、history：每张图片独立 past/future。新建、删除、移动、调整大小、端点、文字和属性更新统一记录；一次手势只提交一次，无变化/取消不提交，新编辑清空 redo。Ctrl+Z、Ctrl+Y、Ctrl+Shift+Z和工具栏按钮；输入框/文字对话框保留输入自身撤销。最多100步、单图快照UTF-8编码预算8 MiB，超限丢最早记录；原图、选择、视口不进历史。
- serialization/model：AnnotationDocument schemaVersion=1，原图尺寸和唯一ID对象；数组为层级顺序，全部原图像素坐标。校验版本、类型、有限坐标、样式、文字及路径数量。没有Konva节点、Blob、URL、选区或历史。
- api与Rust services/editor、commands/editor：系统对话框另存 *.researchclip.json + 同目录唯一 original-*.png；不覆盖原图或已有资料，重新打开还原可编辑对象。项目与标注版本独立均为1，没有数据库迁移/版本转换。未知版本/缺图/损坏/超出目录引用拒绝加载，当前编辑会话不变。JSON提交失败保留本次原图副本并提示，没有孤立文件清理。
- export/AnnotationNode：共用图元属性，独立Konva画布按原图尺寸与pixelRatio=1渲染；原图置底，对象顺序叠加，保留透明度和本机字体。不接收视口/选择/Transformer/工具栏；结束释放Stage/ImageBitmap。合成结果通过官方插件write_image复制真实RGBA。
- main仅新增4个有界Rust用例；Raw IPC为4字节小端JSON长度+UTF-8 JSON+PNG，最多4 MiB文档/约65 MiB PNG；路径只来自系统对话框。overlay仍5项权限。PNG完整流校验到IEND，RGB/灰度/调色板转换RGBA，拒绝动画，保留16,777,216像素限制。
- SettingsView入口明确为“区域截图快捷键”，复用已有官方插件注册、冲突回滚和退出注销，没有新增监听器。

## 实际执行

环境：Windows x64（.NET报告Windows NT 10.0.26200.0）、Node 22.22.2、Rust MSVC 1.99.0、Tauri 2.12.3、Konva 10.7.1/react-konva 19.3.0。WMI系统信息查询被拒绝，未据此推测CPU/GPU/RAM，也没有性能验收结论。

| 命令 | 本轮结果 | 退出码 |
| --- | --- | --- |
| npm run check | TypeScript、ESLint、Vitest 103项/13文件、Vite构建通过 | 0 |
| npm run test:rust | 32项通过，含实际文件重载、禁止覆盖、未知版本/损坏保留、透明/截断PNG及原有坐标/迁移测试 | 0 |
| npm run format:rust / check:rust | fmt、全target clippy -D warnings通过 | 0 |
| npm run tauri -- build --debug --no-bundle | 实际Windows debug exe生成 | 0 |
| npm run desktop:build | 实际Windows release exe生成，无安装包/签名 | 0 |
| node scripts/editor-smoke.mjs history-export-final | Edge实际指针/快捷键/PNG像素9组通过，无pageerror | 0 |
| node scripts/editor-smoke.mjs history-export-dpr2 --dpr-2 | 同样9组，浏览器DPR=2通过 | 0 |
| node scripts/smoke.mjs --run-id history-workflow-final | 原有导航/主题/截图入口/32布局组合及选择层DPR1/1.25/1.5/2 mock通过 | 0 |
| node scripts/editor-native-smoke.mjs native-final | 最终debug构建6组实际Windows文件/剪贴板/重启通过，errors=[] | 0 |
| node scripts/workflow-smoke.mjs settings-history-final --settings-only | 5组实际偏好写入/OS热键冲突/释放/目录取消/重启通过，errors=[] | 0 |

浏览器交互覆盖缩放/平移后的拖动和Transformer原图坐标、各工具、真实Ctrl+Z/Ctrl+Y/Ctrl+Shift+Z、一次拖动一条历史、JSON重载后继续编辑/撤销。单测完整撤销/重做100步，并检查按字节预算限制历史。

实际128×96导出像素：空白角[0,0,0,0]；半透明原图[80,120,159,128]与源图一致；矩形红边、箭头绿头、蓝色顶层笔画均匹配；文字84个有效像素、0个手柄色像素；视口改为3倍且选择不同对象后整图RGBA摘要一致。渲染前后源PNG字节完全一致。DPR=2重复全部断言，未冒充Windows系统200%硬件验收。

native-final使用普通Tauri debug构建，通过两个明确测试标志加载固定320×200 PNG（不是屏幕图像），完成：

1. 系统另存为+Raw IPC写JSON与原图，原始PNG全部字节一致。
2. 系统打开对话框恢复可编辑对象，Delete与Ctrl+Z有效。
3. 系统导出PNG，实际320×200，内容与原始图片不同。
4. 官方插件复制合成图，另一Edge图片接收页粘贴，全部RGBA像素与导出PNG一致。
5. 真实另存为取消，不增加文件、不改已有资料。
6. 实际退出、重启、系统对话框重开项目，标注对象恢复。

测试报告位于artifacts。nativeCaptureVerified不会因fixture测试改为true。SQLite存储/迁移源码保持不变；未清空真实AppData或用户数据。

## 发现与修复

首轮Rust测试发现像素解码后没检查PNG末尾，补Reader.finish()后截断被拒绝。clippy提示旧关闭条件可合并，合并后仍保留异步注销与正常WebView销毁。

history-ui-1在Vite懒依赖优化时发生测试页面重载；先通过实际UI加载编辑器再注入fixture后，后续9组通过。native-1至native-5保留失败报告：打开文件名Edit为1148，与另存为1001不同，UIAutomation未暴露期望按钮/编辑控件，另一次保存控件尚未就绪。通过限定测试PID/窗口的控件编号及就绪检查修复辅助程序；未改变生产对话框或降低重载/导出断言。native-6和最终native-final均完整通过。

早期基础工具96项/7组是历史结果，不代替本轮新增验收。

## 限制与人工复验

- 原生屏幕读取仍受之前Windows PERMISSION_DENIED阻塞，原因未确定；fixture只证明编辑器/文件/剪贴板，不能宣称真实截图全链路通过。
- Paint/Word/GIMP物理Ctrl+V、真实剪贴板占用、只读目录/磁盘满、复杂字体/输入法/触控、Windows DPI硬件矩阵、HDR及macOS未验证，没有性能p95结论。
- 没有自动保存，退出前须另存项目；重开不恢复历史/视口。移动资料须同时保留JSON和同目录原始PNG。JSON提交失败可能留下本次原图副本，明确报告且不删除用户文件。
- 无旋转、多选、层级重排、数据库或Markdown；当前资源上限比完整MVP规划更保守，完整M3/M5未验收。

## 运行与复验

```powershell
npm run desktop:dev
# 设置 -> 区域截图快捷键
# 编辑器 -> 标注 -> 撤销/重做 -> 另存可编辑项目
# 退出重开 -> 截图页“图片编辑器 / 打开项目” -> 打开项目
# 导出标注PNG / 复制合成图片；截图页仍输出原始截图

npm run tauri -- build --debug --no-bundle
node scripts/with-rust.mjs cargo build --manifest-path src-tauri/Cargo.toml --locked --example workflow_input
npm run test:smoke:editor:native -- another-unique-id
```

每次使用新ID，文件仅在独立artifacts测试目录。结果不代替 [截图工作流验收](SCREENSHOT-WORKFLOW-VALIDATION.md) 中仍未通过的硬件场景。

## 基础画布后续复验

下一次基础画布请求已复用本文件所述模块，修复真实Pan位移为零与最低缩放导致fit裁切的问题。最新109项前端/32项Rust、浏览器DPR1/2和实际主屏全屏/区域到编辑器/重启链路通过；此前Windows PERMISSION_DENIED本轮未复现。详见 [基础画布复验](EDITOR-CANVAS-VALIDATION.md)，本文件的旧失败和人工矩阵继续保留。
