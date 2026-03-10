import { useState, useMemo, useCallback } from 'react';
import { Smartphone, ChevronDown, ChevronUp, Clock } from 'lucide-react';

interface SoftwareUsage {
  id: string;
  processName: string;
  windowTitle: string;
  isActive: boolean;
  durationSecs: number;
  lastActiveTime: string;
}

interface SoftwareUsageListProps {
  usages: SoftwareUsage[];
  defaultShowCount?: number;
}

// 软件图标映射
const softwareIcons: Record<string, string> = {
  'chrome.exe': '🌐',
  'firefox.exe': '🦊',
  'edge.exe': '🌊',
  'wps.exe': '📝',
  'word.exe': '📄',
  'excel.exe': '📊',
  'powerpnt.exe': '📽️',
  'wechat.exe': '💬',
  'qq.exe': '🐧',
  'dingtalk.exe': '🔔',
  'default': '📱'
};

function getSoftwareIcon(processName: string): string {
  const lowerName = processName.toLowerCase();
  return softwareIcons[lowerName] || softwareIcons['default'];
}

function formatDuration(seconds: number): string {
  if (seconds < 60) return `${seconds}秒`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}分钟`;
  const hours = Math.floor(seconds / 3600);
  const mins = Math.floor((seconds % 3600) / 60);
  return mins > 0 ? `${hours}小时${mins}分钟` : `${hours}小时`;
}

function formatTimeAgo(timeStr: string): string {
  // 处理空值或无效值
  if (!timeStr || timeStr === 'null' || timeStr === 'undefined') {
    return '未知';
  }

  const date = new Date(timeStr);

  // 检查是否为有效日期
  if (isNaN(date.getTime())) {
    return '未知';
  }

  const now = new Date();
  const diff = Math.floor((now.getTime() - date.getTime()) / 1000);

  // 处理未来时间或无效差值
  if (diff < 0) return '刚刚';
  if (isNaN(diff)) return '未知';

  if (diff < 60) return '刚刚';
  if (diff < 3600) return `${Math.floor(diff / 60)}分钟前`;
  if (diff < 86400) return `${Math.floor(diff / 3600)}小时前`;
  return `${Math.floor(diff / 86400)}天前`;
}

export function SoftwareUsageList({ usages, defaultShowCount = 6 }: SoftwareUsageListProps) {
  const [isExpanded, setIsExpanded] = useState(false);

  // 使用 useMemo 缓存数据，避免重复计算
  // 限制最大显示数量，防止展开过多导致卡顿
  const MAX_EXPANDED_COUNT = 20;

  const { displayedUsages, hasMore, totalCount } = useMemo(() => {
    const hasMoreItems = usages.length > defaultShowCount;
    const displayed = isExpanded
      ? usages.slice(0, MAX_EXPANDED_COUNT)
      : usages.slice(0, defaultShowCount);
    return {
      displayedUsages: displayed,
      hasMore: hasMoreItems || usages.length > MAX_EXPANDED_COUNT,
      totalCount: usages.length
    };
  }, [usages, defaultShowCount, isExpanded]);

  // 使用 useCallback 避免重复创建函数
  const toggleExpanded = useCallback(() => {
    setIsExpanded(prev => !prev);
  }, []);

  const expandOnly = useCallback(() => {
    setIsExpanded(true);
  }, []);

  return (
    <div className="bg-slate-800/50 backdrop-blur-xl rounded-2xl border border-slate-700/50 shadow-xl overflow-hidden">
      {/* Header */}
      <div className="flex items-center justify-between px-6 py-4 border-b border-slate-700/50">
        <div className="flex items-center gap-2">
          <Smartphone className="w-5 h-5 text-purple-400" />
          <h3 className="text-lg font-semibold text-slate-100">
            当前运行软件
            <span className="ml-2 text-sm font-normal text-slate-400">({totalCount})</span>
          </h3>
        </div>
        {hasMore && (
          <button
            onClick={toggleExpanded}
            className="flex items-center gap-1 px-3 py-1.5 text-sm text-slate-400 hover:text-slate-200
                       bg-slate-700/50 hover:bg-slate-700 rounded-lg transition-all duration-200"
            type="button"
          >
            {isExpanded ? (
              <>
                <span>收起</span>
                <ChevronUp className="w-4 h-4" />
              </>
            ) : (
              <>
                <span>展开</span>
                <ChevronDown className="w-4 h-4" />
              </>
            )}
          </button>
        )}
      </div>

      {/* Software List */}
      <div className="divide-y divide-slate-700/30 max-h-[320px] overflow-y-auto scrollbar-thin scrollbar-thumb-slate-600 scrollbar-track-transparent">
        {displayedUsages.length === 0 ? (
          <div className="px-6 py-8 text-center text-slate-500">
            <Smartphone className="w-12 h-12 mx-auto mb-3 opacity-30" />
            <p className="text-sm">暂无软件使用记录</p>
          </div>
        ) : (
          displayedUsages.map((usage, index) => (
            <div
              key={`${usage.id}-${index}`}
              className={`flex items-center gap-4 px-6 py-4 ${
                usage.isActive ? 'bg-emerald-500/5' : 'hover:bg-slate-700/20'
              }`}
              style={{ contentVisibility: 'auto' }}
            >
              {/* Status Indicator */}
              <div className="relative flex-shrink-0">
                <span className="text-2xl">{getSoftwareIcon(usage.processName)}</span>
                {usage.isActive && (
                  <span className="absolute -bottom-0.5 -right-0.5 w-3 h-3 bg-emerald-400 rounded-full border-2 border-slate-800" />
                )}
              </div>

              {/* Software Info */}
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                  <span className="font-medium text-slate-100 truncate">
                    {usage.processName.replace('.exe', '')}
                  </span>
                  {usage.isActive && (
                    <span className="flex-shrink-0 px-2 py-0.5 text-xs font-medium text-emerald-400
                                   bg-emerald-400/10 rounded-full border border-emerald-400/20">
                      使用中
                    </span>
                  )}
                </div>
                <p className="text-sm text-slate-400 truncate mt-0.5">
                  {usage.windowTitle || '无窗口标题'}
                </p>
              </div>

              {/* Duration */}
              <div className="flex-shrink-0 flex items-center gap-1.5 text-sm text-slate-400">
                <Clock className="w-3.5 h-3.5" />
                {usage.isActive ? (
                  <span className="text-emerald-400">{formatDuration(usage.durationSecs)}</span>
                ) : (
                  <span>{formatTimeAgo(usage.lastActiveTime)}</span>
                )}
              </div>
            </div>
          ))
        )}
      </div>

      {/* Footer - Show More / Scroll Hint */}
      {isExpanded && usages.length > 6 && (
        <div className="px-6 py-2 text-center text-xs text-slate-500 border-t border-slate-700/30">
          已显示 {Math.min(usages.length, MAX_EXPANDED_COUNT)} 个软件，可滚动查看
        </div>
      )}
      {hasMore && !isExpanded && (
        <button
          onClick={expandOnly}
          className="w-full flex items-center justify-center gap-2 px-6 py-3 text-sm text-slate-400
                     hover:text-slate-200 hover:bg-slate-700/30 transition-all duration-200
                     border-t border-slate-700/30"
          type="button"
        >
          <span>展开查看更多</span>
          <ChevronDown className="w-4 h-4" />
        </button>
      )}
    </div>
  );
}
