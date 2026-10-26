# Step14：Zotero标准批注只读适配（先行记录）

范围：v4.2.5。在既有PDF.js阅读器、已确认Zotero来源和只读Local API客户端上增量开发。只读Zotero；本机ResearchClip数据库中的派生缓存允许写入。没有Zotero SQLite访问、API写请求、原PDF修改、批注编辑/写回、私有手写转换、Markdown导出PDF或独立备份/恢复产品。

## 已核实的接口与选择

2026-10-26实际安装和运行版本为Zotero10.0.6，GET /api/返回API3、schema44并包含Server ID；20项真实只读样本的annotationPosition为JSON字符串，其中ink使用pageIndex/paths/width。未记录用户批注正文、key或文献路径。一次更大排序读取超时，不能说完整文献/全部类型已验证。真实Zotero窗口操作继续按用户此前选择人工验收。

[Local API官方契约](https://www.zotero.org/support/dev/web_api/v3/local_api)支持本机只读GET和显式limit/start；版本按Server ID分区，不能和云端版本比较。[当前服务器实现](https://github.com/zotero/zotero/blob/main/chrome/content/zotero/xpcom/server/server_localAPI.js)核对children路由及实例校验。[JavaScript API](https://www.zotero.org/support/dev/client_coding/javascript_api)属于Zotero进程中的特权接口，本阶段已有Local API可满足读取，选择不增加插件安装或特权脚本执行流程。

[批注字段定义](https://github.com/zotero/zotero/blob/main/chrome/content/zotero/xpcom/annotations.js)包含highlight、underline、note、image、ink、text；[阅读器坐标契约](https://github.com/zotero/reader/blob/master/src/pdf/lib/coordinates.js)使用PDF内部矩形及viewport点变换，跨页延续可带nextPageRects。只据协议独立实现，未复制AGPL实现或新增依赖。API或数据格式不支持时给出错误/只读缓存状态，不伪造读取成功。

## 四个职责

| 模块 | 职责 |
| --- | --- |
| ZoteroAnnotationIdentity | 保留instance ID、library type/ID、attachment key、原annotation key；不把item key单独作为全局身份 |
| ZoteroAnnotationRepository | 从已确认的PDF来源限定只读请求；校验父附件、库、实例、JSON、版本和预算，取消复用已有Zotero任务所有权 |
| AnnotationCoordinateAdapter | 使用真实PDF页面CropBox/UserUnit和总旋转转换原生矩形；保留原坐标，逐页处理跨页区域；无效或不支持几何不误画 |
| AnnotationReadCache | ResearchClip内按来源身份及PDF SHA256分区保存有界快照、获取时间、版本/截断状态；不写Zotero，不混合私有笔迹 |

## 数据和权限

SQLite19只追加缓存迁移，001–018保持原字节。缓存引用既有PDF资源，格式版本1；来源/哈希不符、未来格式和损坏缓存拒绝使用，事务失败保留旧缓存。每来源快照最多8MiB，全部缓存64MiB；最多1000项批注，20项/页，沿用请求正文2MiB和整体12秒预算。达到列表限制明确提示不完整，网络或解析失败不提交半截成功快照。

只读刷新先验证当前PDF授权原件字节，核对已确认库/附件、当前Server ID及API返回原件URL；然后GET附件children，带实例头。仅有受管副本但无原件授权时可以显示同范围/同哈希的缓存，刷新要求重新选择原件。原始PDF搬移需通过已有授权/来源核对，不绕过文件权限。

缓存自动读取不触发网络；刷新由用户点击，显示进度并可取消。断线、403或不支持接口可保留明确标注时间的旧缓存；实例/来源冲突不能把旧批注叠到当前来源。完整成功重读才替换对应缓存，重复加载按自然身份去重，不重复创建资源ID。

## 显示和支持边界

优先显示高亮、下划线、文字便签/文本、已有区域批注。颜色只接受安全色值，批注文字按纯文本显示；列表可以按颜色及文字筛选，点击定位。下划线方向随PDF旋转，跨页矩形使用各自页面变换。图层明确标记Zotero只读，和ResearchClip来源高亮分开。

ink等未支持类型仍保留原key、库、类型和可读说明，明确标记不支持，不创建ResearchClip HandwritingLayer或Stroke。页面/坐标越界及未知扩展保留诊断，不猜页码。叠加层只在当前可见页生成，有形状数量上限和明确提示，释放跟随PDF会话；不会建立第二PDF渲染器。

## 验收与回退

单元：多个类型/颜色/原key、库与实例隔离、畸形位置、重复key/版本、CropBox/UserUnit/旋转、跨页、超限、取消/过期响应和缓存冲突。隔离数据库演练18→19并核对旧PNG、Markdown、PDF位置和索引不变；Windows实际阅读器结合专门练习PDF及明确的Local API测试服务器验证显示/筛选/定位/重连/重载/重启，不能冒称真实Zotero GUI通过。

关闭批注显示或取消刷新可继续普通PDF阅读；失败不清空已确认来源、旧缓存或笔记。旧程序不写升级库；需要恢复资料时仍须用户明确授权，不能自动覆盖唯一副本。真实Zotero多类型专用文献和物理多屏验收保留独立人工步骤，最终结果以本版QA记录为准。
