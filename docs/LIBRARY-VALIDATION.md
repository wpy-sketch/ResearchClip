# 截图素材库页面验收

执行日期：2026-10-14，Windows / PowerShell，Asia/Hong_Kong。范围仅为已有真实SQLite资料的网格/列表、卡片打开、创建时间排序、收藏、重命名、确认删除、空状态、缩略图失败重试和分页。复用既有编辑器与 [存储层](STORAGE-VALIDATION.md)，不新增数据库迁移、依赖、搜索或完整笔记/标签界面。

## 实现与文件

- LibraryView组合截图入口、空状态、操作弹窗、排序和分页；LibraryCard显示原图缩略图、标题、创建时间和尺寸。网格自适应列数，列表也显示缩略图，沿用CSS主题变量与Lucide图标。
- 点击缩略图、标题或“编辑标注”调用原有openLibraryEditor，不覆盖已有编辑草稿。收藏按钮有aria-pressed；重命名校验仍由Rust完成。删除须确认，说明笔记/关联和受管文件的永久删除；取消保留资料。工作区清理失败继续显示可重试任务。
- 页面每页24条，显示总数、页码与当前范围；翻页替换列表，不无限累积数据。创建时间升降序加ID稳定排序，修改标题/收藏/标注不会重排创建顺序。换排序或收藏范围回到首页；删除末页唯一记录时Rust将请求页回退。
- 新screenshot_page命令在同一SQLite读取事务中取得总数与摘要；sort为枚举允许列表，其他值为参数绑定。每次查询上限50，当前UI固定24，不加载标注正文。旧list_screenshots命令保留供已有工具兼容，仍按修改时间排序。
- Zustand仅保存当前页和会话查询状态，不保存核心资料到localStorage。generation丢弃过期响应，翻页失败保留上一页并显示错误。操作完成后刷新当前范围，避免异步操作将旧导航范围写回。
- 缩略图IPC仅凭资料ID读取受管文件、解码PNG及校验尺寸，不读取原图/标注正文。每卡失败占位、独立重试，不删除损坏资料；换页/重试/卸载释放Blob URL。打开编辑器仍严格校验原图和标注版本。
- main窗口增加allow-screenshot-page；无通用SQL、FS、asset全盘、overlay存储权限。数据库保持v2，迁移SQL和现有用户数据均未重置或覆盖。

本轮修改：src/features/library/LibraryView.tsx、LibraryCard.tsx、store.ts和store.test.ts；src/shared/api/library.ts和library.test.ts；src-tauri/src/storage/library.rs、commands/library.rs、lib.rs、build.rs、capabilities/main.json；src/styles.css；package.json只增加素材库冒烟脚本；scripts/library-native-smoke.mjs、smoke.mjs和check-docs.mjs；README、ARCHITECTURE、ROADMAP和本报告。此前未提交开发文件保留，依赖与锁文件未因本轮改动。

## 实际检查

Node22.22.2/npm10.9.7、Rust MSVC1.99、Tauri2.12.3、Windows WebView2/Edge。以下已执行，最终重跑结果补充于表后；命令成功时退出码均为0。

| 命令 | 实际结果 |
| --- | --- |
| npm run check | 17个文件132项单元测试通过，类型、ESLint与Vite生产构建通过 |
| npm run format:rust | 格式通过 |
| npm run test:rust | 58项通过，包括分页、排序、CRUD重启一致性、损坏图片和原有迁移/恢复 |
| npm run check:rust | --all-targets -D warnings通过 |
| npm run tauri -- build --debug --no-bundle | Windows debug EXE构建通过并已实际运行 |
| npm run desktop:build | 首次Windows release构建通过；最终增量重建因用户正在运行同名EXE而退出1，改用独立target路径见下文 |
| npm run tauri -- build --no-bundle --target x86_64-pc-windows-msvc | 最终源码Windows release构建通过，退出码0，产物在target/x86_64-pc-windows-msvc/release |
| npm run test:smoke:library:native -- library-oct14-2 | 实际Windows应用9组通过；49条记录/3页、损坏缩略图与退出重启，errors为空 |
| npm run test:smoke:library:native -- library-oct14-final | 10组通过，额外16种非空图库主题/视图/视口布局；errors为空 |
| npm run test:smoke:library:native -- library-oct14-complete | 最终源码10组通过；先滚动显示各张懒加载图片，再完整断言24张解码，errors为空 |
| npm run test:smoke:storage:native -- library-storage-oct14 | 原有真实存库/五类图元保存、笔记标签、确认删除和两次退出重启8组通过 |
| npm run test:smoke -- --run-id library-ui-oct14 | 6组界面回归、32种空页布局；overlay mock渲染4种DPR通过 |
| npm run test:smoke:editor -- library-editor-oct14 | 15组标注、坐标/缩放、撤销重做、序列化与真实PNG像素回归通过 |
| npm run check:docs | 19份文档、53个本地链接、权限/注册一致及已发布迁移未改写检查通过 |

