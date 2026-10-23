# Step02：科研工作区

本轮延续模块化单体，SQLite保持schema12。新增依赖仅flexlayout-react 0.11.1（MIT），React19/Node22兼容；未引入完整IDE框架。安装前已在Windows Tauri WebView2完成6组布局实验，证据位于artifacts/dock-evaluation/native-v1/report.json；该实验不替代生产集成验证。

## 组件与职责

App → WorkspaceSplit（兼容入口）→ ResearchWorkspace → FlexLayout → EditorRegistry → 独立EditorSession。

- WorkspaceManager协调模型、可见性、焦点、文档生命周期及项目工作区；使用现有Zustand。
- EditorRegistry注册资料、Markdown、图片、PDF、代码及终端区域；PDF/代码明确不可用，终端没有执行后端。
- EditorSession保存面板元数据和PanelLifecycle；正文、撤销、文件授权仍属于既有文档store/Rust文件服务。图片继续使用既有编辑会话。
- DocumentSessionContext让每份Markdown调用自己的store；全局插图入口定位当前文档。多个文档不共享正文、撤销、光标、项目绑定。
- PanelLifecycle检查dirty/busy/save/dispose；退出检查包括隐藏与其他工作区草稿。所有文档决定完成前不销毁任何草稿。
- ResizeObserver监听每个宿主；Konva按各自会话重新fit；FlexLayout监听容器变化，CSS容器查询处理窄/短面板。

## 选型

评估Dockview与FlexLayout。选择后者的有界JSON模型、React18/19支持、MIT和无运行依赖；现有Windows窗口dragDropEnabled=false，避免原生拖放拦截WebView内部指针。禁止独立浮窗/popout，不新增窗口权限。官方依据：[FlexLayout](https://github.com/caplin/FlexLayout)、[npm元数据](https://registry.npmjs.org/flexlayout-react/0.11.1)、[Dockview](https://dockview.dev/)。许可证原文见docs/licenses/flexlayout-react-MIT.txt。

## 持久化与安全

WorkspacePersistence以researchclip.workspace.layout:{default|project:ID}保存版本1布局；450ms去抖。只保存稳定资源ID、面板类型、标题、比例和可见性；不保存正文、路径、token、Shell命令。结构验证限制64KiB、12面板、8层、64节点并重建允许字段，拒绝未知版本/损坏记录且保留原文。未重新打开资源时不自动覆盖原布局。

每个会话只存在一个正文实例，同一资源重复打开会聚焦已有实例。原生文件服务每次open生成独立授权；重复路径检测仅关闭本次新授权。限制最多3份Markdown（跨工作区/隐藏总计），为Rust现有4个文件授权上限预留另存/重复打开余量。

重启仅恢复可见区域与布局设置；用户明确点击重新打开，按既有项目注册ID定位。未打开文档不产生Markdown空白分栏。未保存草稿不跨进程持久化；布局不代替文件保存。失效ID/权限不足显示错误，未知布局停止写入，不重置资料。

## 兼容与回退

原数据库迁移和原始截图不变，旧截图/Markdown仍通过原服务打开。布局损坏不影响文件本身，可继续当前内存编辑并报告错误。代码级回退可回滚Step02前端提交；不能降级真实数据库。无新增Zotero数据库访问、Agent/终端权限或上传流程。

验收和已知限制见[Step02报告](../qa/WORKSPACE_STEP02.md)。
