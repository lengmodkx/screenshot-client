import React, { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { DeviceSetupWrapper } from "./DeviceSetupWrapper";

// 设备类型名称
const deviceTypeMap: Record<number, string> = {
  1: '智能黑板',
  2: '智能多媒体设备',
  3: '其他'
};



// Mock data for browser preview
const MOCK_CONFIG = {
  interval: 10,
  mode: "cloud",
  local_path: "C:/Screenshots",
  api_url: "http://172.16.10.11:48080",
  token: "mock-token-123",
  username: "admin",
  auto_start: false,
  retention_days: 7,
  capture_mode: "auto",      // "auto" | "camera" | "screen"
  camera_resolution: "1080p",
  // 新增字段
  account_username: "",
  account_password: "",
  tenant_name: "",
  device_code: "",
  device_name: "",
  school_class_id: null,
  device_id: null,
  is_registered: false,
  dept_id: null,
  dept_name: "",
  access_token: null,
  refresh_token: null,
  // 后台运行配置
  autostart_enabled: true,
  show_window_on_start: false,
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
  capture_mode: string;      // "auto" | "camera" | "screen"
  camera_resolution: string; // "480p" | "720p" | "1080p"
  // 新增字段
  account_username: string;
  account_password: string;
  tenant_name: string;
  device_code: string;
  device_name: string;
  school_class_id: number | null;
  device_id: number | null;
  is_registered: boolean;
  dept_id: number | null;
  dept_name: string;
  access_token: string | null;
  refresh_token: string | null;
  // 后台运行配置
  autostart_enabled: boolean;
  show_window_on_start: boolean;
}

interface Stats {
  todayCount: number;
  lastCaptureTime: string | null;
}

function App() {
  // App 组件渲染计数
  const appRenderCount = useRef(0);
  appRenderCount.current++;
  
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
  const [hasCamera, setHasCamera] = useState(true);
  const [previewEnabled] = useState(true);
  const [isLoggedIn, setIsLoggedIn] = useState(false);
  const [actualCaptureMode, setActualCaptureMode] = useState<"camera" | "screen">("camera");
  const [debugInfo, setDebugInfo] = useState<string>("");
  const [showDeviceSetup, setShowDeviceSetup] = useState(false);
  const [classList, setClassList] = useState<Array<{ id: number; class_name: string }>>([]);

  const videoRef = useRef<HTMLVideoElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const timerRef = useRef<number | null>(null);
  const heartbeatTimerRef = useRef<number | null>(null);
  const screenshotTimerRef = useRef<number | null>(null);
  const streamRef = useRef<MediaStream | null>(null);
  const currentImageRef = useRef<string | null>(null);

  // 分辨率映射
  const resolutionMap: Record<string, { width: number; height: number }> = {
    "480p": { width: 640, height: 360 },
    "720p": { width: 1280, height: 720 },
    "1080p": { width: 1920, height: 1080 },
  };

  // 初始化：加载配置（只在组件挂载时执行一次）
  useEffect(() => {
    const init = async () => {
      try {
        const cfg = await invoke<AppConfig>("get_config");
        setConfig(cfg);
      } catch (e) {
        console.error("Failed to load config, using mock:", e);
        setConfig(MOCK_CONFIG);
      }
    };
    init();
  }, []);

  // 自动登录和注册检查 - 只执行一次（修复：移除isInitialized依赖）
  const autoLoginStarted = useRef(false);
  useEffect(() => {
    if (isLoggedIn) return;
    if (autoLoginStarted.current) return;
    if (!config) return;
    
    // 检查是否有账号密码
    if (!config.account_username || !config.account_password) return;

    autoLoginStarted.current = true;
    
    // 直接在这里执行登录逻辑
    const doAutoLogin = async () => {
      try {
        setStatusMessage("正在自动登录...");
        
        const loginResult = await invoke<{
          user_id: number;
          username: string;
          dept_id: number;
          dept_name: string;
          access_token: string;
          refresh_token: string;
          expires_time: string;
        }>("auto_login");

        console.log("自动登录成功");
        setIsLoggedIn(true);

        // 检查是否已注册设备
        if (config.is_registered) {
          setStatusMessage("自动登录成功");
        } else {
          setStatusMessage("请先设置班级和设备信息");
          try {
            const classes = await invoke<Array<{ id: number; className: string }>>("get_class_list");
            setClassList(classes.map(c => ({ id: c.id, class_name: c.className })));
          } catch (e) {
            console.error("获取班级列表失败:", e);
          }
          setShowDeviceSetup(true);
        }
      } catch (e) {
        console.error("自动登录失败:", e);
        setStatusMessage(`自动登录失败: ${e}`);
      }
    };
    
    doAutoLogin();
  }, [config, isLoggedIn]); // 移除isInitialized

  // 启动心跳定时器 - 完全禁用
  // useEffect(() => {
  //   if (showDeviceSetup) return;
  //   if (!isLoggedIn || !config?.is_registered) return;
  //   sendHeartbeat();
  //   heartbeatTimerRef.current = window.setInterval(() => {
  //     sendHeartbeat();
  //   }, 30000);
  //   return () => {
  //     if (heartbeatTimerRef.current) {
  //       clearInterval(heartbeatTimerRef.current);
  //     }
  //   };
  // }, [isLoggedIn, config?.is_registered, showDeviceSetup]);

  // 发送心跳
  const sendHeartbeat = async () => {
    try {
      await invoke("send_heartbeat");
    } catch (e) {
      console.error("心跳失败:", e);
    }
  };

  // 检测摄像头
  // 检测摄像头是否可用（包括权限检查）
  const detectCamera = async (): Promise<boolean> => {
    try {
      // 1. 检查是否有视频设备
      const devices = await navigator.mediaDevices.enumerateDevices();
      const videoDevices = devices.filter(d => d.kind === "videoinput");

      if (videoDevices.length === 0) {
        console.log("No video devices found");
        setHasCamera(false);
        return false;
      }

      // 2. 尝试获取摄像头权限
      const stream = await navigator.mediaDevices.getUserMedia({ video: true });
      stream.getTracks().forEach(track => track.stop()); // 立即释放

      console.log("Camera detected and accessible");
      setHasCamera(true);
      return true;
    } catch (e) {
      console.log("Camera not available:", e);
      setHasCamera(false);
      return false;
    }
  };

  // 自动选择采集模式
  const selectCaptureMode = async (): Promise<"camera" | "screen"> => {
    if (!config) return "screen";

    // 如果配置强制指定了模式
    if (config.capture_mode === "camera") {
      const hasCam = await detectCamera();
      if (hasCam) return "camera";
      console.warn("Camera mode requested but camera not available, falling back to screen");
      return "screen";
    }

    if (config.capture_mode === "screen") {
      return "screen";
    }

    // auto 模式：优先摄像头
    const hasCam = await detectCamera();
    return hasCam ? "camera" : "screen";
  };

  // 配置加载后初始化（登录后才启动截图）- 添加延迟避免注册时卡死
  useEffect(() => {
    if (showDeviceSetup) return; // 设备设置页面不启动截图
    if (!config || isInitialized || !isLoggedIn) return;
    
    // 延迟启动截图，给注册流程足够时间完成
    const timer = setTimeout(() => {
      setIsInitialized(true);
      selectCaptureMode().then((mode) => {
        setActualCaptureMode(mode);
        setStatusMessage(`使用${mode === "camera" ? "摄像头" : "屏幕截图"}模式`);
        if (mode === "camera") {
          initCamera().then(() => {
            if (!cameraError) {
              startCaptureLoop();
            } else {
              setActualCaptureMode("screen");
              startCaptureLoop();
            }
          });
        } else {
          startCaptureLoop();
        }
      });
    }, 500); // 500ms 延迟，确保注册流程完成
    
    return () => clearTimeout(timer);
  }, [config, isLoggedIn, showDeviceSetup]);

  const loadConfig = useCallback(async () => {
    try {
      const cfg = await invoke<AppConfig>("get_config");
      setConfig(cfg);
    } catch (e) {
      console.error("Failed to load config, using mock:", e);
      setConfig(MOCK_CONFIG);
    }
  }, []);

  // 内部处理设备注册
  const handleDeviceRegisterInternal = async (classId: number, deviceType: number, deviceName: string) => {
    console.log("[handleDeviceRegisterInternal] 开始注册:", { classId, deviceType, deviceName });

    try {
      // 直接调用注册
      await invoke("register_device", {
        deviceName: deviceName,
        schoolClassId: classId,
        deviceType: deviceType
      });

      console.log("[handleDeviceRegisterInternal] 注册成功");

      // 关闭设备设置页面
      setShowDeviceSetup(false);
      
      // 延迟设置登录状态
      setTimeout(() => {
        setIsLoggedIn(true);
      }, 100);

    } catch (e) {
      console.error("[handleDeviceRegisterInternal] 设备注册失败:", e);
      alert(`设备注册失败: ${e}`);
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

  // 启动采集循环
  const startCaptureLoop = () => {
    // 立即执行一次
    captureAndPushFrame();

    // 视频推送：每500ms（2帧/秒）
    timerRef.current = window.setInterval(() => {
      captureAndPushFrame();
    }, 500);

    // 截图上传：每5分钟上传一次（用于后台查看静态画面）
    uploadScreenshotFile();
    screenshotTimerRef.current = window.setInterval(() => {
      uploadScreenshotFile();
    }, 5 * 60 * 1000);
  };

  // 捕获帧并推送到视频流（每500ms调用一次，2帧/秒）
  const captureAndPushFrame = async () => {
    if (!config) return;

    // 使用实际选择的采集模式
    const captureMode = actualCaptureMode;

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
        currentImageRef.current = imageData;

        const now = new Date();
        const timeStr = now.toLocaleTimeString();
        setStats(prev => ({
          todayCount: prev.todayCount + 1,
          lastCaptureTime: timeStr
        }));

        // 推送到视频流（不等待响应，避免阻塞）
        if (isLoggedIn && config.is_registered && isOnline) {
          invoke("upload_screenshot_v2", { imageData }).catch(e => {
            console.error("Video push failed:", e);
          });
        }
      } catch (e) {
        console.error("Capture failed:", e);
      }
    } else {
      // 截图模式（调用后端截屏）
      try {
        const imageData = await invoke<string>("capture_screen");
        setCurrentImage(imageData);
        currentImageRef.current = imageData;

        const now = new Date();
        const timeStr = now.toLocaleTimeString();
        setStats(prev => ({
          todayCount: prev.todayCount + 1,
          lastCaptureTime: timeStr
        }));

        // 推送到视频流（不等待响应，避免阻塞）
        if (isLoggedIn && config.is_registered && isOnline) {
          invoke("upload_screenshot_v2", { imageData }).catch(e => {
            console.error("Video push failed:", e);
          });
        }
      } catch (e) {
        console.error("Screen capture failed:", e);
      }
    }
  };

  // 上传截图文件（每5分钟调用一次）
  const uploadScreenshotFile = async () => {
    if (!config || !isLoggedIn || !config.is_registered || !isOnline) return;

    const imageData = currentImageRef.current;
    if (!imageData) {
      console.log("No image available for screenshot upload");
      return;
    }

    try {
      const url = await invoke<string>("upload_screenshot_file", { imageData });
      console.log("Screenshot uploaded:", url);
      setStatusMessage(`截图已上传: ${new Date().toLocaleTimeString()}`);
    } catch (e) {
      console.error("Screenshot upload failed:", e);
    }
  };

  // 手动触发截图（保留原有功能供用户手动使用）
  const captureFrame = async () => {
    await captureAndPushFrame();
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
      await loadConfig();
      setIsLoggedIn(true);  // 设置登录状态
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
      const loginResult = await invoke<{
        user_id: number;
        username: string;
        dept_id: number;
        dept_name: string;
        access_token: string;
        refresh_token: string;
        expires_time: string;
      }>("auto_login");

      console.log("登录成功:", loginResult);

      // 更新登录状态（触发页面切换）
      setIsLoggedIn(true);
      setStatusMessage("登录成功");

      // 重新加载配置以获取最新的用户信息
      await loadConfig();

      // 登录成功后重新获取最新配置来判断是否已注册
      const latestConfig = await invoke<AppConfig>("get_config");

      // 检查是否已注册设备
      if (latestConfig.is_registered) {
        // 已注册，直接进入摄像头页面
        setStatusMessage("登录成功");
      } else {
        // 未注册，显示设备设置页面
        setStatusMessage("请先设置班级和设备信息");
        try {
          setDebugInfo("正在加载班级列表...");
          const classes = await invoke<Array<{ id: number; className: string }>>("get_class_list");
          console.log("班级列表:", classes);
          // 将 className 映射为 class_name 以兼容前端显示
          const formattedClasses = classes.map(c => ({ id: c.id, class_name: c.className }));
          setClassList(formattedClasses);
          setDebugInfo(`加载到 ${classes.length} 个班级`);
        } catch (e) {
          console.error("获取班级列表失败:", e);
          setDebugInfo(`获取班级列表失败: ${e}`);
          // 即使获取班级列表失败，也显示设备注册页面（班级列表为空）
        }
        // 无论如何都显示设备注册页面
        setShowDeviceSetup(true);
      }

    } catch (e) {
      console.error("登录失败:", e);
      setStatusMessage(`登录失败: ${e}`);
    }
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

  // 配置加载中
  if (!config) {
    return <div style={{ padding: '20px' }}>加载中...</div>;
  }

  // 设备设置页面 - 使用稳定的 React 组件
  if (showDeviceSetup) {
    return <DeviceSetupWrapper 
      classList={classList}
      onRegister={handleDeviceRegisterInternal}
    />;
  }

  // 未登录时显示登录界面
  if (!isLoggedIn) {
    return (
      <div className="app" style={{ height: '100vh', overflow: 'auto', justifyContent: 'center', alignItems: 'center', background: 'linear-gradient(135deg, #1a1a2e 0%, #16213e 100%)', padding: '20px', boxSizing: 'border-box' }}>
        <div className="login-container" style={{ width: '100%', maxWidth: '400px', padding: '32px', background: 'rgba(255,255,255,0.95)', borderRadius: '24px', boxShadow: '0 8px 32px rgba(0,0,0,0.3)', margin: 'auto' }}>
          <div style={{ textAlign: 'center', marginBottom: '36px' }}>
            <div style={{ width: '64px', height: '64px', margin: '0 auto 16px', background: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)', borderRadius: '16px', display: 'flex', alignItems: 'center', justifyContent: 'center', fontSize: '32px' }}>
              📷
            </div>
            <h2 style={{ color: '#1a1a2e', fontSize: '26px', fontWeight: '700', margin: 0 }}>智能黑板客户端</h2>
            <p style={{ color: '#666', marginTop: '8px', fontSize: '14px' }}>智慧校园 screenshot 管理系统</p>
          </div>
          <div className="form-group" style={{ marginBottom: '20px' }}>
            <label style={{ display: 'block', marginBottom: '8px', color: '#333', fontWeight: '500', fontSize: '14px' }}>API地址</label>
            <input
              type="text"
              value={config.api_url}
              onChange={(e) => setConfig({ ...config, api_url: e.target.value })}
              placeholder="请输入API地址"
              style={{ width: '100%', padding: '14px 16px', border: '2px solid #e0e0e0', borderRadius: '12px', fontSize: '15px', transition: 'all 0.3s', boxSizing: 'border-box' }}
            />
          </div>
          <div className="form-group" style={{ marginBottom: '20px' }}>
            <label style={{ display: 'block', marginBottom: '8px', color: '#333', fontWeight: '500', fontSize: '14px' }}>租户名称</label>
            <input
              type="text"
              value={config.tenant_name}
              onChange={(e) => setConfig({ ...config, tenant_name: e.target.value })}
              placeholder="请输入租户名称"
              style={{ width: '100%', padding: '14px 16px', border: '2px solid #e0e0e0', borderRadius: '12px', fontSize: '15px', transition: 'all 0.3s', boxSizing: 'border-box' }}
            />
          </div>
          <div className="form-group" style={{ marginBottom: '20px' }}>
            <label style={{ display: 'block', marginBottom: '8px', color: '#333', fontWeight: '500', fontSize: '14px' }}>账号</label>
            <input
              type="text"
              value={config.account_username}
              onChange={(e) => setConfig({ ...config, account_username: e.target.value })}
              placeholder="请输入账号"
              style={{ width: '100%', padding: '14px 16px', border: '2px solid #e0e0e0', borderRadius: '12px', fontSize: '15px', transition: 'all 0.3s', boxSizing: 'border-box' }}
            />
          </div>
          <div className="form-group" style={{ marginBottom: '24px' }}>
            <label style={{ display: 'block', marginBottom: '8px', color: '#333', fontWeight: '500', fontSize: '14px' }}>密码</label>
            <input
              type="password"
              value={config.account_password}
              onChange={(e) => setConfig({ ...config, account_password: e.target.value })}
              placeholder="请输入密码"
              onKeyPress={(e) => e.key === 'Enter' && handleSaveConfig()}
              style={{ width: '100%', padding: '14px 16px', border: '2px solid #e0e0e0', borderRadius: '12px', fontSize: '15px', transition: 'all 0.3s', boxSizing: 'border-box' }}
            />
          </div>
          {statusMessage && (
            <p style={{ textAlign: 'center', marginBottom: '20px', padding: '12px', background: statusMessage.includes('成功') ? '#e8f5e9' : '#ffebee', color: statusMessage.includes('成功') ? '#2e7d32' : '#c62828', borderRadius: '8px', fontSize: '14px' }}>
              {statusMessage}
            </p>
          )}
          <button
            onClick={handleSaveConfig}
            style={{ width: '100%', padding: '16px', background: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)', color: 'white', border: 'none', borderRadius: '12px', fontSize: '16px', fontWeight: '600', cursor: 'pointer', transition: 'all 0.3s', boxShadow: '0 4px 15px rgba(102, 126, 234, 0.4)' }}
            onMouseEnter={(e) => e.currentTarget.style.transform = 'translateY(-2px)'}
            onMouseLeave={(e) => e.currentTarget.style.transform = 'translateY(0)'}
          >
            登 录
          </button>
          {debugInfo && (
            <div style={{ marginTop: '16px', padding: '12px', background: '#f5f5f5', borderRadius: '8px', fontSize: '12px', color: '#666', wordBreak: 'break-all', maxHeight: '200px', overflow: 'auto' }}>
              <strong>调试信息:</strong><br/>
              {debugInfo}
            </div>
          )}
        </div>
      </div>
    );
  }

  return (
    <div className="app" style={{ padding: 0, background: '#0a0a0f', height: '100vh', display: 'flex', flexDirection: 'column' }}>
      {/* 隐藏的 canvas 用于捕获帧 */}
      <canvas ref={canvasRef} style={{ display: 'none' }} />

      {/* 顶部状态栏 - 简洁信息条 */}
      <div style={{
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'center',
        padding: '12px 24px',
        background: 'linear-gradient(90deg, #1a1a2e 0%, #16213e 100%)',
        borderBottom: '1px solid rgba(255,255,255,0.1)',
        color: 'white',
        flexShrink: 0
      }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '24px' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <span style={{ fontSize: '20px' }}>📷</span>
            <span style={{ fontWeight: 600, fontSize: '16px' }}>智能黑板</span>
          </div>
          {config.account_username && (
            <div style={{ display: 'flex', alignItems: 'center', gap: '6px', color: '#a0a0b0', fontSize: '14px' }}>
              <span>👤</span>
              <span>{config.account_username}</span>
            </div>
          )}
          {config.dept_name && (
            <div style={{ display: 'flex', alignItems: 'center', gap: '6px', color: '#a0a0b0', fontSize: '14px' }}>
              <span>🏫</span>
              <span>{config.dept_name}</span>
            </div>
          )}
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          <span style={{
            width: '8px',
            height: '8px',
            borderRadius: '50%',
            background: isOnline ? '#4ade80' : '#ef4444',
            boxShadow: isOnline ? '0 0 8px #4ade80' : '0 0 8px #ef4444'
          }} />
          <span style={{ fontSize: '14px', color: isOnline ? '#4ade80' : '#ef4444' }}>
            {isOnline ? '在线' : '离线'}
          </span>
        </div>
      </div>

      {/* 错误提示 */}
      {cameraError && (
        <div style={{
          padding: '12px 24px',
          background: '#7f1d1d',
          color: '#fecaca',
          textAlign: 'center',
          fontSize: '14px',
          flexShrink: 0
        }}>
          {cameraError}
        </div>
      )}

      {/* 主内容区 - 摄像头预览 */}
      <div style={{
        flex: 1,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        padding: '16px',
        overflow: 'hidden',
        position: 'relative'
      }}>
        <div style={{
          width: '100%',
          height: '100%',
          borderRadius: '12px',
          overflow: 'hidden',
          background: '#000',
          boxShadow: '0 4px 24px rgba(0,0,0,0.5)',
          position: 'relative'
        }}>
          {/* 摄像头预览 - 全屏大画面 */}
          {config.capture_mode === "camera" && hasCamera && previewEnabled && !cameraError && (
            <video
              ref={videoRef}
              autoPlay
              playsInline
              muted
              style={{ width: '100%', height: '100%', objectFit: 'cover', background: '#000' }}
            />
          )}
          {/* 屏幕截图显示 */}
          {config.capture_mode === "screen" && currentImage && (
            <img src={currentImage} alt="预览" style={{ width: '100%', height: '100%', objectFit: 'contain' }} />
          )}
          {/* 无画面时 */}
          {(!previewEnabled || cameraError || !currentImage) && (
            <div style={{
              width: '100%',
              height: '100%',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              flexDirection: 'column',
              gap: '16px',
              color: '#666'
            }}>
              <span style={{ fontSize: '64px', opacity: 0.5 }}>{cameraError ? '⚠️' : '📷'}</span>
              <span style={{ fontSize: '18px' }}>{cameraError ? '摄像头异常' : '正在启动...'}</span>
            </div>
          )}

          {/* 状态浮层 - 右上角 */}
          {statusMessage && (
            <div style={{
              position: 'absolute',
              top: '16px',
              right: '16px',
              padding: '8px 16px',
              background: 'rgba(0,0,0,0.7)',
              backdropFilter: 'blur(8px)',
              borderRadius: '8px',
              color: '#fff',
              fontSize: '13px',
              border: '1px solid rgba(255,255,255,0.1)'
            }}>
              {statusMessage}
            </div>
          )}

          {/* 分辨率选择器 - 左上角 */}
          {config.capture_mode === "camera" && hasCamera && (
            <div style={{
              position: 'absolute',
              top: '16px',
              left: '16px',
              display: 'flex',
              gap: '8px'
            }}>
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
                style={{
                  padding: '8px 12px',
                  background: 'rgba(0,0,0,0.6)',
                  backdropFilter: 'blur(8px)',
                  border: '1px solid rgba(255,255,255,0.2)',
                  borderRadius: '8px',
                  color: '#fff',
                  fontSize: '13px',
                  cursor: 'pointer'
                }}
              >
                <option value="480p" style={{ background: '#1a1a2e' }}>480p</option>
                <option value="720p" style={{ background: '#1a1a2e' }}>720p</option>
                <option value="1080p" style={{ background: '#1a1a2e' }}>1080p</option>
              </select>
            </div>
          )}
        </div>
      </div>

      {/* 底部信息栏 */}
      <div style={{
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'center',
        padding: '16px 24px',
        background: 'linear-gradient(90deg, #1a1a2e 0%, #16213e 100%)',
        borderTop: '1px solid rgba(255,255,255,0.1)',
        color: 'white',
        flexShrink: 0
      }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '32px' }}>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '2px' }}>
            <span style={{ fontSize: '11px', color: '#888' }}>设备</span>
            <span style={{ fontSize: '14px', fontWeight: 500 }}>{config.device_name || '未命名设备'}</span>
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '2px' }}>
            <span style={{ fontSize: '11px', color: '#888' }}>截图间隔</span>
            <span style={{ fontSize: '14px', fontWeight: 500 }}>{config.interval} 秒</span>
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '2px' }}>
            <span style={{ fontSize: '11px', color: '#888' }}>今日截图</span>
            <span style={{ fontSize: '14px', fontWeight: 500, color: '#4ade80' }}>{stats.todayCount} 张</span>
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '2px' }}>
            <span style={{ fontSize: '11px', color: '#888' }}>最后截图</span>
            <span style={{ fontSize: '14px', fontWeight: 500 }}>{stats.lastCaptureTime || '--:--:--'}</span>
          </div>
        </div>

        <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
          <button
            onClick={() => switchCaptureMode(config.capture_mode === "camera" ? "screen" : "camera")}
            style={{
              padding: '10px 20px',
              background: 'rgba(255,255,255,0.1)',
              border: '1px solid rgba(255,255,255,0.2)',
              borderRadius: '8px',
              color: '#fff',
              fontSize: '14px',
              cursor: 'pointer',
              transition: 'all 0.2s',
              display: 'flex',
              alignItems: 'center',
              gap: '6px'
            }}
            onMouseEnter={(e) => e.currentTarget.style.background = 'rgba(255,255,255,0.15)'}
            onMouseLeave={(e) => e.currentTarget.style.background = 'rgba(255,255,255,0.1)'}
          >
            {config.capture_mode === "camera" ? '📷 切截图' : '🎥 切摄像头'}
          </button>
          <button
            onClick={captureFrame}
            style={{
              padding: '10px 20px',
              background: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
              border: 'none',
              borderRadius: '8px',
              color: '#fff',
              fontSize: '14px',
              cursor: 'pointer',
              transition: 'all 0.2s',
              display: 'flex',
              alignItems: 'center',
              gap: '6px'
            }}
            onMouseEnter={(e) => e.currentTarget.style.transform = 'translateY(-2px)'}
            onMouseLeave={(e) => e.currentTarget.style.transform = 'translateY(0)'}
          >
            📸 手动截图
          </button>
          <button
            onClick={() => setShowSettings(true)}
            style={{
              padding: '10px 20px',
              background: 'rgba(255,255,255,0.1)',
              border: '1px solid rgba(255,255,255,0.2)',
              borderRadius: '8px',
              color: '#fff',
              fontSize: '14px',
              cursor: 'pointer',
              transition: 'all 0.2s'
            }}
            onMouseEnter={(e) => e.currentTarget.style.background = 'rgba(255,255,255,0.15)'}
            onMouseLeave={(e) => e.currentTarget.style.background = 'rgba(255,255,255,0.1)'}
          >
            ⚙️ 设置
          </button>
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
              <label>API地址</label>
              <input
                type="text"
                value={config.api_url}
                onChange={(e) => setConfig({ ...config, api_url: e.target.value })}
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
              <label>租户名称</label>
              <input
                type="text"
                value={config.tenant_name}
                onChange={(e) => setConfig({ ...config, tenant_name: e.target.value })}
                placeholder="请输入租户名称"
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
