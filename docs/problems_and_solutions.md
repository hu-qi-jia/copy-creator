# Copy Creator — 问题与解决方案总结

## 一、已解决的问题

### 1. 多窗口方案失败（WebviewWindowBuilder）

**问题**：尝试用 `WebviewWindowBuilder` 动态创建子窗口作为功能面板，`build()` 不报错但窗口不出现。调试多日后放弃。

**解决**：放弃多窗口，改用**单窗口 + 左侧功能栏 + 右侧面板**布局。面板通过 React state 切换内容，窗口始终 440px 宽，不再伸缩。

**教训**：Tauri 2.x 的多窗口 WebviewWindowBuilder 在不同 Windows 版本上有兼容性问题，动态窗口创建不如静态配置可靠。效率工具类产品（Alfred、Raycast）用单窗口伸缩/切换方案更成熟。

---

### 2. Tauri 窗口白色原生背景（WebView2 透明）

**问题**：设置了 `transparent: true` + `decorations: false`，但窗口仍有白色/灰色背景，CSS `background: transparent` 不生效。

**原因**：Windows WebView2 默认有白色背景，CSS 透明无法穿透到底层窗口。

**解决**：
```rust
// lib.rs
window.set_background_color(Some(tauri::window::Color(0, 0, 0, 0)));

// Windows DWM backdrop effect
DwmSetWindowAttribute(hwnd, DWMWA_SYSTEMBACKDROP_TYPE, &3, size_of::<i32>());
```
并配置 `tauri.conf.json`:
```json
"windows": [{ "transparent": true, "decorations": false }]
```

---

### 3. 粘贴时窗口闪烁 / 聚焦不可靠

**问题**：点击内容粘贴时窗口消失/闪烁，且部分应用粘贴不生效。

**根因**：Tauri 浮窗有键盘焦点时，Ctrl+V 投递到自身窗口。尝试了多种方案：

| 方案 | 效果 | 原因 |
|------|------|------|
| `window.hide()` / `window.show()` | 聚焦可靠 ✓ | Windows 隐藏前台窗口时精确激活上一个焦点窗口 |
| `Alt+Escape`（Z 序推底） | 部分失效 ✗ | 激活 Z 序下一个窗口，不一定是用户用的 |
| `SetForegroundWindow(HWND)` | 不可靠 ✗ | 跨进程前台窗口切换有权限限制 |
| `window.minimize()` | 较可靠 △ | 动画比 hide/show 更平滑但仍有视觉变化 |
| `set_position(-9999,-9999)` | 完全不聚焦 ✗ | 移动窗口不改变焦点 |

**最终方案**：`window.hide()` + `window.show()` + 前端 CSS opacity 消除视觉闪烁。

---

### 4. 导航栏状态冲突（设置按钮 vs 功能按钮）

**问题**：点击设置按钮时，功能按钮（剪切板/短语/翻译）仍保持选中状态；反之亦然。

**解决**：在 `GlassIcons.tsx` 中通过 `activePanelType` prop 从父组件 App.tsx 同步激活状态。当 settings 激活时传 `null` 清除导航栏选中；当功能面板激活时传对应 panelType 清除设置按钮选中。

---

### 5. CSS 灰色窗口问题（样式完全失效）

**问题**：Tauri 窗口启动后完全灰色，所有 CSS 失效，持续多日。

**排查过程**：
1. CSS 括号匹配 → 正确
2. `@font-face` 中文路径 → 不存在
3. Vite 日志 → JSON 解析错误 + 模块导入错误

**最终发现两个根因**：

**a) JSON 语法错误**
- `zh-CN.json:41` — `"translation": "翻译","更改"` 多余字符串
- `en.json:41` — `"translation": "Translation",` 尾随逗号（标准 JSON 不允许）
- 单个 JSON 文件解析失败导致 Vite 整个模块图构建崩溃，CSS 无法加载

