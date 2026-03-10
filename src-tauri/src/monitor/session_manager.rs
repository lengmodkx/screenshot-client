use crate::database::{Database, SoftwareSession};
use crate::monitor::process_monitor::{ActiveSession, MonitorEvent};
use chrono::Local;
use std::sync::{Arc, Mutex};

/// 会话管理器
/// 负责处理监控事件，管理会话生命周期，持久化到数据库
pub struct SessionManager {
    db: Arc<Mutex<Database>>,
    active_session: Option<ActiveSession>,
    device_id: i64,
}

impl SessionManager {
    /// 创建新的会话管理器
    pub fn new(device_id: i64) -> Result<Self, String> {
        let db = Database::new()?;

        Ok(Self {
            db: Arc::new(Mutex::new(db)),
            active_session: None,
            device_id,
        })
    }

    /// 处理监控事件
    pub fn handle_event(&mut self,
        event: &MonitorEvent,
    ) -> Result<Option<SoftwareSession>, String> {
        match event {
            MonitorEvent::SessionStarted(session) => {
                // 插入新会话到数据库
                let mut db = self.db.lock().map_err(|e| format!("获取数据库锁失败: {}", e))?;
                db.insert_session(session)?;

                // 创建活跃会话
                let active = ActiveSession {
                    session: session.clone(),
                    last_active_time: std::time::Instant::now(),
                    last_window_title: session.window_title.clone(),
                    total_active_secs: 0,
                };
                self.active_session = Some(active);

                Ok(Some(session.clone()))
            }

            MonitorEvent::SessionSwitched { ended_session, new_session } => {
                // 更新结束的会话
                let end_time = Local::now().timestamp_millis();
                let duration = (end_time - ended_session.start_time) / 1000;

                {
                    let mut db = self.db.lock().map_err(|e| format!("获取数据库锁失败: {}", e))?;
                    db.update_session_end(&ended_session.id, end_time, duration)?;
                }

                // 插入新会话
                {
                    let mut db = self.db.lock().map_err(|e| format!("获取数据库锁失败: {}", e))?;
                    db.insert_session(new_session)?;
                }

                // 更新活跃会话
                let active = ActiveSession {
                    session: new_session.clone(),
                    last_active_time: std::time::Instant::now(),
                    last_window_title: new_session.window_title.clone(),
                    total_active_secs: 0,
                };
                self.active_session = Some(active);

                Ok(Some(new_session.clone()))
            }

            MonitorEvent::SessionEnded(session) => {
                let end_time = Local::now().timestamp_millis();
                let duration = session.duration_secs;

                {
                    let mut db = self.db.lock().map_err(|e| format!("获取数据库锁失败: {}", e))?;
                    db.update_session_end(&session.id, end_time, duration)?;
                }

                self.active_session = None;

                Ok(Some(session.clone()))
            }

            MonitorEvent::UsageUpdated { session_id, duration_secs, window_title } => {
                // 更新会话时长和窗口标题
                if let Some(active) = &mut self.active_session {
                    if active.session.id == *session_id {
                        active.total_active_secs = *duration_secs;

                        if let Some(title) = window_title {
                            active.last_window_title = title.clone();
                            active.session.window_title = title.clone();

                            let mut db = self.db.lock().map_err(|e| format!("获取数据库锁失败: {}", e))?;
                            db.update_session_title(session_id, title)?;
                        }
                    }
                }

                Ok(None)
            }
        }
    }

    /// 关闭当前活跃会话（应用退出时调用）
    pub fn close_active_session(&mut self,
    ) -> Result<Option<SoftwareSession>, String> {
        if let Some(active) = self.active_session.take() {
            let end_time = Local::now().timestamp_millis();
            let duration = active.total_active_secs;

            {
                let mut db = self.db.lock().map_err(|e| format!("获取数据库锁失败: {}", e))?;
                db.update_session_end(&active.session.id, end_time, duration)?;
            }

            let mut session = active.session.clone();
            session.end_time = Some(end_time);
            session.duration_secs = duration;

            Ok(Some(session))
        } else {
            Ok(None)
        }
    }

    /// 获取当前活跃会话
    pub fn get_active_session(&self,
    ) -> Option<&ActiveSession> {
        self.active_session.as_ref()
    }

    /// 获取数据库实例（用于同步调度器）
    pub fn get_db(&self,
    ) -> Arc<Mutex<Database>> {
        self.db.clone()
    }

    /// 获取统计数据
    pub fn get_stats(&self,
    ) -> Result<(u32, u32), String> {
        let db = self.db.lock().map_err(|e| format!("获取数据库锁失败: {}", e))?;
        db.get_stats()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_manager() {
        // 需要实际数据库环境
        // 在生产环境测试
    }
}
