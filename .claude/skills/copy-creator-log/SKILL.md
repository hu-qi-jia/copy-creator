---
name: copy-creator-log
description: Update the Copy Creator project development log (project_process.md). Use when user says "update dev log", "update project log", "update project process", "记录开发进度", or asks to update the project development journal.
---

# Copy Creator 开发日志管理

更新 `docs/project_process.md` 开发日志文件，记录项目进度、新完成事项、当前问题、下一步规划。

## 项目文档位置

- 开发日志: `docs/project_process.md`
- PRD: `docs/PRD.md`
- 架构: `docs/ARCHITECTURE.md`
- 项目代码: `copy-creator/`

## 工作流

### 1. 扫描变更

在更新开发日志之前，先了解当前状态：

- 阅读 `docs/project_process.md` 获取上次记录的进度
- 检查 `copy-creator/src-tauri/src/` 下 Rust 模块的变更
- 检查 `copy-creator/src/` 下前端文件的变更
- 检查是否有新的已完成任务、新引入的问题

### 2. 收集用户输入

向用户确认本次更新需要记录的内容：

- 哪些任务已完成？（从「下一步规划」中移出，加入「已完成事项」）
- 遇到了什么新问题？（追加到「当前面临的问题」）
- 下一步计划有什么变化？（更新「下一步规划」）
- 有没有新的技术笔记？（追加到「技术笔记」）

### 3. 更新文件

只修改有变化的部分，保持其余内容不变：

- **当前阶段**: 如果阶段完成，更新阶段标记（✅ / 🔄）
- **项目状态**: 如有新增维度的状态变化，更新状态表格
- **已完成事项**: 追加新完成项（`- [x] ...`）
- **当前面临的问题**: 移除已解决的问题，追加新问题
- **下一步规划**: 移除已完成的任务，调整优先级
- **技术笔记**: 追加新的踩坑记录
- **最后更新**: 更新日期

### 4. 变更原则

- 保持原文结构，不随意重组章节
- 完成后打勾 `[x]`，不再删除已完成条目（保留历史）
- 新问题追加到列表末尾，编号递增
- 技术笔记每条一行，简洁记录原因和结论

## 示例

用户完成 Phrase 1 的「内置翻译接入」后：

```markdown
## 已完成事项
- [x] 接入内置免费翻译（百度翻译 API）  ← 新增

## 当前面临的问题
1. ~~内置翻译未接入~~  ← 移除此条
...
```

## 关联文档

更新时以 `docs/PRD.md` 和 `docs/ARCHITECTURE.md` 为参考，确保开发日志中的描述与需求文档和架构设计一致。
