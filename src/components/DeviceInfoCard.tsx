import { useState, useEffect } from 'react';
import { Monitor, Clock, Wifi, Settings } from 'lucide-react';

interface DeviceInfoCardProps {
  deviceName: string;
  isOnline?: boolean;
  onSettingsClick?: () => void;
}

export function DeviceInfoCard({ deviceName, isOnline = true, onSettingsClick }: DeviceInfoCardProps) {
  const [uptime, setUptime] = useState('计算中...');

  useEffect(() => {
    // 获取系统启动时间（这里使用页面加载时间作为近似）
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
    const interval = setInterval(updateUptime, 60000); // 每分钟更新

    return () => clearInterval(interval);
  }, []);

  return (
    <div className="bg-slate-800/50 backdrop-blur-xl rounded-2xl border border-slate-700/50 shadow-xl p-6">
      <div className="flex items-center gap-2 mb-5">
        <Monitor className="w-5 h-5 text-blue-400" />
        <h3 className="text-lg font-semibold text-slate-100">设备信息</h3>
      </div>

      <div className="space-y-5">
        {/* 设备名称 */}
        <div>
          <label className="text-xs font-medium text-slate-400 uppercase tracking-wider mb-2 block">
            设备名称
          </label>
          <div className="flex items-center gap-3 px-4 py-3 bg-slate-900/60 rounded-xl border border-slate-700/50">
            <Monitor className="w-5 h-5 text-blue-400" />
            <span className="text-slate-100 font-medium">{deviceName || '未命名设备'}</span>
          </div>
        </div>

        {/* 开机时间 */}
        <div>
          <label className="text-xs font-medium text-slate-400 uppercase tracking-wider mb-2 block">
            运行时长
          </label>
          <div className="flex items-center gap-3 px-4 py-3 bg-slate-900/60 rounded-xl border border-slate-700/50">
            <Clock className="w-5 h-5 text-amber-400" />
            <span className="text-slate-100 font-medium">{uptime}</span>
          </div>
        </div>

        {/* 设备状态 */}
        <div>
          <label className="text-xs font-medium text-slate-400 uppercase tracking-wider mb-2 block">
            设备状态
          </label>
          <div className="flex items-center gap-3 px-4 py-3 bg-slate-900/60 rounded-xl border border-slate-700/50">
            <Wifi className={`w-5 h-5 ${isOnline ? 'text-emerald-400' : 'text-rose-400'}`} />
            <div className="flex items-center gap-2">
              <span className={`w-2.5 h-2.5 rounded-full ${isOnline ? 'bg-emerald-400 animate-pulse' : 'bg-rose-400'}`} />
              <span className="text-slate-100 font-medium">{isOnline ? '在线' : '离线'}</span>
            </div>
          </div>
        </div>

        {/* 设置按钮 */}
        <button
          onClick={onSettingsClick}
          className="w-full flex items-center justify-center gap-2 px-4 py-3 bg-blue-500/20 hover:bg-blue-500/30
                     text-blue-400 rounded-xl border border-blue-500/30 transition-all duration-200
                     hover:shadow-lg hover:shadow-blue-500/10"
        >
          <Settings className="w-5 h-5" />
          <span className="font-medium">设置</span>
        </button>
      </div>
    </div>
  );
}
