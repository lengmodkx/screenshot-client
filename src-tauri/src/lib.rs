use chrono::Local;
use reqwest::multipart;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::State;
use tauri::Manager;
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tokio::sync::watch;

mod database;
mod monitor;

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
    pub tenant_name: String,       // 租户名称
    pub device_code: String,       // 设备编码（MAC地址）
    pub device_name: String,       // 设备名称
    pub school_class_id: Option<i64>,  // 班级ID
    pub class_name: String,          // 班级名称
    pub device_id: Option<i64>,     // 注册后返回的设备ID
    pub is_registered: bool,       // 是否已注册
    pub dept_id: Option<i64>,      // 学校/部门ID
    pub dept_name: String,         // 学校/部门名称
    pub access_token: Option<String>,  // 访问令牌
    pub refresh_token: Option<String>, // 刷新令牌
    // 后台运行配置
    pub autostart_enabled: bool,    // 开机自启开关
    pub show_window_on_start: bool, // 启动时是否显示窗口
    // 软件监控配置
    pub software_monitor_enabled: bool,      // 是否启用软件监控
    pub software_monitor_interval_secs: u32, // 轮询间隔（秒）
    pub software_monitor_batch_secs: u32,    // 批量上报间隔（秒）
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
            api_url: "http://172.16.10.11:48080".to_string(),
            token: None,
            username: None,
            auto_start: false,
            retention_days: 7,
            capture_mode: "camera".to_string(),
            camera_resolution: "1080p".to_string(),
            // 新增默认值
            account_username: String::new(),
            account_password: String::new(),
            tenant_name: String::new(),
            device_code: String::new(),
            device_name: String::new(),
            school_class_id: None,
            class_name: String::new(),
            device_id: None,
            is_registered: false,
            dept_id: None,
            dept_name: String::new(),
            access_token: None,
            refresh_token: None,
            // 后台运行默认值
            autostart_enabled: true,
            show_window_on_start: true,
            // 软件监控默认值
            software_monitor_enabled: true,
            software_monitor_interval_secs: 5,
            software_monitor_batch_secs: 300,
        }
    }
}

