import { useEffect, useRef } from 'react';
import { renderDeviceSetup } from './DeviceSetupNative';

interface Props {
  classList: Array<{ id: number; class_name: string }>;
  onRegister: (classId: number, deviceType: number, deviceName: string) => void;
}

// 这个组件只渲染一次，使用原生DOM处理所有交互
export function NativeDeviceSetup({ classList, onRegister }: Props) {
  const containerRef = useRef<HTMLDivElement>(null);
  const renderedRef = useRef(false);

  useEffect(() => {
    if (containerRef.current && !renderedRef.current) {
      renderedRef.current = true;
      renderDeviceSetup(containerRef.current, classList, onRegister);
    }
  }, []); // 只在挂载时执行一次

  return (
    <div 
      ref={containerRef}
      style={{ width: '100%', height: '100vh' }}
    />
  );
}
