<div align="right">

[English](./README_EN.md) | 中文

</div>

<div align="center">

<img src="copy-creator/public/logo.png" alt="Copy Creator Logo" width="120">

# Copy Creator

**桌面端效率辅助工具**

剪切板管理 · 快捷短语 · 翻译 · 快速操作

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Platform](https://img.shields.io/badge/platform-Windows%20|%20macOS-brightgreen.svg)
![Tauri](https://img.shields.io/badge/Tauri-2.x-ffc131.svg)
![React](https://img.shields.io/badge/React-19-61dafb.svg)

</div>

---

## 项目简介

Copy Creator 是一款轻量级的桌面效率工具（支持 Windows 和 macOS），以悬浮窗形式呈现，关闭后自动驻留系统托盘。它集成了剪切板历史管理、快捷短语、翻译和快速操作等功能，帮助用户在日常工作中提升文本处理效率。

## 主要功能

### 📋 剪切板管理
- 自动记录文本、图片、链接和文件路径的复制历史
- 链接自动识别分类（http/https/ftp/ftps）
- 图片文件自动导入（JPG/PNG，3MB 以内）
- 图片去重与缩略图自动生成
- 支持关键词搜索，快速定位历史内容
- 按类型筛选：全部 / 文本 / 图片 / 链接 / 文件
- 一键粘贴到当前光标位置
- 可设置保留时长（1 周 / 1 个月 / 3 个月），自动清理过期记录

### ⚡ 快捷短语
- 按场景分组管理常用话术和代码片段
- 支持自定义分组，灵活组织内容
- 点击即粘贴，无需手动复制

### 🌐 翻译
- **AI 翻译**：兼容 OpenAI API 格式，可自定义端点、密钥和模型
- **Google 翻译**：免费模式（无需 API 密钥）或付费模式，支持代理配置
- 翻译结果本地缓存，避免重复请求
- 支持 11 种目标语言：中文、英文、日语、韩语、法语、德语、西班牙语、俄语、阿拉伯语、泰语、越南语
- 源语言自动检测

### 🎯 快速操作
- **径向菜单**：鼠标手势（Ctrl+Alt+右键）或键盘快捷键触发，光标处弹出浮动面板，支持剪切板和短语两个标签页，悬停选择即可粘贴
- **快捷翻译弹窗**：键盘快捷键触发，自动读取剪切板文本并翻译为中文，光标处弹出浮动结果面板

### ⚙️ 系统功能
- 全局快捷键唤起/隐藏窗口（可自定义）
- 窗口置顶显示
- 亮色/暗色主题切换
- 开机自启动（启动后最小化到托盘）
- 自定义数据存储路径（支持迁移）
- 可调整侧边栏宽度（60-130px，窄时自动折叠）
- macOS 毛玻璃特效 / Windows Mica/Acrylic 亚克力效果
- 系统托盘驻留，菜单随语言设置动态更新

## 技术栈

| 层级 | 技术选型 |
|:---:|:---|
| 桌面框架 | [Tauri 2.x](https://tauri.app/) (Rust) |
| 前端框架 | React 19 + TypeScript |
| 构建工具 | [Vite](https://vitejs.dev/) |
| UI 样式 | 纯 CSS（iOS 风格磨砂玻璃效果） |
| 状态管理 | [Zustand](https://zustand-demo.pmnd.rs/) |
| 本地存储 | SQLite (rusqlite, bundled) |
| 国际化 | react-i18next（简体中文 / English） |
| 键盘模拟 | enigo 0.3 |
| HTTP 客户端 | reqwest 0.12 |
| 图片处理 | image 0.25 |

## 下载安装

前往 [Releases](https://github.com/hu-qi-jia/copy-creator/releases) 页面下载最新安装包：

| 安装包 | 说明 |
|:---|:---|
| `Copy Creator_x64-setup.exe` | NSIS 安装包 |
| `Copy Creator_x64_zh-CN.msi` | MSI 安装包（中文） |

**系统要求**：Windows 11 / macOS

## 操作说明

### 基本使用

1. **启动应用**：安装后双击桌面图标启动，应用将以悬浮窗形式显示
2. **驻留托盘**：关闭窗口后，应用会自动最小化到系统托盘，继续在后台运行
3. **唤起窗口**：使用全局快捷键（默认可在设置中查看）快速唤起/隐藏窗口

### 剪切板功能

1. 复制任意文本或图片，系统会自动记录到剪切板历史
2. 点击托盘图标或使用快捷键打开主窗口
3. 切换到「剪切板」标签页，浏览或搜索历史记录
4. 点击任意记录即可一键粘贴到当前光标位置

### 快捷短语功能

1. 切换到「短语」标签页
2. 点击「新建分组」创建场景分组（如：客服话术、代码片段等）
3. 在分组中添加常用短语
4. 需要使用时，点击短语即可粘贴到当前输入位置

### 翻译功能

1. 切换到「翻译」标签页
2. 输入或粘贴需要翻译的文本
3. 选择翻译方向（如：中文 → 英文）
4. 点击翻译按钮获取结果
5. 如需使用 AI 翻译，请在设置中配置 API 端点和密钥

### 快速操作

#### 径向菜单
1. 在设置中启用径向菜单功能
2. 使用鼠标手势（Ctrl+Alt+右键，仅 Windows）或自定义键盘快捷键触发
3. 在弹出的浮动面板中，通过悬停选择剪切板记录或短语
4. 释放鼠标或点击即可粘贴选中内容

#### 快捷翻译弹窗
1. 在设置中配置快捷翻译的键盘快捷键
2. 复制需要翻译的文本
3. 按下快捷键，弹窗自动读取剪切板并翻译为中文
4. 点击翻译结果即可复制

### 个性化设置

- **快捷键**：自定义全局快捷键、径向菜单快捷键、快捷翻译快捷键
- **主题**：切换亮色/暗色主题
- **语言**：切换中文 / English 界面语言
- **开机自启**：设置是否开机自动启动
- **存储管理**：自定义数据存储路径，配置剪切板历史保留时长
- **翻译引擎**：选择 Google 翻译或 AI 翻译，配置 API 参数

## 开发指南

### 环境准备

- [Node.js](https://nodejs.org/) (推荐 18+)
- [pnpm](https://pnpm.io/)
- [Rust](https://www.rust-lang.org/)
- [Tauri CLI](https://tauri.app/)

### 本地开发

```bash
# 克隆项目
git clone https://github.com/hu-qi-jia/copy-creator.git
cd copy-creator/copy-creator

# 安装依赖
pnpm install

# 启动开发模式
pnpm tauri dev

# 构建生产版本
pnpm tauri build
```

## 项目结构

```
copy-creator/
├── src/                    # 前端源码
│   ├── components/         # React 组件
│   │   ├── RadialMenu/     # 径向菜单浮动面板
│   │   ├── TranslatePopup/ # 快捷翻译弹窗
│   │   ├── settings/       # 设置面板各分区
│   │   └── ...
│   ├── pages/              # 页面组件
│   │   ├── ClipboardPage/  # 剪切板管理
│   │   ├── PhrasePage/     # 快捷短语
│   │   └── TranslationPage # 翻译
│   ├── stores/             # Zustand 状态管理
│   ├── styles/             # CSS 样式文件
│   ├── i18n/               # 国际化配置
│   └── types/              # TypeScript 类型定义
├── src-tauri/              # Tauri 后端源码
│   ├── src/                # Rust 源码
│   │   ├── clipboard.rs    # 剪切板监听与录制
│   │   ├── paste.rs        # 粘贴操作
│   │   ├── translator.rs   # 翻译引擎
│   │   ├── shortcut.rs     # 快捷键与手势
│   │   ├── tray.rs         # 系统托盘
│   │   ├── db.rs           # SQLite 数据库
│   │   └── lib.rs          # 主入口
│   └── Cargo.toml          # Rust 依赖配置
├── public/                 # 静态资源
└── package.json            # 前端依赖配置
```

## 许可证

本项目采用 [MIT 许可证](LICENSE) 开源。

---

<div align="center">

如果觉得这个项目对你有帮助，欢迎点个 Star 支持一下！

</div>
