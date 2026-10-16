# 科研知识卡片、标签与搜索验收

2026-10-14，Windows / PowerShell，Asia/Hong_Kong。范围仅为用户连续请求的知识卡片和SQLite搜索筛选，不实施Markdown导出、托盘、AI或OCR。沿用 [SQLite v2](STORAGE-VALIDATION.md) 与 [真实素材库](LIBRARY-VALIDATION.md)，本轮不新增迁移或删除用户资料；当前技术契约见 [架构第11节](ARCHITECTURE.md#11-当前科研知识卡片标签与搜索2026-10-14)。

## 行为与数据保护

单击素材图/标题打开知识卡片，左侧原图和时间，右侧标题、Markdown正文、来源、标签和收藏；窄内容视口改为上下布局。“编辑标注”入口仍保留，卡片内也可进入原有图片编辑器。显示原图，不将标注合成后覆盖原图。

正文是SQLite TEXT和受控textarea，HTML/脚本不执行，没有Markdown HTML预览。700ms自动保存全部卡片字段，可点击按钮或Ctrl+S立即保存。每卡一个进行中的写入，新输入不会被迟到响应覆盖；切换即尝试flush但保留独立草稿，失败显示卡片错误和全局待保存入口。最多20个会话，仅淘汰干净内容；重新加载覆盖未保存草稿前须确认，读取失败不清空草稿。

卡片事务同时校验截图及主笔记revision，更新标题/收藏/修改时间、正文/来源、标签及关联，失败整体回滚。主笔记按(created_at,id)选最早一条；已有额外笔记保留并可搜索。同名标签按trim及ASCII大小写复用，移除芯片只解除关联；全局标签删除有确认，并保留截图、笔记、图片，受影响截图revision/updatedAt增加。

自己的卡片事务保证不改标注JSON，在已知编辑器revision一致时同步元数据绑定并保留历史；未知外部变更仍冲突而非自动覆盖。来源只接受http/https、有效host、无账号密码/空白/控制字符，原始及URL编码后的长度均限2048字节。先保存再打开已保存来源；通过官方Rust Opener，不给WebView通用打开文件/URL权限，不注入自动JS链接处理器。

顶部搜索300ms防抖，IME输入期暂停；标题或任意笔记正文匹配，标签采用AND，和收藏状态及创建时间排序组合。%/_/反斜杠转义为字面字符；SQL值均参数绑定，排序为枚举，标签数组通过json_each与EXISTS，不产生重复行。空/全空白关键词不限文本。清除全部条件回到完整素材库，无结果给出引导。计数和分页摘要同一SQLite读取快照，24条分页，当前页缩略图，搜索不读全量原图。英文ASCII大小写不敏感，中文按原文子串；不宣称Unicode全量大小写折叠。

## 依赖和修改文件

新增锁定tauri-plugin-opener2.7.1；[官方文档](https://v2.tauri.app/plugin/opener/)与 [版本/许可证](https://docs.rs/crate/tauri-plugin-opener/2.7.1) 已核对：Tauri2.12、Rust最低1.90，与本机Tauri2.12.3/Rust1.99兼容，MIT/Apache-2.0。安装源码Builder/open_url API已读取并实际编译，不编造调用。使用官方平台启动器，避免本轮手写Windows/macOS分支；macOS运行未验证。只安装Rust插件，npm依赖和锁文件未因本轮改变。Cargo.lock增加其必要依赖，Windows实际下载新crate、locked fetch和新目录release构建成功。

LIKE依据当前小型资料和中文子串需求选择，语义依据 [SQLite表达式文档](https://sqlite.org/lang_expr.html#like)。没有生产HTTP/搜索服务器；原生测试中的临时localhost服务器仅验证默认浏览器打开来源，结束时关闭，不接收截图/笔记内容。

新增features/notes的KnowledgeCardView、PendingCardNotice、store/model和测试，features/tags/store，library/QuickSearch、SearchDialog、use-search；修改LibraryView/Card/store及测试、TagsView、app/card-actions、use-card-workflow、App/ui-store/Toolbar/WorkspaceDialogs、shared/api/library及测试、styles.css。Rust新增storage/knowledge.rs和search.rs，修改storage/mod.rs/library.rs、commands/library.rs、lib.rs/build.rs、main capability及Cargo.toml/lock。新增固定测试辅助程序examples/knowledge_fault.rs和scripts/knowledge-native-smoke.mjs；更新现有smoke/library-native-smoke/check-docs、package检查脚本及README/ARCHITECTURE/ROADMAP和本报告。保留此前未提交工作。

## 实际检查

Node22.22.2/npm10.9.7、Rust MSVC1.99、Tauri2.12.3、Opener2.7.1、Windows WebView2/Edge。下面所有命令均本轮实际执行，退出码0，不借历史结果冒充本轮结果。

| 命令 | 本轮实际结果 |
| --- | --- |
| npm run check | 18个文件146项通过；类型、ESLint和生产构建通过 |
| npm run test:rust | 最终70项通过；包含卡片事务、冲突、重载、标签保留、URL编码和组合查询 |
| npm run check:rust | 最终--all-targets -D warnings通过 |
| npm run format:rust | 最终格式通过 |
| cargo fetch --manifest-path src-tauri/Cargo.toml --locked --target x86_64-pc-windows-msvc（通过with-rust） | Windows锁定依赖取得成功 |
| npm run tauri -- build --debug --no-bundle | 包含最终URL边界及权限元数据的Windows debug构建通过 |
| CARGO_TARGET_DIR=src-tauri/target/knowledge-release；npm run tauri -- build --no-bundle | 全新目录release构建1分38秒；最终增量再构建38秒通过，产物在knowledge-release/release/researchclip.exe |
| npm run test:smoke:knowledge:native -- knowledge-oct14-2 | 实际Windows9组通过，errors为空 |
| npm run test:smoke:knowledge:native -- knowledge-oct14-final | 包含真实Ctrl+S重试的9组通过，errors为空 |
| npm run test:smoke:knowledge:native -- knowledge-oct14-verified | 最终程序9组通过，含8种卡片视口、Ctrl+S、URL和实际重启，errors为空 |
| npm run test:smoke:library:native -- knowledge-library-oct14 | 原素材库10组通过，含16种非空列表布局、卡片/编辑器入口、损坏图片和重启 |
| npm run test:smoke:storage:native -- knowledge-storage-oct14 | 原真实存库/五类图元保存、笔记/标签、删除与两次重启8组通过 |
| npm run test:smoke -- --run-id knowledge-ui-oct14-fixed | 6组、32种界面/主题/尺寸组合通过；overlay mock4种DPR通过 |
| npm run test:smoke:editor -- knowledge-editor-oct14 | 15组真实渲染与PNG像素检查通过，属编辑器fixture，不证明GDI捕获 |
| npm run check:docs | 20份文档、57个本地链接及权限/命令/迁移一致性通过 |

原生证据保留在artifacts/knowledge-knowledge-oct14-2、knowledge-knowledge-oct14-final、knowledge-knowledge-oct14-verified，测试创建29条真实SQLite截图记录并复用受控320×200 PNG。素材库回归保留在artifacts/library-knowledge-library-oct14，存库回归在artifacts/storage-knowledge-storage-oct14。使用独立settings/workspace/profile，不重置真实AppData、不结束用户已有应用。故障工具只接受仓库artifacts内名为workspace的已存在目录、schema2数据库和两个固定触发器操作，不提供业务SQL入口。最终版本无本轮功能阻塞，所有自启动测试应用均已退出。

原生证明：一轮防抖写入保存正文/URL/多个标签/收藏，含script/img文本不执行；官方插件打开默认浏览器并真实访问本机来源页，通用opener IPC被拒绝；实际SQLite trigger阻止保存后旧资料不变、错误可见、切换其他卡片再返回仍有完整失败草稿，解除故障后Ctrl+S更新同一主笔记；卡片与原编辑器往返保存成功；五次快速搜索输入只触发一次分页读取且零原图请求，中英文和特殊字符正确；多个标签/笔记关键词/收藏组合、时间升降序、清除条件正确；卡片移除关联和全局删除标签不删截图/正文；实际退出重启恢复笔记、来源、收藏及标签移除，原图SHA-256相同。

8种知识卡片内容视口（深浅主题×480×400、640×500、960×680、1440×900）在原生WebView中经CDP模拟，检查正文编辑区可用且无水平文档溢出；不是实际多显示器或高DPI硬件验收。card-light.png/card-dark.png留作界面证据。原生开启的默认浏览器来源验收页可手动关闭，临时HTTP服务已停止。

## 失败记录、边界和下一步

初次Rust编译发现两处逗号连写声明的语法错误，修正为独立声明后通过；ESLint发现组件文件导出非组件函数，移到model模块后通过。首次界面knowledge-ui-oct14退出1，旧断言等待占位搜索文案；改为检查真实功能/浏览器不可用文案，保留输入清空、焦点和32种布局断言，完整重跑通过。knowledge-oct14-1退出1：datalist输入的实际可访问角色为combobox，旧脚本按textbox定位失败；按真实语义修正后9组通过。Clippy要求测试模块放在实现之后，移动模块后严格检查通过，未放宽警告或移除断言。

URL编码长度检查发现输入小于2048字节时规范化结果可能超限；已在保存前拒绝过长编码结果并加入重新打开边界测试，HTTP/HTTPS大小写规范化也有测试。查询/保存测试覆盖失败与边界，不仅镜像界面样式。

草稿保留保证的是正常导航/快速切换，尚无崩溃草稿日志或完整原生退出等待机制，强制结束前未成功保存的内存内容不能承诺恢复；退出前确认待保存提示消失。冲突不静默重写外部数据，需先保留草稿再明确重新加载。当前编辑最早笔记，已有额外笔记只保留/搜索，没有新增多笔记管理器。

使用npm run desktop:dev，或最终src-tauri/target/knowledge-release/release/researchclip.exe；先保存并关闭旧应用，避免同一工作区被多个实例占用。本轮独立release目录避开运行中的旧EXE，未替换或关闭用户已有窗口。

1000卡/长正文p95、磁盘满/真实ACL/外部占用、macOS、实际GDI/物理热键/显示器DPI矩阵未执行；本轮原生PNG为受控源，不替代旧捕获硬件记录。下一步建议用真实科研资料验收大图库/长正文性能与故障条件，其他阶段等待新请求。完成后停止。

## 2026-10-15 继续请求复验

重新读取AGENTS、README、ARCHITECTURE和本报告，核对当前源码后确认知识卡片、笔记、标签及组合搜索已实现。本次没有新增业务功能、依赖、迁移或修改用户资料，仅补充复验记录，保留已有未提交工作。

| 本次实际执行 | 结果 |
| --- | --- |
| npm run check | 退出0；18个文件146项测试通过，类型检查、ESLint及Vite生产构建通过 |
| npm run format:rust | 独立重试退出0 |
| npm run test:rust | 退出0；70项通过，0失败 |
| npm run check:rust | 退出0；所有目标Clippy，禁止警告 |
| npm run test:smoke:knowledge:native -- continue-20261015 | 退出0；真实Windows WebView2/SQLite的9组检查通过，errors为空 |
| npm run check:docs（记录前、记录后） | 均退出0；20份文档、57个本地链接及权限/迁移一致性通过 |

首次组合Rust检查在format阶段无诊断输出便返回-1058471934，后续test/clippy因此未在该次组合中执行。未修改代码或放宽断言；单独重试format成功，再分别执行test及clippy成功。异常首次退出的原因尚未确定，重试后没有可复现的检查阻塞。

原生复验使用已有2026-10-14 debug程序和独立artifacts/knowledge-continue-20261015工作区，报告见该目录的report.json。覆盖700ms保存、字面Markdown安全存储、来源打开、真实SQLite保存失败/草稿保留/Ctrl+S重试、卡片与编辑器往返、中文/英文/特殊字符及多条件搜索、标签移除不删截图、8种内容视口，以及实际退出重启和原图哈希不变。测试拥有的应用均已退出；没有重置真实AppData。本次未重新打包Windows release，前述release产物和历史构建结果保持原记录。

边界维持不变：强制结束前尚未保存的内存草稿不保证恢复；1000卡性能、macOS和截图硬件矩阵未在本次验收。下一步建议用真实科研资料验收卡片编辑和检索体验，等待新请求，不自动实施下一阶段。
