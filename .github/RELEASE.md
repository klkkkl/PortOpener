# GitHub Actions 工作流说明

## 📦 自动化流水线

项目包含两个 GitHub Actions 工作流：

### 1. Release 工作流 (`release.yml`)

**触发条件**:
- 推送 tag（格式：`v*`，如 `v0.1.0`）
- 手动触发（workflow_dispatch）

**构建平台**:
- ✅ **Windows** (x64)
  - 输出: `.msi` 安装包, `.exe` 便携版
- ✅ **macOS** (Apple Silicon + Intel)
  - 输出: `.dmg` 镜像, `.app` 应用包
- ✅ **Linux** (x64)
  - 输出: `.deb`, `.AppImage`

**功能**:
- 自动构建所有平台的 release 版本
- 创建 GitHub Release（草稿模式）
- 上传构建产物到 Release

### 2. CI 工作流 (`ci.yml`)

**触发条件**:
- 推送到 `main` 或 `develop` 分支
- Pull Request 到 `main` 或 `develop` 分支

**检查项**:
- ✅ 前端类型检查 (`pnpm check`)
- ✅ Rust 编译检查 (`cargo build`)
- ✅ Rust 单元测试 (`cargo test`)
- ✅ Rust 代码格式 (`cargo fmt`)
- ✅ Rust 代码质量 (`cargo clippy`)

**测试平台**:
- Windows
- macOS
- Linux

---

## 🚀 如何发布新版本

### 方法 1: 使用 Git Tag（推荐）

```bash
# 1. 更新版本号
# 编辑 src-tauri/Cargo.toml 和 src-tauri/tauri.conf.json

# 2. 提交更改
git add .
git commit -m "chore: bump version to 0.1.0"

# 3. 创建并推送 tag
git tag v0.1.0
git push origin v0.1.0

# 4. GitHub Actions 会自动开始构建
# 5. 构建完成后，在 GitHub Releases 页面查看草稿
# 6. 编辑 Release 说明，然后发布
```

### 方法 2: 手动触发

1. 访问 GitHub 仓库的 Actions 页面
2. 选择 "Release" 工作流
3. 点击 "Run workflow"
4. 选择分支并运行

---

## 📝 版本号管理

需要同步更新以下文件中的版本号：

### 1. `src-tauri/Cargo.toml`
```toml
[package]
name = "portopener"
version = "0.1.0"  # 更新这里
```

### 2. `src-tauri/tauri.conf.json`
```json
{
  "productName": "portopener",
  "version": "0.1.0",  // 更新这里
  "identifier": "com.klkkk.portopener"
}
```

### 3. `package.json`
```json
{
  "name": "portopener",
  "version": "0.1.0",  // 更新这里
  "private": true
}
```

---

## 🔧 本地测试构建

在推送 tag 之前，建议先在本地测试构建：

```bash
# 构建 release 版本
pnpm tauri build

# 检查构建产物
# Windows: src-tauri/target/release/bundle/msi/
# macOS: src-tauri/target/release/bundle/dmg/
# Linux: src-tauri/target/release/bundle/deb/
```

---

## 📋 Release Checklist

发布新版本前的检查清单：

- [ ] 更新版本号（Cargo.toml, tauri.conf.json, package.json）
- [ ] 更新 CHANGELOG.md
- [ ] 本地测试构建成功
- [ ] 所有测试通过 (`cargo test`, `pnpm check`)
- [ ] 代码格式正确 (`cargo fmt`)
- [ ] 无 Clippy 警告 (`cargo clippy`)
- [ ] 提交所有更改
- [ ] 创建并推送 tag
- [ ] 等待 GitHub Actions 构建完成
- [ ] 编辑 Release 说明
- [ ] 发布 Release

---

## 🎯 构建产物说明

### Windows
- **MSI 安装包**: 标准 Windows 安装程序，支持卸载
- **EXE 便携版**: 单文件可执行程序，无需安装

### macOS
- **DMG 镜像**: 标准 macOS 安装包，拖拽安装
- **Universal Binary**: 同时支持 Intel 和 Apple Silicon

### Linux
- **DEB 包**: Debian/Ubuntu 系统安装包
- **AppImage**: 通用 Linux 可执行文件，无需安装

---

## 🔐 所需权限

GitHub Actions 需要以下权限：

- ✅ `contents: write` - 创建 Release 和上传文件
- ✅ `GITHUB_TOKEN` - 自动提供，无需配置

---

## 🐛 故障排查

### 构建失败

1. **检查依赖**: 确保所有依赖都在 `Cargo.toml` 和 `package.json` 中
2. **检查版本号**: 确保版本号格式正确
3. **查看日志**: 在 Actions 页面查看详细构建日志
4. **本地复现**: 在本地运行 `pnpm tauri build` 复现问题

### Release 未创建

1. **检查 tag 格式**: 必须以 `v` 开头（如 `v0.1.0`）
2. **检查权限**: 确保仓库设置中启用了 Actions 写权限
3. **查看 Actions**: 检查工作流是否成功运行

### 平台特定问题

- **macOS**: 需要 Xcode Command Line Tools
- **Linux**: 需要 WebKit2GTK 等依赖
- **Windows**: 需要 Visual Studio Build Tools

---

## 📚 相关文档

- [Tauri Actions](https://github.com/tauri-apps/tauri-action)
- [GitHub Actions 文档](https://docs.github.com/en/actions)
- [Tauri 构建指南](https://tauri.app/v1/guides/building/)

---

## 🎉 示例 Release 说明

```markdown
## PortOpener v0.1.0

### 🎉 首次发布

这是 PortOpener 的第一个正式版本！

### ✨ 功能特性

- ✅ TCP 端口转发
- ✅ UDP 端口转发（支持会话超时清理）
- ✅ 规则管理（添加、删除、启动、停止）
- ✅ 规则持久化
- ✅ 实时状态监控
- ✅ 现代化 UI 界面

### 📦 下载

选择适合你系统的安装包：

- **Windows**: `PortOpener_0.1.0_x64_en-US.msi`
- **macOS**: `PortOpener_0.1.0_universal.dmg`
- **Linux**: `portopener_0.1.0_amd64.deb` 或 `portopener_0.1.0_amd64.AppImage`

### 📖 文档

- [快速入门](https://github.com/your-repo/blob/main/QUICKSTART.md)
- [测试指南](https://github.com/your-repo/blob/main/TESTING.md)
- [项目总结](https://github.com/your-repo/blob/main/PROJECT_SUMMARY.md)

### 🐛 已知问题

无

### 🙏 致谢

感谢所有贡献者和测试者！
```

---

**最后更新**: 2026-03-07