**b) Vite 8 (rolldown) 类型导入兼容问题**
```
The requested module does not provide an export named 'UnlistenFn'
```
Vite 8 底层用 rolldown，不支持跨模块 `export type` 重新导出。`@tauri-apps/api/event` 的 `UnlistenFn` 是类型导出，rolldown 无法解析。

**解决**：不在本地导入 `UnlistenFn`，改用本地类型定义 `type UnlistenFn = () => void`。同时清除 `.vite` 缓存目录。

---

### 6. Rust 闭包类型不兼容（E0308）

**问题**：`db.rs` 的 `get_clipboard_records` 中 `if/else` 两个分支的 `query_map` 闭包产生不同类型，编译失败。

```
error[E0308]: `if` and `else` have incompatible types
expected `MappedRows<'_, {closure@...}>`, found `MappedRows<'_, {closure@...}>`
```

**解决**：将收集逻辑移到各自分支内部，避免跨分支共享 `rows` 变量：
```rust
if let Some(q) = search {
    // prepare + query_map + collect 在 if 分支内完成
} else {
    // prepare + query_map + collect 在 else 分支内完成  
}
Ok(records)
```

---

### 7. Tauri 窗口尺寸权限缺失

**问题**：前端调用 `getCurrentWindow().setSize()` 时报错：
```
window.set_size not allowed. Permissions associated with this command: core:window:allow-set-size
```

**解决**：在 `capabilities/default.json` 中添加：
```json
"core:window:allow-set-size",
"core:window:allow-set-position",
"core:window:allow-set-focus",
"core:window:allow-show"
```

---

### 8. 端口占用 / 旧进程残留

**问题**：重启 Tauri dev 时端口 5173 被占用，或 HotKey 已注册导致 panic。

**解决**：启动前先清理：
```bash
taskkill -F -IM copy-creator.exe
# 找到占用 5173 的 PID 并 kill
netstat -ano | grep ":5173" | awk '{print $5}' | xargs -I{} taskkill -PID {} -F
# 清除 Vite 缓存
rm -rf node_modules/.vite
```

---

## 二、关键架构决策

| 决策 | 结论 | 理由 |
|------|------|------|
| 多窗口 vs 单窗口 | **单窗口** | Tauri 动态窗口创建不稳定，单窗口 + React state 更可靠 |
| 面板路由方式 | **React state** | 比 URL param 简单，不需要处理编码问题 |
| CSS 方案 | **纯 CSS + CSS 变量** | 移除 MUI 依赖，减小包体积，完全控制样式 |
| 按钮设计 | **圆角矩形 + tooltip** | 比 3D 玻璃拟态更简洁，亮暗色适配更直观 |
| 粘贴聚焦 | **hide/show + opacity** | 唯一在所有应用中聚焦可靠的方案 |

---

## 三、技术栈兼容性注意事项

1. **Vite 8 不要导入 Tauri 的类型导出** — 所有 `@tauri-apps/api/*` 的 type-only export 需改为本地定义
2. **JSON 必须严格标准格式** — 不能有尾随逗号或多余内容，否则整个 Vite 构建崩溃
3. **Tauri 2.x 权限模型** — `capabilities/default.json` 需要显式列出每个用到的权限
4. **Windows WebView2 透明** — 需要 Rust 侧 `set_background_color` + DWM API 双管齐下
5. **Rust 闭包类型** — `if/else` 分支的闭包即使完全相同也会产生不同类型，需分开处理

---

## 四、最新会话问题与解决（2026-05-15）

### 9. 快捷键/托盘呼出窗口闪烁

**现象**：按一次快捷键或点击一次托盘图标，窗口闪现后消失，需操作 3 次才能稳定显示。

**根因**：
- 全局快捷键 `tauri-plugin-global-shortcut` 的 handler 在 `Press` 和 `Release` 各触发一次
- 托盘 `TrayIconEvent::Click` 在鼠标 `Down` 和 `Up` 各触发一次
- 每次触发都调用 `toggle_window()`，一个物理动作 toggle 两次

**解决**：
- 快捷键：过滤 `ShortcutState::Pressed`（`shortcut.rs:58`）
- 托盘：过滤 `MouseButtonState::Down`（`tray.rs:30`）
- 添加 `AtomicBool TOGGLING` 防重入

