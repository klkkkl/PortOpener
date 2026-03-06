# PortOpener - 端口转发工具

基于 Tauri 2 + SvelteKit + Rust 构建的跨平台端口转发工具。

## 功能特性

- ✅ TCP 端口转发
- ✅ UDP 端口转发
- ✅ 规则管理（添加、删除、启动、停止）
- ✅ 规则持久化（自动保存到配置文件）
- ✅ 实时状态监控
- ✅ 现代化 UI 界面

## 技术栈

- **前端**: SvelteKit (Svelte 5) + TypeScript
- **后端**: Rust + Tauri 2
- **异步运行时**: Tokio
- **包管理器**: pnpm

## 开发环境

### 前置要求

- Node.js 18+
- pnpm
- Rust 1.70+
- 操作系统: Windows / macOS / Linux

### 安装依赖

```bash
pnpm install
```

### 开发模式

```bash
pnpm tauri dev
```

### 构建生产版本

```bash
pnpm tauri build
```

## 使用说明

### 添加转发规则

1. 点击 "Add Rule" 按钮
2. 填写规则信息：
   - **Name**: 规则名称（可选）
   - **Protocol**: 选择 TCP 或 UDP
   - **Listen Address**: 本地监听地址，格式 `host:port`（如 `0.0.0.0:8080`）
   - **Target Address**: 转发目标地址，格式 `host:port`（如 `192.168.1.1:80`）
3. 点击 "Add" 保存规则

### 启动/停止转发

- 点击规则行的 "Start" 按钮启动转发
- 点击 "Stop" 按钮停止转发
- 状态会实时更新显示

### 删除规则

- 确保规则已停止
- 点击 "Delete" 按钮删除规则

## 配置文件

规则配置自动保存在：
- **Windows**: `%APPDATA%\portopener\rules.json`
- **macOS**: `~/Library/Application Support/portopener/rules.json`
- **Linux**: `~/.config/portopener/rules.json`

## 架构说明

### 后端核心

- **TCP 转发**: 使用 `TcpListener` 监听，每个连接 spawn 独立任务进行双向数据拷贝
- **UDP 转发**: 使用 `UdpSocket` + NAT 表维护客户端映射关系
- **任务管理**: 通过 `CancellationToken` 实现优雅停止

### 前端 UI

- 响应式设计，支持深色主题
- 实时状态刷新（2秒间隔）
- 输入验证和错误提示

## 开发计划

### 第一期 ✅ (已完成)
- [x] TCP/UDP 基础转发功能
- [x] 规则管理 UI
- [x] 规则持久化
- [x] 状态监控

### 第二期 (计划中)
- [ ] 流量统计
- [ ] 连接数监控
- [ ] 日志记录
- [ ] 规则导入/导出

### 第三期 (计划中)
- [ ] 多端口批量转发
- [ ] 规则分组管理
- [ ] 系统托盘支持
- [ ] 开机自启动

## License

MIT
