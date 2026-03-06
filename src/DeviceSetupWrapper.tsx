import { memo, useState, useCallback } from 'react';

interface Props {
  classList: Array<{ id: number; class_name: string }>;
  onRegister: (classId: number, deviceType: number, deviceName: string) => void;
}

export const DeviceSetupWrapper = memo(function DeviceSetupWrapper({ classList, onRegister }: Props) {
  const [selectedClassId, setSelectedClassId] = useState<number | null>(null);
  const [selectedDeviceType, setSelectedDeviceType] = useState<number>(1);
  const [customDeviceName, setCustomDeviceName] = useState<string>('');
  const [isSubmitting, setIsSubmitting] = useState(false);

  const handleSubmit = useCallback(async () => {
    if (!selectedClassId) {
      alert('请选择班级');
      return;
    }
    setIsSubmitting(true);
    // 根据设备类型生成设备名称
    let deviceName: string;
    if (selectedDeviceType === 3) {
      // 其他类型使用用户输入的名称
      deviceName = customDeviceName.trim() || '其他设备';
    } else if (selectedDeviceType === 1) {
      deviceName = '智能黑板';
    } else {
      deviceName = '智能多媒体设备';
    }
    try {
      await onRegister(selectedClassId, selectedDeviceType, deviceName);
    } catch (e) {
      setIsSubmitting(false);
    }
  }, [selectedClassId, selectedDeviceType, customDeviceName, onRegister]);

  return (
    <div style={{ minHeight: '100vh', background: '#f0f0f0', padding: '20px' }}>
      <div style={{ maxWidth: '400px', margin: '0 auto', background: 'white', padding: '30px', borderRadius: '10px' }}>
        <h2 style={{ textAlign: 'center', marginBottom: '20px' }}>设备设置</h2>
        
        <label style={{ display: 'block', marginBottom: '12px', fontWeight: 'bold' }}>选择班级</label>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: '8px', marginBottom: '20px' }}>
          {classList.map((cls) => (
            <button
              key={cls.id}
              type="button"
              onClick={() => setSelectedClassId(cls.id)}
              style={{
                padding: '10px 8px',
                fontSize: '14px',
                border: selectedClassId === cls.id ? '2px solid #667eea' : '1px solid #ccc',
                borderRadius: '4px',
                background: selectedClassId === cls.id ? '#e8eaf6' : 'white',
                color: selectedClassId === cls.id ? '#667eea' : '#333',
                cursor: 'pointer'
              }}
            >
              {cls.class_name}
            </button>
          ))}
        </div>

        <label style={{ display: 'block', marginBottom: '12px', fontWeight: 'bold' }}>设备类型</label>
        <div style={{ display: 'flex', gap: '10px', marginBottom: '20px' }}>
          {[
            { id: 1, name: '智能黑板' },
            { id: 2, name: '智能多媒体设备' },
            { id: 3, name: '其他' }
          ].map((type) => (
            <button
              key={type.id}
              type="button"
              onClick={() => setSelectedDeviceType(type.id)}
              style={{
                flex: 1,
                padding: '12px',
                fontSize: '14px',
                border: selectedDeviceType === type.id ? '2px solid #667eea' : '1px solid #ccc',
                borderRadius: '4px',
                background: selectedDeviceType === type.id ? '#e8eaf6' : 'white',
                color: selectedDeviceType === type.id ? '#667eea' : '#333',
                cursor: 'pointer'
              }}
            >
              {type.name}
            </button>
          ))}
        </div>

        {/* 自定义设备名称输入框 - 仅在选择了"其他"时显示 */}
        {selectedDeviceType === 3 && (
          <div style={{ marginBottom: '20px' }}>
            <label style={{ display: 'block', marginBottom: '8px', fontWeight: 'bold' }}>设备名称</label>
            <input
              type="text"
              value={customDeviceName}
              onChange={(e) => setCustomDeviceName(e.target.value)}
              placeholder="请输入设备名称"
              style={{
                width: '100%',
                padding: '12px',
                fontSize: '16px',
                border: '1px solid #ccc',
                borderRadius: '4px',
                boxSizing: 'border-box'
              }}
            />
          </div>
        )}

        <button
          onClick={handleSubmit}
          disabled={isSubmitting}
          style={{
            width: '100%',
            padding: '14px',
            background: isSubmitting ? '#999' : '#667eea',
            color: 'white',
            border: 'none',
            borderRadius: '5px',
            fontSize: '16px',
            marginTop: '20px',
            cursor: isSubmitting ? 'not-allowed' : 'pointer'
          }}
        >
          {isSubmitting ? '提交中...' : '完成设置'}
        </button>
      </div>
    </div>
  );
});
