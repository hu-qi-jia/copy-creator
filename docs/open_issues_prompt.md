# Copy Creator — 仍待解决的问题

> 将此内容粘贴到新对话中，让 Claude 继续修复以下问题。

---

## 项目上下文

- **项目**：Copy Creator — Tauri 2.x + React + TypeScript + Zustand + SQLite 剪切板管理工具
- **窗口**：440×500 无边框透明窗口，WebView2 渲染
- **UI**：单窗口 + 左侧导航栏（剪切板/短语/翻译）+ 右侧面板
- **开发工具链**：Vite 8 (rolldown)、Rust MSVC、pnpm

## 已知约束

- Vite 8 不支持 `export type` 跨模块重新导出，需在本地定义类型（如 `type UnlistenFn = () => void`）
- JSON 必须严格标准格式（无尾随逗号），否则整个 Vite 模块图构建崩溃
- Tauri 2.x 权限模型需在 `capabilities/default.json` 显式列出每个用到的权限

---

## 问题列表

### 1. SQL 注入风险 — 搜索拼接

**文件**：`copy-creator/src-tauri/src/db.rs:133`

**问题**：`get_clipboard_records` 中搜索关键词通过 `format!` 拼入 SQL：

```rust
let pattern = format!("%{}%", q);
let mut stmt = conn.prepare(
    "SELECT ... WHERE content LIKE ?1 ORDER BY ... LIMIT ?2",
)?;
let rows = stmt.query_map(params![pattern, lim], |row| { ... })?;
```

虽然已使用参数化查询的 `?1` 占位符，但 `pattern` 本身是通过 `format!` 从用户输入 `q` 构建的，恶意 `%` 和 `_`（LIKE 通配符）仍会造成意外行为。

**需要做的**：
- 用 SQL 字符串拼接 `'%' || ?1 || '%'` 替代 Rust `format!`
- 同时对 `q` 中的 LIKE 通配符 `%`、`_`、`\` 做转义（可用 `ESCAPE '\'`）

---

### 2. 前端 clipboard-update 事件监听未激活

**文件**：`copy-creator/src/stores/clipboardStore.ts:47-61`

**问题**：`clipboardStore` 中 `init()` 方法已定义了 `listen("clipboard-update", ...)`，Rust 后端也通过 `emit("clipboard-update", ...)` 推送新剪切板记录。但 `init()` 从未被任何组件调用，实时监听器未激活。

```ts
init: () => {
    if (get().initialized) return;
    set({ initialized: true });
    listen<ClipboardRecord>("clipboard-update", (event) => { ... });
    get().loadRecords();
},
```

**需要做的**：在 `ClipboardPage` 或 `App` 的 `useEffect` 中调用 `useClipboardStore.getState().init()` 或添加 hook 调用。

---

### 3. API 凭证硬编码

**文件**：`copy-creator/src-tauri/src/db.rs:78-79`

**问题**：百度翻译 AppID 和 Secret 直接写入 `init_db` 的 SQL 默认值：

```rust
INSERT OR IGNORE INTO settings (key, value) VALUES ('baidu_appid', '20260513002612590');
INSERT OR IGNORE INTO settings (key, value) VALUES ('baidu_secret', 'x81YvHc1JGpqJi8L88I_');
```

**需要做的**：
- 删除这两行硬编码
- 用户首次使用时在设置页自行配置，未配置时翻译功能返回友好错误提示

---

### 4. 过期记录清理未启用

**文件**：`copy-creator/src-tauri/src/db.rs:92-116`

**问题**：`prune_old_records()` 函数已实现，但从未在任何地方被调用。过期剪切板记录不会自动删除。

**需要做的**：
- 在应用启动时（`lib.rs` 的 `setup`）调用一次 `prune_old_records()`
- （可选）用定时器每隔一段时间（如 30 分钟）再次清理

---

### 5. 托盘图标单击事件未实现

**文件**：`copy-creator/src-tauri/src/tray.rs`（或 `lib.rs` 中的托盘创建代码）

**问题**：系统托盘仅实现了右键菜单（显示/退出）。单击托盘图标无任何行为，而架构文档指定「单击托盘图标 → 显示/隐藏窗口」。

**需要做的**：添加托盘图标的 `on_click` 事件处理器，调用切换窗口显示/隐藏的逻辑。

---

### 6. NavigationButton 死代码

**文件**：
- `copy-creator/src/components/NavigationButton.tsx`
- `copy-creator/src/components/NavigationButton.css`

**问题**：这两个文件存在但未被任何其他文件导入或使用。导航功能已由 `GlassIcons.tsx` 实现。

**需要做的**：删除 `NavigationButton.tsx` 和 `NavigationButton.css`。

---

## 修复顺序建议

1. **先做 #6（删死代码）**— 最简单，无副作用
2. **再做 #3（移除硬编码凭证）**— 安全相关，优先处理
3. **然后 #1（SQL 注入）**— 安全问题
4. **接着 #2 和 #4**— 功能修复，有依赖关系
5. **最后 #5**— 独立功能，不影响核心流程
