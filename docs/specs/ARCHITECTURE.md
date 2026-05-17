# Copy Creator — 产品架构文档

## 1. 技术栈

| 层        | 选型                            | 理由                                        |
| -------- | ----------------------------- | ----------------------------------------- |
| 桌面框架     | **Tauri 2.x**                 | 剪贴板监听、全局快捷键、系统托盘等需原生系统调用，Rust 层天然胜任；包体积极小 |
| 前端框架     | **React 18 + TypeScript**     | 生态成熟，组件化开发效率高                             |
| UI 组件库   | **纯 CSS + CSS Variables**     | iOS 风格设计，轻量无依赖，主题切换便捷                     |
| 状态管理     | **Zustand**                   | 轻量，无模板代码，适合中等复杂度                          |
| 本地存储     | **SQLite** (Rust: `rusqlite`) | 嵌入式关系数据库，零配置，Tauri 原生支持                   |
| 剪切板      | Tauri clipboard API + 原生扩展    | 文本和图片监听与写入                                |
| HTTP 客户端 | Rust: `reqwest`               | 翻译 API 调用，异步高性能                           |
| i18n     | `react-i18next`               | 前端国际化，支持多语言                               |
| 构建工具     | Vite                          | 快，React 官方推荐                              |
| 包管理      | pnpm                          | 磁盘高效，速度快                                  |

## 2. 系统架构

```Markdown
┌─────────────────────────────────────────────────────┐
│                  Copy Creator                        │
├─────────────────────────────────────────────────────┤
│                                                      │
│  ┌──────────────────────┐                           │
│  │    React 前端         │  ← TypeScript + 纯 CSS    │
│  │  ┌────┬────┬────┬──┐ │                           │
│  │  │剪切│短语│翻译│设置│ │                           │
│  │  │板页│页  │页  │页 │ │                           │
│  │  └────┴────┴────┴──┘ │                           │
│  │  ┌──────────────────┐ │                           │
│  │  │ Zustand Store    │ │  ← 前端状态               │
│  │  └──────────────────┘ │                           │
│  └──────────┬───────────┘                           │
│             │ invoke / event                          │
│  ┌──────────▼───────────┐                           │
│  │    Tauri 桥接层       │  ← IPC (Tauri Commands)    │
│  └──────────┬───────────┘                           │
│             │                                         │
│  ┌──────────▼───────────┐                           │
│  │    Rust 后端          │                           │
│  │  ┌──────────────────┐ │                           │
│  │  │ 剪切板监听模块    │ │  ← 系统剪切板 Hook         │
│  │  │ 系统托盘模块      │ │  ← 托盘图标与右键菜单      │
│  │  │ 全局快捷键模块    │ │  ← 唤起/隐藏悬浮窗         │
│  │  │ 数据库模块        │ │  ← SQLite CRUD            │
│  │  │ 翻译服务模块      │ │  ← HTTP 客户端            │
│  │  │ 粘贴执行模块      │ │  ← 模拟键盘输入            │
│  │  └──────────────────┘ │                           │
│  └──────────┬───────────┘                           │
│             │                                         │
│  ┌──────────▼───────────┐                           │
│  │    SQLite 本地数据库   │                           │
│  └──────────────────────┘                           │
│                                                      │
└─────────────────────────────────────────────────────┘
```

## 3. 模块详设

### 3.1 Rust 后端模块

#### 剪切板监听模块 `clipboard_monitor`

```
职责:
  - 启动时开始监听系统剪切板变化
  - 检测文本/图片类型变化
  - 写入 clipboard_records 表
  - 通过 Tauri Event 推送新记录到前端

技术:
  - Windows: Windows Clipboard API
  - macOS: NSPasteboard
  - 通过 Tauri event system 向前端推送

频率:
  - 轮询间隔: 500ms（平衡响应度与 CPU 占用）
  - 写入去重: 连续相同内容不重复记录
```

#### 系统托盘模块 `tray_manager`

```
职责:
  - 创建托盘图标（亮/暗色跟随主题）
  - 右键菜单: 显示窗口 / 退出
  - 单击托盘图标: 显示/隐藏悬浮窗
```

#### 全局快捷键模块 `shortcut_manager`

```
职责:
  - 注册全局快捷键
  - 用户可在设置中自定义快捷键组合
  - 按下快捷键 → 切换悬浮窗显示/隐藏

默认快捷键: Alt + Shift + V
```

