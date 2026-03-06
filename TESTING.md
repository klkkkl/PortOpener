# 测试指南

## 快速测试步骤

### 1. 启动应用
```bash
pnpm tauri dev
```

### 2. 测试 TCP 转发

#### 方法 1: 使用本地 HTTP 服务器
```bash
# 终端 1: 启动一个简单的 HTTP 服务器
python -m http.server 8000

# 在应用中添加规则:
# - Protocol: TCP
# - Listen: 0.0.0.0:9000
# - Target: 127.0.0.1:8000

# 终端 2: 测试转发
curl http://localhost:9000
```

#### 方法 2: 使用 netcat
```bash
# 终端 1: 启动 netcat 服务器
nc -l 8000

# 在应用中添加规则:
# - Protocol: TCP
# - Listen: 0.0.0.0:9000
# - Target: 127.0.0.1:8000

# 终端 2: 连接到转发端口
nc localhost 9000
# 输入任意文本，应该在终端 1 中看到
```

### 3. 测试 UDP 转发

```bash
# 终端 1: 启动 UDP 监听
nc -u -l 8000

# 在应用中添加规则:
# - Protocol: UDP
# - Listen: 0.0.0.0:9000
# - Target: 127.0.0.1:8000

# 终端 2: 发送 UDP 数据
echo "test message" | nc -u localhost 9000
# 应该在终端 1 中看到 "test message"
```

### 4. 测试功能清单

- [ ] 添加 TCP 规则
- [ ] 添加 UDP 规则
- [ ] 启动规则（状态变为 Running）
- [ ] 停止规则（状态变为 Stopped）
- [ ] 删除已停止的规则
- [ ] 尝试删除运行中的规则（应该提示错误）
- [ ] 输入验证（错误的地址格式应该提示错误）
- [ ] 规则持久化（重启应用后规则仍然存在）
- [ ] 键盘快捷键（ESC 关闭弹窗，Enter 提交表单）
- [ ] 状态自动刷新（启动规则后观察状态变化）
- [ ] 查看控制台日志（应该看到连接日志）

### 5. 错误场景测试

#### 端口占用
```bash
# 终端 1: 占用端口
nc -l 8080

# 在应用中添加规则监听 0.0.0.0:8080
# 启动规则应该失败，状态显示错误信息
```

#### 目标不可达
```bash
# 添加规则指向不存在的目标
# - Listen: 0.0.0.0:9000
# - Target: 192.168.255.255:9999

# 启动规则，尝试连接应该失败
```

### 6. 性能测试

#### 并发连接测试
```bash
# 使用 ab (Apache Bench) 测试
ab -n 1000 -c 10 http://localhost:9000/

# 或使用 wrk
wrk -t4 -c100 -d30s http://localhost:9000/
```

### 7. 日志检查

启动应用后，在控制台应该看到类似的日志：
```
[TCP] Listening on 0.0.0.0:9000
[TCP] New connection from 127.0.0.1:xxxxx -> 127.0.0.1:8000
[TCP] Stopped listening on 0.0.0.0:9000
```

## 常见问题

### Q: 规则启动失败
A: 检查端口是否被占用，目标地址是否可达

### Q: 规则无法删除
A: 确保规则已停止（状态为 Stopped）

### Q: 配置文件在哪里？
A:
- Windows: `%APPDATA%\portopener\rules.json`
- macOS: `~/Library/Application Support/portopener/rules.json`
- Linux: `~/.config/portopener/rules.json`

### Q: 如何查看详细日志？
A: 在开发模式下运行 `pnpm tauri dev`，日志会显示在终端中

## 下一步

如果所有测试通过，可以构建生产版本：
```bash
pnpm tauri build
```

构建产物位于 `src-tauri/target/release/bundle/`
