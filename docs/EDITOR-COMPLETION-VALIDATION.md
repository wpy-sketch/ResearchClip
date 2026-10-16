# 基础标注、历史、无损项目与 PNG 导出复验

2026-10-14，Windows / PowerShell，Asia/Hong_Kong。复用 EditorView、EditorCanvas、AnnotationNode、Zustand 会话和 Rust 项目服务，不新增数据库或 Markdown。历史记录见 [编辑器验收](EDITOR-VALIDATION.md) 和 [基础画布复验](EDITOR-CANVAS-VALIDATION.md)。以下为本次实际结果。

## 实现与修改

- 现有工具包含选择/拖动/Delete、矩形与椭圆手柄缩放、箭头端点、连续向量画笔、文字插入/双击修改，以及颜色、线宽、字号。唯一ID、JSON对象和原图分离，原图坐标不随视口缩放改变。沿用现有状态管理。
- 修复 model.moveArrowEndpoint：导入箭头若首点有非零局部偏移，移动终点应保留首点；移动起点保留原终点。新增 zoom/pan 坐标、重载、undo/redo、单次提交、失败保留文档/历史的单测。
- 修复 AnnotationNode 的变形失败恢复：尺寸校验拒绝或缩放值无效时，恢复Konva节点位置与scale，显示错误，保持JSON/历史不变。测试通过明确注入无效transform事件覆盖错误路径；其余正常拖动使用真实鼠标，未将注入事件冒充手势测试。
- 沿用各图独立的有界历史，Ctrl+Z、Ctrl+Y、Ctrl+Shift+Z 和工具栏操作有效。创建、删除、移动、缩放、属性更新均记录；一次手势提交一次，新编辑清空redo，取消/无变化不记录。最多100条和8 MiB向量快照，原图与视口不进历史。
- 沿用版本1项目JSON与唯一原始PNG。未来版本、损坏、缺图或越界引用拒绝加载并保留会话；另存和导出禁止覆盖用户文件。重开恢复可编辑对象，但不恢复历史/视口。JSON提交失败保留本次原图副本并报告，不删除文件。
- Rust文件往返测试扩充为文字、矩形、椭圆、偏移箭头与画笔，检查顺序、原始PNG字节及各图元无效数据。
- 浏览器脚本新增导入偏移箭头的真实端点拖动、单步历史、undo/redo/Esc，以及Esc取消真实Transformer调整大小。
- Windows脚本通过实际UI创建五类图元，经系统Save/Open对话框保存、重载、删除/撤销、重启恢复，再导出和复制合成图片。
- 保留上轮独立全屏快捷键并收尾注册生命周期，见 [双快捷键验收](SHORTCUT-VALIDATION.md)。

主要修改文件：src/features/editor/model.ts、AnnotationNode.tsx、model.test.ts、store.test.ts；src-tauri/src/services/editor.rs内测试；scripts/editor-smoke.mjs、editor-native-smoke.mjs、workflow-smoke.mjs；README、ARCHITECTURE、ROADMAP及本次验收文档。未升级依赖或修改SQLite迁移。

## 实际执行

下表退出码均为0。Node22.22.2/npm10.9.7、RustMSVC1.99、Tauri2.12.3，其他依赖使用现有锁文件。

