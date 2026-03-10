use rusqlite::{Connection, Result as SqlResult, params};
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use chrono::Local;

/// 软件使用会话记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoftwareSession {
    pub id: String,
    pub process_name: String,
    pub window_title: String,
    pub exe_path: String,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub duration_secs: i64,
    pub device_id: i64,
    pub synced: bool,
}

impl SoftwareSession {
    pub fn new(
        process_name: String,
        window_title: String,
        exe_path: String,
        device_id: i64,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            process_name,
            window_title,
            exe_path,
            start_time: Local::now().timestamp_millis(),
            end_time: None,
            duration_secs: 0,
            device_id,
            synced: false,
        }
    }
}

/// 同步队列记录
#[derive(Debug)]
pub struct SyncQueueItem {
    pub id: i64,
    pub session_id: String,
    pub retry_count: i32,
    pub next_retry_at: i64,
}

pub struct Database {
    conn: Connection,
}

impl Database {
    /// 初始化数据库连接
    pub fn new() -> Result<Self, String> {
        let db_path = Self::get_db_path()?;
        let conn = Connection::open(db_path).map_err(|e| format!("打开数据库失败: {}", e))?;

        let db = Self { conn };
        db.init_tables()?;

        Ok(db)
    }

    /// 获取数据库文件路径
    fn get_db_path() -> Result<PathBuf, String> {
        let data_dir = dirs::data_dir()
            .ok_or("无法获取数据目录")?
            .join("ScreenshotClient");

        if !data_dir.exists() {
            std::fs::create_dir_all(&data_dir).map_err(|e| format!("创建数据目录失败: {}", e))?;
        }

        Ok(data_dir.join("software_monitor.db"))
    }

    /// 初始化数据表
    fn init_tables(&self) -> Result<(), String> {
        // 软件使用会话表
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS software_sessions (
                id TEXT PRIMARY KEY,
                process_name TEXT NOT NULL,
                window_title TEXT,
                exe_path TEXT,
                start_time INTEGER NOT NULL,
                end_time INTEGER,
                duration_secs INTEGER DEFAULT 0,
                device_id INTEGER NOT NULL,
                synced INTEGER DEFAULT 0,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        ).map_err(|e| format!("创建software_sessions表失败: {}", e))?;

        // 同步队列表
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS sync_queue (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL UNIQUE,
                retry_count INTEGER DEFAULT 0,
                next_retry_at INTEGER,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (session_id) REFERENCES software_sessions(id)
            )",
            [],
        ).map_err(|e| format!("创建sync_queue表失败: {}", e))?;

