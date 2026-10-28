# 小说阅读器

基于 [Tauri v2](https://v2.tauri.app/) 的本地小说阅读器，支持 Windows。

## 免责声明

本项目为个人学习与技术研究项目，非番茄小说官方项目，与番茄小说及其关联公司无任何隶属、合作或授权关系。

本项目不提供、不托管、不传播任何小说正文、封面或版权内容。所有内容的版权归原作者、出版方或相应平台所有。

本项目使用的接口可能为非公开接口，可能随时变更、失效或受到限制。使用者应自行遵守所在地区法律法规以及相关平台的服务条款。

严禁将本项目用于商业用途、批量抓取、内容传播或其他侵犯版权的行为。通过本项目获取的内容请勿传播、转载或长期保存。

因使用本项目产生的任何法律责任和风险，由使用者自行承担。

## 功能

- 📖 **TXT 导入** — 自动检测编码（UTF-8 / GB18030 / UTF-16）
- 📑 **章节解析** — 识别「第X章」「楔子」「番外」等标题，超大章节自动拆分
- 🌙 **日间 / 夜间模式** — CSS 变量驱动的主题切换；Windows 11 原生标题栏通过 DWM 同步配色，Windows 10 跟随明暗模式
- 🔤 **字体 & 行距调节** — 下拉菜单 + 快捷按钮
- 📚 **书架管理** — 重命名、删除、阅读进度记忆、本地/在线标签
- 🏬 **书城榜单** — 推荐榜、完本榜、巅峰榜、口碑榜
- 🔎 **番茄搜索** — 关键词搜索、书籍详情、评分、标签、简介、封面
- 👀 **详情试读** — 目录抽屉、前两章试读、章节数与连载状态
- 📡 **在线阅读** — 按需加载章节、后续 3 章内存预加载、阅读进度记忆
- ➕ **加入书架** — 在线书加入书架、书架内继续阅读、进度同步
- ⬇️ **导出 TXT** — Web 免签接口 + App 签名接口
- 🖼️ **HEIC 封面** — 内置 libheif / libde265 解码
- 📜 **自定义滚动条** — 原生滚动条隐藏，统一风格的滚动条
- 🖥️ **单实例锁** — 重复启动时激活已有窗口
- 🌐 **双模式运行** — Tauri 桌面端 / 本地 HTTP 浏览器端共用业务逻辑与书架数据

## 更新日志

版本更新记录见 [CHANGELOG.md](CHANGELOG.md)。

## 快速开始

### 环境要求

- [Node.js](https://nodejs.org/) ≥ 18
- [Rust](https://www.rust-lang.org/) ≥ 1.77
- Windows 10+

### 安装依赖

```bash
npm install
```

### 开发模式

双击 `start.bat` 即可：它会自动启动 2048 端口的 HTTP 服务、等待就绪，再打开桌面窗口；浏览器端需要在浏览器中手动打开 `http://localhost:2048`。关闭窗口后自动停掉服务。运行日志统一写在 `logs/`（已 gitignore）。

也可以手动分两个终端：

```bash
# 终端 1：启动 HTTP 服务（浏览器模式，同时为 tauri dev 提供前端）
cargo run --manifest-path src-tauri/Cargo.toml --bin server

# 终端 2：启动 Tauri 开发服务器
npx tauri dev
```

### 构建

```bash
npx tauri build
```

产物在 `src-tauri/target/release/bundle/`。

## 项目结构

```
├── frontend/             # 前端（纯 HTML/CSS/JS）
│   ├── index.html
│   ├── styles.css
│   ├── renderer.js       # 阅读器逻辑、书架、书城、主题等
│   └── tauri-bridge.js   # Tauri / HTTP 双模式桥接
├── src-tauri/
│   ├── src/
│   │   ├── main.rs       # 程序入口
│   │   ├── lib.rs        # Tauri 命令、窗口管理、DWM 标题栏
│   │   ├── shared.rs     # 本地书架、章节解析、在线书数据持久化
│   │   ├── fanqie.rs     # 番茄书城、目录、正文、封面接口
│   │   ├── heif.rs       # HEIC 封面本地解码
│   │   └── bin/
│   │       └── server.rs # HTTP 服务（浏览器模式）
│   ├── resources/heif/   # HEIF 运行时 DLL
│   ├── tests/            # 集成测试
│   ├── capabilities/     # Tauri 权限配置
│   ├── Cargo.toml
│   └── tauri.conf.json
├── .github/workflows/    # GitHub Actions 构建流程
├── start.bat             # 启动 HTTP 服务和桌面窗口
├── start.ps1             # 启动脚本实现
├── books/                # 本地书正文（已 gitignore）
├── books.json            # 本地书架数据（已 gitignore）
├── online-books.json     # 在线书数据与进度（已 gitignore）
├── settings.json         # 用户设置（已 gitignore）
└── exports/              # 导出的 TXT（已 gitignore）
```

## 数据位置

桌面端与网页端共用同一份书架、阅读进度和设置：

- **源码/开发布局**：从 exe 所在目录向上查找同时包含 `package.json` 与 `src-tauri` 的目录，也就是项目根目录；
- **便携版**：数据放在 exe 同目录；
- 可用环境变量 `NOVELREADER_DATA_DIR` 强制指定数据目录。

数据文件职责：

```text
books/             本地书正文
books.json         本地书书架记录和本地进度
online-books.json  所有在线书、书架状态、阅读进度
settings.json      主题、字号、行距、上次打开的书
```

在线书不区分是否加入书架，统一保存在 `online-books.json`；  
`books.json` 只保存本地书。

## 待改进

- **主题切换上框同步**：Windows 原生标题栏只能通过 DWM 改颜色，无法和网页内容的 CSS 过渡同步；若要标题栏、内容区完全一致地渐变，需要改为无边框窗口并自绘标题栏。
- **跟随系统主题**：当前只支持日间/夜间，尚未加入“跟随系统”模式。
- **自绘标题栏**：可实现整窗统一主题、任意标题栏高度和自定义窗口按钮，但需要自行处理拖动、缩放、Snap Layout、圆角和阴影。

## 技术栈

| 层 | 技术 |
|---|---|
| 桌面框架 | Tauri v2 (Rust) |
| 前端 | 原生 HTML/CSS/JS，零框架 |
| WebView | WebView2 (Windows) |
| 浏览器模式 | Rust + tiny_http |
| 编码检测 | TextDecoder + 整本 fatal 解码验证 |
