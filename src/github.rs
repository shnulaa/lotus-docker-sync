use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use base64::Engine;
use colored::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkflowRun {
    pub id: u64,
    pub status: String,
    pub conclusion: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkflowRunsResponse {
    pub workflow_runs: Vec<WorkflowRun>,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct Repository {
    pub id: u64,
    pub name: String,
    pub full_name: String,
    pub html_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub login: String,
    pub id: u64,
}

pub struct GitHubClient {
    client: Client,
    token: String,
    username: Option<String>,
}

impl GitHubClient {
    pub fn new(token: &str) -> Self {
        Self {
            client: Client::new(),
            token: token.to_string(),
            username: None,
        }
    }
    
    pub async fn get_username(&mut self) -> Result<String> {
        if let Some(ref username) = self.username {
            return Ok(username.clone());
        }
        
        let response = self
            .client
            .get("https://api.github.com/user")
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "docker-sync-cli")
            .send()
            .await?;
            
        if !response.status().is_success() {
            return Err(anyhow!("Failed to get user info: {}", response.status()));
        }
        
        let user: User = response.json().await?;
        self.username = Some(user.login.clone());
        Ok(user.login)
    }
    
    pub async fn ensure_sync_repo(&mut self) -> Result<String> {
        let username = self.get_username().await?;
        let repo_name = format!("{}/docker-sync", username);
        
        // Check if repository exists
        if self.repo_exists(&repo_name).await? {
            // 检查并更新workflow文件
            self.ensure_workflow(&repo_name).await?;
            return Ok(repo_name);
        }
        
        println!("{}", "🔧 首次使用：正在创建同步仓库（可能需要一些时间）...".blue());
        
        // Create repository
        self.create_repo("docker-sync", &username).await?;
        
        // Upload workflow file
        self.upload_workflow(&repo_name).await?;
        
        println!("{} 初始化完成！仓库地址: {}", "✅".green(), format!("https://github.com/{}", repo_name).cyan());
        Ok(repo_name)
    }
    
    async fn ensure_workflow(&self, repo_name: &str) -> Result<()> {
        // 检查workflow文件是否存在并更新
        let url = format!(
            "https://api.github.com/repos/{}/contents/.github/workflows/docker-sync.yml",
            repo_name
        );
        
        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "docker-sync-cli")
            .send()
            .await?;
        
        if response.status().is_success() {
            // 获取现有文件的SHA用于更新
            let file_info: serde_json::Value = response.json().await?;
            if let Some(sha) = file_info["sha"].as_str() {
                println!("{} 正在更新工作流文件...", "📋".yellow());
                self.update_workflow(repo_name, sha).await?;
            }
        } else {
            // Workflow不存在，创建它
            println!("{} 工作流不存在，正在创建...", "📋".yellow());
            self.upload_workflow(repo_name).await?;
        }
        
        Ok(())
    }
    
    async fn update_workflow(&self, repo_name: &str, sha: &str) -> Result<()> {
        let workflow_content = include_str!("../workflow-template.yml");
        let encoded_content = base64::engine::general_purpose::STANDARD.encode(workflow_content);
        
        let payload = json!({
            "message": "Update docker sync workflow",
            "content": encoded_content,
            "sha": sha,
            "branch": "main"
        });
        
        let url = format!(
            "https://api.github.com/repos/{}/contents/.github/workflows/docker-sync.yml",
            repo_name
        );
        
        let response = self
            .client
            .put(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "docker-sync-cli")
            .json(&payload)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Failed to update workflow: {}", error_text));
        }
        
        println!("{} 工作流已更新", "⚙️".green());
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        Ok(())
    }
    
    async fn repo_exists(&self, repo_name: &str) -> Result<bool> {
        let url = format!("https://api.github.com/repos/{}", repo_name);
        
        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "docker-sync-cli")
            .send()
            .await?;
            
        Ok(response.status().is_success())
    }
    
    async fn create_repo(&self, name: &str, username: &str) -> Result<()> {
        let payload = json!({
            "name": name,
            "description": "Docker image sync repository - automatically sync Docker Hub images to GHCR",
            "private": false,
            "auto_init": true,
            "has_issues": false,
            "has_projects": false,
            "has_wiki": false
        });
        
        let response = self
            .client
            .post("https://api.github.com/user/repos")
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "docker-sync-cli")
            .json(&payload)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Failed to create repository: {}", error_text));
        }
        
        println!("{} 仓库已创建: {}", "📁".blue(), format!("https://github.com/{}/{}", username, name).cyan());
        
        Ok(())
    }
    
    async fn set_actions_permissions(&self, owner: &str, repo: &str) -> Result<()> {
        // 首先启用Actions
        let enable_url = format!(
            "https://api.github.com/repos/{}/{}/actions/permissions",
            owner, repo
        );
        
        let enable_payload = json!({
            "enabled": true,
            "allowed_actions": "all"
        });
        
        let _ = self
            .client
            .put(&enable_url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "docker-sync-cli")
            .json(&enable_payload)
            .send()
            .await;
        
        // 然后设置workflow权限
        let url = format!(
            "https://api.github.com/repos/{}/{}/actions/permissions/workflow",
            owner, repo
        );
        
        let payload = json!({
            "default_workflow_permissions": "write",
            "can_approve_pull_request_reviews": true
        });
        
        let response = self
            .client
            .put(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "docker-sync-cli")
            .json(&payload)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            println!("{} 警告: 无法设置 Actions 权限: {}", "⚠️".yellow(), error_text);
            println!("{} 请手动在以下地址启用 'Read and write permissions':", "📋".yellow());
            println!("   https://github.com/{}/{}/settings/actions", owner, repo);
        } else {
            println!("{} Actions 权限已配置", "🔐".green());
        }
        
        Ok(())
    }
    
    async fn upload_workflow(&self, repo_name: &str) -> Result<()> {
        println!("{} 正在配置 GitHub Action 工作流...", "📋".yellow());
        
        let workflow_content = include_str!("../workflow-template.yml");
        let encoded_content = base64::engine::general_purpose::STANDARD.encode(workflow_content);
        
        let payload = json!({
            "message": "Add docker sync workflow",
            "content": encoded_content,
            "branch": "main"
        });
        
        let url = format!(
            "https://api.github.com/repos/{}/contents/.github/workflows/docker-sync.yml",
            repo_name
        );
        
        let response = self
            .client
            .put(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "docker-sync-cli")
            .json(&payload)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Failed to upload workflow: {}", error_text));
        }
        
        println!("{} GitHub Action 工作流已配置", "⚙️".green());
        
        // 设置Actions权限为读写
        let parts: Vec<&str> = repo_name.split('/').collect();
        if parts.len() == 2 {
            self.set_actions_permissions(parts[0], parts[1]).await?;
        }
        
        // 等待GitHub处理workflow文件
        println!("{} 等待 GitHub 处理工作流...", "⏳".blue());
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        
        Ok(())
    }
    
    pub async fn trigger_sync(&mut self, image: &str) -> Result<u64> {
        let repo_name = self.ensure_sync_repo().await?;
        
        let url = format!(
            "https://api.github.com/repos/{}/actions/workflows/docker-sync.yml/dispatches",
            repo_name
        );
        
        let payload = json!({
            "ref": "main",
            "inputs": {
                "docker_images": image
            }
        });
        
        // 重试逻辑，等待 workflow 被 GitHub 识别
        let mut retries = 5;
        loop {
            let response = self
                .client
                .post(&url)
                .header("Authorization", format!("Bearer {}", self.token))
                .header("Accept", "application/vnd.github.v3+json")
                .header("User-Agent", "docker-sync-cli")
                .json(&payload)
                .send()
                .await?;
                
            if response.status().is_success() || response.status().as_u16() == 204 {
                break;
            }
            
            let status = response.status();
            let error_text = response.text().await?;
            
            if status.as_u16() == 404 && retries > 0 {
                retries -= 1;
                println!("{} 等待工作流就绪... (10秒后重试)", "⏳".yellow());
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                continue;
            }
            
            return Err(anyhow!("Failed to trigger workflow: {}", error_text));
        }
        
        // GitHub doesn't return the run ID directly, so we need to fetch recent runs
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        self.get_latest_run_id(&repo_name).await
    }
    
    async fn get_latest_run_id(&self, repo_name: &str) -> Result<u64> {
        let url = format!(
            "https://api.github.com/repos/{}/actions/workflows/docker-sync.yml/runs?per_page=1",
            repo_name
        );
        
        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "docker-sync-cli")
            .send()
            .await?;
            
        if !response.status().is_success() {
            return Err(anyhow!("Failed to get workflow runs"));
        }
        
        let runs: WorkflowRunsResponse = response.json().await?;
        
        runs.workflow_runs
            .first()
            .map(|run| run.id)
            .ok_or_else(|| anyhow!("No workflow runs found"))
    }
    
    pub async fn get_run_status(&self, run_id: u64, repo_name: &str) -> Result<String> {
        let url = format!(
            "https://api.github.com/repos/{}/actions/runs/{}",
            repo_name, run_id
        );
        
        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "docker-sync-cli")
            .send()
            .await?;
            
        if !response.status().is_success() {
            return Err(anyhow!("Failed to get run status"));
        }
        
        let run: WorkflowRun = response.json().await?;
        
        if run.status == "completed" {
            match run.conclusion.as_deref() {
                Some("success") => Ok("completed".to_string()),
                Some("failure") => Ok("failure".to_string()),
                Some("cancelled") => Ok("cancelled".to_string()),
                _ => Ok("completed".to_string()),
            }
        } else {
            Ok(run.status)
        }
    }
    
    pub async fn get_job_steps(&self, run_id: u64, repo_name: &str) -> Result<Vec<serde_json::Value>> {
        let jobs_url = format!(
            "https://api.github.com/repos/{}/actions/runs/{}/jobs",
            repo_name, run_id
        );
        
        let response = self
            .client
            .get(&jobs_url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "docker-sync-cli")
            .send()
            .await?;
            
        if !response.status().is_success() {
            return Ok(vec![]);
        }
        
        let jobs: serde_json::Value = response.json().await?;
        
        if let Some(jobs_array) = jobs["jobs"].as_array() {
            if let Some(job) = jobs_array.first() {
                if let Some(steps) = job["steps"].as_array() {
                    return Ok(steps.clone());
                }
            }
        }
        
        Ok(vec![])
    }
    
    pub async fn get_run_logs(&self, run_id: u64, repo_name: &str) -> Result<String> {
        // First get the jobs for this run
        let jobs_url = format!(
            "https://api.github.com/repos/{}/actions/runs/{}/jobs",
            repo_name, run_id
        );
        
        let response = self
            .client
            .get(&jobs_url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "docker-sync-cli")
            .send()
            .await?;
            
        if !response.status().is_success() {
            return Ok(String::new());
        }
        
        let jobs: serde_json::Value = response.json().await?;
        let mut all_logs = String::new();
        
        if let Some(jobs_array) = jobs["jobs"].as_array() {
            for job in jobs_array {
                if let Some(job_id) = job["id"].as_u64() {
                    if let Ok(job_logs) = self.get_job_logs(job_id, repo_name).await {
                        all_logs.push_str(&job_logs);
                        all_logs.push('\n');
                    }
                }
            }
        }
        
        Ok(all_logs)
    }
    
    async fn get_job_logs(&self, job_id: u64, repo_name: &str) -> Result<String> {
        let url = format!(
            "https://api.github.com/repos/{}/actions/jobs/{}/logs",
            repo_name, job_id
        );
        
        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "docker-sync-cli")
            .send()
            .await?;
            
        if response.status().is_success() {
            Ok(response.text().await?)
        } else {
            Ok(String::new())
        }
    }
    
    pub async fn delete_package(&self, package_name: &str) -> Result<()> {
        let username = self.username.as_ref().ok_or_else(|| anyhow!("Username not set"))?;
        
        // 直接删除整个 package
        let delete_url = format!(
            "https://api.github.com/users/{}/packages/container/{}",
            username, package_name
        );
        
        println!("{} 正在删除 {}...", "🗑️".yellow(), package_name);
        
        let response = self
            .client
            .delete(&delete_url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "docker-sync-cli")
            .send()
            .await?;
        
        if response.status().is_success() || response.status().as_u16() == 204 {
            println!("{} 已删除 '{}'", "✓".green(), package_name);
        } else if response.status().as_u16() == 404 {
            println!("{} '{}' 不存在", "⚠️".yellow(), package_name);
        } else {
            let err = response.text().await.unwrap_or_default();
            println!("{} 删除失败: {}", "✗".red(), err);
        }
        
        Ok(())
    }
    
    pub async fn delete_package_version(&self, package_name: &str, tag: &str) -> Result<()> {
        let username = self.username.as_ref().ok_or_else(|| anyhow!("Username not set"))?;
        
        // 获取所有版本，找到匹配 tag 的版本
        let versions_url = format!(
            "https://api.github.com/users/{}/packages/container/{}/versions",
            username, package_name
        );
        
        let response = self
            .client
            .get(&versions_url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "docker-sync-cli")
            .send()
            .await?;
        
        if !response.status().is_success() {
            if response.status().as_u16() == 404 {
                return Ok(()); // package 不存在
            }
            return Ok(());
        }
        
        let versions: Vec<serde_json::Value> = response.json().await?;
        
        // 找到匹配 tag 的版本
        for version in &versions {
            if let Some(tags) = version["metadata"]["container"]["tags"].as_array() {
                let has_tag = tags.iter().any(|t| t.as_str() == Some(tag));
                if has_tag {
                    if let Some(version_id) = version["id"].as_u64() {
                        // 如果只有一个版本，删除整个 package
                        if versions.len() == 1 {
                            return self.delete_package(package_name).await;
                        }
                        
                        // 否则只删除这个版本
                        let delete_url = format!(
                            "https://api.github.com/users/{}/packages/container/{}/versions/{}",
                            username, package_name, version_id
                        );
                        
                        println!("{} 正在删除 {}:{}...", "🗑️".yellow(), package_name, tag);
                        
                        let del_response = self
                            .client
                            .delete(&delete_url)
                            .header("Authorization", format!("Bearer {}", self.token))
                            .header("Accept", "application/vnd.github.v3+json")
                            .header("User-Agent", "docker-sync-cli")
                            .send()
                            .await?;
                        
                        if del_response.status().is_success() || del_response.status().as_u16() == 204 {
                            println!("{} 已删除 {}:{}", "✓".green(), package_name, tag);
                        } else {
                            let err = del_response.text().await.unwrap_or_default();
                            println!("{} 删除失败: {}", "✗".red(), err);
                        }
                        return Ok(());
                    }
                }
            }
        }
        
        Ok(())
    }
    
    pub async fn package_version_exists(&self, package_name: &str, tag: &str) -> Result<bool> {
        let username = self.username.as_ref().ok_or_else(|| anyhow!("Username not set"))?;
        
        let versions_url = format!(
            "https://api.github.com/users/{}/packages/container/{}/versions",
            username, package_name
        );
        
        let response = self
            .client
            .get(&versions_url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "docker-sync-cli")
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Ok(false);
        }
        
        let versions: Vec<serde_json::Value> = response.json().await?;
        
        for version in &versions {
            if let Some(tags) = version["metadata"]["container"]["tags"].as_array() {
                if tags.iter().any(|t| t.as_str() == Some(tag)) {
                    return Ok(true);
                }
            }
        }
        
        Ok(false)
    }
    
    #[allow(dead_code)]
    pub async fn package_exists(&self, package_name: &str) -> Result<bool> {
        let username = self.username.as_ref().ok_or_else(|| anyhow!("Username not set"))?;
        
        let url = format!(
            "https://api.github.com/users/{}/packages/container/{}",
            username, package_name
        );
        
        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "docker-sync-cli")
            .send()
            .await?;
        
        Ok(response.status().is_success())
    }
}