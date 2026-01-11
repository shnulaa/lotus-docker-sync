# Docker Sync

<p align="center">
  <a href="https://github.com/shnulaa/lotus-docker-sync/releases"><img src="https://img.shields.io/github/v/release/shnulaa/lotus-docker-sync?include_prereleases" alt="最新版本"></a>
  <a href="https://github.com/shnulaa/lotus-docker-sync/actions/workflows/build.yml"><img src="https://github.com/shnulaa/lotus-docker-sync/actions/workflows/build.yml/badge.svg" alt="构建状态"></a>
  <a href="https://github.com/shnulaa/lotus-docker-sync/releases"><img src="https://img.shields.io/github/downloads/shnulaa/lotus-docker-sync/total" alt="下载量"></a>
</p>

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

#### Linux/macOS 安装

```bash
# 下载最新版本
curl -L -o docker-sync https://github.com/shnulaa/lotus-docker-sync/releases/latest/download/docker-sync-linux-amd64

# 添加执行权限
chmod +x docker-sync

# 移动到系统路径（可选）
sudo mv docker-sync /usr/local/bin/

# 或者直接运行
./docker-sync --help
```

#### Windows 安装

```powershell
# 下载到当前目录
Invoke-WebRequest -Uri "https://github.com/shnulaa/lotus-docker-sync/releases/latest/download/docker-sync-windows-amd64.exe" -OutFile "docker-sync.exe"

# 运行
.\docker-sync.exe --help
```

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

### 大镜像同步时间长

大型镜像（如 Ubuntu、Node.js 等）同步时间可能需要 5-10 分钟，请耐心等待。小镜像（如 Alpine 系列）通常 1-2 分钟完成。

## 支持项目

如果这个工具对你有帮助，请：

- ⭐ 给项目点个 Star
- 🐛 遇到问题请提 [Issue](https://github.com/shnulaa/lotus-docker-sync/issues)
- 💡 有建议或想法也欢迎讨论

## 免责声明

本工具仅供学习和个人开发使用。使用本工具时请注意：

- 仅同步公开的开源镜像，不要同步商业或私有镜像
- 请遵守 Docker Hub、GitHub 及相关服务的使用条款
- 请遵守镜像内软件的开源许可证
- 本工具不提供任何担保，使用风险自负
- 请遵守当地法律法规

## 许可证

MIT License
