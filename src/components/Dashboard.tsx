import { useState, useEffect, useRef } from 'react';
import { VideoPreview } from './VideoPreview';
import { SoftwareUsageList } from './SoftwareUsageList';
import { ThemeToggle } from './ThemeToggle';
import { invoke } from '@tauri-apps/api/core';
import {
  Camera,
  Settings,
  LogOut,
  RefreshCw,
  Monitor,
  Clock,
  ChevronRight,
  HardDrive,
  Activity,
  Check
} from 'lucide-react';

interface DashboardProps {
  config: {
    device_name: string;
    capture_mode: string;
    camera_resolution: string;
    interval: number;
    account_username?: string;
    dept_name?: string;
    class_name?: string;
    school_class_id?: number | null;
    device_code?: string;
    device_type?: string;
    is_registered: boolean;
    api_url: string;
  };
  hasCamera: boolean;
  actualCaptureMode: 'camera' | 'screen';
  currentImage: string | null;
  onToggleMode: () => void;
  onSettingsClick: () => void;
  onLogout: () => void;
  onResolutionChange?: (resolution: string) => void;
}

interface SoftwareUsage {
  id: string;
  processName: string;
  windowTitle: string;
  isActive: boolean;
  durationSecs: number;
  lastActiveTime: string;
}

