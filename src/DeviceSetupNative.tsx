
// 设备类型名称
const deviceTypeMap: Record<number, string> = {
  1: '智能黑板',
  2: '智能多媒体设备',
  3: '其他'
};

// 纯原生 DOM 实现，完全绕过 React 渲染机制
export function renderDeviceSetup(
  container: HTMLElement,
  classList: Array<{ id: number; class_name: string }>,
  onRegister: (classId: number, deviceType: number, deviceName: string) => void
) {
  // 清空容器
  container.innerHTML = '';
  
  // 状态存储在闭包中
  let selectedClassId: number | null = null;
  let selectedDeviceType = 1;
  let isSubmitting = false;

  // 创建容器
  const wrapper = document.createElement('div');
  wrapper.style.cssText = 'min-height:100vh;background:#f0f0f0;padding:20px;';
  
  const card = document.createElement('div');
  card.style.cssText = 'max-width:400px;margin:0 auto;background:white;padding:30px;border-radius:10px;';
  
  // 标题
  const title = document.createElement('h2');
  title.textContent = '设备设置';
  title.style.cssText = 'text-align:center;margin-bottom:20px;';
  card.appendChild(title);

  // 班级选择
  const classLabel = document.createElement('label');
  classLabel.textContent = '选择班级';
  classLabel.style.cssText = 'display:block;margin-bottom:12px;font-weight:bold;';
  card.appendChild(classLabel);

  const classGrid = document.createElement('div');
  classGrid.style.cssText = 'display:grid;grid-template-columns:repeat(3,1fr);gap:8px;margin-bottom:20px;';

  // 渲染班级按钮
  const classButtons: HTMLButtonElement[] = [];
  classList.forEach((cls) => {
    const btn = document.createElement('button');
    btn.textContent = cls.class_name;
    btn.style.cssText = 'padding:10px 8px;font-size:14px;border:1px solid #ccc;border-radius:4px;background:white;cursor:pointer;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;';
    btn.onclick = () => {
      selectedClassId = cls.id;
      // 更新所有按钮样式
      classButtons.forEach((b, i) => {
        if (classList[i].id === selectedClassId) {
          b.style.border = '2px solid #667eea';
          b.style.background = '#e8eaf6';
          b.style.color = '#667eea';
        } else {
          b.style.border = '1px solid #ccc';
          b.style.background = 'white';
          b.style.color = '#333';
        }
      });
    };
    classButtons.push(btn);
    classGrid.appendChild(btn);
  });
  card.appendChild(classGrid);

  // 设备类型选择
  const typeLabel = document.createElement('label');
  typeLabel.textContent = '设备类型';
  typeLabel.style.cssText = 'display:block;margin-bottom:12px;font-weight:bold;';
  card.appendChild(typeLabel);

  const typeGrid = document.createElement('div');
  typeGrid.style.cssText = 'display:flex;gap:10px;margin-bottom:20px;';

  const typeButtons: HTMLButtonElement[] = [];
  [1, 2, 3].forEach((type) => {
    const btn = document.createElement('button');
    btn.textContent = deviceTypeMap[type];
    btn.style.cssText = 'flex:1;padding:12px;font-size:14px;border:1px solid #ccc;border-radius:4px;background:white;cursor:pointer;';
    btn.onclick = () => {
      selectedDeviceType = type;
      typeButtons.forEach((b, i) => {
        if ([1, 2, 3][i] === selectedDeviceType) {
          b.style.border = '2px solid #667eea';
          b.style.background = '#e8eaf6';
          b.style.color = '#667eea';
        } else {
          b.style.border = '1px solid #ccc';
          b.style.background = 'white';
          b.style.color = '#333';
        }
      });
    };
    typeButtons.push(btn);
    typeGrid.appendChild(btn);
  });
  // 默认选中第一个
  if (typeButtons[0]) {
    typeButtons[0].style.border = '2px solid #667eea';
    typeButtons[0].style.background = '#e8eaf6';
    typeButtons[0].style.color = '#667eea';
  }
  card.appendChild(typeGrid);

  // 提交按钮
  const submitBtn = document.createElement('button');
  submitBtn.textContent = '完成设置';
  submitBtn.style.cssText = 'width:100%;padding:14px;background:#667eea;color:white;border:none;border-radius:5px;font-size:16px;margin-top:20px;cursor:pointer;';
  submitBtn.onclick = async () => {
    if (isSubmitting) return;
    if (!selectedClassId) {
      alert('请选择班级');
      return;
    }
    isSubmitting = true;
    submitBtn.textContent = '提交中...';
    submitBtn.style.background = '#999';
    
    const deviceName = selectedDeviceType === 3 
      ? '其他设备' 
      : (selectedDeviceType === 1 ? '智能黑板' : '智能多媒体设备');
    
    try {
      await onRegister(selectedClassId, selectedDeviceType, deviceName);
    } catch (e) {
      isSubmitting = false;
      submitBtn.textContent = '完成设置';
      submitBtn.style.background = '#667eea';
    }
  };
  card.appendChild(submitBtn);

  wrapper.appendChild(card);
  container.appendChild(wrapper);

  // 返回清理函数
  return () => {
    container.innerHTML = '';
  };
}
