use anyhow::{anyhow, Result};
use clap::{Arg, Command};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::process;
use std::time::Duration;
use tokio::time::sleep;

mod auth;
mod config;
mod github;
mod registry;

use auth::{open_github_token_page, GitHubAuth};
use config::Config;
use github::GitHubClient;

#[tokio::main]
async fn main() -> Result<()> {
    let matches = Command::new("docker-sync")
        .version("1.0.0")
        .about("Docker image sync tool - automatically sync Docker Hub images to GHCR")
        .subcommand_required(false)
        .arg_required_else_help(false)
        .subcommand(
            Command::new("pull")
                .about("Pull an image, auto-sync if not available in GHCR")
                .arg(Arg::new("image").num_args(1..).help(
                    "Image name to pull (supports multiple, e.g. nginx:alpine redis:7 mysql:8.0)",
                ))
                .arg(
                    Arg::new("quiet")
                        .short('q')
                        .long("quiet")
                        .action(clap::ArgAction::SetTrue)
                        .help("Suppress verbose output"),
                )
                .arg(
                    Arg::new("verbose")
                        .short('v')
                        .long("verbose")
                        .action(clap::ArgAction::SetTrue)
                        .help("Verbose output"),
                ),
        )
        .subcommand(
            Command::new("auth")
                .about("Authentication management")
                .subcommand(Command::new("login").about("Login to GitHub using OAuth"))
                .subcommand(Command::new("logout").about("Logout and clear stored token"))
                .subcommand(Command::new("status").about("Show authentication status"))
                .subcommand(
                    Command::new("token")
                        .about("Set GitHub token manually")
                        .arg(
                            Arg::new("token")
                                .required(true)
                                .help("GitHub Personal Access Token"),
                        ),
                ),
        )
        .subcommand(
            Command::new("config")
                .about("Configuration management")
                .subcommand(
                    Command::new("set-proxy")
                        .about("Set proxy for GitHub API access")
                        .arg(
                            Arg::new("proxy")
                                .required(true)
                                .help("Proxy URL (支持 http://, https://, socks5://, 可包含用户名密码: user:pass@host:port)")
                        )
                )
                .subcommand(
                    Command::new("clear-proxy")
                        .about("Clear proxy settings")
                )
                .subcommand(
                    Command::new("show")
                        .about("Show current configuration")
                )
                .subcommand(
                    Command::new("test-proxy")
                        .about("Test proxy connection to GitHub API")
                )
        )
        .arg(Arg::new("image").help("Image name to pull (shorthand for 'pull' command)"));

    let matches = matches.try_get_matches();

    match matches {
        Ok(matches) => {
            if let Some(pull_matches) = matches.subcommand_matches("pull") {
                let images: Vec<&String> = pull_matches.get_many("image").unwrap().collect();
                let quiet = pull_matches.get_flag("quiet");
                let verbose = pull_matches.get_flag("verbose");

                handle_pull(images, quiet, verbose).await?;
            } else if let Some(auth_matches) = matches.subcommand_matches("auth") {
                handle_auth(auth_matches).await?;
            } else if let Some(config_matches) = matches.subcommand_matches("config") {
                handle_config(config_matches).await?;
            } else if let Some(image) = matches.get_one::<String>("image") {
                // Shorthand: docker-sync nginx:latest
                handle_pull(vec![image], false, false).await?;
            } else {
                // Show help if no arguments
                println!("Docker Sync - Docker Hub 镜像同步工具");
                println!();
                println!("使用方法:");
                println!("  docker-sync <镜像名>                    同步单个镜像");
                println!("  docker-sync pull <镜像1> <镜像2> ...    批量同步镜像");
                println!();
                println!("认证管理:");
                println!("  docker-sync auth login                  GitHub OAuth 登录");
                println!("  docker-sync auth status                 查看登录状态");
                println!("  docker-sync auth logout                 登出");
                println!();
                println!("配置管理:");
                println!("  docker-sync config set-proxy <URL>     设置代理");
                println!("  docker-sync config clear-proxy         清除代理");
                println!("  docker-sync config test-proxy          测试代理连接");
                println!("  docker-sync config show                显示配置");
                println!();
                println!("示例:");
                println!("  docker-sync nginx:alpine               同步 nginx:alpine");
                println!("  docker-sync pull redis:7 mysql:8.0     批量同步");
                println!("  docker-sync config set-proxy http://127.0.0.1:7890");
                println!("  docker-sync config set-proxy socks5://user:pass@127.0.0.1:1080");
                println!();
                println!("更多帮助: docker-sync --help");
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}

async fn handle_pull(images: Vec<&String>, quiet: bool, verbose: bool) -> Result<()> {
    let config = Config::load().await?;

    if config.github_token.is_none() {
        println!("{}", "🔐 需要先登录认证".yellow());
        println!(
            "{}",
            "运行 'docker-sync auth login' 进行 GitHub 认证".cyan()
        );
        return Ok(());
    }

    let mut github_client = GitHubClient::new_with_proxy(
        config.github_token.as_ref().unwrap(),
        config.proxy.as_deref()
    );
    let username = github_client.get_username().await?;

    if images.len() > 1 && !quiet {
        println!("{} 准备同步 {} 个镜像...", "📦".blue(), images.len());
    }

    for (idx, image) in images.iter().enumerate() {
        if images.len() > 1 && !quiet {
            println!();
            println!(
                "{} [{}/{}] 处理镜像: {}",
                "▶".cyan(),
                idx + 1,
                images.len(),
                image.cyan()
            );
        }

        let ghcr_image = format!("{}/{}/{}", config.nju_registry, username, image);

        // 解析 package 名称和 tag
        let (package_name, tag) = if image.contains(':') {
            let parts: Vec<&str> = image.split(':').collect();
            (parts[0], parts[1])
        } else {
            (image.as_str(), "latest")
        };

        if !quiet {
            println!("{} {}", "🔍 检查镜像".blue(), ghcr_image.cyan());
        }

        // 检查特定版本是否存在，存在则先删除
        if github_client
            .package_version_exists(package_name, tag)
            .await?
        {
            if !quiet {
                println!(
                    "{} 镜像 {}:{} 已存在，先删除...",
                    "🗑️".yellow(),
                    package_name,
                    tag
                );
            }
            github_client
                .delete_package_version(package_name, tag)
                .await?;
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }

        if !quiet {
            println!("{} 启动 GitHub Action 同步...", "🚀".bright_blue());
            println!("{} 注意：大镜像同步时间较长，请耐心等待", "💡".yellow());
        }

        // Trigger GitHub Action
        let run_id = github_client.trigger_sync(image).await?;
        let repo_name = format!("{}/docker-sync", username);

        if !quiet {
            println!("{} 工作流已启动，ID: {}", "📋".yellow(), run_id);
        }

        // Monitor progress
        monitor_sync_progress(&github_client, run_id, &repo_name, quiet, verbose).await?;

        // Pull from GHCR after sync
        if !quiet {
            println!(
                "{} 同步完成！正在从 {} 拉取镜像...",
                "🎉".green(),
                ghcr_image.cyan()
            );
        }
        pull_from_ghcr(&ghcr_image).await?;
    }

    if images.len() > 1 && !quiet {
        println!();
        println!("{} 全部 {} 个镜像同步完成！", "🎉".green(), images.len());
    }

    Ok(())
}

async fn monitor_sync_progress(
    github_client: &GitHubClient,
    run_id: u64,
    repo_name: &str,
    quiet: bool,
    _verbose: bool,
) -> Result<()> {
    let pb = if !quiet {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.blue} {msg}")
                .unwrap(),
        );
        pb.set_message("等待同步完成...");
        pb.enable_steady_tick(Duration::from_millis(100));
        Some(pb)
    } else {
        None
    };

    let mut sync_completed = false;
    let mut printed_steps: std::collections::HashSet<String> = std::collections::HashSet::new();

    while !sync_completed {
        let status = github_client.get_run_status(run_id, repo_name).await?;

        match status.as_str() {
            "completed" => {
                sync_completed = true;
                if let Some(pb) = &pb {
                    pb.finish_with_message("✅ 同步成功！");
                }
            }
            "in_progress" | "queued" => {
                // 获取当前步骤
                if let Ok(steps) = github_client.get_job_steps(run_id, repo_name).await {
                    for step in &steps {
                        let step_status = step["status"].as_str().unwrap_or("");
                        let step_name = step["name"].as_str().unwrap_or("");
                        let conclusion = step["conclusion"].as_str().unwrap_or("");

                        if step_status == "completed" && conclusion == "success" {
                            // 只输出一次
                            if !printed_steps.contains(step_name) {
                                printed_steps.insert(step_name.to_string());
                                if let Some(pb) = &pb {
                                    pb.suspend(|| {
                                        println!("  {} {}", "✓".green(), step_name);
                                    });
                                }
                            }
                        } else if step_status == "in_progress" {
                            if let Some(pb) = &pb {
                                pb.set_message(format!("正在执行: {}", step_name));
                            }
                        }
                    }
                }
            }
            "failure" | "cancelled" => {
                if let Some(pb) = &pb {
                    pb.finish_with_message("❌ 同步失败！");
                }

                // 获取错误信息
                if let Ok(logs) = github_client.get_run_logs(run_id, repo_name).await {
                    println!("\n{}", "📋 错误详情:".red());
                    for line in logs.lines() {
                        if line.contains("Error")
                            || line.contains("error")
                            || line.contains("denied")
                            || line.contains("failed")
                        {
                            println!("{}", line.red());
                        }
                    }
                }

                return Err(anyhow!("GitHub Action 同步失败: {}", status));
            }
            _ => {
                if let Some(pb) = &pb {
                    pb.set_message(format!("状态: {}", status));
                }
            }
        }

        sleep(Duration::from_secs(3)).await;
    }

    Ok(())
}

#[allow(dead_code)]
fn format_log_line(line: &str) -> String {
    if line.contains("✅") || line.contains("Successfully") {
        line.green().to_string()
    } else if line.contains("❌") || line.contains("Error") || line.contains("Failed") {
        line.red().to_string()
    } else if line.contains("🔄") || line.contains("Pulling") || line.contains("Pushing") {
        line.yellow().to_string()
    } else {
        line.to_string()
    }
}

async fn pull_from_ghcr(image: &str) -> Result<()> {
    // 检查 docker 是否安装
    let docker_check = process::Command::new("docker").arg("--version").output();

    match docker_check {
        Ok(output) if output.status.success() => {
            // Docker 已安装，执行 pull
            let mut cmd = process::Command::new("docker");
            cmd.arg("pull").arg(image);

            let status = cmd.status()?;
            if !status.success() {
                return Err(anyhow!("拉取镜像失败"));
            }
            Ok(())
        }
        _ => {
            // Docker 未安装
            println!();
            println!("{}", "⚠️  未检测到 Docker，请手动拉取镜像:".yellow());
            println!("   docker pull {}", image.cyan());
            Ok(())
        }
    }
}

async fn handle_auth(matches: &clap::ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("login", _)) => {
            println!("{}", "🔐 Starting GitHub authentication...".blue());

            // 实现真正的OAuth Device Flow
            match GitHubAuth::login_with_browser().await {
                Ok(token) => {
                    let mut config = Config::load().await.unwrap_or_default();
                    config.github_token = Some(token);
                    config.save().await?;

                    println!("{}", "✅ Authentication successful!".green());

                    // 验证并显示用户名
                    let mut github_client =
                        GitHubClient::new(config.github_token.as_ref().unwrap());
                    if let Ok(username) = github_client.get_username().await {
                        println!("{} Authenticated as: {}", "👤".blue(), username.cyan());
                    }
                }
                Err(e) => {
                    println!("{} Authentication failed: {}", "❌".red(), e);
                    println!();
                    println!("{}", "Fallback: Manual token creation".yellow());
                    open_github_token_page()?;
                    println!("{}", "After creating your token, save it with:".yellow());
                    println!("{}", "docker-sync auth token YOUR_TOKEN".cyan());
                }
            }

            Ok(())
        }
        Some(("token", sub_matches)) => {
            let token = sub_matches.get_one::<String>("token").unwrap();

            let mut config = Config::load().await.unwrap_or_default();
            config.github_token = Some(token.clone());
            config.save().await?;

            println!("{}", "✅ Token saved successfully".green());

            // Verify token
            let mut github_client = GitHubClient::new(token);
            match github_client.get_username().await {
                Ok(username) => {
                    println!("{} Authenticated as: {}", "👤".blue(), username.cyan());
                }
                Err(e) => {
                    println!("{} Warning: Could not verify token: {}", "⚠️".yellow(), e);
                }
            }

            Ok(())
        }
        Some(("logout", _)) => {
            let mut config = Config::load().await.unwrap_or_default();
            config.github_token = None;
            config.save().await?;

            println!("{}", "✅ Logged out successfully".green());
            Ok(())
        }
        Some(("status", _)) => {
            let config = Config::load().await?;

            if let Some(_) = config.github_token {
                println!("{}", "✅ Authenticated".green());

                // Try to get username
                let mut github_client = GitHubClient::new(config.github_token.as_ref().unwrap());
                match github_client.get_username().await {
                    Ok(username) => println!("Username: {}", username.cyan()),
                    Err(_) => println!("{}", "⚠️  Token may be invalid".yellow()),
                }
            } else {
                println!("{}", "❌ Not authenticated".red());
                println!("{}", "Run 'docker-sync auth login' to authenticate".cyan());
            }
            Ok(())
        }
        _ => {
            println!("Available auth commands:");
            println!("  login   - Authenticate with GitHub");
            println!("  logout  - Clear stored authentication");
            println!("  status  - Show authentication status");
            println!("  token   - Set token manually");
            Ok(())
        }
    }
}

