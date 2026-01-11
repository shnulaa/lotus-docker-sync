# Lotus Docker Sync

<p align="center">
  <a href="https://github.com/shnulaa/lotus-docker-sync/releases"><img src="https://img.shields.io/github/v/release/shnulaa/lotus-docker-sync?include_prereleases" alt="最新版本"></a>
  <a href="https://github.com/shnulaa/lotus-docker-sync/actions/workflows/build.yml"><img src="https://github.com/shnulaa/lotus-docker-sync/actions/workflows/build.yml/badge.svg" alt="构建状态"></a>
  <a href="https://github.com/shnulaa/lotus-docker-sync/releases"><img src="https://img.shields.io/github/downloads/shnulaa/lotus-docker-sync/total" alt="下载量"></a>
</p>

一个 Docker Hub 镜像同步工具，用于将 Docker Hub 上的镜像同步至 GitHub Container Registry（GHCR），提升在复杂网络环境下的镜像获取稳定性与使用体验。

## ✨ 核心亮点

- 🏠 **创建专属镜像库**：在你的 GitHub 账号下自动创建私有镜像仓库
- 🚀 **一键同步**：自动将 Docker Hub 镜像同步到你的 GHCR
- 🇨🇳 **国内加速访问**：使用 `ghcr.nju.edu.cn` 镜像源，告别龟速下载
- 🔐 **完全私有**：所有镜像存储在你的 GitHub 账号下，安全可控
- ⚡ **智能管理**：自动更新镜像版本，无需手动维护GitHub Container Registry 国内加速：使用 ghcr.nju.edu.cn 镜像源 

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
# 国内加速（推荐）- 从你的专属镜像库拉取
docker pull ghcr.nju.edu.cn/你的用户名/nginx:alpine

# 或直接从你的 GHCR 私有库
docker pull ghcr.io/你的用户名/nginx:alpine
```

🎉 **恭喜！你现在拥有了自己的 Docker 镜像私有库！**

## 功能特性

- 🔐 **OAuth 登录**：无需手动创建 Token，浏览器授权即可
- 🏗️ **自动建库**：首次使用自动在你的 GitHub 创建专属镜像仓库
- � **自动同步***：自动触发 GitHub Action 同步镜像
- 📊 **实时进度**：显示同步步骤和进度
- 🇨🇳 **国内加速**：使用 `ghcr.nju.edu.cn` 镜像源
- 🗑️ **智能更新**：自动删除旧版本，同步最新镜像
- ⚡ **零配置**：一键登录，立即使用

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

1. **创建专属仓库**：首次使用时在你的 GitHub 账号下创建 `docker-sync` 仓库
2. **自动化同步**：通过 GitHub Action 从 Docker Hub 拉取镜像并推送到你的 GHCR
3. **私有镜像库**：所有镜像存储在 `ghcr.io/你的用户名/` 下，完全私有
4. **国内加速**：使用南京大学镜像 `ghcr.nju.edu.cn` 加速访问

💡 **相当于拥有了自己的 Docker Hub 私有镜像站！**

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
- 本项目为技术学习与个人开发用途的自动化工具。使用本项目即表示您已理解并同意以下条款：
- 本工具仅用于同步公开可访问且具有合法再分发许可的 Docker 镜像
- 使用者应自行确认镜像及其包含软件的许可证与使用条件
- 禁止同步商业授权镜像、私有镜像、破解镜像或其他未经授权内容
- 使用过程中应遵守 Docker Hub、GitHub、GitHub Container Registry 及相关服务条款
- 本项目不对镜像内容的合法性、可用性或适用性作出任何保证
- 因使用本工具产生的风险或法律责任由使用者自行承担
- 使用者应遵守所在国家或地区的相关法律法规

## 许可证

MIT License
