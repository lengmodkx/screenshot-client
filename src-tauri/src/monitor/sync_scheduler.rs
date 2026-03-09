use crate::database::{Database, SoftwareSession};
use crate::monitor::process_monitor::MonitorEvent;
use reqwest::Client;
use serde::Serialize;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::time::sleep;

/// 同步错误类型
#[derive(Debug)]
pub enum SyncError {
    Network(String),
    Server(u16, String),
    Database(String),
    MaxRetriesReached,
}

impl std::fmt::Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncError::Network(msg) => write!(f, "网络错误: {}", msg),
            SyncError::Server(status, msg) => write!(f, "服务器错误 {}: {}", status, msg),
            SyncError::Database(msg) => write!(f, "数据库错误: {}", msg),
            SyncError::MaxRetriesReached => write!(f, "达到最大重试次数"),
        }
    }
}

impl std::error::Error for SyncError {}

/// 实时上报请求体
#[derive(Debug, Serialize)]
struct RealtimeRequest {
    device_code: String,
    event: String,
    session: SessionPayload,
}

/// 会话数据
#[derive(Debug, Serialize)]
struct SessionPayload {
    id: String,
    process_name: String,
    window_title: String,
    exe_path: String,
    start_time: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    end_time: Option<i64>,
    duration_secs: i64,
    device_id: i64,
}

impl From<&SoftwareSession> for SessionPayload {
    fn from(s: &SoftwareSession) -> Self {
        Self {
            id: s.id.clone(),
            process_name: s.process_name.clone(),
            window_title: s.window_title.clone(),
            exe_path: s.exe_path.clone(),
            start_time: s.start_time,
            end_time: s.end_time,
            duration_secs: s.duration_secs,
            device_id: s.device_id,
        }
    }
}

/// 批量上报请求体
#[derive(Debug, Serialize)]
struct BatchRequest {
    device_code: String,
    sessions: Vec<SessionPayload>,
}

/// 同步调度器配置
#[derive(Debug, Clone)]
pub struct SyncConfig {
    pub api_url: String,
    pub device_code: String,
    pub token: String,
    pub batch_interval: Duration,
    pub max_retries: u32,
    pub batch_size: usize,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            api_url: String::new(),
            device_code: String::new(),
            token: String::new(),
            batch_interval: Duration::from_secs(300), // 5分钟
            max_retries: 5,
            batch_size: 50,
        }
    }
}

/// 同步调度器
pub struct SyncScheduler {
    config: SyncConfig,
    db: Arc<Mutex<Database>>,
    client: Client,
    last_batch_time: Instant,
}

impl SyncScheduler {
    /// 创建新的同步调度器
    pub fn new(config: SyncConfig, db: Arc<Mutex<Database>>) -> Result<Self, String> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .no_proxy()
            .build()
            .map_err(|e| format!("创建HTTP客户端失败: {}", e))?;

