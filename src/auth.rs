use anyhow::{anyhow, Result};
use colored::*;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use tokio::time::{sleep, Duration};

const CLIENT_ID: &str = "Ov23li7Y8uyN0cW2UHeS";

#[derive(Debug, Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
}

#[derive(Debug, Deserialize)]
struct AccessTokenResponse {
    access_token: Option<String>,
    #[allow(dead_code)]
    token_type: Option<String>,
    #[allow(dead_code)]
    scope: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

pub struct GitHubAuth {
    client: Client,
}

impl GitHubAuth {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
    
    pub async fn login_with_browser() -> Result<String> {
        let auth = Self::new();
        
        // 1. 获取设备码
        let device_code_response = auth.get_device_code().await?;
        
        // 2. 显示验证码并打开浏览器
        println!();
        println!("{}", "📋 Please complete the following steps:".yellow());
        println!("1. Open your browser to: {}", device_code_response.verification_uri.cyan());
        println!("2. Enter this code: {}", device_code_response.user_code.bright_green().bold());
        println!("3. Authorize the application");
        println!();
        
        // 打开浏览器
        #[cfg(windows)]
        {
            let _ = webbrowser::open(&device_code_response.verification_uri);
        }
        
        #[cfg(not(windows))]
        {
            let _ = std::process::Command::new("open")
                .arg(&device_code_response.verification_uri)
                .spawn();
        }
        
        println!("{}", "⏳ Waiting for authorization...".blue());
        
        // 3. 轮询获取访问令牌
        let token = auth.poll_for_token(&device_code_response).await?;
        
        Ok(token)
    }
    
    async fn get_device_code(&self) -> Result<DeviceCodeResponse> {
        let mut params = HashMap::new();
        params.insert("client_id", CLIENT_ID);
        params.insert("scope", "repo workflow write:packages read:packages delete:packages delete_repo admin:repo_hook");
        
        let response = self
            .client
            .post("https://github.com/login/device/code")
            .header("Accept", "application/json")
            .header("User-Agent", "docker-sync-cli")
            .form(&params)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Failed to get device code: {}", error_text));
        }
        
        let device_code: DeviceCodeResponse = response.json().await?;
        Ok(device_code)
    }
    
    async fn poll_for_token(&self, device_code: &DeviceCodeResponse) -> Result<String> {
        let mut interval = device_code.interval;
        let max_attempts = device_code.expires_in / interval;
        let mut attempts = 0;
        
        loop {
            attempts += 1;
            if attempts > max_attempts {
                return Err(anyhow!("Authentication timeout. Please try again."));
            }
            
            sleep(Duration::from_secs(interval)).await;
            
            let mut params = HashMap::new();
            params.insert("client_id", CLIENT_ID);
            params.insert("device_code", device_code.device_code.as_str());
            params.insert("grant_type", "urn:ietf:params:oauth:grant-type:device_code");
            
            let response = self
                .client
                .post("https://github.com/login/oauth/access_token")
                .header("Accept", "application/json")
                .header("User-Agent", "docker-sync-cli")
                .form(&params)
                .send()
                .await?;
                
            if response.status().is_success() {
                let token_response: AccessTokenResponse = response.json().await?;
                
                if let Some(token) = token_response.access_token {
                    return Ok(token);
                } else if let Some(error) = token_response.error {
                    match error.as_str() {
                        "authorization_pending" => {
                            // 继续等待
                            print!(".");
                            let _ = std::io::Write::flush(&mut std::io::stdout());
                        }
                        "slow_down" => {
                            // 减慢轮询速度
                            interval += 5;
                        }
                        "expired_token" => {
                            return Err(anyhow!("Device code expired. Please try again."));
                        }
                        "access_denied" => {
                            return Err(anyhow!("Access denied by user."));
                        }
                        _ => {
                            let desc = token_response.error_description.unwrap_or_default();
                            return Err(anyhow!("Authentication error: {} - {}", error, desc));
                        }
                    }
                }
            }
        }
    }
}

// 备用：手动创建token页面
pub fn open_github_token_page() -> Result<()> {
    let token_url = "https://github.com/settings/tokens/new?description=docker-sync-cli&scopes=repo,workflow,write:packages";
    
    println!("{}", "🔐 Opening GitHub Personal Access Token creation page...".blue());
    println!();
    println!("{}", "📋 Please follow these steps:".yellow());
    println!("1. Your browser will open to GitHub token creation page");
    println!("2. The description and scopes are pre-filled");
    println!("3. Click 'Generate token'");
    println!("4. Copy the generated token");
    println!("5. Run: docker-sync auth token YOUR_COPIED_TOKEN");
    println!();
    
    #[cfg(windows)]
    {
        let _ = webbrowser::open(token_url);
    }
    
    #[cfg(not(windows))]
    {
        println!("Please visit: {}", token_url.cyan());
    }
    
    Ok(())
}