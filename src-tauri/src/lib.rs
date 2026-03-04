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
    pub capture_mode: String,      // "camera" | "screen"
    pub camera_resolution: String,  // "480p" | "720p" | "1080p"
    // 新增字段
    pub account_username: String,   // 登录账号
    pub account_password: String,   // 登录密码
    pub device_code: String,       // 设备编码（MAC地址）
    pub device_name: String,       // 设备名称
    pub school_class_id: Option<i64>,  // 班级ID
    pub device_id: Option<i64>,     // 注册后返回的设备ID
    pub is_registered: bool,       // 是否已注册
    pub dept_id: Option<i64>,      // 学校/部门ID
    pub dept_name: String,         // 学校/部门名称
    pub access_token: Option<String>,  // 访问令牌
    pub refresh_token: Option<String>, // 刷新令牌
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
            api_url: "http://192.168.1.18:48080".to_string(),
            token: None,
            username: None,
            auto_start: false,
            retention_days: 7,
            capture_mode: "camera".to_string(),
            camera_resolution: "1080p".to_string(),
            // 新增默认值
            account_username: String::new(),
            account_password: String::new(),
            device_code: String::new(),
            device_name: String::new(),
            school_class_id: None,
            device_id: None,
            is_registered: false,
            dept_id: None,
            dept_name: String::new(),
            access_token: None,
            refresh_token: None,
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
fn detect_camera() -> Result<Vec<String>, String> {
    // 使用 nokhwa 库枚举可用摄像头
    // 如果没有摄像头，返回空列表
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        let output = Command::new("powershell")
            .args(["-Command", "Get-PnpDevice -Class Camera -Status OK | Select-Object -ExpandProperty FriendlyName"])
            .output();

        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let cameras: Vec<String> = stdout
                    .lines()
                    .filter(|s| !s.trim().is_empty())
                    .map(|s| s.trim().to_string())
                    .collect();
                if cameras.is_empty() {
                    Err("未检测到摄像头".to_string())
                } else {
                    Ok(cameras)
                }
            }
            Err(_) => Err("检测摄像头失败".to_string()),
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("暂不支持此平台".to_string())
    }
}

// 获取 MAC 地址作为设备编码
#[tauri::command]
fn get_mac_address() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        // 获取第一个 MAC 地址
        let output = Command::new("powershell")
            .args(["-Command", "Get-NetAdapter | Where-Object {$_.Status -eq 'Up'} | Select-Object -First 1 -ExpandProperty MacAddress"])
            .output();

        match output {
            Ok(out) => {
                let mac = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if mac.is_empty() {
                    // 如果获取不到，返回一个随机编码
                    Ok(format!("DEVICE_{}", uuid::Uuid::new_v4().to_string().replace("-", "")[..8].to_uppercase()))
                } else {
                    // 格式化 MAC 地址
                    let mac_clean = mac.replace(":", "-").replace("-", "");
                    Ok(format!("DEV_{}", mac_clean.to_uppercase()))
                }
            }
            Err(_) => Ok(format!("DEVICE_{}", uuid::Uuid::new_v4().to_string().replace("-", "")[..8].to_uppercase())),
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        Ok(format!("DEVICE_{}", uuid::Uuid::new_v4().to_string().replace("-", "")[..8].to_uppercase()))
    }
}

// 登录响应结构
#[derive(Debug, Serialize, Deserialize)]
struct LoginResponse {
    code: i32,
    msg: String,
    data: Option<LoginData>,
}

#[derive(Debug, Serialize, Deserialize)]
struct LoginData {
    user_id: i64,
    username: String,
    dept_id: i64,
    dept_name: String,
    access_token: String,
    refresh_token: String,
    expires_time: String,
}

// 自动登录
#[tauri::command]
async fn auto_login(state: State<'_, AppState>) -> Result<LoginData, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?.clone();

    if config.account_username.is_empty() || config.account_password.is_empty() {
        return Err("请先配置账号密码".to_string());
    }

    let client = reqwest::Client::new();
    let response = client
        .post(&format!("{}/client/inspection/login", config.api_url))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "username": config.account_username,
            "password": config.account_password
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status().is_success() {
        let result: LoginResponse = response.json().await.map_err(|e| e.to_string())?;

        if result.code == 0 {
            if let Some(data) = result.data {
                // 保存登录信息到配置
                let mut config = state.config.lock().map_err(|e| e.to_string())?;
                config.access_token = Some(data.access_token.clone());
                config.refresh_token = Some(data.refresh_token.clone());
                config.dept_id = Some(data.dept_id);
                config.dept_name = data.dept_name.clone();
                config.is_registered = false; // 重置注册状态，需要重新注册
                save_config(&config)?;

                Ok(data)
            } else {
                Err("登录响应数据为空".to_string())
            }
        } else {
            Err(result.msg)
        }
    } else {
        Err(format!("登录失败: {}", response.status()))
    }
}

