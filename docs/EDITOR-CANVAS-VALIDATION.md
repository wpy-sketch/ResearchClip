# 基础编辑画布复验

2026-10-13。本轮请求仅针对编辑工作区。已读取 AGENTS 和 ARCHITECTURE；复用此前建立的 React/TypeScript、Konva/react-konva 和 Zustand 模块，改动限画布行为、坐标规则与回归验证。此前已实现的标注/历史/项目文件功能保留，本轮没有增加标注工具。

## 请求与现有模块

| 要求 | 已有实现与本轮核对 |
| --- | --- |
| 独立页面/顶部工具栏 | src/features/editor/EditorView.tsx，由 app/App.tsx 延迟加载；截图完成后进入 |
| 中央图片画布 | EditorCanvas.tsx，原图在独立非交互 Konva Layer，图元在另一层 |
| 缩放/适应窗口 | geometry.ts 的锚点缩放与 fit；ResizeObserver 只在 fit 模式重新计算，manual 模式保留视口 |
| 平移 | 工具、空格与中键；本轮修复新平移手势被取消版本 effect 清除的问题 |
| 鼠标坐标 | CSS 客户区位置先扣 pan 再除 zoom；平移时状态栏使用更新后的视口，不额外除 DPR |
| 独立编辑状态 | store.ts 的每图 EditorSession，图片 Blob/独立 URL 与文档/视口分离；关闭预览不释放编辑器图片 |
| 可序列化 Annotation | types.ts 的版本化文档，唯一ID/原图像素尺寸与图元；没有 Blob、Konva 节点、URL 或视口 |
| 图片加载失败 | HTMLImageElement 加载错误、实际尺寸不符均可见提示，不修改原图 |
| 无损原则 | 视口操作不变更文档/源 Blob，截图交接和重新打开保持各图状态 |

## 本轮修复与测试

1. Pan 在 pointerdown 取消旧动作后立即建立，原 cancelVersion effect 随后将其清空。新增实际位移断言先复现 0 != 40；Pan 保存开始时的取消版本，只有后续取消才清掉该手势。工具、中键、空格三种入口均验证真实位移。
2. 平移后的坐标提示改用新视口；空格快捷键限定于画布焦点，顶部“适应窗口”按钮能正常用空格激活。
3. 旧 2% 最低缩放会让超宽图片不能完整适应窄画布。最低缩放根据现有像素上限与最小客户区设置，宽/高极端比例均完整 fit；低于2%时缩小不会突然放大，缩放标签保留有效数字。
4. 正向/逆向坐标转换都拒绝非法输入，补图片尺寸、极端比例、负平移、小比例坐标往返测试。
5. 新增 app/open-editor.test.ts：关闭截图预览只撤销预览 URL，编辑器的独立 URL/Blob 和文档保留；重复打开保持视口/对象，坏资源不改变导航/已有会话。

## 实际执行

环境沿用 [编辑器历史验收](EDITOR-VALIDATION.md) 的 Windows x64 / Node 22.22.2 / Rust MSVC 1.99.0 / Tauri 2.12.3。

| 命令 | 结果 | 退出码 |
| --- | --- | --- |
| npm run check | 类型、ESLint、109项/14文件、Vite生产构建通过 | 0 |
| npm run test:rust | 原有32项通过，含截图坐标/裁切/会话/PNG，Rust源码未改 | 0 |
| npm run tauri -- build --debug --no-bundle | Windows debug exe生成 | 0 |
| npm run desktop:build | Windows release exe生成 | 0 |
| npm run test:smoke:editor -- canvas-framework-fit-fixed | Edge真实画布交互12组通过，errors=[] | 0 |
| npm run test:smoke:editor -- canvas-framework-dpr2 --dpr-2 | 同样12组，浏览器DPR=2通过 | 0 |
| npm run test:smoke -- --run-id canvas-framework-workflow | 原有导航、截图入口、32布局/主题组合，以及选择层DPR1/1.25/1.5/2 mock通过 | 0 |
| npm run test:smoke:editor:native -- canvas-framework-native | 实际Windows编辑器/项目工作流6组通过，图片明确为fixture | 0 |
| capture_probe -- 1 | 原生主屏捕获成功、PNG头尺寸匹配2560×1440；GDI计数1→1；非法显示器拒绝 | 0 |

新增浏览器组检查实际三种平移位移及坐标提示、按钮空格激活、fit模式随窗口缩小、manual模式随窗口变大仍保留视口、16384×512源图在260 CSS像素工作区完整显示且继续缩小。260像素是显式组件测试客户区，不宣称桌面最小窗口允许260像素。

原有工具交互作为回归执行，未据此增加功能；浏览器fixture/overlay mock不作为原生截图证据。独立GDI探针本轮成功，之前PERMISSION_DENIED未复现，原因仍不确定，不能扩展为所有显示配置可用。

实际应用内全屏/区域到编辑器验收见下一节。所有失败报告保留：canvas-pan-before-fix复现产品平移错误；canvas-framework-final的480窗口工作区实际大于预想260px，保持“低于2%”断言并明确组件宽度后通过。canvas-real-screen首次遇到异步设置保存尚未完成，第二次尺寸状态栏包含子节点，测试exact文本定位错误；分别补提交等待、按状态栏尺寸前缀校验，不改变生产接口或减少像素断言。

## 边界与手工检查

本轮不增加依赖或Rust/权限/数据库变更，未删除数据或覆盖既有文件。最小正缩放仅保障坐标数学与fit，不意味着WebView能解码任意极端尺寸图片；解码失败仍按既有错误流程处理。

需要单独复验真实Windows125/150/200%及混合DPI、副屏/负原点捕获、显示器插拔/HDR/远程桌面、复杂字体/触控及macOS。下一步是这些手工矩阵，本轮不继续开发后续功能。

启动：npm run desktop:dev；截图后选择“打开编辑器”。模块结构和完整使用方法见 [README](../README.md) 与 [ARCHITECTURE](ARCHITECTURE.md)。

## 应用内真实截图结果

npm run test:smoke:desktop -- --run-id canvas-real-screen-3 退出0，29组检查通过，errors=[]。在真实主屏上的独立测试浏览器显示色块，图案没有传入Rust捕获API；实际GDI读取屏幕。主屏物理2560×1440，测试浏览器DPR=1；副屏2560×1600、原点(-2560,0)仅枚举，本轮不称其捕获验收通过。

- 启动与重启各两张全屏PNG：物理尺寸匹配，9个屏幕标记全部匹配，覆盖层/主窗口排除与图像方向检查通过。
- 实际原生PNG进入Editor，中央图片层有像素，平移后图像边界按40/30 CSS像素移动；关闭截图预览、重新进入编辑器后，独立图像和原始尺寸仍在。
- 无效/缺失显示器、并发BUSY、错误后恢复、Blob替换和清理通过。
- 区域层焦点/物理边界、实时选区尺寸、冻结原生帧与裁剪PNG资源/预览像素一致；零面积、反向拖动、Esc、边界外指针夹取通过。
- 连续20次区域会话（10次预览、10次Esc），唯一会话、覆盖层销毁、主窗口焦点恢复，无自动文件；main与overlay权限和资源ID隔离通过。
- 重启再次检查全屏、区域和编辑器交接，偏好恢复后通过界面切回共享测试初态。未清空真实AppData/用户设置。

该报告原区域检查文字仍含旧“file/preview”描述，实际断言已读取受限PNG资源而不创建临时文件；源脚本现已更正为resource/preview。结果只证明本轮主屏场景，未宣称所有显示器、物理快捷键或全部Windows配置通过。
