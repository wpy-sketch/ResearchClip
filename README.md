# ResearchClip

面向科研人员的本地桌面截图与知识管理工具。当前交付为**工程初始化**：欢迎主窗口、版本信息与真实Rust通信按钮。截图、图片编辑、素材库、笔记、标签、搜索、导出和托盘均未实现。

## 当前工程

基于当前稳定的官方create-tauri-app React TypeScript模板核对并整理，采用Tauri 2 + Rust、React + TypeScript + Vite、Tailwind CSS、Zustand、Vitest、ESLint，使用npm。保留SQLite基础代码及未来Konva依赖，但欢迎窗口不初始化数据库、不挂载编辑器。

- 顶部：ResearchClip。
- 中间：欢迎使用 ResearchClip；测试Rust通信按钮。
- 底部：应用版本，桌面端由Tauri官方getVersion读取。
- 按钮调用Rust test_connection，返回实际后端消息；等待时禁用重复请求，错误可见。
- 浏览器仅预览界面，显示构建版本和预览提示，按钮禁用，不伪造Rust连接。
- 已移除旧的三页占位界面与未使用图标库，没有模板logo、计数器或greet输入演示。

本轮没有新增截图、标注、存储业务表或数据库迁移，已有用户数据不被删除/重置。历史M1源码中的storage模块与诊断接口保留，只有显式调用get_workspace_status才打开工作区数据库。

## 环境与运行

Windows先验收。需要Node >=22.12、npm（本轮10.9.7）、Rust MSVC（Cargo.toml最低1.99）、C++ Build Tools / Windows SDK和WebView2。安装说明：[Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)。macOS需要Rust与Xcode Command Line Tools，本轮未在Mac验证。

```powershell
npm ci
npm run desktop:dev
```

本机已有项目内隔离Rust工具链，scripts/with-rust.mjs会自动使用；其他机器没有.tools时使用PATH中的Rust。不修改系统PATH，不提交.tools。

```powershell
# 浏览器预览：http://127.0.0.1:1420
npm run dev

# Windows release可执行文件
npm run desktop:build
# src-tauri/target/release/researchclip.exe
```

当前使用no-bundle，不生成安装包，也未签名。package-lock.json与src-tauri/Cargo.lock纳入版本控制；没有yarn/pnpm锁文件。当前npm锁文件的下载地址使用本机已有镜像配置；换源后需重新验证完整性和构建。

## 检查命令

```powershell
npm run check          # TypeScript、ESLint、Vitest、前端生产构建
npm run test:rust      # Rust / SQLite / IPC契约
npm run format:rust    # rustfmt
npm run check:rust     # clippy，warning视为失败
npm run desktop:build # Windows实际编译
npm run test:smoke     # 本机Edge、生产前端预览
npm run test:smoke -- --desktop # Windows exe/WebView2/真实按钮通信
```

格式和lint需要rustfmt/clippy组件。冒烟使用playwright-core与本机Edge/WebView2，不下载独立浏览器。原生测试子进程使用独立的WebView2资料目录和本地调试端口；测试结束关闭进程。报告/截图写入忽略的artifacts，不覆盖历史M1证据。

本轮实际结果见 [初始化验收](docs/INITIALIZATION-VALIDATION.md)。历史 [M1验收](docs/M1-VALIDATION.md) 仅记录当时的三页骨架，不能代替本轮结果。

## 当前目录结构

```text
ResearchClip/
  README.md / AGENTS.md
  docs/                       # PRD、架构、路线图、验收记录
  src/
    app/                      # App组合、Zustand状态及测试
    features/home/            # 欢迎页
    shared/
      api/                    # application IPC与既有工作区诊断
      types/                  # TypeScript契约
    main.tsx / styles.css / vite-env.d.ts
  src-tauri/
    src/
      commands/               # test_connection、既有诊断
      domain/                 # 可序列化DTO
      storage/                # 保留SQLite基础模块
      lib.rs / main.rs
    migrations/               # 仅已有v1，不新增迁移
    capabilities/ / icons/
    Cargo.toml / Cargo.lock / build.rs / tauri.conf.json
  scripts/                    # Rust工具链包装、冒烟
  assets/                     # 应用图标源
  package.json / package-lock.json
  vite.config.ts / tsconfig*.json / eslint.config.js
  .gitignore / .gitattributes / .editorconfig
```

.gitignore排除依赖、编译产物、测试资料、工具链、生成权限、日志、env和编辑器本地配置，保留两个锁文件。未来feature/services/capture目录按实际请求创建，不预先放入无作用stub。

## 设计文档与边界

[AGENTS](AGENTS.md)规定逐步实现、测试、数据保护与如实报告；[PRD](docs/PRD.md)定义首发产品；[ARCHITECTURE](docs/ARCHITECTURE.md)定义分层和数据/接口；[ROADMAP](docs/ROADMAP.md)定义后续阶段与客观验收。

旧DATA-MODEL/DEVELOPMENT-PLAN和M1验证保留为历史材料，实施以当前用户请求及正式设计为准。数据库仍为既有schema v1，原图/标注/笔记业务未实现。未来数据根使用Tauri app_data_dir，Windows通常为%APPDATA%/io.researchclip.desktop；初始化欢迎窗口不会在该目录建库。

## 已知限制与下一步

- macOS运行、安装包/签名和截图硬件矩阵尚未验证。
- 浏览器按钮禁用是预览模式设计；真实Rust测试必须运行desktop:dev或Windows exe。
- 1420被占用时Vite明确失败，请关闭旧开发服务再运行。
- 下一候选步骤为M2A Windows捕获实验，等待新请求。本轮只完成初始化。