原生证据保留在artifacts/library-library-oct14-2、library-library-oct14-final、library-library-oct14-complete、storage-library-storage-oct14，编辑器证据在artifacts/editor-library-editor-oct14。原生测试使用全新、显式命名的settings/workspace和WebView profile；真实SQLite表及PNG通过生产业务命令创建，正常启动不插入示例资料，未改动用户AppData。PNG为明确标记的320×200受控测试源，不冒充此刻真实屏幕截图。最终分页整数转换修正后，132项前端、58项Rust、格式、Clippy和debug构建均已重跑通过。

原生验收证明：空库无自动示例；49条真实记录分三页无重复/漏项；按创建时间升降序；真实IPC观察首屏24次缩略图请求、零原图请求，单击打开后仅请求该原图；重命名/收藏/取消收藏与SQLite一致；损坏一张测试缩略图只影响该卡，恢复文件后重试显示；删除取消保留文件，确认仅移除选中记录/文件并回退末页；真实退出重启保持48条资料、修改标题、收藏、删除结果和原图SHA-256。

非空图库16种组合为深浅主题×网格/列表×480×400、640×500、960×680、1440×900。通过CDP在原生WebView中模拟内容视口，检查当前页24卡、卡片边界与无水平文档溢出；这不是Win32外框缩放、高DPI或多显示器硬件验收。截图文件library-dark-grid.png与library-light-list.png保留。

## 失败记录与边界

初次npm run check退出1：新增分页DTO测试预期含annotationData，而摘要解析正确省略它；修正测试预期为摘要结构后132项通过，未改弱校验。初次library-oct14-1原生脚本退出1：尝试替换Tauri内部invoke观察请求，但该属性不可写，读取计数无效；改为监听实际ipc.localhost请求，并加入24次缩略图的正向断言后完整通过。失败报告保留，不把前四组部分成功当作整轮通过。

library-oct14-verified退出1：等待屏幕外的所有loading=lazy图片立即解码，依赖了浏览器的预加载距离，原生页面已显示正常卡片。测试改为逐卡滚动显示，保留全部24张自然尺寸/解码断言；library-oct14-complete完整通过。最终两次检查启动曾因自动权限审核超时而未执行，按返回指示各重试一次后npm run check和npm run test:rust均实际通过，未绕过权限拒绝。

最终默认release重建遇到Windows文件占用（拒绝移除researchclip.exe），只读核对为用户正在运行的release窗口；保留该窗口和未保存草稿，未结束其进程。改用已核对Tauri CLI帮助的npm run tauri -- build --no-bundle --target x86_64-pc-windows-msvc，成功生成独立src-tauri/target/x86_64-pc-windows-msvc/release/researchclip.exe，退出码0，release构建1分13秒。默认输出仍需用户保存并关闭旧应用后再重建；本轮交付已有最终debug及独立路径的最终release。没有安装包/签名或macOS验证。

自动分页store测试覆盖过期范围/排序响应、页替换、翻页错误保留旧页、删除/取消收藏后的回退、重命名后的revision刷新、空库和恢复失败。Rust真实临时库测试覆盖创建时间稳定排序/同时间ID、参数与sort拒绝、创建/改名/收藏/删除/重开一致性、未来标注独立预览、坏缩略图及缺原图保留记录。

1000卡/大图库p95、磁盘满/外部文件占用与macOS未执行；当前分页限制了每页前端资源，不能据此声称性能门槛全部通过。实时GDI与物理截图快捷键沿用旧独立记录，本轮不重新验证屏幕访问。没有本轮功能阻塞；完整笔记、搜索和Markdown仍等待新的请求。下一步建议以真实科研资料复验大图库和日常重命名/收藏/删除。本轮完成后停止。
