use crate::database::SoftwareSession;
use crate::monitor::windows_api::{get_foreground_process, ProcessInfo, should_monitor};
use std::time::{Duration, Instant};

/// 监控事件类型
#[derive(Debug, Clone)]
pub enum MonitorEvent {
    /// 新会话开始
    SessionStarted(SoftwareSession),
    /// 会话切换（旧会话结束，新会话开始）
    SessionSwitched {
        ended_session: SoftwareSession,
        new_session: SoftwareSession,
    },
    /// 当前会话结束
    SessionEnded(SoftwareSession),
    /// 使用时长更新
    UsageUpdated {
        session_id: String,
        duration_secs: i64,
        window_title: Option<String>,
    },
}

/// 活跃会话信息
#[derive(Debug, Clone)]
pub struct ActiveSession {
    pub session: SoftwareSession,
    pub last_active_time: Instant,
    pub last_window_title: String,
    pub total_active_secs: i64,
}

/// 进程监控配置
#[derive(Debug, Clone)]
pub struct MonitorConfig {
    /// 轮询间隔
    pub check_interval: Duration,
    /// 闲置阈值（超过此时间未活动则认为闲置）
    pub idle_threshold: Duration,
    /// 设备ID
    pub device_id: i64,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            check_interval: Duration::from_secs(5),
            idle_threshold: Duration::from_secs(300), // 5分钟
            device_id: 0,
        }
    }
}

/// 进程监控器
pub struct ProcessMonitor {
    config: MonitorConfig,
    active_session: Option<ActiveSession>,
    last_check_time: Instant,
}

impl ProcessMonitor {
    /// 创建新的监控器
    pub fn new(config: MonitorConfig) -> Self {
        Self {
            config,
            active_session: None,
            last_check_time: Instant::now(),
        }
    }

    /// 执行一次轮询检查
    /// 返回产生的事件列表
    pub fn tick(&mut self) -> Vec<MonitorEvent> {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_check_time);
        self.last_check_time = now;

        let mut events = Vec::new();

        // 获取当前前台进程
        let current_process = match get_foreground_process() {
            Some(p) => p,
            None => {
                // 没有前台进程，结束当前会话
                if let Some(active) = self.active_session.take() {
                    events.push(self.end_session(active));
                }
                return events;
            }
        };

        // 判断是否应该监控此进程
        if !should_monitor(&current_process) {
            // 不应该监控，结束当前会话（如果有）
            if let Some(active) = self.active_session.take() {
                events.push(self.end_session(active));
            }
            return events;
        }

        // 检查是否是当前活跃会话
        let idle_threshold = self.config.idle_threshold;
        match &mut self.active_session {
            Some(active) => {
                if active.session.process_name == current_process.name {
                    // 同一会话，更新信息
                    let event = Self::update_active_session(active, &current_process, elapsed, idle_threshold);
                    if let Some(e) = event {
                        events.push(e);
                    }
                } else {
                    // 会话切换
                    let old_session = self.active_session.take().unwrap();
                    events.push(self.end_session(old_session));

                    let new_session = self.start_session(&current_process);
                    events.push(MonitorEvent::SessionStarted(new_session.session.clone()));
                    self.active_session = Some(new_session);
                }
            }
            None => {
                // 新会话
                let new_session = self.start_session(&current_process);
                events.push(MonitorEvent::SessionStarted(new_session.session.clone()));
                self.active_session = Some(new_session);
            }
        }

        events
    }

    /// 开始新会话
    fn start_session(&self, process: &ProcessInfo) -> ActiveSession {
        let session = SoftwareSession::new(
            process.name.clone(),
            process.window_title.clone(),
            process.exe_path.clone(),
            self.config.device_id,
        );

        ActiveSession {
            last_active_time: Instant::now(),
            last_window_title: process.window_title.clone(),
            total_active_secs: 0,
            session,
        }
    }

    /// 结束会话
    fn end_session(&self, mut active: ActiveSession) -> MonitorEvent {
        let end_time = chrono::Local::now().timestamp_millis();
        active.session.end_time = Some(end_time);
        active.session.duration_secs = active.total_active_secs;

        MonitorEvent::SessionEnded(active.session)
    }

    /// 更新活跃会话
    fn update_active_session(
        active: &mut ActiveSession,
        process: &ProcessInfo,
        elapsed: Duration,
        idle_threshold: Duration,
    ) -> Option<MonitorEvent> {
        let elapsed_secs = elapsed.as_secs() as i64;

        // 检测闲置状态
        if process.window_title != active.last_window_title {
            // 窗口标题变化，重置活跃时间
            active.last_active_time = Instant::now();
            active.last_window_title = process.window_title.clone();

            // 窗口标题变化，通知更新
            return Some(MonitorEvent::UsageUpdated {
                session_id: active.session.id.clone(),
                duration_secs: active.total_active_secs,
                window_title: Some(process.window_title.clone()),
            });
        }

        // 检查是否闲置
        let idle_time = Instant::now().duration_since(active.last_active_time);
        if idle_time < idle_threshold {
            // 未闲置，累加使用时长
            active.total_active_secs += elapsed_secs;

            // 每30秒上报一次使用时长
            if active.total_active_secs % 30 < elapsed_secs {
                return Some(MonitorEvent::UsageUpdated {
                    session_id: active.session.id.clone(),
                    duration_secs: active.total_active_secs,
                    window_title: None,
                });
            }
        }

        None
    }

    /// 强制结束当前会话（用于应用退出时）
    pub fn force_end_session(&mut self) -> Option<MonitorEvent> {
        self.active_session.take().map(|active| self.end_session(active))
    }

    /// 获取当前活跃会话信息
    pub fn get_active_session(&self) -> Option<&ActiveSession> {
        self.active_session.as_ref()
    }

    /// 更新配置
    pub fn update_config(&mut self, config: MonitorConfig) {
        self.config = config;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monitor_config_default() {
        let config = MonitorConfig::default();
        assert_eq!(config.check_interval, Duration::from_secs(5));
        assert_eq!(config.idle_threshold, Duration::from_secs(300));
    }
}