#### 数据库模块 `db`

```
职责:
  - SQLite 初始化与迁移
  - 各表 CRUD 操作
  - 过期数据定时清理（每小时检查一次）

表:
  - clipboard_records
  - phrase_groups
  - phrases
  - translation_history
  - settings

数据库路径:
  - Windows: %APPDATA%/copy-creator/data.db
  - macOS:   ~/Library/Application Support/copy-creator/data.db
```

#### 翻译服务模块 `translator`

```
职责:
  - 调用 AI 翻译（用户配置的 OpenAI 兼容 API）
  - 调用内置免费翻译（百度翻译 / 有道翻译）
  - 翻译结果缓存（同文本+同语言+同引擎命中即返回）

接口封装:
  trait Translator {
    async fn translate(text: &str, source_lang: &str, target_lang: &str) -> Result<String>;
  }

  impl AITranslator    // OpenAI 兼容 API
  impl BuiltinTranslator // 百度/有道免费 API
```

#### 粘贴执行模块 `paste_executor`

```
职责:
  - 将文本写入剪切板
  - 模拟 Ctrl+V / Cmd+V 粘贴操作
  - Phase 2: 终端特殊适配
```

### 3.2 React 前端模块

```
src/
├── main.tsx                        # 入口
├── App.tsx                         # 主窗口布局 + 面板路由（React state）
├── components/                     # 通用组件
│   ├── GlassIcons.tsx              # 玻璃拟态图标按钮栏（左侧导航）
│   ├── GlassIcons.css              # 按钮栏样式（3D 玻璃拟态效果）
│   ├── Icons.tsx                   # 自定义 SVG 图标集（SF Symbols 风格）
│   ├── SearchInput.tsx             # 搜索输入组件
│   ├── IosSelect.tsx               # iOS 风格选择组件
│   ├── SettingsContent.tsx         # 设置表单主组件
│   ├── SettingsDialog.tsx          # 设置弹窗模式
│   └── settings/                   # 设置子组件
│       ├── index.ts                # 导出文件
│       ├── LanguageSection.tsx     # 语言设置（语言切换 + 快捷键录制 + 剪切板保留时长）
│       ├── StorageSection.tsx      # 存储设置（存储位置显示 + 自定义文件夹选择）
│       └── TranslationSection.tsx  # 翻译引擎设置（百度/Google/AI 配置）
├── pages/                          # 页面组件
│   ├── ClipboardPage/              # 剪切板页
│   │   ├── index.tsx               # 主组件（搜索 + 分类筛选 + 列表）
│   │   ├── ImageThumb.tsx          # 图片缩略图组件（支持悬浮预览）
│   │   └── utils.tsx               # 剪切板类型工具（分类图标 + 预览组件）
│   ├── PhrasePage/                 # 快捷短语页
│   │   ├── index.tsx               # 主组件（搜索 + 场景组 + 短语列表）
│   │   ├── GroupChips.tsx          # 场景组标签组件
│   │   ├── GroupDialog.tsx         # 新建/编辑场景组弹窗
│   │   ├── PhraseList.tsx          # 短语列表组件
│   │   ├── PhraseDialog.tsx        # 新建/编辑短语弹窗
│   │   └── ManageGroupsDialog.tsx  # 管理场景组弹窗
│   └── TranslationPage.tsx         # 翻译页（输入 + 结果展示）
├── stores/                         # Zustand 状态管理
│   ├── clipboardStore.ts           # 剪切板状态
│   ├── phraseStore.ts              # 快捷短语状态
│   ├── translationStore.ts         # 翻译状态
│   └── settingsStore.ts            # 设置状态
├── styles/                         # CSS 模块化样式
│   ├── index.css                   # 主入口（导入所有模块）
│   ├── base.css                    # 基础样式 + CSS 变量（主题色 + 动画）
│   ├── layout.css                  # 布局样式（容器 + 侧边栏 + 面板）
│   ├── components.css              # 通用组件样式（弹窗 + 按钮 + 表单）
│   ├── clipboard.css               # 剪切板页面样式
│   ├── phrases.css                 # 快捷短语页面样式
│   ├── translation.css             # 翻译页面样式
│   └── settings.css                # 设置页面样式
├── utils/
│   └── paste.ts                    # 粘贴操作工具函数
├── i18n/                           # 国际化
│   ├── index.ts
│   ├── zh-CN.json
│   └── en.json
└── types/
    └── index.ts                    # 公共类型定义
```