        // 创建索引
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_sessions_device_time ON software_sessions(device_id, start_time)",
            [],
        ).map_err(|e| format!("创建索引失败: {}", e))?;

        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_sessions_synced ON software_sessions(synced)",
            [],
        ).map_err(|e| format!("创建索引失败: {}", e))?;

        Ok(())
    }

    /// 插入新的会话记录
    pub fn insert_session(&mut self, session: &SoftwareSession) -> Result<(), String> {
        self.conn.execute(
            "INSERT INTO software_sessions (id, process_name, window_title, exe_path, start_time, end_time, duration_secs, device_id, synced)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                session.id,
                session.process_name,
                session.window_title,
                session.exe_path,
                session.start_time,
                session.end_time,
                session.duration_secs,
                session.device_id,
                if session.synced { 1 } else { 0 }
            ],
        ).map_err(|e| format!("插入会话记录失败: {}", e))?;

        // 同时加入同步队列
        self.add_to_sync_queue(&session.id)?;

        Ok(())
    }

    /// 更新会话记录（结束时间和时长）
    pub fn update_session_end(&mut self, session_id: &str, end_time: i64, duration_secs: i64) -> Result<(), String> {
        self.conn.execute(
            "UPDATE software_sessions SET end_time = ?1, duration_secs = ?2 WHERE id = ?3",
            params![end_time, duration_secs, session_id],
        ).map_err(|e| format!("更新会话记录失败: {}", e))?;

        Ok(())
    }

    /// 更新会话窗口标题
    pub fn update_session_title(&mut self, session_id: &str, window_title: &str) -> Result<(), String> {
        self.conn.execute(
            "UPDATE software_sessions SET window_title = ?1 WHERE id = ?2",
            params![window_title, session_id],
        ).map_err(|e| format!("更新窗口标题失败: {}", e))?;

        Ok(())
    }

    /// 添加到同步队列
    fn add_to_sync_queue(&mut self, session_id: &str) -> Result<(), String> {
        let next_retry = Local::now().timestamp_millis();

        self.conn.execute(
            "INSERT OR IGNORE INTO sync_queue (session_id, next_retry_at) VALUES (?1, ?2)",
            params![session_id, next_retry],
        ).map_err(|e| format!("添加到同步队列失败: {}", e))?;

        Ok(())
    }

    /// 获取待同步的记录
    pub fn get_pending_sync(&self, limit: usize) -> Result<Vec<SoftwareSession>, String> {
        let mut stmt = self.conn.prepare(
            "SELECT s.id, s.process_name, s.window_title, s.exe_path, s.start_time, s.end_time, s.duration_secs, s.device_id, s.synced
             FROM software_sessions s
             INNER JOIN sync_queue q ON s.id = q.session_id
             WHERE q.next_retry_at <= ?1
             ORDER BY q.retry_count ASC, s.start_time ASC
             LIMIT ?2"
        ).map_err(|e| format!("准备查询失败: {}", e))?;

        let current_time = Local::now().timestamp_millis();

        let sessions = stmt.query_map(
            params![current_time, limit as i64],
            |row| {
                Ok(SoftwareSession {
                    id: row.get(0)?,
                    process_name: row.get(1)?,
                    window_title: row.get(2)?,
                    exe_path: row.get(3)?,
                    start_time: row.get(4)?,
                    end_time: row.get(5)?,
                    duration_secs: row.get(6)?,
                    device_id: row.get(7)?,
                    synced: row.get::<_, i32>(8)? != 0,
                })
            }
        ).map_err(|e| format!("查询失败: {}", e))?;

        let mut result = Vec::new();
        for session in sessions {
            result.push(session.map_err(|e| format!("解析会话失败: {}", e))?);
        }

        Ok(result)
    }

    /// 标记记录已同步
    pub fn mark_synced(&mut self, session_ids: &[String]) -> Result<(), String> {
        let tx = self.conn.transaction().map_err(|e| format!("开始事务失败: {}", e))?;

        for id in session_ids {
            tx.execute(
                "UPDATE software_sessions SET synced = 1 WHERE id = ?1",
                params![id],
            ).map_err(|e| format!("标记同步状态失败: {}", e))?;

            tx.execute(
                "DELETE FROM sync_queue WHERE session_id = ?1",
                params![id],
            ).map_err(|e| format!("从同步队列删除失败: {}", e))?;
        }

        tx.commit().map_err(|e| format!("提交事务失败: {}", e))?;

        Ok(())
    }

    /// 更新重试信息
    pub fn update_retry(&mut self, session_id: &str, retry_count: i32, next_retry_at: i64) -> Result<(), String> {
        self.conn.execute(
            "UPDATE sync_queue SET retry_count = ?1, next_retry_at = ?2 WHERE session_id = ?3",
            params![retry_count, next_retry_at, session_id],
        ).map_err(|e| format!("更新重试信息失败: {}", e))?;

        Ok(())
    }

    /// 删除超过30天的已同步记录（清理旧数据）
    pub fn cleanup_old_records(&mut self, days: i32) -> Result<u32, String> {
        let cutoff = Local::now().timestamp_millis() - (days as i64 * 24 * 60 * 60 * 1000);

        let count = self.conn.execute(
            "DELETE FROM software_sessions WHERE synced = 1 AND start_time < ?1",
            params![cutoff],
        ).map_err(|e| format!("清理旧记录失败: {}", e))?;

        Ok(count as u32)
    }

    /// 获取统计数据
    pub fn get_stats(&self) -> Result<(u32, u32), String> {
        let total: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM software_sessions",
            [],
            |row| row.get(0),
        ).map_err(|e| format!("查询总数失败: {}", e))?;

        let pending: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM sync_queue",
            [],
            |row| row.get(0),
        ).map_err(|e| format!("查询待同步数失败: {}", e))?;

        Ok((total as u32, pending as u32))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = SoftwareSession::new(
            "notepad.exe".to_string(),
            "无标题 - 记事本".to_string(),
            "C:\\Windows\\notepad.exe".to_string(),
            123,
        );

        assert_eq!(session.process_name, "notepad.exe");
        assert!(session.id.len() > 0);
    }
}