async fn handle_config(matches: &clap::ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("set-proxy", sub_matches)) => {
            let proxy = sub_matches.get_one::<String>("proxy").unwrap();
            
            let mut config = Config::load().await.unwrap_or_default();
            config.proxy = Some(proxy.clone());
            config.save().await?;
            
            println!("{} 代理已设置为: {}", "✅".green(), proxy.cyan());
            Ok(())
        }
        Some(("clear-proxy", _)) => {
            let mut config = Config::load().await.unwrap_or_default();
            config.proxy = None;
            config.save().await?;
            
            println!("{} 代理设置已清除", "✅".green());
            Ok(())
        }
        Some(("show", _)) => {
            let config = Config::load().await.unwrap_or_default();
            
            println!("{}", "📋 当前配置:".blue());
            println!("  认证状态: {}", if config.github_token.is_some() { "已登录".green() } else { "未登录".red() });
            println!("  默认镜像源: {}", config.default_registry.cyan());
            println!("  代理设置: {}", 
                if let Some(proxy) = &config.proxy { 
                    proxy.cyan() 
                } else { 
                    "未设置".dimmed() 
                }
            );
            Ok(())
        }
        Some(("test-proxy", _)) => {
            let config = Config::load().await.unwrap_or_default();
            
            if let Some(proxy) = &config.proxy {
                println!("{} 测试代理连接: {}", "🔍".blue(), proxy.cyan());
                test_proxy_connection(proxy).await?;
            } else {
                println!("{} 未设置代理", "⚠️".yellow());
            }
            Ok(())
        }
        _ => {
            println!("可用的配置命令:");
            println!("  set-proxy <URL>  - 设置代理 (支持 http://, https://, socks5://)");
            println!("  clear-proxy      - 清除代理设置");
            println!("  show             - 显示当前配置");
            println!("  test-proxy       - 测试代理连接");
            println!();
            println!("代理示例:");
            println!("  docker-sync config set-proxy http://127.0.0.1:7890");
            println!("  docker-sync config set-proxy socks5://127.0.0.1:1080");
            println!("  docker-sync config set-proxy http://user:pass@127.0.0.1:7890");
            Ok(())
        }
    }
}
async fn test_proxy_connection(proxy_url: &str) -> Result<()> {
    use reqwest::Client;
    use std::time::Duration;
    
    println!("{} 正在测试代理连接...", "⏳".yellow());
    
    // 检测代理类型
    if proxy_url.starts_with("http://") {
        println!("{} 检测到 HTTP 代理", "🌐".blue());
    } else if proxy_url.starts_with("socks5://") {
        println!("{} 检测到 SOCKS5 代理", "🌐".blue());
    } else {
        println!("{} 未知代理协议", "⚠️".yellow());
    }
    
    // 创建代理配置
    let client = match reqwest::Proxy::all(proxy_url) {
        Ok(proxy) => {
            println!("{} 代理配置解析成功", "✓".green());
            match Client::builder()
                .proxy(proxy)
                .timeout(Duration::from_secs(10))
                .build() 
            {
                Ok(client) => {
                    println!("{} HTTP 客户端创建成功", "✓".green());
                    client
                }
                Err(e) => {
                    println!("{} HTTP 客户端创建失败: {}", "❌".red(), e);
                    return Err(anyhow!("客户端创建失败"));
                }
            }
        }
        Err(e) => {
            println!("{} 代理配置解析失败: {}", "❌".red(), e);
            return Err(anyhow!("代理配置无效"));
        }
    };
    
    // 测试连接到 GitHub API
    println!("{} 测试连接到 GitHub API...", "🔍".blue());
    
    match client
        .get("https://api.github.com")
        .header("User-Agent", "docker-sync-cli-test")
        .send()
        .await 
    {
        Ok(response) => {
            let status = response.status();
            println!("{} GitHub API 响应: {}", "✓".green(), status);
            
            if status.is_success() {
                println!("{} 代理连接测试成功！", "🎉".green());
                
                // 显示响应头信息
                if let Some(server) = response.headers().get("server") {
                    println!("  服务器: {:?}", server);
                }
            } else if status == 403 {
                println!("{} 代理连接正常！(403 是预期响应，因为未提供认证)", "🎉".green());
                println!("  这表明代理服务器工作正常，可以访问 GitHub API");
            } else {
                println!("{} API 返回状态码: {} (可能正常)", "⚠️".yellow(), status);
                println!("  代理连接本身是成功的");
            }
        }
        Err(e) => {
            println!("{} 连接失败: {}", "❌".red(), e);
            
            // 提供诊断建议
            let error_msg = e.to_string();
            if error_msg.contains("timeout") {
                println!("{} 可能原因: 代理服务器响应超时", "💡".yellow());
                println!("  建议: 检查代理服务器是否正常运行");
            } else if error_msg.contains("connection") || error_msg.contains("refused") {
                println!("{} 可能原因: 无法连接到代理服务器", "💡".yellow());
                println!("  建议: 检查代理地址和端口是否正确");
            } else if error_msg.contains("socks") {
                println!("{} 可能原因: SOCKS5 代理配置问题", "💡".yellow());
                println!("  建议: 尝试使用 HTTP 代理格式 (http://{})", 
                    proxy_url.strip_prefix("socks5://").unwrap_or(proxy_url));
            } else if error_msg.contains("dns") {
                println!("{} 可能原因: DNS 解析失败", "💡".yellow());
                println!("  建议: 检查网络连接或使用 IP 地址");
            }
            
            return Err(anyhow!("代理连接测试失败"));
        }
    }
    
    Ok(())
}