// 班级信息
#[derive(Debug, Serialize, Deserialize)]
struct ClassInfo {
    id: i64,
    class_name: String,
}

// 班级列表响应
#[derive(Debug, Serialize, Deserialize)]
struct ClassListResponse {
    code: i32,
    msg: String,
    data: Option<Vec<ClassInfo>>,
}

// 获取班级列表
#[tauri::command]
async fn get_class_list(state: State<'_, AppState>) -> Result<Vec<ClassInfo>, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?.clone();

    let token = config.access_token.ok_or("未登录")?;

    let client = reqwest::Client::new();
    let response = client
        .get(&format!("{}/admin-api/hc/school-class/simple-list", config.api_url))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status().is_success() {
        let result: ClassListResponse = response.json().await.map_err(|e| e.to_string())?;

        if result.code == 0 {
            Ok(result.data.unwrap_or_default())
        } else {
            Err(result.msg)
        }
    } else {
        Err(format!("获取班级列表失败: {}", response.status()))
    }
}

// 设备注册响应
#[derive(Debug, Serialize, Deserialize)]
struct RegisterResponse {
    code: i32,
    msg: String,
    data: Option<RegisterData>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RegisterData {
    id: i64,
    device_name: String,
    device_code: String,
    device_type: i32,
    dept_id: i64,
    status: i32,
    register_type: i32,
}

// 注册设备
#[tauri::command]
async fn register_device(
    device_name: String,
    school_class_id: i64,
    state: State<'_, AppState>,
) -> Result<RegisterData, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?.clone();

    let token = config.access_token.ok_or("未登录")?;
    let device_code = if config.device_code.is_empty() {
        // 如果没有设备编码，先生成一个
        let mac = get_mac_address()?;
        let mut cfg = state.config.lock().map_err(|e| e.to_string())?;
        cfg.device_code = mac.clone();
        save_config(&cfg)?;
        mac
    } else {
        config.device_code
    };

    let dept_id = config.dept_id.ok_or("未获取到部门ID")?;

    let client = reqwest::Client::new();
    let response = client
        .post(&format!("{}/client/inspection/register", config.api_url))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&[
            ("deviceCode", device_code.as_str()),
            ("deviceName", device_name.as_str()),
            ("deptId", dept_id.to_string().as_str()),
        ])
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status().is_success() {
        let result: RegisterResponse = response.json().await.map_err(|e| e.to_string())?;

        if result.code == 0 {
            if let Some(data) = result.data {
                // 保存注册信息
                let mut config = state.config.lock().map_err(|e| e.to_string())?;
                config.device_name = data.device_name.clone();
                config.school_class_id = Some(school_class_id);
                config.device_id = Some(data.id);
                config.is_registered = true;
                save_config(&config)?;

                Ok(data)
            } else {
                Err("注册响应数据为空".to_string())
            }
        } else {
            Err(result.msg)
        }
    } else {
        Err(format!("注册失败: {}", response.status()))
    }
}

// 发送心跳
#[tauri::command]
async fn send_heartbeat(state: State<'_, AppState>) -> Result<bool, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?.clone();

    let token = config.access_token.ok_or("未登录")?;
    let device_code = config.device_code;

    if device_code.is_empty() {
        return Err("设备未注册".to_string());
    }

    let client = reqwest::Client::new();
    let response = client
        .post(&format!("{}/client/inspection/heartbeat", config.api_url))
        .header("Authorization", format!("Bearer {}", token))
        .form(&[("deviceCode", device_code.as_str())])
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status().is_success() {
        let result: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
        if result.get("code").and_then(|c| c.as_i64()).unwrap_or(-1) == 0 {
            Ok(true)
        } else {
            Err(result.get("msg").and_then(|m| m.as_str()).unwrap_or("心跳失败").to_string())
        }
    } else {
        Err(format!("心跳请求失败: {}", response.status()))
    }
}

// 上传截图到新 API
#[tauri::command]
async fn upload_screenshot_v2(
    image_data: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?.clone();

    let token = config.access_token.ok_or("未登录")?;
    let device_code = config.device_code;

    if device_code.is_empty() {
        return Err("设备未注册".to_string());
    }

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

    let form = multipart::Form::new()
        .part("file", part)
        .text("deviceCode", device_code);

    let client = reqwest::Client::new();
    let response = client
        .post(&format!("{}/client/inspection/screenshot/upload", config.api_url))
        .header("Authorization", format!("Bearer {}", token))
        .multipart(form)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status().is_success() {
        let result: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
        if result.get("code").and_then(|c| c.as_i64()).unwrap_or(-1) == 0 {
            Ok(true)
        } else {
            Err(result.get("msg").and_then(|m| m.as_str()).unwrap_or("上传失败").to_string())
        }
    } else {
        Err(format!("上传失败: {}", response.status()))
    }
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
            detect_camera,
            get_mac_address,
            auto_login,
            get_class_list,
            register_device,
            send_heartbeat,
            upload_screenshot_v2,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