| 命令 | 本次结果 |
| --- | --- |
| npm run check | 类型、ESLint、14个文件112项测试及Vite生产构建通过 |
| npm run format:rust | 格式检查通过 |
| npm run test:rust | 39项通过，项目文件往返涵盖全部图元 |
| npm run check:rust | clippy --all-targets -D warnings 通过 |
| npm run tauri -- build --debug --no-bundle | Windows debug EXE通过，实际用于下列原生运行 |
| npm run desktop:build | Windows release EXE通过；无安装包、签名或macOS构建 |
| npm run test:smoke:editor -- annotations-history-oct14 | DPR1，13组实际画布检查通过，errors为空 |
| npm run test:smoke:editor -- annotations-history-oct14-dpr2 --dpr-2 | DPR2，14组通过，增加手柄调整期间Esc取消，errors为空 |
| npm run test:smoke:editor:native -- annotations-history-oct14-all-types | 实际Windows应用6组通过，真实系统对话框、文件、重启与图片剪贴板 |
| npm run test:smoke:editor -- annotations-final-oct14-fixed | 最终修复版本DPR1，15组通过，包含无效变形恢复 |
| npm run test:smoke:editor -- annotations-final-oct14-dpr2 --dpr-2 | 最终修复版本DPR2，15组通过 |
| npm run test:smoke:editor:native -- annotations-final-oct14-native | 最终Windows debug版本6组通过，全部图元与实际剪贴板再验证 |
| npm run test:smoke -- --run-id annotations-history-oct14-ui | 6组含32种界面尺寸/主题组合；区域层mock在DPR1/1.25/1.5/2通过，不代表原生捕获 |
| npm run test:smoke:workflow -- independent-hotkeys-oct14 --settings-only | Windows两组快捷键注册生命周期7组通过；不触发截图 |

报告位于独立artifacts/editor-annotations-history-oct14、editor-annotations-history-oct14-dpr2、editor-native-annotations-history-oct14-all-types、editor-annotations-final-oct14-fixed、editor-annotations-final-oct14-dpr2、editor-native-annotations-final-oct14-native、annotations-history-oct14-ui、workflow-independent-hotkeys-oct14目录。测试配置与文件均隔离，不重置真实AppData。最终变形修复后重跑check及debug/release构建，均退出0。

实际PNG像素：128×96；改变视口/选择后RGBA哈希一致；透明角[0,0,0,0]；半透明区域与源图解码一致；矩形红色、箭头头部绿色、交叠上层蓝色；文字区域84个有色像素；编辑手柄颜色像素为0；源Blob字节不变；JSON重载后文字可修改/撤销。DPR1/2结果一致。

原生编辑器使用明确标记的320×200受控透明PNG，不冒充屏幕截图。五类图元按原数组顺序、唯一ID保存，原始PNG字节一致，合成导出320×200。官方插件写真正图片到Windows剪贴板，由独立Edge查看器读取，与保存的合成PNG全部RGBA像素哈希一致。重启恢复五类对象；取消另存保留原图/项目/导出三份文件。

## 失败、限制与下一步

初次沙箱内check退出1：Vitest遇到Windows沙箱临时目录ENOENT，9个文件88项通过、5个文件未加载；没有删除断言。获执行授权后重跑完整check，112项通过。沙箱内Rust格式检查因路径访问拒绝退出1，授权后同一检查退出0。

editor-annotations-final-oct14报告退出1：新增错误恢复测试预期“尺寸”文案，但既有nonEmpty校验实际返回“标注数据无效。”；节点恢复断言已通过。将错误断言改为精确匹配两种实际文案，保留位置/scale/文档/历史断言后，最终DPR1/2各15组通过。失败报告保留，未修改生产校验以迎合测试。

本次真实屏幕快捷键诊断退出1，Windows返回PERMISSION_DENIED；旧主屏成功报告不替代此次失败。受控图片验证不能证明当前真实截图链路，详见 [双快捷键验收](SHORTCUT-VALIDATION.md)。上轮审批额度耗尽导致的未执行诊断，本轮取得授权后重试，没有绕过审批。

仍需人工复验当前桌面的物理快捷键和真实截图到编辑器、Paint/Word物理Ctrl+V、剪贴板占用、只读/磁盘满、复杂字体/输入法、实际WindowsDPI、多显示器负原点、HDR、旋转、插拔、远程桌面与macOS。浏览器DPR2不替代硬件缩放；没有性能p95结论，完整MVP未验收。

运行 npm run desktop:dev。标注后“另存可编辑项目”，将JSON与同目录original PNG一起保留；重启从“图片编辑器 / 打开项目”恢复。下一步建议完成上述人工复验；数据库与Markdown需新请求。本次完成后停止。