export function Dashboard({
  config,
  hasCamera,
  actualCaptureMode,
  currentImage,
  onToggleMode,
  onSettingsClick,
  onLogout,
  onResolutionChange,
}: DashboardProps) {
  const [showResolutionMenu, setShowResolutionMenu] = useState(false);
  const [softwareUsages, setSoftwareUsages] = useState<SoftwareUsage[]>([]);
  const [uptime, setUptime] = useState('计算中...');
  const [currentTime, setCurrentTime] = useState(new Date());

  // 调试日志
  useEffect(() => {
    console.log('[Dashboard] currentImage changed:', currentImage ? `length=${currentImage.length}` : 'null');
  }, [currentImage]);

  useEffect(() => {
    console.log('[Dashboard] actualCaptureMode:', actualCaptureMode);
  }, [actualCaptureMode]);

  // 更新当前时间
  useEffect(() => {
    const timer = setInterval(() => setCurrentTime(new Date()), 1000);
    return () => clearInterval(timer);
  }, []);

  // 计算运行时长
  useEffect(() => {
    const bootTime = performance.timing?.navigationStart || Date.now();

    const updateUptime = () => {
      const now = Date.now();
      const diff = now - bootTime;
      const days = Math.floor(diff / (1000 * 60 * 60 * 24));
      const hours = Math.floor((diff % (1000 * 60 * 60 * 24)) / (1000 * 60 * 60));
      const minutes = Math.floor((diff % (1000 * 60 * 60)) / (1000 * 60));

      if (days > 0) {
        setUptime(`${days}天${hours}小时${minutes}分钟`);
      } else if (hours > 0) {
        setUptime(`${hours}小时${minutes}分钟`);
      } else {
        setUptime(`${minutes}分钟`);
      }
    };

    updateUptime();
    const interval = setInterval(updateUptime, 60000);
    return () => clearInterval(interval);
  }, []);

  // 屏幕截图现在由 App.tsx 中的 captureAndPushFrame 处理
  // 这里只接收 currentImage prop 并传递给 VideoPreview
  useEffect(() => {
    setCurrentImageProp(currentImage);
  }, [currentImage]);

  const [currentImageProp, setCurrentImageProp] = useState<string | null>(currentImage);

  // 定期刷新软件使用数据 - 改为60秒一次，减少频繁请求
  const lastFetchTime = useRef<number>(0);
  const isFetching = useRef<boolean>(false);

  useEffect(() => {
    const fetchSoftwareUsages = async () => {
      // 防止重复请求
      if (isFetching.current) return;

      const now = Date.now();
      // 节流：至少间隔60秒
      if (now - lastFetchTime.current < 60000) return;

      isFetching.current = true;
      try {
        const usages = await invoke<SoftwareUsage[]>('get_software_usages');
        setSoftwareUsages(usages || []);
        lastFetchTime.current = Date.now();
      } catch (e) {
        console.error('获取软件使用数据失败:', e);
      } finally {
        isFetching.current = false;
      }
    };

    // 只在页面可见时获取
    const handleVisibilityChange = () => {
      if (document.visibilityState === 'visible') {
        fetchSoftwareUsages();
      }
    };

    fetchSoftwareUsages();
    const interval = setInterval(fetchSoftwareUsages, 60000);
    document.addEventListener('visibilitychange', handleVisibilityChange);

    return () => {
      clearInterval(interval);
      document.removeEventListener('visibilitychange', handleVisibilityChange);
    };
  }, []);

  const handleRefresh = async () => {
    try {
      const usages = await invoke<SoftwareUsage[]>('get_software_usages');
      setSoftwareUsages(usages || []);
      lastFetchTime.current = Date.now();
    } catch (e) {
      console.error('刷新失败:', e);
    }
  };

  const isOnline = true;

  const resolutions = [
    { value: '480p', label: '480p', desc: '640x360' },
    { value: '720p', label: '720p', desc: '1280x720' },
    { value: '1080p', label: '1080p', desc: '1920x1080' },
  ];

  const handleResolutionChange = async (resolution: string) => {
    if (onResolutionChange) {
      await onResolutionChange(resolution);
    }
    setShowResolutionMenu(false);
  };

  return (
    <div className="h-screen bg-slate-950 flex overflow-hidden">
      {/* 左侧边栏 - 设备信息 */}
      <aside className="w-72 h-full bg-slate-900/80 backdrop-blur-xl border-r border-slate-800 flex flex-col">
        {/* Logo & Title */}
        <div className="p-6 border-b border-slate-800">
          <div className="flex items-center gap-3">
            <div className="w-12 h-12 rounded-xl bg-gradient-to-br from-blue-500 to-purple-600 flex items-center justify-center shadow-lg shadow-blue-500/20">
              <Camera className="w-6 h-6 text-white" />
            </div>
            <div>
              <h1 className="text-lg font-bold text-slate-100">设备监控</h1>
              <p className="text-xs text-slate-400">智能黑板系统</p>
            </div>
          </div>
        </div>

        {/* 设备信息卡片 */}
        <div className="flex-1 p-6 space-y-6 overflow-y-auto min-h-0">
          {/* 设备状态概览 */}
          <div className="bg-gradient-to-br from-blue-500/10 to-purple-500/10 rounded-2xl p-5 border border-blue-500/20">
            <div className="flex items-center gap-2 mb-4">
              <div className={`w-3 h-3 rounded-full ${isOnline ? 'bg-emerald-400 animate-pulse' : 'bg-rose-400'}`} />
              <span className="text-sm font-medium text-slate-300">
                {isOnline ? '设备在线' : '设备离线'}
              </span>
            </div>
            <h3 className="text-xl font-bold text-slate-100 mb-1">
              {config.device_name || '未命名设备'}
            </h3>
            <p className="text-sm text-slate-400">
              {config.dept_name || '未设置部门'}
            </p>
            {config.class_name && (
              <p className="text-sm text-slate-400 mt-1">
                <span className="text-slate-500">班级:</span> {config.class_name}
              </p>
            )}
          </div>

          {/* 信息列表 */}
          <div className="space-y-4">
            {/* 运行时长 */}
            <div className="group">
              <label className="text-xs font-medium text-slate-500 uppercase tracking-wider mb-2 block">
                运行时长
              </label>
              <div className="flex items-center gap-3 p-3 bg-slate-800/50 rounded-xl border border-slate-700/50 group-hover:border-slate-600/50 transition-colors">
                <div className="w-10 h-10 rounded-lg bg-amber-500/10 flex items-center justify-center">
                  <Clock className="w-5 h-5 text-amber-400" />
                </div>
                <div>
                  <p className="text-slate-100 font-medium">{uptime}</p>
                  <p className="text-xs text-slate-500">自开机以来</p>
                </div>
              </div>
            </div>

            {/* 当前模式 */}
            <div className="group">
              <label className="text-xs font-medium text-slate-500 uppercase tracking-wider mb-2 block">
                采集模式
              </label>
              <div className="flex items-center gap-3 p-3 bg-slate-800/50 rounded-xl border border-slate-700/50 group-hover:border-slate-600/50 transition-colors">
                <div className="w-10 h-10 rounded-lg bg-blue-500/10 flex items-center justify-center">
                  {actualCaptureMode === 'camera' ? (
                    <Camera className="w-5 h-5 text-blue-400" />
                  ) : (
                    <Monitor className="w-5 h-5 text-blue-400" />
                  )}
                </div>
                <div className="flex-1">
                  <p className="text-slate-100 font-medium">
                    {actualCaptureMode === 'camera' ? '摄像头模式' : '屏幕截图模式'}
                  </p>
                  <p className="text-xs text-slate-500">
                    {hasCamera ? '摄像头可用' : '使用屏幕截图'}
                  </p>
                </div>
                <button
                  onClick={onToggleMode}
                  className="p-2 rounded-lg hover:bg-slate-700/50 text-slate-400 hover:text-slate-200 transition-colors"
                  title="切换模式"
                >
                  <RefreshCw className="w-4 h-4" />
                </button>
              </div>
            </div>

            {/* 分辨率 */}
            <div className="group relative">
              <label className="text-xs font-medium text-slate-500 uppercase tracking-wider mb-2 block">
                分辨率设置
              </label>
              <button
                onClick={() => setShowResolutionMenu(!showResolutionMenu)}
                className="w-full flex items-center gap-3 p-3 bg-slate-800/50 rounded-xl border border-slate-700/50 hover:border-slate-600/50 transition-colors text-left"
              >
                <div className="w-10 h-10 rounded-lg bg-purple-500/10 flex items-center justify-center">
                  <Activity className="w-5 h-5 text-purple-400" />
                </div>
                <div className="flex-1">
                  <p className="text-slate-100 font-medium">{config.camera_resolution || '1080p'}</p>
                  <p className="text-xs text-slate-500">点击切换分辨率</p>
                </div>
                <ChevronRight className={`w-4 h-4 text-slate-500 transition-transform ${showResolutionMenu ? 'rotate-90' : ''}`} />
              </button>

              {/* 分辨率下拉菜单 */}
              {showResolutionMenu && (
                <div className="absolute z-50 w-full mt-2 bg-slate-800 rounded-xl border border-slate-700 shadow-xl overflow-hidden">
                  {resolutions.map((res) => (
                    <button
                      key={res.value}
                      onClick={() => handleResolutionChange(res.value)}
                      className={`w-full flex items-center justify-between px-4 py-3 text-left hover:bg-slate-700/50 transition-colors ${
                        config.camera_resolution === res.value ? 'bg-purple-500/10' : ''
                      }`}
                    >
                      <div>
                        <p className={`font-medium ${config.camera_resolution === res.value ? 'text-purple-400' : 'text-slate-200'}`}>
                          {res.label}
                        </p>
                        <p className="text-xs text-slate-500">{res.desc}</p>
                      </div>
                      {config.camera_resolution === res.value && (
                        <Check className="w-4 h-4 text-purple-400" />
                      )}
                    </button>
                  ))}
                </div>
              )}
            </div>

            {/* 存储位置 */}
            <div className="group">
              <label className="text-xs font-medium text-slate-500 uppercase tracking-wider mb-2 block">
                存储位置
              </label>
              <div className="flex items-center gap-3 p-3 bg-slate-800/50 rounded-xl border border-slate-700/50">
                <div className="w-10 h-10 rounded-lg bg-emerald-500/10 flex items-center justify-center">
                  <HardDrive className="w-5 h-5 text-emerald-400" />
                </div>
                <div>
                  <p className="text-slate-100 font-medium">本地存储</p>
                  <p className="text-xs text-slate-500">已启用自动备份</p>
                </div>
              </div>
            </div>
          </div>
        </div>

        {/* 底部操作区 */}
        <div className="p-6 border-t border-slate-800 space-y-3">
          <button
            onClick={onSettingsClick}
            className="w-full flex items-center justify-between px-4 py-3 bg-blue-500/10 hover:bg-blue-500/20
                       text-blue-400 rounded-xl border border-blue-500/20 transition-all duration-200
                       hover:shadow-lg hover:shadow-blue-500/10"
          >
            <div className="flex items-center gap-3">
              <Settings className="w-5 h-5" />
              <span className="font-medium">设置</span>
            </div>
            <ChevronRight className="w-4 h-4" />
          </button>

          <button
            onClick={onLogout}
            className="w-full flex items-center justify-between px-4 py-3 bg-rose-500/10 hover:bg-rose-500/20
                       text-rose-400 rounded-xl border border-rose-500/20 transition-all duration-200"
          >
            <div className="flex items-center gap-3">
              <LogOut className="w-5 h-5" />
              <span className="font-medium">退出登录</span>
            </div>
            <ChevronRight className="w-4 h-4" />
          </button>

          <div className="flex items-center justify-between pt-2">
            <span className="text-xs text-slate-500">
              {currentTime.toLocaleTimeString()}
            </span>
            <ThemeToggle />
          </div>
        </div>
      </aside>

      {/* 右侧主内容区 */}
      <main className="flex-1 flex flex-col min-w-0 h-full">
        {/* 顶部标题栏 */}
        <header className="h-16 bg-slate-900/50 backdrop-blur-xl border-b border-slate-800 flex items-center justify-between px-6">
          <div className="flex items-center gap-4">
            <h2 className="text-lg font-semibold text-slate-100">实时监控</h2>
            <span className="text-sm text-slate-500">
              {config.account_username}
            </span>
          </div>

          <div className="flex items-center gap-3">
            <button
              onClick={handleRefresh}
              className="flex items-center gap-2 px-4 py-2 rounded-lg bg-slate-800/50 hover:bg-slate-700/50
                         text-slate-300 hover:text-slate-100 border border-slate-700/50 transition-all duration-200"
            >
              <RefreshCw className="w-4 h-4" />
              <span className="text-sm font-medium">刷新软件列表</span>
            </button>
          </div>
        </header>

        {/* 主内容 */}
        <div className="flex-1 p-6 overflow-auto">
          <div className="max-w-6xl mx-auto space-y-6">
            {/* 视频预览区域 - 大尺寸 */}
            <div className="relative rounded-2xl overflow-hidden border border-slate-700/50 shadow-2xl bg-slate-900"
                 style={{ height: '480px' }}>
              <VideoPreview
                hasCamera={hasCamera}
                actualCaptureMode={actualCaptureMode}
                currentImage={currentImageProp}
                onToggleMode={onToggleMode}
              />
            </div>

            {/* 软件使用列表 */}
            <SoftwareUsageList usages={softwareUsages} defaultShowCount={6} />
          </div>
        </div>
      </main>
    </div>
  );
}