**API 对照**（容易写错）：
| 模块 | 正确类型 | 正确字段名 |
|------|---------|-----------|
| global-shortcut | `ShortcutState::{Pressed, Released}` | `.state` |
| tray | `MouseButtonState::{Down, Up}` — 不是 Pressed/Released | `.button_state` — 不是 `.state` |

---

### 10. 关闭按钮和 hide() 无效

**现象**：右上角关闭按钮点击无效，快捷键无法隐藏窗口。

**根因**：`capabilities/default.json` 缺少 `"core:window:allow-hide"` 权限。Tauri 2 每个 window 操作都需显式声明。

---

### 11. Google 翻译 API Key 输入框不显示

**现象**：翻译引擎默认选中 Google 时，Google API Key 输入框不出现。

**根因**：
- Zustand store `settingsStore.ts` 中 `defaultEngine` = `"builtin"`
- 但 `<select>` 中删除了 `"builtin"` 选项（只保留 google/ai）
- React 受控组件：UI 显示 Google（第一个 option），但 state 仍为 "builtin"
- 条件 `localEngine === "google"` → false，输入框不渲染

**教训**：修改 UI option 列表时，必须同步更新 store 默认值。UI 与 state 不同步是隐性 bug。

---

### 12. 过期记录未自动删除

**现象**：`prune_old_records()` 已实现（SQL DELETE）但仅启动时调用一次，托盘应用运行数天不清理。

**解决**：`lib.rs` 启动时调用 + `std::thread::spawn` 后台线程每 3600s 执行一次。

---

### 13. 图片悬浮放大预览不生效（反复尝试 4 次）

**现象**：鼠标悬浮剪切板图片缩略图时，`transform: scale(1.22)` 不生效或被裁切。

**根因**（关键 CSS 规范坑）：
- `.panel-window-body` 有 `overflow-y: auto; overflow-x: hidden;`
- `.clipboard-list` 有 `overflow-y: auto`
- **CSS 规范**：当 overflow 一个轴为 `auto/scroll/hidden`，另一个轴为 `visible` 时，`visible` 被**强制计算为 `auto`**
- 所有祖先容器都产生 clipping 上下文，缩放后的子元素被裁剪
- 即使 JS 直接设置 `el.style.transform` 也无法绕过祖先 overflow clipping
- 即使移除 `overflow: hidden`，`overflow-y: auto` 也会产生同样的 clipping

**历次尝试**：

| 尝试 | 方案 | 结果 |
|------|------|------|
| 1 | CSS `:hover` + `overflow: visible` | 失败，祖先 overflow 强制覆盖 |
| 2 | 移除父容器 `overflow: hidden` | 失败，overflow-y: auto 同样裁剪 |
| 3 | JS `onMouseEnter` 直接改 inline transform | 失败，同受 overflow clipping 限制 |
| 4 | **Fixed-position overlay + React state** | **成功** |

**最终方案代码**：
```tsx
// 状态
const [hoverPreview, setHoverPreview] = useState<{src: string} | null>(null);

// 缩略图 onMouseEnter
onMouseEnter={(e) => {
  const img = imageSrcs[r.id];
  if (!img) return;
  setHoverPreview({ src: img });
}}

// 渲染 fixed overlay（在组件最底部）
{hoverPreview && (
  <div className="thumb-hover-overlay">
    <img src={hoverPreview.src} alt="" />
  </div>
)}
```

```css
.thumb-hover-overlay {
  position: fixed;        /* 跳出所有 overflow 容器 */
  inset: 0;
  z-index: 2000;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(6px);
  pointer-events: none;   /* 关键：让鼠标事件穿透，onMouseLeave 正常触发 */
  animation: fadeIn 0.15s ease;
}
```

**核心原理**：`position: fixed` 脱离正常文档流和所有 overflow 容器，`pointer-events: none` 保证 overlay 不拦截鼠标事件，缩略图的 `onMouseLeave` 能正常触发。

