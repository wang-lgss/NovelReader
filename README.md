# 小说阅读器

基于 [Tauri v2](https://v2.tauri.app/) 的本地小说阅读器，支持 Windows。

## 功能

- 📖 **TXT 导入** — 自动检测编码（UTF-8 / GB18030 / UTF-16）
- 📑 **章节解析** — 识别「第X章」「楔子」「番外」等标题，超大章节自动拆分
- 🌙 **日间 / 夜间模式** — CSS 变量驱动的主题切换，WebView2 原生背景色同步
- 🔤 **字体 & 行距调节** — 下拉菜单 + 快捷按钮
- 📚 **书架管理** — 重命名、删除、阅读进度记忆
- 📜 **自定义滚动条** — 原生滚动条隐藏，统一风格的滚动条
- 🖥️ **单实例锁** — 重复启动时激活已有窗口
- 🌐 **双模式运行** — Tauri 桌面端 / Node.js 浏览器端共用业务逻辑

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

```bash
# 终端 1：启动 Node.js 后端（浏览器模式）
node src-tauri/src/bin/server.js

# 终端 2：启动 Tauri 开发服务器
npx tauri dev
```

或者直接双击 `start.bat`。

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
│   ├── renderer.js       # 阅读器逻辑、书架、主题等
│   └── tauri-bridge.js   # Tauri / HTTP 双模式桥接
├── src-tauri/
│   ├── src/
│   │   ├── main.rs       # 程序入口
│   │   ├── lib.rs        # Tauri 命令、窗口管理
│   │   ├── shared.rs     # 共享业务逻辑（章节解析、数据持久化）
│   │   └── bin/
│   │       └── server.rs # Node.js 后端（浏览器模式）
│   ├── Cargo.toml
│   └── tauri.conf.json
├── books/                # 书籍存储（已 gitignore）
├── books.json            # 书架数据（已 gitignore）
└── settings.json         # 用户设置（已 gitignore）
```

## 技术栈

| 层 | 技术 |
|---|---|
| 桌面框架 | Tauri v2 (Rust) |
| 前端 | 原生 HTML/CSS/JS，零框架 |
| WebView | WebView2 (Windows) |
| 浏览器模式 | Node.js + tiny_http |
| 编码检测 | TextDecoder + 启发式评分 |
