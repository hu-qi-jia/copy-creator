<div align="right">

[English](./README_EN.md) | 中文

</div>

<div align="center">

<img src="copy-creator/public/logo.png" alt="Copy Creator Logo" width="120">

# Copy Creator

**PC 端效率辅助工具**

剪切板管理 · 快捷短语 · 翻译

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Platform](https://img.shields.io/badge/platform-Windows%2010+-brightgreen.svg)
![Tauri](https://img.shields.io/badge/Tauri-2.x-ffc131.svg)
![React](https://img.shields.io/badge/React-19-61dafb.svg)

</div>

---

## 项目简介

Copy Creator 是一款轻量级的 Windows 桌面效率工具，以悬浮窗形式呈现，关闭后自动驻留系统托盘。它集成了剪切板历史管理、快捷短语和翻译三大核心功能，帮助用户在日常工作中提升文本处理效率。

## 主要功能

### 📋 剪切板管理
- 自动记录文本和图片的复制历史
- 支持关键词搜索，快速定位历史内容
- 一键粘贴到当前光标位置
- 可设置保留时长，自动清理过期记录

### ⚡ 快捷短语
- 按场景分组管理常用话术和代码片段
- 支持自定义分组，灵活组织内容
- 点击即粘贴，无需手动复制

### 🌐 翻译
- **AI 翻译**：兼容 OpenAI API 格式，可自定义端点和模型
- **内置翻译**：免费翻译服务，开箱即用
- 翻译结果本地缓存，避免重复请求

### ⚙️ 系统功能
- 全局快捷键唤起/隐藏窗口
- 窗口置顶显示
- 亮色/暗色主题切换
- 开机自启动

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

## 下载安装

前往 [Releases](https://github.com/hu-qi-jia/copy-creator/releases) 页面下载最新安装包：

| 安装包 | 说明 |
|:---|:---|
| `Copy Creator_x64-setup.exe` | NSIS 安装包 |
| `Copy Creator_x64_zh-CN.msi` | MSI 安装包（中文） |

**系统要求**：Windows 11

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

### 个性化设置

- **快捷键**：自定义全局快捷键
- **主题**：切换亮色/暗色主题
- **开机自启**：设置是否开机自动启动
- **存储管理**：配置剪切板历史保留时长

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
│   ├── pages/              # 页面组件
│   ├── stores/             # Zustand 状态管理
│   ├── styles/             # CSS 样式文件
│   ├── i18n/               # 国际化配置
│   └── types/              # TypeScript 类型定义
├── src-tauri/              # Tauri 后端源码
│   ├── src/                # Rust 源码
│   └── Cargo.toml          # Rust 依赖配置
├── public/                 # 静态资源
└── package.json            # 前端依赖配置
```

## 许可证

本项目采用 [MIT 许可证](LICENSE) 开源。

---

<div align="center">

如果觉得这个项目对你有帮助，欢迎点个 Star 支持一下！

感谢baihejiangnan的贡献！


</div>