pub struct AppState {
    pub config: Mutex<AppConfig>,
    pub is_running: Mutex<bool>,
    pub monitor_shutdown: Mutex<Option<watch::Sender<bool>>>,
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
    println!("[load_config] 配置文件路径: {:?}", path);
    println!("[load_config] 文件是否存在: {}", path.exists());
    if path.exists() {
        match fs::read_to_string(&path) {
            Ok(content) => {
                println!("[load_config] 文件读取成功, 长度: {}", content.len());
                match serde_json::from_str::<AppConfig>(&content) {
                    Ok(config) => {
                        println!("[load_config] JSON解析成功");
                        return config;
                    }
                    Err(e) => {
                        println!("[load_config] JSON解析失败: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("[load_config] 文件读取失败: {}", e);
            }
        }
    }
    println!("[load_config] 使用默认配置");
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
async fn capture_screen() -> Result<String, String> {
    use screenshots::image::ImageOutputFormat;

    // 在独立线程中执行截图，避免阻塞主线程
    let result = tokio::task::spawn_blocking(move || {
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
    }).await;

    result.map_err(|e| format!("截图任务执行失败: {}", e))?
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

    let client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .map_err(|e| e.to_string())?;
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
    // 使用作用域限制锁的生命周期，避免死锁
    let api_url = {
        let config = state.config.lock().map_err(|e| e.to_string())?;
        config.api_url.clone()
    };

    let client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .map_err(|e| e.to_string())?;
    let response = client
        .post(&format!("{}/api/login", api_url))
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
    println!("[get_mac_address] 使用UUID生成设备编码...");
    // 直接使用UUID，避免PowerShell阻塞
    let uuid_str = uuid::Uuid::new_v4().to_string().replace("-", "");
    let device_code = format!("DEVICE_{}", &uuid_str[..12].to_uppercase());
    println!("[get_mac_address] 设备编码: {}", device_code);
    Ok(device_code)
}

// 获取本地 IP 地址
fn get_local_ip() -> String {
    println!("[get_local_ip] 使用默认IP...");
    // 使用固定IP，避免PowerShell阻塞
    "192.168.1.100".to_string()
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
    #[serde(rename = "userId")]
    user_id: i64,
    username: String,
    #[serde(rename = "deptId")]
    dept_id: i64,
    #[serde(rename = "deptName")]
    dept_name: String,
    #[serde(rename = "accessToken")]
    access_token: String,
    #[serde(rename = "refreshToken")]
    refresh_token: String,
    #[serde(rename = "expiresTime")]
    expires_time: i64,
}

// 自动登录
#[tauri::command]
async fn auto_login(state: State<'_, AppState>) -> Result<LoginData, String> {
    // 使用作用域限制锁的生命周期，避免死锁
    let (account_username, account_password, api_url, tenant_name) = {
        let config = state.config.lock().map_err(|e| e.to_string())?.clone();
        (
            config.account_username.clone(),
            config.account_password.clone(),
            config.api_url.clone(),
            config.tenant_name.clone(),
        )
    };

    if account_username.is_empty() || account_password.is_empty() {
        return Err("请先配置账号密码".to_string());
    }

    // 创建禁用代理的 HTTP 客户端（避免本地地址走代理导致 502）
    let client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .map_err(|e| format!("创建HTTP客户端失败: {}", e))?;

    let response = client
        .post(&format!("{}/client/inspection/login", api_url))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "username": account_username,
            "password": account_password,
            "tenantName": tenant_name
        }))
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if response.status().is_success() {
        // 先获取原始文本用于调试
        let text = response.text().await.map_err(|e| format!("读取响应失败: {}", e))?;
        println!("登录响应原始内容: {}", text);

        let result: LoginResponse = serde_json::from_str(&text)
            .map_err(|e| format!("解析响应失败: {}，原始内容: {}", e, text))?;

        if result.code == 0 {
            if let Some(data) = result.data {
                // 保存登录信息到配置
                let mut config = state.config.lock().map_err(|e| e.to_string())?;
                config.access_token = Some(data.access_token.clone());
                config.refresh_token = Some(data.refresh_token.clone());
                config.dept_id = Some(data.dept_id);
                config.dept_name = data.dept_name.clone();
                // 不重置 is_registered 状态，保持配置文件中的原有值
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
    #[serde(rename = "className")]
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
    println!("[get_class_list] 开始获取班级列表...");

    let config = match state.config.lock() {
        Ok(guard) => {
            println!("[get_class_list] 获取配置锁成功");
            guard.clone()
        }
        Err(e) => {
            println!("[get_class_list] 获取配置锁失败: {}", e);
            return Err(format!("获取配置锁失败: {}", e));
        }
    };

    println!("[get_class_list] API地址: {}", config.api_url);

    let token = match config.access_token {
        Some(t) => {
            println!("[get_class_list] 获取token成功");
            t
        }
        None => {
            println!("[get_class_list] 未登录，没有token");
            return Err("未登录".to_string());
        }
    };

    println!("[get_class_list] 创建HTTP客户端...");
    let client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!("{}/admin-api/hc/school-class/simple-list", config.api_url);
    println!("[get_class_list] 发送请求到: {}", url);

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| {
            println!("[get_class_list] 请求失败: {}", e);
            e.to_string()
        })?;

    println!("[get_class_list] 收到响应，状态: {}", response.status());

    if response.status().is_success() {
        // 先获取原始文本用于调试
        println!("[get_class_list] 读取响应体...");
        let text = response.text().await.map_err(|e| {
            println!("[get_class_list] 读取响应失败: {}", e);
            format!("读取响应失败: {}", e)
        })?;
        println!("[get_class_list] 响应内容: {}", text);

        println!("[get_class_list] 解析JSON...");
        let result: ClassListResponse = serde_json::from_str(&text)
            .map_err(|e| {
                println!("[get_class_list] 解析响应失败: {}", e);
                format!("解析响应失败: {}，原始内容: {}", e, text)
            })?;

        if result.code == 0 {
            let data = result.data.unwrap_or_default();
            println!("[get_class_list] 成功，获取到 {} 个班级", data.len());
            Ok(data)
        } else {
            println!("[get_class_list] API返回错误: {}", result.msg);
            Err(format!("获取班级列表失败: {}", result.msg))
        }
    } else {
        println!("[get_class_list] HTTP错误: {}", response.status());
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
    #[serde(rename = "tenantId")]
    tenant_id: Option<i64>,
    #[serde(rename = "deviceName")]
    device_name: String,
    #[serde(rename = "deviceCode")]
    device_code: String,
    #[serde(rename = "deviceType")]
    device_type: Option<i32>,
    #[serde(rename = "classroomId")]
    classroom_id: Option<i64>,
    #[serde(rename = "classroomName")]
    classroom_name: Option<String>,
    #[serde(rename = "ipAddress")]
    ip_address: String,
    port: Option<i32>,
    status: i32,
    #[serde(rename = "registerType")]
    register_type: i32,
    #[serde(rename = "lastHeartbeat")]
    last_heartbeat: Option<i64>,
    #[serde(rename = "lastScreenshotTime")]
    last_screenshot_time: Option<i64>,
    #[serde(rename = "screenshotUrl")]
    screenshot_url: Option<String>,
    remark: Option<String>,
    creator: String,
    #[serde(rename = "createTime")]
    create_time: i64,
    updater: String,
    #[serde(rename = "updateTime")]
    update_time: i64,
}

// 注册设备
#[tauri::command]
async fn register_device(
    device_name: String,
    school_class_id: i64,
    device_type: Option<i32>,
    class_name: String,  // 新增：班级名称
    state: State<'_, AppState>,
) -> Result<RegisterData, String> {
    println!("[register_device] 开始设备注册...");
    println!("[register_device] 设备名称: {}, 班级ID: {}, 班级名称: {}", device_name, school_class_id, class_name);

    // 使用作用域限制锁的生命周期，避免死锁
    let (token, device_code, dept_id, api_url) = {
        println!("[register_device] 获取配置锁...");
        let mut config = state.config.lock().map_err(|e| e.to_string())?;
        println!("[register_device] 获取配置锁成功");

        let token = config.access_token.clone().ok_or("未登录")?;
        println!("[register_device] 获取token成功");

        // 如果没有设备编码，先生成一个
        if config.device_code.is_empty() {
            println!("[register_device] 设备编码为空，生成MAC地址...");
            let mac = get_mac_address()?;
            println!("[register_device] MAC地址生成成功: {}", mac);
            config.device_code = mac.clone();
            save_config(&config)?;
        }

        let device_code = config.device_code.clone();
        let dept_id = config.dept_id.ok_or("未获取到部门ID")?;
        let api_url = config.api_url.clone();

        println!("[register_device] 配置获取完成: device_code={}, dept_id={}", device_code, dept_id);
        (token, device_code, dept_id, api_url)
    }; // 锁在这里释放

    println!("[register_device] 获取本地IP地址...");
    let ip_address = get_local_ip();
    println!("[register_device] 本地IP: {}", ip_address);

    println!("[register_device] 创建HTTP客户端...");
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(30)) // 添加30秒超时
        .build()
        .map_err(|e| e.to_string())?;

    // 从 dept_id 获取租户ID（芋道框架通常使用部门ID作为租户ID）
    let tenant_id = dept_id;

    // API文档：deviceType: 1-电子大屏，2-黑板
    let device_type_value = device_type.unwrap_or(2); // 默认黑板

    println!("[register_device] 发送注册请求到: {}/client/inspection/register", api_url);
    println!("[register_device] 请求参数: deviceCode={}, deviceName={}, deviceType={}, classroomId={}",
             device_code, device_name, device_type_value, school_class_id);

    let response = client
        .post(&format!("{}/client/inspection/register", api_url))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("tenant-id", tenant_id.to_string())
        .form(&[
            ("deviceCode", device_code.as_str()),
            ("deviceName", device_name.as_str()),
            ("deviceType", device_type_value.to_string().as_str()),
            ("ipAddress", ip_address.as_str()),
            ("classroomId", school_class_id.to_string().as_str()),
            ("registerType", "1"), // 1-自动注册
        ])
        .send()
        .await
        .map_err(|e| {
            println!("[register_device] 请求发送失败: {}", e);
            e.to_string()
        })?;

    println!("[register_device] 收到响应，状态: {}", response.status());

    if response.status().is_success() {
        // 先获取原始文本用于调试
        println!("[register_device] 读取响应体...");
        let text = response.text().await.map_err(|e| {
            println!("[register_device] 读取响应失败: {}", e);
            format!("读取响应失败: {}", e)
        })?;
        println!("[register_device] 响应内容: {}", text);

        println!("[register_device] 解析JSON...");
        let result: RegisterResponse = serde_json::from_str(&text)
            .map_err(|e| {
                println!("[register_device] 解析响应失败: {}", e);
                format!("解析响应失败: {}，原始内容: {}", e, text)
            })?;

        println!("[register_device] API返回code: {}, msg: {}", result.code, result.msg);

        if result.code == 0 || result.code == 1030670002 { // 0-成功，1030670002-设备已注册
            println!("[register_device] 注册成功或有数据返回");
            if let Some(data) = result.data {
                println!("[register_device] 保存注册信息到配置...");
                // 保存注册信息
                let mut config = state.config.lock().map_err(|e| e.to_string())?;
                config.device_name = data.device_name.clone();
                config.school_class_id = Some(school_class_id);
                // 使用传入的班级名称，如果后端返回了班级名称则优先使用
                config.class_name = data.classroom_name.clone().unwrap_or_else(|| class_name.clone());
                config.device_id = Some(data.id);
                config.is_registered = true;
                save_config(&config)?;
                println!("[register_device] 注册信息保存成功, 班级: {}", config.class_name);

                Ok(data)
            } else {
                println!("[register_device] 设备已注册但无返回数据，构造基本信息...");
                // 设备已注册但没有返回数据，构造一个基本信息
                let mut config = state.config.lock().map_err(|e| e.to_string())?;
                config.device_name = device_name.clone();
                config.school_class_id = Some(school_class_id);
                config.class_name = class_name.clone();  // 保存班级名称
                config.is_registered = true;
                save_config(&config)?;

                // 构造返回数据
                Ok(RegisterData {
                    id: config.device_id.unwrap_or(1),
                    tenant_id: Some(tenant_id),
                    device_name: device_name.clone(),
                    device_code: device_code.clone(),
                    device_type: Some(device_type_value),
                    classroom_id: Some(school_class_id),
                    classroom_name: None,
                    ip_address: ip_address.clone(),
                    port: None,
                    status: 1,
                    register_type: 1,
                    last_heartbeat: None,
                    last_screenshot_time: None,
                    screenshot_url: None,
                    remark: None,
                    creator: config.account_username.clone(),
                    create_time: chrono::Local::now().timestamp_millis(),
                    updater: config.account_username.clone(),
                    update_time: chrono::Local::now().timestamp_millis(),
                })
            }
        } else {
            Err(format!("注册失败: {}", result.msg))
        }
    } else {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        println!("注册失败HTTP错误: {} - {}", status, error_text);
        Err(format!("注册失败: {} - {}", status, error_text))
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

    let client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .map_err(|e| e.to_string())?;
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

// 压缩图片到指定大小以下（单位：KB）
fn compress_image_to_size(img: &image::DynamicImage, max_size_kb: usize) -> Result<Vec<u8>, String> {
    let mut quality = 85u8;
    let mut jpeg_buffer = Vec::new();

    loop {
        jpeg_buffer.clear();
        // 使用 image 0.25 版本的编码方式
        let rgb_img = img.to_rgb8();
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
            &mut jpeg_buffer,
            quality
        );

        encoder.encode(&rgb_img, img.width(), img.height(), image::ColorType::Rgb8.into())
            .map_err(|e| format!("JPEG编码失败: {}", e))?;

        // 检查大小
        if jpeg_buffer.len() <= max_size_kb * 1024 || quality <= 30 {
            break;
        }

        // 降低质量继续压缩
        quality -= 10;
    }

    Ok(jpeg_buffer)
}

// 调整图片分辨率
fn resize_image_for_stream(img: &image::DynamicImage) -> image::DynamicImage {
    // 限制最大分辨率为 1280x720 (720p)
    let max_width = 1280u32;
    let max_height = 720u32;

    let (width, height) = (img.width(), img.height());

    if width <= max_width && height <= max_height {
        return img.clone();
    }

    // 计算缩放比例，保持宽高比
    let scale = (max_width as f32 / width as f32)
        .min(max_height as f32 / height as f32);

    let new_width = (width as f32 * scale) as u32;
    let new_height = (height as f32 * scale) as u32;

    img.resize(new_width, new_height, image::imageops::FilterType::Lanczos3)
}

// 推送视频帧到服务端（按API文档实现）
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

    // 解析 Base64 图片数据（支持 PNG 和 JPEG 格式）
    let base64_data = image_data
        .strip_prefix("data:image/png;base64,")
        .or_else(|| image_data.strip_prefix("data:image/jpeg;base64,"))
        .unwrap_or(&image_data);

    let image_bytes = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        base64_data,
    ).map_err(|e| format!("Base64解码失败: {}", e))?;

    // 加载图片
    let img = image::load_from_memory(&image_bytes)
        .map_err(|e| format!("加载图片失败: {}", e))?;

    println!("[VideoPush] 原始图片: {}x{} bytes, 设备: {}", img.width(), img.height(), device_code);

    // 调整分辨率（最大1280x720）
    let resized_img = resize_image_for_stream(&img);

    // 压缩图片到 100KB 以内
    let jpeg_buffer = compress_image_to_size(&resized_img, 100)?;

    // 将 JPEG 数据编码为 Base64（不包含 data:image/jpeg;base64, 前缀）
    let jpeg_base64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        &jpeg_buffer,
    );

    // 按 API 文档格式发送：deviceCode + data (Base64编码的JPEG)
    let form = multipart::Form::new()
        .text("deviceCode", device_code)
        .text("data", jpeg_base64);

    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .post(&format!("{}/client/inspection/video/push", config.api_url))
        .header("Authorization", format!("Bearer {}", token))
        .multipart(form)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status = response.status();
    let body = response.text().await.map_err(|e| e.to_string())?;
    println!("[VideoPush] API响应: status={}, body={}", status, body);

    if status.is_success() {
        let result: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
        let code = result.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
        if code == 0 || code == 200 {
            println!("[VideoPush] 推流成功");
            Ok(true)
        } else {
            let msg = result.get("msg").and_then(|m| m.as_str()).unwrap_or("上传失败").to_string();
            println!("[VideoPush] 推流失败: {}", msg);
            Err(msg)
        }
    } else {
        println!("[VideoPush] HTTP错误: {}", status);
        Err(format!("上传失败: {}", status))
    }
}

// 上传截图（按API文档实现，每5-10分钟上传一次）
#[tauri::command]
async fn upload_screenshot_file(
    image_data: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?.clone();

    let token = config.access_token.ok_or("未登录")?;
    let device_code = config.device_code;

    if device_code.is_empty() {
        return Err("设备未注册".to_string());
    }

    // 解析 Base64 图片数据
    let base64_data = image_data
        .strip_prefix("data:image/png;base64,")
        .unwrap_or(&image_data);

    let image_bytes = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        base64_data,
    ).map_err(|e| e.to_string())?;

