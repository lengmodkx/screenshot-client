import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";

// Mock data for browser preview
const MOCK_CONFIG = {
  interval: 10,
  mode: "cloud",
  local_path: "C:/Screenshots",
  api_url: "http://192.168.1.18:48080",
  token: "mock-token-123",
  username: "admin",
  auto_start: false,
  retention_days: 7,
  capture_mode: "camera",
  camera_resolution: "1080p",
  // 新增字段
  account_username: "",
  account_password: "",
  device_code: "",
  device_name: "",
  school_class_id: null,
  device_id: null,
  is_registered: false,
  dept_id: null,
  dept_name: "",
  access_token: null,
  refresh_token: null,
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
  capture_mode: string;      // "camera" | "screen"
  camera_resolution: string; // "480p" | "720p" | "1080p"
  // 新增字段
  account_username: string;
  account_password: string;
  device_code: string;
  device_name: string;
  school_class_id: number | null;
  device_id: number | null;
  is_registered: boolean;
  dept_id: number | null;
  dept_name: string;
  access_token: string | null;
  refresh_token: string | null;
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
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [hasCamera, setHasCamera] = useState(true);
  const [previewEnabled, setPreviewEnabled] = useState(true);
  const [isLoggedIn, setIsLoggedIn] = useState(false);

  // 检查 sessionStorage 登录状态
  useEffect(() => {
    const loggedIn = sessionStorage.getItem('isLoggedIn') === 'true';
    if (loggedIn) {
      setIsLoggedIn(true);
    }
  }, []);

  const videoRef = useRef<HTMLVideoElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const timerRef = useRef<number | null>(null);
  const heartbeatTimerRef = useRef<number | null>(null);
  const streamRef = useRef<MediaStream | null>(null);

  // 分辨率映射
  const resolutionMap: Record<string, { width: number; height: number }> = {
    "480p": { width: 640, height: 360 },
    "720p": { width: 1280, height: 720 },
    "1080p": { width: 1920, height: 1080 },
  };

  // 初始化：加载配置、检查网络、检测摄像头、自动登录
  useEffect(() => {
    loadConfig();
    checkNetwork();
    detectCamera();
    const interval = setInterval(checkNetwork, 30000);
    return () => clearInterval(interval);
  }, []);

  // 自动登录和注册检查
  useEffect(() => {
    if (config && isInitialized) {
      // 检查是否有账号密码
      if (!config.account_username || !config.account_password) {
        // 账号密码为空，显示设置弹窗让用户配置
        setShowSettings(true);
        return;
      }

    }
  }, [config, isInitialized]);

  // 启动心跳定时器
  useEffect(() => {
    if (isLoggedIn && config?.is_registered) {
      // 立即发送一次心跳
      sendHeartbeat();

      // 每30秒发送一次心跳
      heartbeatTimerRef.current = window.setInterval(() => {
        sendHeartbeat();
      }, 30000);

      return () => {
        if (heartbeatTimerRef.current) {
          clearInterval(heartbeatTimerRef.current);
        }
      };
    }
  }, [isLoggedIn, config?.is_registered]);

  // 发送心跳
  const sendHeartbeat = async () => {
    try {
      await invoke("send_heartbeat");
    } catch (e) {
      console.error("心跳失败:", e);
    }
  };

  // 检测摄像头
  const detectCamera = async () => {
    try {
      const devices = await navigator.mediaDevices.enumerateDevices();
      const videoDevices = devices.filter(d => d.kind === "videoinput");
      setHasCamera(videoDevices.length > 0);
    } catch {
      setHasCamera(false);
    }
  };

  // 配置加载后初始化并自动启动
  useEffect(() => {
    if (config && !isInitialized) {
      const captureMode = config.capture_mode || "camera";

      // 如果有摄像头且模式为摄像头模式，初始化摄像头
      if (captureMode === "camera" && hasCamera) {
        initCamera().then(() => {
          if (!cameraError) {
            captureFrame();
            timerRef.current = window.setInterval(() => {
              captureFrame();
            }, config.interval * 1000);
          }
        });
      } else {
        // 无摄像头或模式为截图模式，直接开始截图
        captureFrame();
        timerRef.current = window.setInterval(() => {
          captureFrame();
        }, config.interval * 1000);
      }
      setIsInitialized(true);
    }
  }, [config, hasCamera]);

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

      // 获取配置的分辨率
      const resolution = config?.camera_resolution || "1080p";
      const { width, height } = resolutionMap[resolution] || resolutionMap["1080p"];

      const stream = await navigator.mediaDevices.getUserMedia({
        video: {
          width: { ideal: width },
          height: { ideal: height }
        }
      });

      // 停止之前的流
      if (streamRef.current) {
        streamRef.current.getTracks().forEach(track => track.stop());
      }

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
    if (!config) return;

    const captureMode = config.capture_mode || "camera";

    // 摄像头模式
    if (captureMode === "camera" && hasCamera && !cameraError && videoRef.current && canvasRef.current) {
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

        // 保存或上传（已登录且已注册时上传到服务器）
        if (isLoggedIn && config.is_registered && isOnline) {
          try {
            await invoke("upload_screenshot_v2", { imageData });
            setStatusMessage(`已上传 - ${timeStr}`);
          } catch (e) {
            console.error("Upload failed:", e);
            setStatusMessage(`上传失败 - ${timeStr}`);
          }
        } else if (config.mode === "local") {
          await saveLocally(imageData);
          setStatusMessage(`已保存本地 - ${timeStr}`);
        }

        await invoke("cleanup_old_files");
      } catch (e) {
        console.error("Capture failed:", e);
        setStatusMessage(`截图失败: ${e}`);
      }
    } else {
      // 截图模式（调用后端截屏）
      try {
        const imageData = await invoke<string>("capture_screen");
        setCurrentImage(imageData);

        const now = new Date();
        const timeStr = now.toLocaleTimeString();
        setStats(prev => ({
          todayCount: prev.todayCount + 1,
          lastCaptureTime: timeStr
        }));

        // 保存或上传（已登录且已注册时上传到服务器）
        if (isLoggedIn && config.is_registered && isOnline) {
          try {
            await invoke("upload_screenshot_v2", { imageData });
            setStatusMessage(`已上传 - ${timeStr}`);
          } catch (e) {
            console.error("Upload failed:", e);
            setStatusMessage(`上传失败 - ${timeStr}`);
          }
        } else if (config.mode === "local") {
          await saveLocally(imageData);
          setStatusMessage(`已保存本地 - ${timeStr}`);
        }

        await invoke("cleanup_old_files");
      } catch (e) {
        console.error("Screen capture failed:", e);
        setStatusMessage(`截屏失败: ${e}`);
      }
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

  const handleSaveConfig = async () => {
    if (!config) return;

    // 检查是否已填写账号密码
    if (!config.account_username || !config.account_password) {
      setStatusMessage("请填写账号和密码");
      return;
    }

    try {
      // 先保存配置
      await invoke("update_config", { newConfig: config });

      // 重新加载配置确保生效
      const updatedConfig = await invoke<AppConfig>("get_config");
      setConfig(updatedConfig);

      setStatusMessage("正在登录...");

      // 直接调用后端登录接口，使用当前输入的值
      await invoke<{
        user_id: number;
        username: string;
        dept_id: number;
        dept_name: string;
        access_token: string;
        refresh_token: string;
        expires_time: string;
      }>("auto_login");

      // 更新登录状态
      setIsLoggedIn(true);
      setStatusMessage("登录成功");

      // 保存登录状态到 sessionStorage
      sessionStorage.setItem('isLoggedIn', 'true');

      // 重新加载页面
      window.location.reload();

    } catch (e) {
      console.error("登录失败:", e);
      setStatusMessage(`登录失败: ${e}`);
    }
  };

  // 全屏切换
  const toggleFullscreen = () => {
    if (!isFullscreen) {
      if (videoRef.current?.requestFullscreen) {
        videoRef.current.requestFullscreen();
      }
    } else {
      if (document.exitFullscreen) {
        document.exitFullscreen();
      }
    }
    setIsFullscreen(!isFullscreen);
  };

  // 切换预览开启/关闭
  const togglePreview = () => {
    if (previewEnabled) {
      // 停止视频流
      if (streamRef.current) {
        streamRef.current.getTracks().forEach(track => track.stop());
        streamRef.current = null;
      }
    } else {
      // 重新启动视频流
      if (config?.capture_mode === "camera" && hasCamera && !cameraError) {
        initCamera();
      }
    }
    setPreviewEnabled(!previewEnabled);
  };

  // 切换模式
  const switchCaptureMode = async (mode: string) => {
    if (!config) return;

    const newConfig = { ...config, capture_mode: mode };
    setConfig(newConfig);

    try {
      await invoke("update_config", { newConfig });
    } catch (e) {
      console.error("Save config failed:", e);
    }

    // 重启定时器
    if (timerRef.current) {
      clearInterval(timerRef.current);
    }
    captureFrame();
    timerRef.current = window.setInterval(() => {
      captureFrame();
    }, config.interval * 1000);

    // 根据模式初始化或停止摄像头
    if (mode === "camera" && hasCamera) {
      await initCamera();
    } else if (mode === "screen") {
      if (streamRef.current) {
        streamRef.current.getTracks().forEach(track => track.stop());
        streamRef.current = null;
      }
    }

    setStatusMessage(mode === "camera" ? "已切换到摄像头模式" : "已切换到截图模式");
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

  // 未登录时显示登录界面
  if (!isLoggedIn) {
    return (
      <div className="app" style={{ justifyContent: 'center', alignItems: 'center' }}>
        <div className="login-container" style={{ width: '100%', maxWidth: '360px', padding: '40px', background: 'white', borderRadius: '20px', boxShadow: '0 4px 20px rgba(0,0,0,0.1)' }}>
          <h2 style={{ textAlign: 'center', marginBottom: '30px', color: '#2e7d32', fontSize: '24px' }}>智能黑板客户端</h2>
          <div className="form-group">
            <label>API地址</label>
            <input
              type="text"
              value={config.api_url}
              onChange={(e) => setConfig({ ...config, api_url: e.target.value })}
              placeholder="请输入API地址"
            />
          </div>
          <div className="form-group">
            <label>账号</label>
            <input
              type="text"
              value={config.account_username}
              onChange={(e) => setConfig({ ...config, account_username: e.target.value })}
              placeholder="请输入账号"
            />
          </div>
          <div className="form-group">
            <label>密码</label>
            <input
              type="password"
              value={config.account_password}
              onChange={(e) => setConfig({ ...config, account_password: e.target.value })}
              placeholder="请输入密码"
              onKeyPress={(e) => e.key === 'Enter' && handleSaveConfig()}
            />
          </div>
          {statusMessage && <p className="error" style={{ textAlign: 'center', marginBottom: '20px' }}>{statusMessage}</p>}
          <button
            onClick={handleSaveConfig}
            style={{ width: '100%', padding: '16px', background: 'linear-gradient(135deg, #43a047 0%, #66bb6a 100%)', color: 'white', border: 'none', borderRadius: '12px', fontSize: '16px', fontWeight: '600', cursor: 'pointer' }}
          >
            登录
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="app">
      {/* 隐藏的 canvas 用于捕获帧 */}
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
        {isLoggedIn && (
          <>
            <button
              className="btn-small"
              onClick={() => switchCaptureMode(config.capture_mode === "camera" ? "screen" : "camera")}
            >
              {config.capture_mode === "camera" ? "📷 切换截图" : "🎥 切换摄像头"}
            </button>
            <button className="btn-small" onClick={() => setShowSettings(true)}>
              ⚙️ 设置
            </button>
          </>
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
            {cameraError ? '异常' : (config.is_registered ? '工作中' : '未注册')}
          </span>
        </div>
        <div className="info-row">
          <span className="info-label">登录状态</span>
          <span className="info-value" style={{ color: isLoggedIn ? '#2e7d32' : '#d32f2f' }}>
            {isLoggedIn ? '已登录' : '未登录'}
          </span>
        </div>
        <div className="info-row">
          <span className="info-label">在线状态</span>
          <span className="info-value" style={{ color: isOnline ? '#2e7d32' : '#d32f2f' }}>
            {isOnline ? '在线' : '离线'}
          </span>
        </div>
        <div className="info-row">
          <span className="info-label">数据来源</span>
          <span className="info-value">
            {config.capture_mode === "camera" ? "摄像头" : "屏幕截图"}
            {!hasCamera && <span style={{ fontSize: '12px', color: '#f57c00' }}> (无摄像头)</span>}
          </span>
        </div>
        {config.dept_name && (
          <div className="info-row">
            <span className="info-label">学校</span>
            <span className="info-value">{config.dept_name}</span>
          </div>
        )}
        {config.device_name && (
          <div className="info-row">
            <span className="info-label">设备名称</span>
            <span className="info-value">{config.device_name}</span>
          </div>
        )}
        <div className="info-row">
          <span className="info-label">截图间隔</span>
          <span className="info-value">{config.interval} 秒</span>
        </div>
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
        <div className="preview-header">
          <span>🖼️ 最新截图</span>
          {config.capture_mode === "camera" && hasCamera && (
            <div className="preview-controls">
              <select
                value={config.camera_resolution}
                onChange={async (e) => {
                  const newRes = e.target.value;
                  setConfig({ ...config, camera_resolution: newRes });
                  await invoke("update_config", {
                    newConfig: { ...config, camera_resolution: newRes }
                  });
                  initCamera();
                }}
                style={{ marginRight: '8px', padding: '4px' }}
              >
                <option value="480p">480p</option>
                <option value="720p">720p</option>
                <option value="1080p">1080p</option>
              </select>
              <button
                onClick={togglePreview}
                style={{ marginRight: '8px', padding: '4px 8px' }}
              >
                {previewEnabled ? '暂停预览' : '开启预览'}
              </button>
              <button onClick={toggleFullscreen} style={{ padding: '4px 8px' }}>
                {isFullscreen ? '退出全屏' : '全屏'}
              </button>
            </div>
          )}
        </div>
        <div className="preview-body">
          <div className="preview-image">
            {/* 摄像头预览 */}
            {config.capture_mode === "camera" && hasCamera && previewEnabled && !cameraError && (
              <video
                ref={videoRef}
                autoPlay
                playsInline
                muted
                style={{ width: '100%', height: '100%', objectFit: 'contain', background: '#000' }}
              />
            )}
            {/* 截图显示 */}
            {currentImage && (
              <img src={currentImage} alt="预览" style={{ width: '100%', height: '100%', objectFit: 'contain' }} />
            )}
            {!currentImage && (
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
            <div className="form-group">
              <label>截图模式</label>
              <select
                value={config.capture_mode}
                onChange={(e) => setConfig({ ...config, capture_mode: e.target.value })}
              >
                <option value="camera">摄像头模式</option>
                <option value="screen">截图模式</option>
              </select>
            </div>
            {config.capture_mode === "camera" && (
              <div className="form-group">
                <label>摄像头分辨率</label>
                <select
                  value={config.camera_resolution}
                  onChange={(e) => setConfig({ ...config, camera_resolution: e.target.value })}
                >
                  <option value="480p">480p (640x360)</option>
                  <option value="720p">720p (1280x720)</option>
                  <option value="1080p">1080p (1920x1080)</option>
                </select>
              </div>
            )}
            <div className="form-group">
              <label>API地址</label>
              <input
                type="text"
                value={config.api_url}
                onChange={(e) => setConfig({ ...config, api_url: e.target.value })}
                disabled={isLoggedIn}
              />
            </div>
            <div className="form-group">
              <label>账号</label>
              <input
                type="text"
                value={config.account_username}
                onChange={(e) => setConfig({ ...config, account_username: e.target.value })}
                placeholder="请输入账号"
                disabled={isLoggedIn}
              />
            </div>
            <div className="form-group">
              <label>密码</label>
              <input
                type="password"
                value={config.account_password}
                onChange={(e) => setConfig({ ...config, account_password: e.target.value })}
                placeholder="请输入密码"
                disabled={isLoggedIn}
              />
            </div>
            <div className="form-group">
              <label>设备名称</label>
              <input
                type="text"
                value={config.device_name}
                onChange={(e) => setConfig({ ...config, device_name: e.target.value })}
                placeholder="已注册设备名称"
                disabled={config.is_registered || !isLoggedIn}
              />
            </div>
            {config.is_registered && (
              <div className="form-group">
                <label>设备编码</label>
                <input
                  type="text"
                  value={config.device_code}
                  disabled
                />
              </div>
            )}
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
