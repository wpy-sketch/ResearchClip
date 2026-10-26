# Step15：Zotero标准批注受控写入（先行决策）

范围v4.2.6，依赖已正式发布并完成匿名附件校验的Step14/v4.2.5。先核实接口、独立资料库演练，再实现；本记录的计划不代表尚未执行的测试通过。

## 接口核实与路线

2026-10-26读取实际安装包application.ini与本机代码，Zotero10.0.6、Build20261007153950、Gecko140。当前[官方Local API](https://www.zotero.org/support/dev/web_api/v3/local_api)在Zotero10起支持授权写入；此前Step14仅使用GET并不意味着接口不能写。优先使用官方HTTP写请求，不预先增加业务插件或云端API。[写入契约](https://www.zotero.org/support/dev/web_api/v3/write_requests)与实际安装包实现均需核对，不能只按主分支文档假设当前版本可用。

POST /api/local/authorize使用Server ID及应用名称，弹出Zotero授权；密钥只留在Rust内存，按实例和已确认来源限定。单次密钥可能在校验时就消耗，401需要用户再次主动授权；不自动弹窗轰炸。记住授权是Zotero自身选择，ResearchClip本版不将密钥写入本机数据库、前端状态、日志或URL。实例切换立即失效。

候选写入为POST items中的annotation对象、PATCH单项的文字/评论与颜色、DELETE单项；更新/删除带当前对象版本。HTTP200批量结果仍需检查failed/successful，不作为保存成功依据。成功后再GET核对完整库、父附件、原key、类型、位置和目标文字/颜色；删除复读确认不存在。超时或断线可能已提交，显示待核实而非伪装失败后盲目重试。

## 能力与边界

先实际验证highlight、underline、note（普通文字便签）。自由文本text、区域image、ink和未知类型在未验证前禁用写入并标记未实现；ink不转换为私有手写。能力由实际版本、接口/字段检查和专用资料库测试共同决定，旧版或未验证版本只读。只读库/权限错误由Zotero权威检查，界面不得推断有权限；本地API密钥本身没有细粒度库限制，ResearchClip须在Rust再限定明确确认的文献/附件。

每次操作重新校验当前PDF原件授权、内容哈希、实例、library type/ID、父文献及attachment key。修改/删除前先读取目标，要求标准批注、正确父附件与允许类型及版本。UI不能提交任意URL、SQL、文件路径、Markdown正文或私有笔迹。新Zotero key是外部系统规定的局部身份，不作为ResearchClip全局ID。

坐标复用PDF.js同一页文字选择或既有页面选区及物理坐标；显示旋转独立于原PDF坐标。原始矩形由受校验的CropBox/UserUnit、规范物理位置转换，不接受NaN、倒置、越界页码、超量矩形或未知几何。保留PDF原件和已有ResearchClip位置/摘录，不为了写入重排或改写PDF。

## 架构与存储

沿用ZoteroClient受限loopback客户端、既有任务所有权、PDF会话、来源模型、只读批注仓库与cache19。新增有界写请求/能力/授权/往返确认层及明确的等待授权、写入中、核实中、已核实、权限错误、版本冲突、状态未知提示。写入完成只更新同一来源的派生缓存，不新增第二批注数据库或全局状态框架。

本阶段计划保持SQLite19，无新增业务迁移；如实现证明确有持久化变更，另做版本化事务及隔离旧数据验证。原001–019迁移和用户文件不改。密钥与未完成任务不写入布局。关闭/取消在提交前可取消；请求已发送后明确提示不能保证远端撤销，通过复读核实，不能声称取消等于未写入。

## 独立Zotero验证与回退

使用全新隔离profile与data目录、独立本机端口、无账号/同步的专用校准PDF。先检查实际资料目录和profile与测试标记一致，再建立虚构测试文献；禁止触碰默认资料库或直接写Zotero SQLite。按[官方插件生命周期](https://www.zotero.org/support/dev/zotero_7_for_developers)构造的测试夹具仅负责种子/边界控制，任何授权弹窗替身须明确标记。参考[官方Local API测试](https://github.com/zotero/zotero/blob/main/test/tests/server_localAPITest.js)对授权提示边界的单独测试方法；HTTP、Zotero数据层与CRUD不能用假服务冒充。

逐类型创建→GET→改字/颜色→GET→删除→GET，核对分页只读显示、四旋转/CropBox/UserUnit、原件SHA、单次密钥、拒绝授权、401/403/412/428、只读库、断线、版本冲突和未知提交状态。产品不导入测试夹具或跳过真实用户授权。失败时保留只读批注与PDF阅读，禁用未经验证类型并解释；不得用直接数据库写入或未经核实的私有接口补成功。

真实桌面授权对话框、专用测试资料库API验证、模拟HTTP和ResearchClip原生交互分别记账。默认用户资料做只读前后审计；无Markdown导出PDF、独立备份/恢复、Markdown自动写Zotero或私有笔迹转换。交付后停止，下一阶段需新请求。

## 已获得的实现证据

独立Zotero10.0.6已完成Python30个真实HTTP请求与Rust产品客户端三类型CRUD/401/412/复读/原PDF不变；真实授权GUI与真实只读群组仍单列未验收。安装版本存在两个实际差异：早期认证错误可能不附LocalAPI响应头，只解释为拒绝；JSON导入按字段顺序设置批注属性，因此创建时显式先序列化annotationType，不能直接使用字母排序的Value映射。成功响应始终要求实例和API身份头，实际POST结果必须逐项检查。

原生仅增加一个main-only有类型IPC zotero_annotation_write，operation限定capabilities/authorize/forget/submit/verify；来源DTO禁止额外字段，renderer只传规范身份。ZoteroSessions内部最多三个来源会话保留密钥与未核实WriteIntent；单次密钥在尝试前消耗。待核实键只用于GET复核，不允许重发。PDF关闭取消关联任务并清除密钥，迟到授权不会重新存入已取消所有者。草稿只在会话内保存；关闭提示与阅读位置保存分开，不在此阶段假称已有持久双向同步。
