# Docker Sync

一个 Docker Hub 镜像同步工具，自动将 Docker Hub 镜像同步到 GitHub Container Registry (GHCR)，解决国内访问 Docker Hub 困难的问题。

## 快速开始

### 1. 下载

从 [Releases](https://github.com/shnulaa/lotus-docker-sync/releases) 下载对应平台的二进制文件：

| 平台 | 下载 |
|------|------|
| Linux x86_64 | `docker-sync-linux-amd64` |
| macOS x86_64 | `docker-sync-darwin-amd64` |
| macOS ARM64 | `docker-sync-darwin-arm64` |
| Windows x86_64 | `docker-sync-windows-amd64.exe` |

### 2. 登录 GitHub

```bash
# 首次使用需要登录（会自动打开浏览器）
docker-sync auth login
```

### 3. 同步镜像

```bash
# 同步 nginx
docker-sync nginx:alpine

# 同步 redis
docker-sync redis:7-alpine

# 同步 mysql
docker-sync mysql:8.0
```

### 4. 使用镜像

同步完成后，可以通过以下方式拉取镜像：

```bash
# 国内加速（推荐）
docker pull ghcr.nju.edu.cn/你的用户名/nginx:alpine

# 或直接从 GHCR
docker pull ghcr.io/你的用户名/nginx:alpine
```

## 功能特性

- 🔐 **OAuth 登录**：无需手动创建 Token，浏览器授权即可
- 🔄 **自动同步**：自动触发 GitHub Action 同步镜像
- 📊 **实时进度**：显示同步步骤和进度
- 🇨🇳 **国内加速**：使用 `ghcr.nju.edu.cn` 镜像源
- 🗑️ **智能更新**：自动删除旧版本，同步最新镜像
- ⚡ **首次自动配置**：自动创建仓库和 GitHub Action

## 命令说明

```bash
# 同步镜像（简写）
docker-sync nginx:alpine

# 同步镜像（完整）
docker-sync pull nginx:alpine

# 登录
docker-sync auth login

# 查看登录状态
docker-sync auth status

# 登出
docker-sync auth logout

# 删除已同步的镜像
docker-sync delete nginx
```

## 工作原理

1. **首次使用**：自动在你的 GitHub 账号下创建 `docker-sync` 仓库
2. **触发同步**：通过 GitHub Action 从 Docker Hub 拉取镜像并推送到 GHCR
3. **国内访问**：使用南京大学镜像 `ghcr.nju.edu.cn` 加速访问

## 示例输出

```
$ docker-sync nginx:alpine

🔍 检查镜像 ghcr.nju.edu.cn/shnulaa/nginx:alpine
🚀 启动 GitHub Action 同步...
📋 工作流已启动，ID: 1234567890
  ✓ Set up job
  ✓ Checkout repository
  ✓ Set up Docker Buildx
  ✓ Log in to GitHub Container Registry
  ✓ Sync image
✅ 同步成功！
🎉 同步完成！正在从 ghcr.nju.edu.cn/shnulaa/nginx:alpine 拉取镜像...
```

## 配置文件

配置文件位置：
- Linux/macOS: `~/.config/docker-sync-cli/config.json`
- Windows: `%APPDATA%\docker-sync-cli\config.json`

## 常见问题

### 同步失败：permission_denied

重新登录以获取最新权限：
```bash
docker-sync auth logout
docker-sync auth login
```

### 未安装 Docker

如果本地未安装 Docker，同步完成后会提示手动拉取命令。

## 许可证

MIT License