    // 加载图片并转换为JPEG
    let img = image::load_from_memory(&image_bytes)
        .map_err(|e| format!("加载图片失败: {}", e))?;

    // 调整分辨率（最大1280x720）
    let resized_img = resize_image_for_stream(&img);

    // 压缩图片到 100KB 以内
    let jpeg_buffer = compress_image_to_size(&resized_img, 100)?;

    // 构建 multipart 表单
    let part = multipart::Part::bytes(jpeg_buffer)
        .file_name("screenshot.jpg")
        .mime_str("image/jpeg")
        .map_err(|e| e.to_string())?;

    let form = multipart::Form::new()
        .text("deviceCode", device_code)
        .part("file", part);

    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .post(&format!("{}/client/inspection/uploadScreenshot", config.api_url))
        .header("Authorization", format!("Bearer {}", token))
        .multipart(form)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status().is_success() {
        let result: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
        let code = result.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
        if code == 0 || code == 200 {
            // 返回截图URL
            let url = result.get("data")
                .and_then(|d| d.as_str())
                .unwrap_or("")
                .to_string();
            Ok(url)
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
// 检查是否是从开机自启启动
fn is_autostart(args: &[String]) -> bool {
    args.iter().any(|arg| arg == "--autostart")
}

pub fn run() {
    env_logger::init();
    let config = load_config();

    // 判断是否需要显示窗口：
    // 1. 未登录（无账号密码）→ 显示登录页
    // 2. 已登录但未注册 → 显示设备注册页
    // 3. 已登录且已注册 → 最小化到托盘
    let need_show_window = config.account_username.is_empty()
        || config.account_password.is_empty()
        || !config.is_registered;

    println!("启动检查: username={}, password={}, is_registered={}",
        !config.account_username.is_empty(),
        !config.account_password.is_empty(),
        config.is_registered);
    println!("是否需要显示窗口: {}", need_show_window);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // 当检测到第二个实例时，显示已存在的窗口
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .manage(AppState {
            config: Mutex::new(config),
            is_running: Mutex::new(false),
            monitor_shutdown: Mutex::new(None),
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
            upload_screenshot_file,
            toggle_window,
            exit_app,
            start_software_monitor,
            stop_software_monitor,
            get_software_monitor_stats,
            get_software_usages,
            push_all_running_software,
        ])
        .setup(move |app| {
            println!("[setup] ========== setup 钩子开始执行 ==========");

            // 获取命令行参数
            let args: Vec<String> = std::env::args().collect();
            let from_autostart = is_autostart(&args);
            println!("[setup] 启动参数: {:?}, 是否自启: {}", args, from_autostart);

            // 创建系统托盘
            println!("[setup] 创建系统托盘...");
            if let Some(icon) = app.default_window_icon() {
                match TrayIconBuilder::new()
                    .icon(icon.clone())
                    .tooltip("截图客户端")
                    .show_menu_on_left_click(false)
                    .on_tray_icon_event(|tray, event| {
                        if let TrayIconEvent::Click { .. } = event {
                            let app = tray.app_handle();
                            toggle_window_visibility(app);
                        }
                    })
                    .build(app)
                {
                    Ok(_) => println!("[setup] 托盘创建成功"),
                    Err(e) => println!("[setup] 托盘创建失败（非致命）: {}", e),
                }
            } else {
                println!("[setup] 警告: 无法获取默认窗口图标，跳过托盘创建");
            }

            // 设置全局快捷键 Ctrl+Shift+S（暂时禁用，避免热键冲突）
            // #[cfg(desktop)]
            // {
            //     use tauri_plugin_global_shortcut::GlobalShortcutExt;
            //     app.handle().plugin(
            //         tauri_plugin_global_shortcut::Builder::new()
            //             .with_handler(|app, shortcut, _event| {
            //                 if shortcut.matches(
            //                     tauri_plugin_global_shortcut::Modifiers::CONTROL | tauri_plugin_global_shortcut::Modifiers::SHIFT,
            //                     tauri_plugin_global_shortcut::Code::KeyS,
            //                 ) {
            //                     toggle_window_visibility(app);
            //                 }
            //             })
            //             .build(),
            //     )?;
            //
            //     // 注册快捷键
            //     let shortcut = tauri_plugin_global_shortcut::Shortcut::new(
            //         Some(tauri_plugin_global_shortcut::Modifiers::CONTROL | tauri_plugin_global_shortcut::Modifiers::SHIFT),
            //         tauri_plugin_global_shortcut::Code::KeyS,
            //     );
            //     app.global_shortcut().register(shortcut)?;
            // }

            // 设置窗口行为：关闭时最小化到托盘
            println!("setup: 开始设置窗口");
            if let Some(window) = app.get_webview_window("main") {
                println!("setup: 获取到主窗口");

                // 启用开发者工具快捷键 (F12)
                #[cfg(debug_assertions)]
                {
                    window.open_devtools();
                }

                // 确保窗口可以调整大小
                let _ = window.set_resizable(true);
                let _ = window.set_min_size(Some(tauri::LogicalSize::new(600.0, 400.0)));

                let window_clone = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = window_clone.hide();
                    }
                });

                // 根据用户登录/注册状态决定是否显示窗口：
                // - 未登录 → 显示登录页
                // - 已登录未注册 → 显示设备注册页
                // - 已登录已注册 → 最小化到托盘
                println!("setup: need_show_window = {}", need_show_window);
                if need_show_window {
                    println!("显示窗口（登录或注册页面）");
                    let show_result = window.show();
                    let focus_result = window.set_focus();
                    println!("setup: show_result = {:?}, focus_result = {:?}", show_result, focus_result);
                } else {
                    println!("已登录且已注册，显示窗口");
                    // 临时显示窗口用于预览
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            } else {
                println!("setup: 未能获取主窗口");
            }

            // 启动软件监控服务（如果已登录且已注册且配置启用）
            if !need_show_window {
                println!("启动软件监控服务...");
                let app_handle = app.app_handle().clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = init_software_monitor(app_handle).await {
                        log::error!("启动软件监控失败: {}", e);
                    }
                });
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// 切换窗口显示/隐藏
fn toggle_window_visibility(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if let Ok(true) = window.is_visible() {
            let _ = window.hide();
        } else {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

// 命令：切换窗口显示/隐藏
#[tauri::command]
fn toggle_window(app: tauri::AppHandle) {
    toggle_window_visibility(&app);
}

// 命令：完全退出应用
#[tauri::command]
async fn exit_app(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    // 停止软件监控服务
    stop_software_monitor_internal(&state).await;
    app.exit(0);
    Ok(())
}

// ========== 软件监控相关命令 ==========

use crate::monitor::{
    run_background_sync, MonitorConfig, MonitorEvent, ProcessMonitor, SessionManager,
    SyncConfig, SyncScheduler,
};

/// 初始化软件监控服务
async fn init_software_monitor(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();

    // 获取配置
    let (enabled, device_id, api_url, device_code, token, interval_secs, batch_secs) = {
        let config = state.config.lock().map_err(|e| e.to_string())?;
        (
            config.software_monitor_enabled,
            config.device_id.unwrap_or(0),
            config.api_url.clone(),
            config.device_code.clone(),
            config.access_token.clone().unwrap_or_default(),
            config.software_monitor_interval_secs,
            config.software_monitor_batch_secs,
        )
    };

    if !enabled {
        log::info!("软件监控已禁用");
        return Ok(());
    }

    if device_id == 0 {
        log::warn!("设备ID未设置，无法启动软件监控");
        return Ok(());
    }

    // 创建会话管理器
    let mut session_manager = SessionManager::new(device_id)?;
    let db = session_manager.get_db();

    // 创建同步调度器
    let sync_config = SyncConfig {
        api_url,
        device_code,
        token,
        batch_interval: Duration::from_secs(batch_secs as u64),
        ..Default::default()
    };
    let sync_scheduler = SyncScheduler::new(sync_config, db.clone())?;

    // 创建关闭信号
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    {
        let mut shutdown_guard = state.monitor_shutdown.lock().map_err(|e| e.to_string())?;
        *shutdown_guard = Some(shutdown_tx);
    }

    // 启动后台同步任务
    tokio::spawn(run_background_sync(sync_scheduler, shutdown_rx));

    // 启动监控循环
    let monitor_config = MonitorConfig {
        check_interval: Duration::from_secs(interval_secs as u64),
        device_id,
        ..Default::default()
    };
    let mut monitor = ProcessMonitor::new(monitor_config);

    log::info!("软件监控服务已启动");

    // 启动时立即推送一次全量软件列表
    println!("[init_software_monitor] 启动时推送全量软件列表...");
    if let Err(e) = push_all_running_software(state.clone().into()).await {
        log::error!("启动时全量推送失败: {}", e);
    }

    // 监控循环
    loop {
        // 检查关闭信号
        {
            let shutdown_guard = state.monitor_shutdown.lock().map_err(|e| e.to_string())?;
            if shutdown_guard.is_none() {
                log::info!("收到关闭信号，停止软件监控");
                break;
            }
        }

        // 执行一次轮询
        let events = monitor.tick();

        for event in events {
            match &event {
                MonitorEvent::SessionStarted(session) => {
                    log::debug!("软件启动: {} - {}", session.process_name, session.window_title);
                    // 新软件打开时，实时推送 started 事件
                    println!("[monitor] 新软件启动，实时推送: {}", session.process_name);
                    if let Err(e) = push_software_realtime(&state, "started", session).await {
                        log::error!("实时推送软件启动失败: {}", e);
                    }
                }
                MonitorEvent::SessionEnded(session) => {
                    log::debug!("软件关闭: {}，使用时长: {}秒", session.process_name, session.duration_secs);
                    // 软件关闭时，实时推送 stopped 事件
                    println!("[monitor] 软件关闭，实时推送: {}", session.process_name);
                    if let Err(e) = push_software_realtime(&state, "stopped", session).await {
                        log::error!("实时推送软件关闭失败: {}", e);
                    }
                }
                _ => {}
            }

            // 处理事件
            if let Err(e) = session_manager.handle_event(&event) {
                log::error!("处理监控事件失败: {}", e);
            }
        }

        // 等待下一次轮询
        tokio::time::sleep(Duration::from_secs(interval_secs as u64)).await;
    }

    // 关闭当前活跃会话
    if let Some(session) = session_manager.close_active_session()? {
        log::info!("关闭活跃会话: {}", session.process_name);
    }

    Ok(())
}

/// 停止软件监控服务
async fn stop_software_monitor_internal(state: &AppState) {
    let mut shutdown_guard = state.monitor_shutdown.lock().unwrap();
    if let Some(tx) = shutdown_guard.take() {
        let _ = tx.send(true);
        log::info!("已发送软件监控关闭信号");
    }
}

/// 命令：启动软件监控
#[tauri::command]
async fn start_software_monitor(app: tauri::AppHandle) -> Result<(), String> {
    init_software_monitor(app).await
}

/// 命令：停止软件监控
#[tauri::command]
async fn stop_software_monitor(state: State<'_, AppState>) -> Result<(), String> {
    stop_software_monitor_internal(&state).await;
    Ok(())
}

/// 命令：获取软件监控统计信息
#[tauri::command]
fn get_software_monitor_stats(state: State<AppState>) -> Result<serde_json::Value, String> {
    use crate::database::Database;

    let device_id = {
        let config = state.config.lock().map_err(|e| e.to_string())?;
        config.device_id.unwrap_or(0)
    };

    let db = Database::new()?;
    let (total, pending) = db.get_stats()?;

    Ok(serde_json::json!({
        "total_sessions": total,
        "pending_sync": pending,
        "device_id": device_id,
    }))
}

/// 软件使用信息结构体
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SoftwareUsage {
    id: String,
    process_name: String,
    window_title: String,
    is_active: bool,
    duration_secs: u32,
    last_active_time: String,
}

/// 命令：获取当前运行的软件列表（在独立线程执行避免阻塞UI）
#[tauri::command]
async fn get_software_usages() -> Result<Vec<SoftwareUsage>, String> {
    use crate::monitor::windows_api::{enumerate_user_processes, get_foreground_process};
    use std::sync::mpsc;

    println!("[get_software_usages] 开始执行...");

    // 在独立线程执行进程枚举
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let processes = enumerate_user_processes();

        let foreground = get_foreground_process();
        let foreground_pid = foreground.as_ref().map(|p| p.pid);

        // 限制返回数量，优先保留前台进程和常用软件
        let max_count = 30;
        let usages: Vec<SoftwareUsage> = processes
            .into_iter()
            .take(max_count)
            .map(|p| {
                let is_active = Some(p.pid) == foreground_pid;
                SoftwareUsage {
                    id: p.pid.to_string(),
                    process_name: p.name.clone(),
                    window_title: p.window_title.clone(),
                    is_active,
                    duration_secs: 0,
                    last_active_time: chrono::Local::now().to_rfc3339(),
                }
            })
            .collect();

        println!("[get_software_usages] 返回 {} 个软件", usages.len());
        let _ = tx.send(usages);
    });

    let usages = rx.recv().map_err(|e| e.to_string())?;
    Ok(usages)
}

/// 全量推送当前运行软件列表到服务器
#[tauri::command]
async fn push_all_running_software(state: State<'_, AppState>) -> Result<(), String> {
    use crate::monitor::windows_api::{enumerate_user_processes, get_foreground_process};
    use std::sync::mpsc;

    println!("[push_all_running_software] 开始全量推送...");

    // 获取配置
    let (api_url, device_code, token, device_name, class_name, school_class_id, device_type, dept_id) = {
        let config = state.config.lock().map_err(|e| e.to_string())?;
        // 如果设备名称为空，使用设备类型作为默认名称
        let final_device_name = if config.device_name.is_empty() {
            "智能黑板".to_string() // 默认设备类型名称
        } else {
            config.device_name.clone()
        };
        (
            config.api_url.clone(),
            config.device_code.clone(),
            config.access_token.clone().unwrap_or_default(),
            final_device_name,
            config.class_name.clone(),
            config.school_class_id.unwrap_or(0),
            "智能黑板".to_string(), // 设备类型
            config.dept_id.unwrap_or(0), // 部门ID
        )
    };

    if token.is_empty() {
        return Err("未登录，无法推送软件信息".to_string());
    }

    // 在独立线程执行进程枚举
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let processes = enumerate_user_processes();
        let foreground = get_foreground_process();
        let foreground_pid = foreground.as_ref().map(|p| p.pid);

        // 限制返回数量
        let max_count = 50;
        let usages: Vec<SoftwareUsage> = processes
            .into_iter()
            .take(max_count)
            .map(|p| {
                let is_active = Some(p.pid) == foreground_pid;
                SoftwareUsage {
                    id: p.pid.to_string(),
                    process_name: p.name.clone(),
                    window_title: p.window_title.clone(),
                    is_active,
                    duration_secs: 0,
                    last_active_time: chrono::Local::now().to_rfc3339(),
                }
            })
            .collect();

        let _ = tx.send(usages);
    });

    let usages = rx.recv().map_err(|e| e.to_string())?;
    println!("[push_all_running_software] 获取到 {} 个软件", usages.len());

    if usages.is_empty() {
        println!("[push_all_running_software] 没有运行中的软件，跳过推送");
        return Ok(());
    }

    // 构建请求体 - 按照API文档格式，添加设备信息和班级信息
    #[derive(Debug, serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    struct BatchSoftwareRequest {
        device_code: String,
        device_name: String,
        device_type: String,
        dept_id: i64,
        class_id: i64,
        class_name: String,
        sessions: Vec<SoftwareSessionPayload>,
    }

