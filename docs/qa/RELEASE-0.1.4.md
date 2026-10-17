# v0.1.4 高级标注验收

本轮只新增马赛克、局部模糊、编号并发布 v0.1.4，不新建工程、不更改截图实现或数据库 schema。执行环境沿用 Windows NT 10.0.26200 x64、Node 22.22.2、npm 10.9.7、Rust MSVC 1.99、Tauri 2.12.3、Konva 10（锁文件沿用）。不新增依赖或权限。

## 实现及契约

- `types.ts` 扩展 Annotation 联合：mosaic/blur 的 width、height、strength；number 的 value、size；共同保留唯一 ID、原图 x/y、color、strokeWidth。schemaVersion=1 保留，旧程序安全拒绝未知类型，现有旧项目可继续读取。
- `model.ts`、`serialization.ts`、`store.ts` 复用已有绘制、移动、缩放、JSON 校验、每图会话及有界 history。矩形手势结束只提交一次；换工具/Esc 取消。数值范围 1–9999、圆直径 16–200，手工改数字不改变任何其他对象；默认最大剩余数值+1，删除最高编号后允许复用该编号。
- `effects.ts::renderEffectPatch` 从不可变原图按 floor/ceil/边界夹取裁出像素，直接调用已安装 Konva Pixelate / Blur 函数处理 ImageData，失败抛出；不是半透明色块，也不依赖吞掉滤镜异常的缓存回退。查阅 [Konva 官方滤镜 API](https://konvajs.org/api/Konva.Filters.html)，并核对锁定库的实现和类型后编译验证。
- `EffectPatch.tsx` 将计算出的真实像素同步到现有底图 Layer，清理旧画布；导出复用相同函数。局部 clip + copy 合成替换原像素，避免透明像素叠加后变得更不透明。滤镜先作用原图；矢量对象始终在上方，重叠滤镜后者覆盖而非叠加强度。
- `AnnotationNode.tsx` 复用现有 Transformer 和拖动历史，编号保持圆形。`EditorCanvas.tsx` 沿用原图坐标转换及缩放/平移；属性栏固定高度、窄窗口横向滚动，避免切换工具后画布跳动。
- `export.ts::renderComposite` 是 PNG、剪贴板、贴图、Markdown 合成和 OCR 的共同入口。输出原分辨率，无选框/手柄；临时滤镜画布在 finally 清理，不修改原始 Blob。
- Rust `services/editor.rs::validate_document` 同步检查新对象、强度、编号及滤镜有效区域总面积上限 16,777,216；裁切到实际图像再计数，重叠区域累加。文件/SQLite 继续使用已有受限命令与安全保存，不增加迁移。

## 实际检查（持续补记，未完成项不标通过）

| 命令 | 结果 |
| --- | --- |
| `npm run check` | PASS/0：33 文件、234 项；类型、ESLint、生产前端构建 |
| `npm run test:rust` | PASS/0：107 项、0 失败/忽略；含高级 JSON/原图/安全重开验证 |
| `npm run check:rust` / `npm run format:rust` | PASS/0：Clippy 全 targets -D warnings、fmt |
| `node scripts/advanced-renderer-smoke.mjs v014-initial` | PASS/0：4 组真实 Edge 像素/预览检查；不代表原生捕获 |
| `node scripts/with-rust.mjs tauri build --debug --no-bundle` | PASS/0：Windows 原生调试程序，属性栏稳定布局后重建 |

像素实测：马赛克相邻块内像素均 `[128,128,100,255]`；模糊区域方差从 16256.24 降至约 0.25，强度增加后约 0.147；透明区域 alpha 保持 128；区域外像素不变；编号位于滤镜上方。0.5×/1×/2× 视口和选中 Transformer 下，原分辨率导出 RGBA 完全一致，实时预览像素与导出一致。

首轮类型检查 FAIL（Konva ImageConfig/Filter 联合类型和新判别联合），随后改用真实带 image 的滤镜节点及函数类型判断；首轮 ESLint FAIL（effect 同步 setState），改为通过 ref 同步外部 Konva 节点并释放资源。上述最终类型、lint 和像素检查均已重新执行通过，没有禁用规则或移除断言。

## Windows 人工清单（本表均 NOT_RUN）

1. 真实截图中选择文字/图表局部做两种滤镜，拖动、调整尺寸及强度；对照原图确认位置。
2. 125/150/200% 与混合 DPI、多屏，放大/缩小和平移后绘制及重开；检查指针坐标、窄窗属性栏。
3. 重叠滤镜、靠边/越界区域、透明图片、大图多区域及大强度性能；验证超额报错保留编辑。
4. 多个编号、改色/大小/数值、删除中间和最后一个编号、撤销/重做；保存重启后继续编辑。
5. 把含新工具的合成 PNG 粘贴到其他软件，创建贴图、插入 Markdown 与 OCR，检查最终图像。
6. 完整生产 EXE 操作、安装/卸载、长时间内存稳定性及 macOS 仍需专项验收；历史 QA-001/QA-002 未据此关闭。

## 原生验收及发布

真实系统对话框、项目重开、SQLite/进程重启、原图哈希与 PNG 像素检查、既有编辑器回归及生产构建/发布结果在完成后补记。只有实际执行项可标 PASS。全部测试使用 artifacts 下的隔离目录和明确控制 PNG，不删除用户资料。


## 原生及既有功能回归实际结果

- `node scripts/advanced-native-smoke.mjs v014-initial` PASS/0：[5 组证据](evidence/v014-advanced-native.json)。真实窗口分别在缩小/放大视口绘制；编号 1/2/3 删除中间后保留 1/3、新增编号独立、手改 42/大小 44；移动和 Transformer 改区尺寸、Ctrl+Z/重做；系统项目保存/打开、改变模糊强度；真实 PNG 在不同视口下字节一致，SQLite 保存、实际关闭/重启和原图字节一致。
- 原生 PNG 样例：马赛克相邻像素 `[62,52,180,255]`，模糊断点红色通道从 255 变为 141，区域外 `[5,5,180,255]` 不变。不是仅验证组件存在。
- [高级渲染像素证据](evidence/v014-advanced-renderer.json) 对应 4 组，透明 alpha、方差、真实预览、矢量在滤镜之上和 0.5×/1×/2× 导出相同均已执行。
- `node scripts/editor-native-smoke.mjs advanced-v014` PASS/0：[6 组](evidence/v014-editor-native.json)。原有 5 类标注、系统保存/重开、原尺寸 PNG、跨应用图片剪贴板、取消及进程重启通过。
- `node scripts/split-native-smoke.mjs advanced-v014` PASS/0：[11 组](evidence/v014-split-native.json)。既有 Markdown、左右分栏缩放坐标、原图不变、保存冲突保护、重启及窄窗口通过。
- `node scripts/editor-smoke.mjs advanced-v014 --dpr-2` PASS/0：[16 组](evidence/v014-editor-dpr2.json)。DPR=2 浏览器渲染与交互、旧形状/箭头端点/文字/撤销/透明 PNG 等通过；不是 Windows 混合 DPI 设备测试。
- `npm run check:docs` PASS/0：40 文档、230 本地链接（本节追加后会复验）。

本轮未出现原生/像素验收失败。未执行的完整硬件、长稳、macOS 和生产全流程保持 NOT_RUN，不将旧测试记录当作本轮通过。业务改动限于编辑器与其 Rust JSON 校验，原截图模块和其他标注逻辑未重写。


## 最终构建及交付材料

`npm ci --offline` PASS/0：294 包，按现有锁文件干净安装，无依赖升级；npm 报告 0 已知漏洞，不等同安全审计。随后 `npm run check` PASS/0：33 文件/234 项测试、类型、ESLint 和前端生产构建。`npm run desktop:build` PASS/0：16,232,960 字节，Windows PE FileVersion/ProductVersion 均 0.1.4。

生产 EXE SHA256：`c219260ad4728c53899e6fcc6987d0e1cdba611cff901a517806e060cd8263fa`。

`node scripts/advanced-renderer-smoke.mjs v014-final` PASS/0，额外验证模糊本身的半透明区域 alpha 也保持 128；未用马赛克结果代替模糊断言。最终 `npm run lint`、`npm run check:docs` PASS/0，后者 40 文档/235 本地链接。Rust 代码在上述 107 项及 Clippy/fmt 通过后未再更改。生产 EXE 未执行完整人工流程，安装器/签名未在本轮制作。

本次按用户授权上传源码、新 tag 和非预发布 Release，保留所有旧版本与附件；公开发布及匿名下载校验结果完成后以单独回执提交补记。

## 修改文件

```text
CHANGELOG.md
README.md
docs/ARCHITECTURE.md
docs/RELEASE-NOTES-0.1.4.md
docs/qa/RELEASE-0.1.4.md
docs/qa/evidence/v014-advanced-native.json
docs/qa/evidence/v014-advanced-renderer.json
docs/qa/evidence/v014-editor-dpr2.json
docs/qa/evidence/v014-editor-native.json
docs/qa/evidence/v014-split-native.json
package-lock.json
package.json
scripts/advanced-native-smoke.mjs
scripts/advanced-renderer-smoke.mjs
src-tauri/Cargo.lock
src-tauri/Cargo.toml
src-tauri/src/services/editor.rs
src-tauri/tauri.conf.json
src/features/editor/AnnotationNode.tsx
src/features/editor/EditorCanvas.tsx
src/features/editor/EditorView.tsx
src/features/editor/EffectPatch.tsx
src/features/editor/advanced.test.ts
src/features/editor/effects.ts
src/features/editor/export.ts
src/features/editor/model.ts
src/features/editor/serialization.ts
src/features/editor/store.ts
src/features/editor/types.ts
src/styles.css
```


## 公开发布回执

- [v0.1.4 Release](https://github.com/wpy-sketch/ResearchClip/releases/tag/v0.1.4)，ID `415798601`，UTC `2026-10-17T12:44:05Z`。实际 draft=false、prerelease=false、Latest=true。
- 源码及 tag `v0.1.4` 指向 `27228f42f5661ca605be0fd9ae18adc400d6dfcc`，正常原子推送到 main。发布回执为后续文档提交，不移动源码标签。
- 三个附件：`ResearchClip-v0.1.4-windows-x64.exe`（16,232,960 字节）、`SOURCE.txt`（1,143 字节）、`SHA256SUMS.txt`（179 字节）；服务端 digest、字节数及匿名官方附件 API 下载 SHA256 均匹配：[完整证据](evidence/v014-publication.json)。本轮发布一次成功，没有覆盖或重建旧发布。
- 五个旧版本的 Release ID、预发布属性、附件 ID/名称/尺寸/摘要均保持不变。有限源码扫描 314 文件 / 2,118,986 字节，未发现常见凭据模式；不等同完整安全审计。测试图片、数据库、浏览器会话、工具链和缓存没有上传。
- 本地启动程序：`src-tauri/target/release/researchclip.exe`；未签名，需要 WebView2。本轮没有新 NSIS 包。
- 回执同时更新本报告、`docs/qa/TEST_REPORT.md`，新增 `docs/qa/evidence/v014-publication.json`。业务源码与测试在发布后没有改动。

结论：本轮所列可执行检查 PASS，可交付此功能版本。完整硬件/大图性能/长稳/macOS/生产手工矩阵仍 NOT_RUN，历史问题按原报告保留。本阶段结束，不自动开发后续功能。
