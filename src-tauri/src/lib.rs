use chrono::Local;
use reqwest::multipart;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

// 配置结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub interval: u32,
    pub mode: String,
    pub local_path: String,
    pub api_url: String,
    pub token: Option<String>,
    pub username: Option<String>,
    pub auto_start: bool,
    pub retention_days: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        let default_path = dirs::picture_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Screenshots")
            .to_string_lossy()
            .to_string();

        Self {
            interval: 10,
            mode: "local".to_string(),
            local_path: default_path,
            api_url: "http://localhost:3000".to_string(),
            token: None,
            username: None,
            auto_start: false,
            retention_days: 7,
        }
    }
}

pub struct AppState {
    pub config: Mutex<AppConfig>,
    pub is_running: Mutex<bool>,
}

fn get_config_path() -> PathBuf {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ScreenshotClient");

    if !config_dir.exists() {
        fs::create_dir_all(&config_dir).ok();
    }

    config_dir.join("config.json")
}

fn load_config() -> AppConfig {
    let path = get_config_path();
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(config) = serde_json::from_str(&content) {
                return config;
            }
        }
    }
    AppConfig::default()
}

fn save_config(config: &AppConfig) -> Result<(), String> {
    let path = get_config_path();
    let content = serde_json::to_string_pretty(config)
        .map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn capture_screen() -> Result<String, String> {
    use screenshots::image::ImageOutputFormat;

    let screens = screenshots::Screen::all().map_err(|e| e.to_string())?;

    if screens.is_empty() {
        return Err("没有找到显示器".to_string());
    }

    let screen = &screens[0];
    let capture = screen.capture().map_err(|e| e.to_string())?;

    let mut buffer = Cursor::new(Vec::new());
    capture
        .write_to(&mut buffer, ImageOutputFormat::Png)
        .map_err(|e| e.to_string())?;

    let base64_data = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        buffer.into_inner(),
    );

    Ok(format!("data:image/png;base64,{}", base64_data))
}

#[tauri::command]
fn save_screenshot_to_local(image_data: String, state: State<AppState>) -> Result<String, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;

    let save_dir = PathBuf::from(&config.local_path);
    if !save_dir.exists() {
        fs::create_dir_all(&save_dir).map_err(|e| e.to_string())?;
    }

    let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
    let filename = format!("screenshot_{}.png", timestamp);
    let file_path = save_dir.join(&filename);

    let base64_data = image_data
        .strip_prefix("data:image/png;base64,")
        .unwrap_or(&image_data);

    let image_bytes = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        base64_data,
    ).map_err(|e| e.to_string())?;

    fs::write(&file_path, image_bytes).map_err(|e| e.to_string())?;

    Ok(file_path.to_string_lossy().to_string())
}

#[tauri::command]
async fn upload_screenshot(
    image_data: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?.clone();

    let token = config.token.ok_or("未登录，请先登录")?;

    let base64_data = image_data
        .strip_prefix("data:image/png;base64,")
        .unwrap_or(&image_data);

    let image_bytes = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        base64_data,
    ).map_err(|e| e.to_string())?;

    let part = multipart::Part::bytes(image_bytes)
        .file_name("screenshot.png")
        .mime_str("image/png")
        .map_err(|e| e.to_string())?;

    let form = multipart::Form::new().part("file", part);

    let client = reqwest::Client::new();
    let response = client
        .post(&format!("{}/api/screenshot/upload", config.api_url))
        .header("Authorization", format!("Bearer {}", token))
        .multipart(form)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status().is_success() {
        let result: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
        Ok(result.to_string())
    } else {
        Err(format!("上传失败: {}", response.status()))
    }
}

#[tauri::command]
async fn login(
    username: String,
    password: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?.clone();

    let client = reqwest::Client::new();
    let response = client
        .post(&format!("{}/api/login", config.api_url))
        .json(&serde_json::json!({
            "username": username,
            "password": password
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status().is_success() {
        let result: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;

        if let Some(token) = result.get("token").and_then(|t| t.as_str()) {
            let mut config = state.config.lock().map_err(|e| e.to_string())?;
            config.token = Some(token.to_string());
            config.username = Some(username);
            config.mode = "cloud".to_string();
            save_config(&config)?;

            Ok(token.to_string())
        } else {
            Err("登录响应格式错误".to_string())
        }
    } else {
        Err(format!("登录失败: {}", response.status()))
    }
}

#[tauri::command]
fn logout(state: State<AppState>) -> Result<(), String> {
    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    config.token = None;
    config.username = None;
    config.mode = "local".to_string();
    save_config(&config)?;
    Ok(())
}

#[tauri::command]
fn get_config(state: State<AppState>) -> Result<AppConfig, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    Ok(config.clone())
}

#[tauri::command]
fn update_config(new_config: AppConfig, state: State<AppState>) -> Result<(), String> {
    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    *config = new_config;
    save_config(&config)?;
    Ok(())
}

#[tauri::command]
fn get_running_state(state: State<AppState>) -> Result<bool, String> {
    let is_running = state.is_running.lock().map_err(|e| e.to_string())?;
    Ok(*is_running)
}

#[tauri::command]
fn set_running_state(running: bool, state: State<AppState>) -> Result<(), String> {
    let mut is_running = state.is_running.lock().map_err(|e| e.to_string())?;
    *is_running = running;
    Ok(())
}

#[tauri::command]
fn cleanup_old_files(state: State<AppState>) -> Result<u32, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?.clone();

    if config.mode != "local" {
        return Ok(0);
    }

    let save_dir = PathBuf::from(&config.local_path);
    if !save_dir.exists() {
        return Ok(0);
    }

    let retention_days = chrono::Duration::days(config.retention_days as i64);
    let cutoff = Local::now() - retention_days;

    let mut deleted_count = 0;

    if let Ok(entries) = fs::read_dir(&save_dir) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    if let Some(filename) = entry.path().file_name() {
                        let filename = filename.to_string_lossy();
                        if filename.starts_with("screenshot_") && filename.ends_with(".png") {
                            if let Ok(modified) = metadata.modified() {
                                let modified_time: chrono::DateTime<Local> = modified.into();
                                if modified_time < cutoff {
                                    if fs::remove_file(entry.path()).is_ok() {
                                        deleted_count += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(deleted_count)
}

#[tauri::command]
async fn check_network(api_url: String) -> Result<bool, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;

    match client.get(&format!("{}/api/health", api_url)).send().await {
        Ok(response) => Ok(response.status().is_success()),
        Err(_) => Ok(false),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();
    let config = load_config();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_http::init())
        .manage(AppState {
            config: Mutex::new(config),
            is_running: Mutex::new(false),
        })
        .invoke_handler(tauri::generate_handler![
            capture_screen,
            save_screenshot_to_local,
            upload_screenshot,
            login,
            logout,
            get_config,
            update_config,
            get_running_state,
            set_running_state,
            cleanup_old_files,
            check_network,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