    #[derive(Debug, serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    struct SoftwareSessionPayload {
        id: String,
        process_name: String,
        window_title: String,
        exe_path: String,
        start_time: i64,
        end_time: Option<i64>,
        duration_secs: i64,
        device_id: i64,
    }

    let request = BatchSoftwareRequest {
        device_code: device_code.clone(),
        device_name: device_name.clone(),
        device_type: device_type.clone(),
        dept_id,
        class_id: school_class_id,
        class_name: class_name.clone(),
        sessions: usages
            .into_iter()
            .map(|u| SoftwareSessionPayload {
                id: u.id.clone(),
                process_name: u.process_name,
                window_title: u.window_title,
                exe_path: String::new(), // 全量推送时可能没有exe_path
                start_time: chrono::Local::now().timestamp_millis(),
                end_time: None,
                duration_secs: 0,
                device_id: 0,
            })
            .collect(),
    };

    // 打印调试信息
    let request_json = serde_json::to_string_pretty(&request).unwrap_or_default();
    println!("[push_all_running_software] 批量推送请求体:\n{}", request_json);

    // 发送全量推送请求
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .no_proxy()
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .post(&format!("{}/client/software/usage/batch", api_url))
        .header("Authorization", format!("Bearer {}", token))
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("网络请求失败: {}", e))?;

    if response.status().is_success() {
        let result: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
        let code = result.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
        if code == 0 || code == 200 {
            println!("[push_all_running_software] 全量推送成功");
            Ok(())
        } else {
            let msg = result.get("msg").and_then(|m| m.as_str()).unwrap_or("推送失败");
            Err(format!("服务器返回错误: {}", msg))
        }
    } else {
        Err(format!("HTTP错误: {}", response.status()))
    }
}

