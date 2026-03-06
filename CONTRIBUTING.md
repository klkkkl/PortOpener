# Contributing to PortOpener

感谢你对 PortOpener 项目的关注！我们欢迎所有形式的贡献。

## 🤝 如何贡献

### 报告 Bug

如果你发现了 bug，请创建一个 Issue 并包含以下信息：

- **Bug 描述**: 清晰简洁地描述问题
- **复现步骤**: 详细的复现步骤
- **期望行为**: 你期望发生什么
- **实际行为**: 实际发生了什么
- **环境信息**:
  - 操作系统（Windows/macOS/Linux）
  - 应用版本
  - 相关日志

### 提出新功能

如果你有新功能的想法：

1. 先检查 Issues 中是否已有类似建议
2. 创建一个 Feature Request Issue
3. 描述功能的用途和价值
4. 如果可能，提供使用场景示例

### 提交代码

#### 开发环境设置

```bash
# 1. Fork 并克隆仓库
git clone https://github.com/your-username/PortOpener.git
cd PortOpener

# 2. 安装依赖
pnpm install

# 3. 启动开发模式
pnpm tauri dev
```

#### 开发流程

1. **创建分支**
   ```bash
   git checkout -b feature/your-feature-name
   # 或
   git checkout -b fix/your-bug-fix
   ```

2. **编写代码**
   - 遵循现有代码风格
   - 添加必要的注释
   - 确保代码通过所有检查

3. **测试**
   ```bash
   # 前端检查
   pnpm check

   # Rust 测试
   cargo test --manifest-path src-tauri/Cargo.toml

   # 代码格式
   cargo fmt --manifest-path src-tauri/Cargo.toml

   # 代码质量
   cargo clippy --manifest-path src-tauri/Cargo.toml
   ```

4. **提交更改**
   ```bash
   git add .
   git commit -m "feat: add new feature"
   # 或
   git commit -m "fix: resolve bug"
   ```

5. **推送并创建 PR**
   ```bash
   git push origin feature/your-feature-name
   ```
   然后在 GitHub 上创建 Pull Request

#### Commit 消息规范

使用 [Conventional Commits](https://www.conventionalcommits.org/) 格式：

- `feat:` 新功能
- `fix:` Bug 修复
- `docs:` 文档更新
- `style:` 代码格式（不影响功能）
- `refactor:` 重构
- `perf:` 性能优化
- `test:` 测试相关
- `chore:` 构建/工具相关

示例：
```
feat: add UDP session timeout configuration
fix: resolve memory leak in TCP forwarding
docs: update installation guide
```

## 📋 代码规范

### Rust 代码

- 使用 `rustfmt` 格式化代码
- 遵循 Rust 命名约定
- 添加必要的文档注释
- 处理所有 `Result` 和 `Option`
- 避免 `unwrap()`，使用 `?` 或 `unwrap_or_else()`

### TypeScript/Svelte 代码

- 使用 TypeScript 类型注解
- 遵循 ESLint 规则
- 使用 Prettier 格式化
- 组件保持简洁，单一职责

## 🧪 测试指南

### 单元测试

```bash
# Rust 单元测试
cargo test --manifest-path src-tauri/Cargo.toml

# 查看测试覆盖率
cargo tarpaulin --manifest-path src-tauri/Cargo.toml
```

### 集成测试

参考 `TESTING.md` 进行手动测试：
- TCP 转发功能
- UDP 转发功能
- 规则管理
- 持久化功能

## 📚 项目结构

```
PortOpener/
├── .github/
│   ├── workflows/       # GitHub Actions
│   └── RELEASE.md       # 发布指南
├── src/                 # 前端代码
│   ├── routes/
│   │   └── +page.svelte # 主界面
│   └── app.html
├── src-tauri/           # Rust 后端
│   ├── src/
│   │   ├── lib.rs       # Tauri 命令
│   │   ├── forwarder.rs # 转发核心
│   │   └── main.rs
│   ├── Cargo.toml
│   └── tauri.conf.json
├── docs/                # 文档
├── README.md
├── QUICKSTART.md
├── TESTING.md
└── CONTRIBUTING.md      # 本文件
```

## 🎯 开发优先级

### 高优先级
- Bug 修复
- 性能优化
- 安全问题

### 中优先级
- 新功能（已在 Roadmap 中）
- 文档改进
- 测试覆盖

### 低优先级
- 代码重构
- UI 美化
- 实验性功能

## 🔍 代码审查

所有 PR 都需要经过代码审查：

- ✅ 代码符合规范
- ✅ 测试通过
- ✅ 文档更新（如需要）
- ✅ 无破坏性更改（或已说明）
- ✅ 性能无明显下降

## 📞 联系方式

- **GitHub Issues**: 报告 bug 或提出建议
- **GitHub Discussions**: 讨论功能和想法
- **Pull Requests**: 提交代码贡献

## 🙏 致谢

感谢所有贡献者的付出！你的贡献让 PortOpener 变得更好。

---

## 常见问题

### Q: 我不会 Rust，可以贡献吗？
A: 当然！你可以：
- 改进文档
- 报告 bug
- 提出功能建议
- 改进前端 UI
- 编写测试用例

### Q: 如何选择要做的任务？
A: 查看 Issues 中标记为 `good first issue` 或 `help wanted` 的问题。

### Q: PR 多久会被审查？
A: 通常在 1-3 天内。如果超过一周未回复，可以在 PR 中评论提醒。

### Q: 我的 PR 被拒绝了怎么办？
A: 不要气馁！查看审查意见，修改后重新提交。我们欢迎讨论和改进。

---

**再次感谢你的贡献！** 🎉
