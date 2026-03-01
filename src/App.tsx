import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";

interface AppConfig {
  interval: number;
  mode: string;
  local_path: string;
  api_url: string;
  token: string | null;
  username: string | null;
  auto_start: boolean;
  retention_days: number;
}

function App() {
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [isRunning, setIsRunning] = useState(false);
  const [currentImage, setCurrentImage] = useState<string | null>(null);
  const [lastSaveTime, setLastSaveTime] = useState<string | null>(null);
  const [showSettings, setShowSettings] = useState(false);
  const [showLogin, setShowLogin] = useState(false);
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [loginError, setLoginError] = useState("");
  const [isOnline, setIsOnline] = useState(true);
  const [statusMessage, setStatusMessage] = useState("");
  const timerRef = useRef<number | null>(null);

  // 加载配置
  useEffect(() => {
    loadConfig();
    checkNetwork();
    const interval = setInterval(checkNetwork, 30000);
    return () => clearInterval(interval);
  }, []);

  // 定时截图
  useEffect(() => {
    if (isRunning && config) {
      startScreenshot();
      timerRef.current = window.setInterval(() => {
        startScreenshot();
      }, config.interval * 1000);
    } else {
      if (timerRef.current) {
        clearInterval(timerRef.current);
        timerRef.current = null;
      }
    }
    return () => {
      if (timerRef.current) {
        clearInterval(timerRef.current);
      }
    };
  }, [isRunning, config]);

  const loadConfig = async () => {
    try {
      const cfg = await invoke<AppConfig>("get_config");
      setConfig(cfg);
      if (cfg.auto_start) {
        setIsRunning(true);
      }
    } catch (e) {
      console.error("Failed to load config:", e);
    }
  };

  const checkNetwork = async () => {
    if (!config) return;
    try {
      const online = await invoke<boolean>("check_network", { apiUrl: config.api_url });
      setIsOnline(online);
    } catch {
      setIsOnline(false);
    }
  };

  const startScreenshot = async () => {
    if (!config) return;

    try {
      const imageData = await invoke<string>("capture_screen");
      setCurrentImage(imageData);

      const now = new Date();
      setLastSaveTime(now.toLocaleTimeString());

      if (config.mode === "cloud" && config.token && isOnline) {
        try {
          await invoke("upload_screenshot", { imageData });
          setStatusMessage(`已上传 - ${now.toLocaleTimeString()}`);
        } catch (e) {
          console.error("Upload failed, saving locally:", e);
          await saveLocally(imageData);
          setStatusMessage(`上传失败，已保存本地 - ${now.toLocaleTimeString()}`);
        }
      } else {
        await saveLocally(imageData);
        setStatusMessage(`已保存本地 - ${now.toLocaleTimeString()}`);
      }

      // 清理旧文件
      await invoke("cleanup_old_files");
    } catch (e) {
      console.error("Screenshot failed:", e);
      setStatusMessage(`截图失败: ${e}`);
    }
  };

  const saveLocally = async (imageData: string) => {
    try {
      await invoke("save_screenshot_to_local", { imageData });
    } catch (e) {
      console.error("Save locally failed:", e);
    }
  };

  const handleLogin = async () => {
    if (!username || !password) {
      setLoginError("请输入用户名和密码");
      return;
    }
    try {
      await invoke("login", { username, password });
      setShowLogin(false);
      setLoginError("");
      loadConfig();
      setStatusMessage("登录成功");
    } catch (e) {
      setLoginError(`登录失败: ${e}`);
    }
  };

  const handleLogout = async () => {
    try {
      await invoke("logout");
      loadConfig();
      setStatusMessage("已登出");
    } catch (e) {
      console.error("Logout failed:", e);
    }
  };

  const handleSaveConfig = async () => {
    if (!config) return;
    try {
      await invoke("update_config", { newConfig: config });
      setShowSettings(false);
      setStatusMessage("配置已保存");
    } catch (e) {
      console.error("Save config failed:", e);
    }
  };

  const toggleRunning = async () => {
    const newState = !isRunning;
    setIsRunning(newState);
    await invoke("set_running_state", { running: newState });
  };

  const manualCapture = async () => {
    await startScreenshot();
  };

  if (!config) {
    return <div className="loading">加载中...</div>;
  }

  return (
    <div className="app">
      <header className="header">
        <h1>截图客户端</h1>
        <div className="status-indicator">
          <span className={`status-dot ${isOnline ? 'online' : 'offline'}`}></span>
          <span>{isOnline ? '在线' : '离线'}</span>
        </div>
      </header>

      <main className="main">
        <div className="preview">
          {currentImage ? (
            <img src={currentImage} alt="预览" />
          ) : (
            <div className="preview-placeholder">
              点击"开始"启动截图
            </div>
          )}
        </div>

        <div className="info">
          <p>模式: {config.mode === 'cloud' ? '云端上传' : '本地保存'}</p>
          <p>间隔: {config.interval}秒</p>
          {config.username && <p>用户: {config.username}</p>}
          {lastSaveTime && <p>最后保存: {lastSaveTime}</p>}
          {statusMessage && <p className="status-msg">{statusMessage}</p>}
        </div>

        <div className="controls">
          <button
            className={`btn ${isRunning ? 'stop' : 'start'}`}
            onClick={toggleRunning}
          >
            {isRunning ? '停止' : '开始'}
          </button>
          <button className="btn capture" onClick={manualCapture}>
            立即截图
          </button>
        </div>

        <div className="actions">
          <button onClick={() => setShowSettings(true)}>设置</button>
          {config.mode === 'cloud' && !config.token && (
            <button onClick={() => setShowLogin(true)}>登录</button>
          )}
          {config.token && (
            <button onClick={handleLogout}>登出</button>
          )}
        </div>
      </main>

      {/* 设置弹窗 */}
      {showSettings && (
        <div className="modal">
          <div className="modal-content">
            <h2>设置</h2>
            <div className="form-group">
              <label>截图间隔（秒）</label>
              <input
                type="number"
                value={config.interval}
                onChange={(e) => setConfig({ ...config, interval: parseInt(e.target.value) || 10 })}
                min="1"
                max="300"
              />
            </div>
            <div className="form-group">
              <label>存储模式</label>
              <select
                value={config.mode}
                onChange={(e) => setConfig({ ...config, mode: e.target.value })}
              >
                <option value="local">本地保存</option>
                <option value="cloud">云端上传</option>
              </select>
            </div>
            <div className="form-group">
              <label>本地保存路径</label>
              <input
                type="text"
                value={config.local_path}
                onChange={(e) => setConfig({ ...config, local_path: e.target.value })}
              />
            </div>
            <div className="form-group">
              <label>API地址</label>
              <input
                type="text"
                value={config.api_url}
                onChange={(e) => setConfig({ ...config, api_url: e.target.value })}
              />
            </div>
            <div className="form-group">
              <label>保留天数</label>
              <input
                type="number"
                value={config.retention_days}
                onChange={(e) => setConfig({ ...config, retention_days: parseInt(e.target.value) || 7 })}
                min="1"
                max="365"
              />
            </div>
            <div className="modal-actions">
              <button onClick={handleSaveConfig}>保存</button>
              <button className="cancel" onClick={() => setShowSettings(false)}>取消</button>
            </div>
          </div>
        </div>
      )}

      {/* 登录弹窗 */}
      {showLogin && (
        <div className="modal">
          <div className="modal-content">
            <h2>登录</h2>
            <div className="form-group">
              <label>用户名</label>
              <input
                type="text"
                value={username}
                onChange={(e) => setUsername(e.target.value)}
              />
            </div>
            <div className="form-group">
              <label>密码</label>
              <input
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
              />
            </div>
            {loginError && <p className="error">{loginError}</p>}
            <div className="modal-actions">
              <button onClick={handleLogin}>登录</button>
              <button className="cancel" onClick={() => setShowLogin(false)}>取消</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export default App;
