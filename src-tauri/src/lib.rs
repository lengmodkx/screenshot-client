use chrono::Local;
use reqwest::multipart;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;
use tauri::Manager;
use tauri::tray::{TrayIconBuilder, TrayIconEvent};

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
    pub device_id: Option<i64>,     // 注册后返回的设备ID
    pub is_registered: bool,       // 是否已注册
    pub dept_id: Option<i64>,      // 学校/部门ID
    pub dept_name: String,         // 学校/部门名称
    pub access_token: Option<String>,  // 访问令牌
    pub refresh_token: Option<String>, // 刷新令牌
    // 后台运行配置
    pub autostart_enabled: bool,    // 开机自启开关
    pub show_window_on_start: bool, // 启动时是否显示窗口
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
            device_id: None,
            is_registered: false,
            dept_id: None,
            dept_name: String::new(),
            access_token: None,
            refresh_token: None,
            // 后台运行默认值
            autostart_enabled: true,
            show_window_on_start: false,
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
    state: State<'_, AppState>,
) -> Result<RegisterData, String> {
    println!("[register_device] 开始设备注册...");
    println!("[register_device] 设备名称: {}, 班级ID: {}", device_name, school_class_id);

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
                config.device_id = Some(data.id);
                config.is_registered = true;
                save_config(&config)?;
                println!("[register_device] 注册信息保存成功");

                Ok(data)
            } else {
                println!("[register_device] 设备已注册但无返回数据，构造基本信息...");
                // 设备已注册但没有返回数据，构造一个基本信息
                let mut config = state.config.lock().map_err(|e| e.to_string())?;
                config.device_name = device_name.clone();
                config.school_class_id = Some(school_class_id);
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

    // 解析 Base64 图片数据
    let base64_data = image_data
        .strip_prefix("data:image/png;base64,")
        .unwrap_or(&image_data);

    let image_bytes = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        base64_data,
    ).map_err(|e| e.to_string())?;

    // 加载图片
    let img = image::load_from_memory(&image_bytes)
        .map_err(|e| format!("加载图片失败: {}", e))?;

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

    if response.status().is_success() {
        let result: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
        let code = result.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
        if code == 0 || code == 200 {
            Ok(true)
        } else {
            Err(result.get("msg").and_then(|m| m.as_str()).unwrap_or("上传失败").to_string())
        }
    } else {
        Err(format!("上传失败: {}", response.status()))
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
        ])
        .setup(move |app| {
            // 获取命令行参数
            let args: Vec<String> = std::env::args().collect();
            let from_autostart = is_autostart(&args);

            // 创建系统托盘
            let tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("截图客户端")
                .show_menu_on_left_click(false)
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { .. } = event {
                        let app = tray.app_handle();
                        toggle_window_visibility(app);
                    }
                })
                .build(app)?;

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
                    println!("已登录且已注册，最小化到托盘");
                    let _ = window.hide();
                }
            } else {
                println!("setup: 未能获取主窗口");
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
fn exit_app(app: tauri::AppHandle) {
    app.exit(0);
}