## 4. 数据流

### 4.1 剪切板记录流程

```
用户 Ctrl+C 复制文本
  → 系统剪切板更新
  → Rust 剪切板监听器检测到变化
  → 去重判断（与上一条相同则跳过）
  → 写入 SQLite clipboard_records 表
  → 通过 Tauri Event 推送到前端
  → React 通过 useTauriEvent 接收
  → Zustand store 更新
  → 列表 UI 重渲染
```

### 4.2 粘贴短语/剪切板记录流程

```
用户点击列表条目
  → 前端调用 Tauri Command: paste(text)
  → Rust 将文本写入系统剪切板
  → Rust 模拟 Ctrl+V / Cmd+V
  → 文本粘贴到当前光标位置
  → 悬浮窗自动隐藏（可配置）
```

### 4.3 翻译流程

```
用户粘贴文本到输入框 → 点击翻译按钮
  → 前端调用 Tauri Command: translate(text, target_lang)
  → Rust 读取 settings 判断默认引擎
  → Rust 调用对应的翻译服务
    ├─ AI 翻译: POST 用户配置的 API 端点
    └─ 内置翻译: POST 百度/有道免费 API
  → 翻译结果写入 translation_history（缓存）
  → 返回结果到前端
  → React 显示翻译结果
```

## 5. 窗口架构

```
单窗口设计:
  - 无边框透明窗口（decorations: false, transparent: true）
  - 左侧玻璃拟态图标栏 (~90px)，右侧内容面板 (~330px)
  - 收起态: ~90 × 500px，仅展示图标栏
  - 展开态: ~420 × 500px，图标栏 + 功能面板并排
  - 面板切换通过 CSS transition 实现滑入/滑出动画
  - 面板内容通过 URL param `?p=` 路由（clipboard / phrases / translate / settings）
  - 窗口支持拖拽调整尺寸（resizable: true）
  - 置顶模式通过 toggle_always_on_top 命令切换
  - 失焦不关闭（工具属性）

与 Tauri 多窗口方案的对比:
  - 放弃: WebviewWindowBuilder 动态创建子窗口（调试复杂、平台兼容性问题）
  - 采用: 单窗口 + URL param 路由，所有内容在同一个 WebView 内渲染
  - 优势: 开发调试简单、无窗口间通信开销、前端工具链完整支持
```

## 6. 构建与打包

| 目标         | 工具                   | 命令                 |
| ---------- | -------------------- | ------------------ |
| 开发调试       | Vite + Tauri CLI     | `pnpm tauri dev`   |
| Windows 打包 | Tauri Bundler → .msi | `pnpm tauri build` |
| macOS 打包   | Tauri Bundler → .dmg | `pnpm tauri build` |
| 自动更新       | Tauri updater        | GitHub Releases 分发 |

## 7. 项目目录结构

```
copy-creator/
├── src-tauri/                  # Rust 后端
│   ├── src/
│   │   ├── main.rs
│   │   ├── lib.rs              # Tauri 命令注册
│   │   ├── clipboard.rs        # 剪切板监听
│   │   ├── tray.rs             # 系统托盘
│   │   ├── shortcut.rs         # 全局快捷键
│   │   ├── db.rs               # 数据库
│   │   ├── translator.rs       # 翻译服务
│   │   └── paste.rs            # 粘贴执行
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/                        # React 前端
│   ├── components/             # 通用组件
│   │   └── settings/           # 设置子组件
│   ├── pages/                  # 页面组件
│   │   ├── ClipboardPage/      # 剪切板页
│   │   └── PhrasePage/         # 快捷短语页
│   ├── stores/                 # Zustand 状态管理
│   ├── styles/                 # CSS 模块化样式
│   ├── utils/                  # 工具函数
│   ├── i18n/                   # 国际化
│   └── types/                  # 类型定义
├── docs/                       # 项目文档
│   ├── PRD.md                  # 产品需求文档
│   ├── ARCHITECTURE.md         # 产品架构文档
│   └── project_process.md      # 开发日志
├── package.json
├── pnpm-lock.yaml
├── vite.config.ts
└── tsconfig.json
```

