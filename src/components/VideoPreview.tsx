import { useEffect, useRef, useState } from 'react';
import { Camera, Monitor, RefreshCw } from 'lucide-react';

interface VideoPreviewProps {
  hasCamera: boolean;
  actualCaptureMode: 'camera' | 'screen';
  currentImage?: string | null;
  onToggleMode?: () => void;
}

export function VideoPreview({ hasCamera, actualCaptureMode, currentImage, onToggleMode }: VideoPreviewProps) {
  const videoRef = useRef<HTMLVideoElement>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const streamRef = useRef<MediaStream | null>(null);

  // 调试日志
  useEffect(() => {
    console.log('[VideoPreview] currentImage:', currentImage ? `length=${currentImage.length}` : 'null');
    console.log('[VideoPreview] actualCaptureMode:', actualCaptureMode);
  }, [currentImage, actualCaptureMode]);

  useEffect(() => {
    if (actualCaptureMode !== 'camera') {
      setIsLoading(false);
      setError(null);
      return;
    }

    // 摄像头模式：初始化预览
    const initPreview = async () => {
      try {
        setIsLoading(true);
        setError(null);

        // 检查是否有视频设备
        const devices = await navigator.mediaDevices.enumerateDevices();
        const videoDevices = devices.filter(d => d.kind === 'videoinput');

        if (videoDevices.length === 0) {
          setError('未检测到摄像头设备');
          setIsLoading(false);
          return;
        }

        // 获取摄像头流
        const stream = await navigator.mediaDevices.getUserMedia({
          video: {
            width: { ideal: 1280 },
            height: { ideal: 720 },
            facingMode: 'user'
          },
          audio: false
        });

        streamRef.current = stream;

        if (videoRef.current) {
          videoRef.current.srcObject = stream;
          await videoRef.current.play();
        }

        setIsLoading(false);
      } catch (err) {
        console.error('Camera preview error:', err);
        const errorMsg = err instanceof Error ? err.message : String(err);
        if (errorMsg.includes('Permission denied') || errorMsg.includes('NotAllowed')) {
          setError('需要摄像头权限');
        } else {
          setError('摄像头启动失败');
        }
        setIsLoading(false);
      }
    };

    // 延迟初始化，给主组件的捕获流让路
    const timer = setTimeout(() => {
      initPreview();
    }, 100);

    return () => {
      clearTimeout(timer);
      if (streamRef.current) {
        streamRef.current.getTracks().forEach(track => track.stop());
        streamRef.current = null;
      }
    };
  }, [actualCaptureMode]);

  const isCameraMode = actualCaptureMode === 'camera' && hasCamera && !error;

  return (
    <div className="relative w-full h-full bg-slate-900 rounded-2xl overflow-hidden border border-slate-700/50 shadow-2xl">
      {/* Header */}
      <div className="absolute top-0 left-0 right-0 z-10 flex items-center justify-between px-4 py-3 bg-gradient-to-b from-black/60 to-transparent">
        <div className="flex items-center gap-2 text-white/90">
          {isCameraMode ? (
            <>
              <Camera className="w-4 h-4" />
              <span className="text-sm font-medium">摄像头画面</span>
            </>
          ) : (
            <>
              <Monitor className="w-4 h-4" />
              <span className="text-sm font-medium">屏幕画面</span>
            </>
          )}
        </div>

        {onToggleMode && (
          <button
            onClick={onToggleMode}
            className="p-2 rounded-lg bg-white/10 hover:bg-white/20 text-white/80 transition-all duration-200"
            title="切换预览模式"
          >
            <RefreshCw className="w-4 h-4" />
          </button>
        )}
      </div>

      {/* Video/Screen Content */}
      <div className="relative w-full h-full flex items-center justify-center">
        {isCameraMode ? (
          <video
            ref={videoRef}
            autoPlay
            playsInline
            muted
            className="w-full h-full object-cover"
          />
        ) : currentImage ? (
          <img
            src={currentImage}
            alt="屏幕截图"
            className="w-full h-full object-contain"
          />
        ) : (
          <div className="flex flex-col items-center justify-center text-slate-500">
            <Monitor className="w-16 h-16 mb-4 opacity-50" />
            <p className="text-sm">{hasCamera ? '屏幕预览模式' : '摄像头不可用，显示屏幕画面'}</p>
            <p className="text-xs mt-2 opacity-60">正在获取屏幕截图...</p>
          </div>
        )}

        {isLoading && (
          <div className="absolute inset-0 flex items-center justify-center bg-slate-900/80">
            <div className="flex items-center gap-3 text-white/70">
              <div className="w-5 h-5 border-2 border-white/30 border-t-white rounded-full animate-spin" />
              <span className="text-sm">正在初始化摄像头...</span>
            </div>
          </div>
        )}

        {error && (
          <div className="absolute inset-0 flex items-center justify-center bg-slate-900/90">
            <div className="text-center">
              <Camera className="w-12 h-12 mx-auto mb-3 text-slate-600" />
              <p className="text-white/70 text-sm">{error}</p>
              <p className="text-white/40 text-xs mt-2">已自动切换到屏幕预览模式</p>
            </div>
          </div>
        )}
      </div>

      {/* Bottom Status Bar */}
      <div className="absolute bottom-0 left-0 right-0 px-4 py-2 bg-gradient-to-t from-black/60 to-transparent">
        <div className="flex items-center justify-between text-white/70 text-xs">
          <div className="flex items-center gap-2">
            <span className="flex items-center gap-1.5">
              <span className={`w-2 h-2 rounded-full ${isCameraMode ? 'bg-emerald-400 animate-pulse' : 'bg-amber-400'}`} />
              {isCameraMode ? '摄像头预览中' : '屏幕预览模式'}
            </span>
          </div>
          <span>{isCameraMode ? '实时视频流' : '自动切换'}</span>
        </div>
      </div>
    </div>
  );
}
