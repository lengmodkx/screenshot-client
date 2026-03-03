import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";

// Mock data for browser preview
const MOCK_CONFIG = {
  interval: 10,
  mode: "cloud",
  local_path: "C:/Screenshots",
  api_url: "http://localhost:8080",
  token: "mock-token-123",
  username: "admin",
  auto_start: false,
  retention_days: 7
};

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

interface Stats {
  todayCount: number;
  lastCaptureTime: string | null;
}

function App() {
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [currentImage, setCurrentImage] = useState<string | null>(null);
  const [stats, setStats] = useState<Stats>({ todayCount: 0, lastCaptureTime: null });
  const [showSettings, setShowSettings] = useState(false);
  const [showLogin, setShowLogin] = useState(false);
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [loginError, setLoginError] = useState("");
  const [isOnline, setIsOnline] = useState(true);
  const [statusMessage, setStatusMessage] = useState("");
  const [cameraError, setCameraError] = useState<string | null>(null);
  const [isInitialized, setIsInitialized] = useState(false);

  const videoRef = useRef<HTMLVideoElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const timerRef = useRef<number | null>(null);
  const streamRef = useRef<MediaStream | null>(null);

  // 初始化：加载配置、检查网络、初始化摄像头
  useEffect(() => {
    loadConfig();
    checkNetwork();
    const interval = setInterval(checkNetwork, 30000);
    return () => clearInterval(interval);
  }, []);

  // 配置加载后初始化摄像头并自动启动
  useEffect(() => {
    if (config && !isInitialized) {
      initCamera().then(() => {
        // 摄像头初始化完成后启动定时器
        if (config && !cameraError) {
          captureFrame();
          timerRef.current = window.setInterval(() => {
            captureFrame();
          }, config.interval * 1000);
        }
      });
      setIsInitialized(true);
    }
  }, [config]);

  // 监听 cameraError 变化，处理摄像头出错后恢复的情况
  useEffect(() => {
    if (config && !cameraError && isInitialized && !timerRef.current) {
      captureFrame();
      timerRef.current = window.setInterval(() => {
        captureFrame();
      }, config.interval * 1000);
    }

    return () => {
      if (timerRef.current) {
        clearInterval(timerRef.current);
        timerRef.current = null;
      }
    };
  }, [cameraError, config, isInitialized]);

  const loadConfig = async () => {
    try {
      const cfg = await invoke<AppConfig>("get_config");
      setConfig(cfg);
    } catch (e) {
      console.error("Failed to load config, using mock:", e);
      setConfig(MOCK_CONFIG);
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

  // 初始化摄像头
  const initCamera = async () => {
    try {
      setCameraError(null);
      setStatusMessage("正在启动摄像头...");

      const stream = await navigator.mediaDevices.getUserMedia({
        video: {
          width: { ideal: 1920 },
          height: { ideal: 1080 }
        }
      });

      streamRef.current = stream;

      if (videoRef.current) {
        videoRef.current.srcObject = stream;
        await videoRef.current.play();
      }

      setStatusMessage("摄像头已启动");
      console.log("Camera initialized successfully");
    } catch (e: unknown) {
      console.error("Camera init failed:", e);
      const errorMessage = e instanceof Error ? e.message : String(e);

      if (errorMessage.includes("Permission denied") || errorMessage.includes("NotAllowed")) {
        setCameraError("需要摄像头权限才能截图，请允许访问");
      } else if (errorMessage.includes("NotFoundError") || errorMessage.includes("no video device")) {
        setCameraError("未检测到摄像头设备");
      } else {
        setCameraError(`摄像头启动失败: ${errorMessage}`);
      }
      setStatusMessage("摄像头启动失败");
    }
  };

  // 从摄像头捕获帧
  const captureFrame = async () => {
    if (!config || !videoRef.current || !canvasRef.current || cameraError) return;

    try {
      const video = videoRef.current;
      const canvas = canvasRef.current;

      // 设置 canvas 尺寸与视频一致
      if (canvas.width !== video.videoWidth || canvas.height !== video.videoHeight) {
        canvas.width = video.videoWidth || 640;
        canvas.height = video.videoHeight || 480;
      }

      // 绘制当前帧
      const ctx = canvas.getContext('2d');
      if (!ctx) return;

      ctx.drawImage(video, 0, 0, canvas.width, canvas.height);

      // 转为 base64
      const imageData = canvas.toDataURL('image/png');
      setCurrentImage(imageData);

      const now = new Date();
      const timeStr = now.toLocaleTimeString();
      setStats(prev => ({
        todayCount: prev.todayCount + 1,
        lastCaptureTime: timeStr
      }));

      // 保存或上传
      if (config.mode === "cloud" && config.token && isOnline) {
        try {
          await invoke("upload_screenshot", { imageData });
          setStatusMessage(`已上传 - ${timeStr}`);
        } catch (e) {
          console.error("Upload failed, saving locally:", e);
          await saveLocally(imageData);
          setStatusMessage(`上传失败，已保存本地 - ${timeStr}`);
        }
      } else {
        await saveLocally(imageData);
        setStatusMessage(`已保存本地 - ${timeStr}`);
      }

      await invoke("cleanup_old_files");
    } catch (e) {
      console.error("Capture failed:", e);
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

      // 重启定时器以应用新的间隔
      if (timerRef.current) {
        clearInterval(timerRef.current);
        timerRef.current = window.setInterval(() => {
          captureFrame();
        }, config.interval * 1000);
      }
    } catch (e) {
      console.error("Save config failed:", e);
    }
  };

  // 组件卸载时清理摄像头
  useEffect(() => {
    return () => {
      if (streamRef.current) {
        streamRef.current.getTracks().forEach(track => track.stop());
      }
      if (timerRef.current) {
        clearInterval(timerRef.current);
      }
    };
  }, []);

  if (!config) {
    return <div className="loading">加载中...</div>;
  }

  return (
    <div className="app">
      {/* 隐藏的视频元素用于摄像头预览 */}
      <video
        ref={videoRef}
        style={{ display: 'none' }}
        playsInline
        muted
      />
      <canvas ref={canvasRef} style={{ display: 'none' }} />

      {/* 顶部状态栏 */}
      <div className="top-bar">
        <h1>截图客户端</h1>
        <div className="status-badge">
          <span className={`status-dot ${isOnline ? 'online' : 'offline'}`}></span>
          <span>{isOnline ? '在线' : '离线'}</span>
        </div>
      </div>

      {/* 副按钮 - 移除开始/停止和手动截图按钮 */}
      <div className="secondary-controls" style={{ justifyContent: 'center' }}>
        <button className="btn-small" onClick={() => setShowSettings(true)}>
          ⚙️ 设置
        </button>
        {config.mode === 'cloud' && !config.token && (
          <button className="btn-small" onClick={() => setShowLogin(true)}>
            🔐 登录
          </button>
        )}
        {config.token && (
          <button className="btn-small" onClick={handleLogout}>
            🚪 登出
          </button>
        )}
      </div>

      {/* 错误提示 */}
      {cameraError && (
        <div className="error-banner">
          {cameraError}
        </div>
      )}

      {/* 信息展示区 */}
      <div className="info-section">
        <div className="info-row">
          <span className="info-label">运行状态</span>
          <span className="info-value" style={{ color: cameraError ? '#d32f2f' : '#2e7d32' }}>
            {cameraError ? '异常' : '工作中'}
          </span>
        </div>
        <div className="info-row">
          <span className="info-label">数据来源</span>
          <span className="info-value">摄像头</span>
        </div>
        <div className="info-row">
          <span className="info-label">存储模式</span>
          <span className="info-value">{config.mode === 'cloud' ? '云端上传' : '本地保存'}</span>
        </div>
        <div className="info-row">
          <span className="info-label">截图间隔</span>
          <span className="info-value">{config.interval} 秒</span>
        </div>
        {config.username && (
          <div className="info-row">
            <span className="info-label">登录用户</span>
            <span className="info-value">{config.username}</span>
          </div>
        )}
      </div>

      {/* 统计区 */}
      <div className="stats-section">
        <div className="stat-card">
          <div className="stat-number">{stats.todayCount}</div>
          <div className="stat-label">今日截图</div>
        </div>
        <div className="stat-card">
          <div className="stat-number">{stats.lastCaptureTime || '--:--:--'}</div>
          <div className="stat-label">最后截图</div>
        </div>
      </div>

      {/* 预览区 */}
      <div className="preview-section">
        <div className="preview-header">🖼️ 最新截图</div>
        <div className="preview-body">
          <div className="preview-image">
            {currentImage ? (
              <img src={currentImage} alt="预览" />
            ) : (
              <div className="preview-placeholder">
                {cameraError ? '摄像头异常' : '正在启动...'}
              </div>
            )}
          </div>
          {statusMessage && <div className="status-text">{statusMessage}</div>}
        </div>
      </div>

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
