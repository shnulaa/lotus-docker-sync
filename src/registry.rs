use anyhow::Result;
use reqwest::Client;

#[allow(dead_code)]
pub struct RegistryClient {
    client: Client,
}

#[allow(dead_code)]
impl RegistryClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
    
    pub async fn image_exists(&self, image: &str) -> Result<bool> {
        // Parse image name and tag
        let (registry_image, tag) = if let Some(pos) = image.rfind(':') {
            let (img, tag) = image.split_at(pos);
            (img, &tag[1..]) // Remove the ':'
        } else {
            (image, "latest")
        };
        
        // Extract registry and image name
        let parts: Vec<&str> = registry_image.split('/').collect();
        if parts.len() < 3 {
            return Ok(false);
        }
        
        let registry = parts[0];
        let namespace = parts[1];
        let image_name = parts[2..].join("/");
        
        // Construct manifest URL
        let manifest_url = format!(
            "https://{}/v2/{}/{}/manifests/{}",
            registry, namespace, image_name, tag
        );
        
        // Make HEAD request to check if manifest exists
        let response = self
            .client
            .head(&manifest_url)
            .header("Accept", "application/vnd.docker.distribution.manifest.v2+json")
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await;
        
        match response {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false), // 网络错误时假设镜像不存在
        }
    }
}