/// 推送单个软件使用信息到服务器 - 严格按照API文档格式
async fn push_software_realtime(
    state: &AppState,
    event_type: &str, // "started" 或 "stopped"
    session: &crate::database::SoftwareSession,
) -> Result<(), String> {
    let (api_url, device_code, token, device_name, _class_name, school_class_id, device_type, dept_id) = {
        let config = state.config.lock().map_err(|e| e.to_string())?;
        // 如果设备名称为空，使用设备类型作为默认名称
        let final_device_name = if config.device_name.is_empty() {
            "智能黑板".to_string()
        } else {
            config.device_name.clone()
        };
        (
            config.api_url.clone(),
            config.device_code.clone(),
            config.access_token.clone().unwrap_or_default(),
            final_device_name,
            config.class_name.clone(),
            config.school_class_id.unwrap_or(0),
            "智能黑板".to_string(),
            config.dept_id.unwrap_or(0), // 部门ID
        )
    };

    if token.is_empty() {
        return Err("未登录".to_string());
    }

    // 严格按照API文档构建请求体
    // API文档要求: deviceCode, event, session
    #[derive(Debug, serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    struct RealtimeRequest {
        device_code: String,
        device_name: String,
        device_type: String,
        dept_id: i64,
        class_id: i64,
        class_name: String,
        event: String,
        session: SessionPayload,
    }

    #[derive(Debug, serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    struct SessionPayload {
        id: String,
        process_name: String,
        window_title: String,
        exe_path: String,
        start_time: i64,
        #[serde(skip_serializing_if = "Option::is_none")]
        end_time: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        duration_secs: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        device_id: Option<i64>,
    }

    let session_payload = SessionPayload {
        id: session.id.clone(),
        process_name: session.process_name.clone(),
        window_title: session.window_title.clone(),
        exe_path: session.exe_path.clone(),
        start_time: session.start_time,
        end_time: session.end_time,
        duration_secs: if event_type == "stopped" { Some(session.duration_secs) } else { None },
        device_id: Some(session.device_id),
    };

    let request = RealtimeRequest {
        device_code,
        device_name,
        device_type,
        dept_id,
        class_id: school_class_id,
        class_name: _class_name,
        event: event_type.to_string(),
        session: session_payload,
    };

    // 打印调试信息
    let request_json = serde_json::to_string_pretty(&request).unwrap_or_default();
    println!("[push_software_realtime] 实时推送请求体:\n{}", request_json);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .no_proxy()
        .build()
        .map_err(|e| e.to_string())?;

    println!("[push_software_realtime] 推送 {} 事件: {}", event_type, session.process_name);

    let response = client
        .post(&format!("{}/client/software/usage/realtime", api_url))
        .header("Authorization", format!("Bearer {}", token))
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("网络请求失败: {}", e))?;

    if response.status().is_success() {
        let result: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
        let code = result.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
        if code == 0 || code == 200 {
            println!("[push_software_realtime] 推送成功: {}", session.process_name);
            Ok(())
        } else {
            let msg = result.get("msg").and_then(|m| m.as_str()).unwrap_or("推送失败");
            Err(format!("服务器返回错误: {}", msg))
        }
    } else {
        Err(format!("HTTP错误: {}", response.status()))
    }
}
