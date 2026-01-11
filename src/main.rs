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
                .arg(Arg::new("image").required(true).help("Image name to pull"))
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
                .subcommand(
                    Command::new("login")
                        .about("Login to GitHub using OAuth")
                )
                .subcommand(
                    Command::new("logout")
                        .about("Logout and clear stored token")
                )
                .subcommand(
                    Command::new("status")
                        .about("Show authentication status")
                )
                .subcommand(
                    Command::new("token")
                        .about("Set GitHub token manually")
                        .arg(
                            Arg::new("token")
                                .required(true)
                                .help("GitHub Personal Access Token")
                        )
                )
        )
        .arg(Arg::new("image").help("Image name to pull (shorthand for 'pull' command)"));

    let matches = matches.try_get_matches();
    
    match matches {
        Ok(matches) => {
            if let Some(pull_matches) = matches.subcommand_matches("pull") {
                let image = pull_matches.get_one::<String>("image").unwrap();
                let quiet = pull_matches.get_flag("quiet");
                let verbose = pull_matches.get_flag("verbose");
                
                handle_pull(image, quiet, verbose).await?;
            } else if let Some(auth_matches) = matches.subcommand_matches("auth") {
                handle_auth(auth_matches).await?;
            } else if let Some(image) = matches.get_one::<String>("image") {
                // Shorthand: docker-sync nginx:latest
                handle_pull(image, false, false).await?;
            } else {
                // Show help if no arguments
                println!("Use 'docker-sync --help' for usage information");
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}

async fn handle_pull(image: &str, quiet: bool, verbose: bool) -> Result<()> {
    let config = Config::load().await?;
    
    if config.github_token.is_none() {
        println!("{}", "🔐 需要先登录认证".yellow());
        println!("{}", "运行 'docker-sync auth login' 进行 GitHub 认证".cyan());
        return Ok(());
    }

    let mut github_client = GitHubClient::new(config.github_token.as_ref().unwrap());

    // Get username for image path
    let username = github_client.get_username().await?;
    let ghcr_image = format!("{}/{}/{}", config.nju_registry, username, image);

    // 解析 package 名称和 tag
    let (package_name, tag) = if image.contains(':') {
        let parts: Vec<&str> = image.split(':').collect();
        (parts[0], parts[1])
    } else {
        (image, "latest")
    };

    if !quiet {
        println!("{} {}", "🔍 检查镜像".blue(), ghcr_image.cyan());
    }

    // 检查特定版本是否存在，存在则先删除
    if github_client.package_version_exists(package_name, tag).await? {
        if !quiet {
            println!("{} 镜像 {}:{} 已存在，先删除...", "🗑️".yellow(), package_name, tag);
        }
        github_client.delete_package_version(package_name, tag).await?;
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
        println!("{} 同步完成！正在从 {} 拉取镜像...", "🎉".green(), ghcr_image.cyan());
    }
    pull_from_ghcr(&ghcr_image).await
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
                        if line.contains("Error") || line.contains("error") || line.contains("denied") || line.contains("failed") {
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
    let docker_check = process::Command::new("docker")
        .arg("--version")
        .output();
    
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
                    let mut github_client = GitHubClient::new(config.github_token.as_ref().unwrap());
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