---

### 14. 图片粘贴卡顿

**现象**：点击剪切板图片粘贴时界面明显卡顿。

**根因**：
- `window.minimize()` 在 Windows 11 有 ~300ms 最小化动画
- `thread::sleep(200ms)` + `thread::sleep(80ms)` = 280ms 额外等待
- 全部在主命令线程同步执行，前端 `invoke()` 阻塞等待

**解决（三重优化）**：

1. `window.minimize()` → `window.hide()`（无动画，即时生效）
2. 延迟 200ms → 100ms
3. **后台线程**：将 hide/paste/show 移入 `std::thread::spawn`，command 在 clipboard write 后立即返回

```rust
#[tauri::command]
pub fn paste_image(app: AppHandle, path: String) -> Result<(), String> {
    // ... 读取文件、解码 PNG、写剪贴板 ...
    app.clipboard().write_image(&tauri_img).map_err(...)?;

    // clipboard write 完成，立即返回；hide/paste/show 在后台执行
    let handle = app.clone();
    std::thread::spawn(move || {
        paste_with_defocus(&handle).ok();
    });

    Ok(())  // 不阻塞前端
}

fn paste_with_defocus(app: &AppHandle) -> Result<(), String> {
    window.hide()?;           // 即时隐藏，无动画
    sleep(100ms);             // 等待焦点转移
    enigo Ctrl+V;             // 执行粘贴
    window.show()?;           // 恢复窗口
    window.set_focus()?;
    Ok(())
}
```

---

### 15. 新建短语按钮不透明度问题

**现象**：亮色模式按钮仍有半透明感，暗色模式按钮几乎看不到。

**根因**：
- 亮色 `#f5f5f7` → 父容器 `backdrop-filter: blur(40px)` 导致按钮视觉半透明
- 暗色 `rgba(255, 255, 255, 0.15)` 仅 15% 不透明度

**解决**：
- 亮色：`#ffffff`（纯白，不透明）
- 暗色：`#3a3a3c`（iOS 标准暗色系统灰，不透明）

---

### 16. 暗色模式快捷短语卡片左侧竖条不可见

**现象**：`.phrase-card { --color: #111; }` 在暗色背景上消失。

**解决**：`[data-theme="dark"] .phrase-card { --color: #fff; }`

---

## 五、开发环境常见问题

| 问题 | 解决 |
|------|------|
| `cargo tauri` 命令不存在 | `npx tauri dev`（tauri-cli 未全局安装） |
| `npm run tauri` 不存在 | package.json 缺少 tauri 脚本，用 npx |
| Vite 端口 5173 占用 | `npx kill-port 5173` |
| 编译失败：无法删除 exe | `taskkill /F /IM copy-creator.exe` |
| Tauri exe 进程残留 | 每次重启前 kill 旧进程 |

## 六、未解决的问题 / 待验证（给接手者）

1. **图片悬浮 overlay 预览** — 已实现但需在 Tauri WebView2 窗口中实测，确认 fixed 定位和 backdrop-filter 正常
2. **图片粘贴后台线程 100ms 延迟** — 如果焦点未转移导致 Ctrl+V 无效，尝试增大到 150ms
3. **capabilities 中 `allow-minimize` 权限** — 已添加但最终未使用（hide 替代了 minimize），可保留或清理
4. **快捷键录制** — 更新快捷键后需要重启才能生效

## 七、关键设计约束速查

- **CSS overflow 单轴陷阱**：`overflow-y: auto` 强制 `overflow-x` 也为 auto，裁剪子元素
- **玻璃拟态子元素不透明**：需显式设置 `backdrop-filter: none`
- **Tauri 权限**：每个 window 操作必须在 capabilities 声明
- **托盘 vs 快捷键 API 差异**：事件类型名、字段名都不同，见第 9 节对照表
- **AppHandle 跨线程**：可 Clone + Send，`WebviewWindow` 操作尽量在主线程
- **JSON 严格标准**：不能有尾随逗号或多于内容，否则 Vite 构建崩溃
