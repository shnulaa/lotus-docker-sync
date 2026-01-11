# Docker Sync CLI

一个独立的Docker镜像同步工具，自动将Docker Hub镜像同步到GitHub Container Registry，解决国内访问Docker Hub困难的问题。

## 功能特性

- 🔄 **独立工具**：不替换原生docker命令，安全可靠
- 🚀 **自动同步**：检测到镜像不存在时自动触发GitHub Action同步
- 📊 **实时日志**：显示同步进度和构建日志
- 🇨🇳 **国内加速**：自动使用 `ghcr.nju.edu.cn` 镜像源
- ⚡ **智能缓存**：已同步的镜像直接从GHCR拉取
- 🎯 **按需同步**：只同步你需要的镜像，不浪费资源
- 📝 **简洁语法**：支持简写模式

## 安装

### 前置要求

1. **Docker**：确保已安装Docker
2. **Rust**：安装Rust工具链
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```
3. **GitHub Token**：创建具有以下权限的Personal Access Token：
   - `actions:write` (触发workflow)
   - `packages:write` (推送到GHCR)

### Linux/macOS 安装

```bash
git clone https://github.com/shnulaa/lotus-docker-sync.git
cd lotus-docker-sync/docker-sync-cli
chmod +x install.sh
./install.sh
```

### Windows 安装

```powershell
git clone https://github.com/shnulaa/lotus-docker-sync.git
cd lotus-docker-sync/docker-sync-cli
.\install.ps1
```

## 配置

安装完成后，配置GitHub信息：

### Windows用户（推荐）
```bash
# 自动打开浏览器创建token
docker-sync config --login

# 然后配置token和仓库
docker-sync config --token YOUR_COPIED_TOKEN --repo shnulaa/lotus-docker-sync
```

### 手动配置
```bash
# 1. 访问 https://github.com/settings/tokens/new
# 2. 创建token，需要 'repo' 和 'packages:write' 权限
# 3. 配置
docker-sync config --token YOUR_TOKEN --repo shnulaa/lotus-docker-sync
```

### 查看配置
```bash
# 查看当前配置状态
docker-sync config
```

## 使用方法

### 基本使用

```bash
# 完整命令格式
docker-sync pull nginx:latest
docker-sync pull redis:7-alpine
docker-sync pull mysql:8.0

# 简写格式（自动添加pull）
docker-sync nginx:latest
docker-sync redis:7-alpine
docker-sync mysql:8.0

# 静默模式
docker-sync pull -q node:18-alpine

# 详细模式
docker-sync pull -v python:3.11-alpine
```

### 工作流程

1. **检查GHCR**：首先检查 `ghcr.nju.edu.cn/shnulaa/image:tag` 是否存在
2. **直接拉取**：如果存在，直接从GHCR拉取（快速）
3. **触发同步**：如果不存在，自动触发GitHub Action同步
4. **实时监控**：显示同步进度和日志
5. **完成拉取**：同步完成后从GHCR拉取镜像

### 与原生Docker命令配合

```bash
# 使用docker-sync同步镜像
docker-sync nginx:latest

# 使用原生docker运行容器
docker run -d nginx:latest

# 其他docker命令正常使用
docker build -t myapp .
docker ps
docker logs container_id
```

### 示例输出

```bash
$ docker-sync nginx:stable-alpine3.23

🔍 Checking ghcr.nju.edu.cn/shnulaa/nginx:stable-alpine3.23...
❌ Image not found, triggering sync...
🚀 Starting GitHub Action sync...
📋 Workflow started with ID: 1234567890

⠋ Sync in progress... (in_progress)
[2024-01-11 10:30:15] ✅ Checkout repository
[2024-01-11 10:30:25] ✅ Log in to GitHub Container Registry
[2024-01-11 10:30:30] 🔄 Pulling nginx:stable-alpine3.23 from Docker Hub...
[2024-01-11 10:31:45] 🏷️  Tagging image...
[2024-01-11 10:31:50] ⬆️  Pushing to ghcr.io/shnulaa/nginx:stable-alpine3.23...
[2024-01-11 10:32:30] ✅ Successfully synced nginx:stable-alpine3.23

✅ Sync completed successfully!
🎉 Now pulling from ghcr.nju.edu.cn/shnulaa/nginx:stable-alpine3.23...

stable-alpine3.23: Pulling from shnulaa/nginx
e7c96db7181b: Pull complete
...
Status: Downloaded newer image for ghcr.nju.edu.cn/shnulaa/nginx:stable-alpine3.23
```

## 其他Docker命令

docker-sync专注于镜像同步，其他Docker操作使用原生命令：

```bash
docker run -d nginx:latest    # 使用原生docker
docker build -t myapp .       # 使用原生docker
docker ps                     # 使用原生docker
```

## 配置文件

配置文件位置：
- Linux/macOS: `~/.config/docker-sync-cli/config.json`
- Windows: `%APPDATA%\docker-sync-cli\config.json`

示例配置：
```json
{
  "github_token": "ghp_xxxxxxxxxxxx",
  "github_repo": "shnulaa/lotus-docker-sync",
  "ghcr_registry": "ghcr.io",
  "nju_registry": "ghcr.nju.edu.cn"
}
```

## 卸载

### Linux/macOS
```bash
sudo rm /usr/local/bin/docker-sync
```

### Windows
```powershell
# 如果安装在用户目录
Remove-Item "$env:USERPROFILE\.cargo\bin\docker-sync.exe"
```

## 故障排除

### 常见问题

1. **GitHub Token权限不足**
   ```
   Error: Failed to trigger workflow: Bad credentials
   ```
   解决：确保token具有 `actions:write` 和 `packages:write` 权限

2. **同步超时**
   ```
   Error: Sync timeout after 10 minutes
   ```
   解决：检查GitHub Action是否正常运行，可能是网络问题

3. **镜像不存在**
   ```
   Error: Image not found in Docker Hub
   ```
   解决：确认镜像名称和标签正确

### 调试模式

使用详细模式查看完整日志：
```bash
docker-sync pull -v nginx:latest
```

### 常用别名（可选）

如果你想要更简洁的使用体验，可以设置别名：

```bash
# Linux/macOS
echo 'alias ds="docker-sync"' >> ~/.bashrc
source ~/.bashrc

# 使用
ds nginx:latest
ds pull redis:alpine
```

```powershell
# Windows PowerShell
echo 'Set-Alias ds docker-sync' >> $PROFILE
. $PROFILE

# 使用
ds nginx:latest
ds pull redis:alpine
```

## 贡献

欢迎提交Issue和Pull Request！

## 许可证

MIT License