        Ok(Self {
            config,
            db,
            client,
            last_batch_time: Instant::now(),
        })
    }

    /// 发送实时事件
    pub async fn send_realtime(&self, event: &MonitorEvent) -> Result<(), SyncError> {
        let request = match event {
            MonitorEvent::SessionStarted(session) => {
                Some(("started", session))
            }
            MonitorEvent::SessionEnded(session) => {
                Some(("ended", session))
            }
            MonitorEvent::SessionSwitched { new_session, .. } => {
                Some(("switched", new_session))
            }
            _ => None,
        };

        if let Some((event_type, session)) = request {
            let payload = RealtimeRequest {
                device_code: self.config.device_code.clone(),
                event: event_type.to_string(),
                session: session.into(),
            };

            let response = self
                .client
                .post(format!("{}/client/software/usage/realtime", self.config.api_url))
                .header("Authorization", format!("Bearer {}", self.config.token))
                .json(&payload)
                .send()
                .await
                .map_err(|e| SyncError::Network(e.to_string()))?;

            if !response.status().is_success() {
                let status = response.status().as_u16();
                let text = response.text().await.unwrap_or_default();
                return Err(SyncError::Server(status, text));
            }

            // 实时上报成功，标记为已同步
            self.mark_synced(&[session.id.clone()]).await?;
        }

        Ok(())
    }

    /// 执行批量同步
    pub async fn sync_batch(&mut self) -> Result<u32, SyncError> {
        let sessions = {
            let db = self.db.lock().map_err(|e| SyncError::Database(e.to_string()))?;
            db.get_pending_sync(self.config.batch_size)
                .map_err(|e| SyncError::Database(e))?
        };

        if sessions.is_empty() {
            return Ok(0);
        }

        let payload = BatchRequest {
            device_code: self.config.device_code.clone(),
            sessions: sessions.iter().map(|s| s.into()).collect(),
        };

        let response = self
            .client
            .post(format!("{}/client/software/usage/batch", self.config.api_url))
            .header("Authorization", format!("Bearer {}", self.config.token))
            .json(&payload)
            .send()
            .await
            .map_err(|e| SyncError::Network(e.to_string()))?;

        if response.status().is_success() {
            // 批量上报成功
            let ids: Vec<String> = sessions.iter().map(|s| s.id.clone()).collect();
            self.mark_synced(&ids).await?;

            self.last_batch_time = Instant::now();
            Ok(sessions.len() as u32)
        } else {
            // 上报失败，更新重试信息
            let status = response.status().as_u16();
            let text = response.text().await.unwrap_or_default();

            // 4xx错误不重试
            if (400..500).contains(&status) {
                let ids: Vec<String> = sessions.iter().map(|s| s.id.clone()).collect();
                self.mark_synced(&ids).await?; // 标记为已处理，不再重试
                return Err(SyncError::Server(status, text));
            }

            // 5xx错误更新重试计数
            self.update_retry_info(&sessions).await?;
            Err(SyncError::Server(status, text))
        }
    }

    /// 标记记录已同步
    async fn mark_synced(&self, ids: &[String]) -> Result<(), SyncError> {
        let mut db = self.db.lock().map_err(|e| SyncError::Database(e.to_string()))?;
        db.mark_synced(&ids.to_vec())
            .map_err(|e| SyncError::Database(e))?;
        Ok(())
    }

    /// 更新重试信息
    async fn update_retry_info(&self, sessions: &[SoftwareSession]) -> Result<(), SyncError> {
        use chrono::Local;

        let mut db = self.db.lock().map_err(|e| SyncError::Database(e.to_string()))?;

        for session in sessions {
            // 从数据库查询当前重试次数
            let next_retry = Local::now().timestamp_millis() + 60000; // 1分钟后重试
            db.update_retry(&session.id, 1, next_retry)
                .map_err(|e| SyncError::Database(e))?;
        }

        Ok(())
    }

    /// 检查是否应该执行批量同步
    pub fn should_sync_batch(&self) -> bool {
        Instant::now().duration_since(self.last_batch_time) >= self.config.batch_interval
    }

    /// 更新配置
    pub fn update_config(&mut self, config: SyncConfig) {
        self.config = config;
    }

    /// 带退避重试的批量同步
    pub async fn sync_batch_with_retry(&mut self) -> Result<u32, SyncError> {
        let mut retries = 0;

        loop {
            match self.sync_batch().await {
                Ok(count) => return Ok(count),
                Err(e) => {
                    retries += 1;

                    if retries >= self.config.max_retries {
                        return Err(SyncError::MaxRetriesReached);
                    }

                    // 指数退避等待
                    let wait_secs = 2_u64.pow(retries);
                    sleep(Duration::from_secs(wait_secs)).await;
                }
            }
        }
    }
}

/// 运行后台同步任务
pub async fn run_background_sync(
    mut scheduler: SyncScheduler,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(60)); // 每分钟检查一次

    loop {
        tokio::select! {
            _ = interval.tick() => {
                if scheduler.should_sync_batch() {
                    if let Err(e) = scheduler.sync_batch_with_retry().await {
                        log::error!("批量同步失败: {}", e);
                    }
                }
            }
            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    log::info!("收到关闭信号，停止后台同步任务");
                    break;
                }
            }
        }
    }

    // 退出前尝试最后一次同步
    log::info!("应用退出前执行最后一次同步...");
    match scheduler.sync_batch_with_retry().await {
        Ok(count) => log::info!("最后同步完成: {} 条记录", count),
        Err(e) => log::error!("最后同步失败: {}", e),
    }
}
