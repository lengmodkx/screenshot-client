# 智能黑板视频流客户端

一款用于学校智能黑板/多媒体设备的实时视频流推送工具，支持远程查看教室实时画面。

## 功能特性

- **实时视频流**：以 2 帧/秒的频率推送视频帧到服务端，支持远程实时查看
- **定时截图**：每 5 分钟上传一次截图，供后台查看设备静态画面
- **软件使用监控**：实时监控并上报设备上运行的软件，支持启动/停止事件推送
- **设备自动注册**：首次登录后自动注册设备，支持智能黑板、电子大屏等设备类型
- **自动登录**：保存登录凭据，下次启动自动登录
- **心跳保活**：每 30 秒发送心跳，保持设备在线状态
- **网络自适应**：实时检测网络状态，自动处理断网重连
- **双模式采集**：
  - 摄像头模式：调用摄像头采集画面
  - 屏幕截图模式：截取屏幕画面
- **后台运行**：最小化到系统托盘，后台持续推送数据和监控软件

## 技术栈

- **Tauri 2.x** - 轻量级桌面应用框架
- **React 19** - 前端框架
- **TypeScript** - 类型安全
- **Rust** - 后端视频流推送和文件处理
- **Vite 7** - 构建工具
- **Tailwind CSS 4** - 样式框架

## 环境要求

- Node.js 20.19+ 或 22.12+
- Rust 1.70+
- Windows 10/11

## 开发环境搭建

```bash
# 1. 克隆仓库
git clone https://github.com/lengmodkx/screenshot-client.git
cd screenshot-client

# 2. 安装依赖
npm install

# 3. 运行开发模式
npm run tauri dev
```

## 构建发布

```bash
# 构建生产版本
npm run tauri build
```

构建产物位于 `src-tauri/target/release/` 目录：
- `screenshot-client.exe` - 直接运行的 exe 文件
- `bundle/nsis/` - NSIS 安装包
- `bundle/msi/` - MSI 安装包

## 配置说明

配置文件位于 `%APPDATA%/ScreenshotClient/config.json`

| 配置项 | 说明 | 默认值 |
|--------|------|--------|
| api_url | API 服务器地址 | http://172.16.10.11:48080 |
| account_username | 登录账号 | - |
| account_password | 登录密码 | - |
| tenant_name | 租户名称 | - |
| device_code | 设备编码（MAC 地址生成） | 自动生成 |
| device_name | 设备名称 | - |
| school_class_id | 班级 ID | - |
| device_id | 设备 ID（注册后返回） | - |
| is_registered | 是否已注册 | false |
| capture_mode | 采集模式（camera/screen） | camera |
| camera_resolution | 摄像头分辨率（480p/720p/1080p） | 1080p |

## API 接口

客户端实现了以下接口规范（详见 `docs/api-docs/rust-client-video-api.md`）：

### 1. 登录认证
```
POST /client/inspection/login
Content-Type: application/json

Body: { "tenantName": "租户名称", "username": "账号", "password": "密码" }
Response: { "code": 0, "data": { "accessToken": "xxx", "refreshToken": "xxx" } }
```

### 2. 设备注册
```
POST /client/inspection/register
Content-Type: application/x-www-form-urlencoded
Authorization: Bearer <token>

Body: deviceCode=DEV_xxx&deviceName=xxx&deviceType=2&ipAddress=xxx&classroomId=1&registerType=1
Response: { "code": 0, "data": { "id": 123, "deviceName": "xxx" } }
```

### 3. 推送视频帧（核心接口）
```
POST /client/inspection/video/push
Content-Type: multipart/form-data
Authorization: Bearer <token>

Body: deviceCode=DEV_xxx&data=<Base64编码的JPEG图片>
Response: { "code": 0, "data": true }

推送频率：2 帧/秒（每 500ms）
图片大小：≤100KB
分辨率：≤1280x720
```

### 4. 设备心跳
```
POST /client/inspection/heartbeat
Content-Type: application/x-www-form-urlencoded
Authorization: Bearer <token>

Body: deviceCode=DEV_xxx
Response: { "code": 0, "data": true }

发送频率：每 30 秒
```

### 5. 上传截图
```
POST /client/inspection/uploadScreenshot
Content-Type: multipart/form-data
Authorization: Bearer <token>

Body: deviceCode=DEV_xxx&file=<JPEG文件>
Response: { "code": 0, "data": "http://xxx/screenshot/xxx.jpg" }

上传频率：每 5 分钟
```

### 6. 实时推送软件使用事件
```
POST /client/software/usage/realtime
Content-Type: application/json
Authorization: Bearer <token>

Body: {
  "deviceCode": "DEV_xxx",
  "event": "started",
  "session": {
    "id": "uuid",
    "processName": "chrome.exe",
    "windowTitle": "Google",
    "exePath": "C:\\Program Files\\...",
    "startTime": 1709965845000,
    "deptId": 362,
    "classId": 9
  }
}
Response: { "code": 0, "data": true }

触发时机：软件启动或停止时实时推送
```

### 7. 批量上报软件使用记录
```
POST /client/software/usage/batch
Content-Type: application/json
Authorization: Bearer <token>

Body: {
  "deviceCode": "DEV_xxx",
  "sessions": [
    {
      "id": "uuid",
      "processName": "chrome.exe",
      "windowTitle": "Google",
      "exePath": "...",
      "startTime": 1709965845000,
      "deptId": 362,
      "classId": 9
    }
  ]
}
Response: { "code": 0, "data": true }

使用场景：启动时全量推送当前运行软件、离线后补传
```

## 系统端接口（供参考）

Web 前端/管理系统使用以下接口查看视频流：

### 获取视频流信息
```
GET /admin-api/erp/inspection/video/url?deviceCode=DEV_xxx
Authorization: Bearer <admin_token>

Response: {
  "code": 0,
  "data": {
    "deviceCode": "DEV_xxx",
    "streamUrl": "/admin-api/erp/inspection/video/stream/DEV_xxx",
    "status": 1
  }
}
```

### FLV 视频流播放
```
GET /admin-api/erp/inspection/video/stream/{deviceCode}

说明：使用 flv.js 播放器播放实时视频流
延迟：约 3-10 秒
```

## 使用说明

### 首次使用

1. 启动应用后进入登录页面
2. 输入租户名称、账号、密码
3. 登录成功后选择班级和设备类型
4. 完成设备注册，自动进入视频流推送页面

### 日常使用

1. 应用启动后自动登录
2. 自动开始推送视频流（2 帧/秒）
3. 每 5 分钟上传一次截图
4. 每 30 秒发送心跳保持在线

### 远程查看

1. 在管理系统中查看设备列表
2. 点击设备查看实时视频流
3. 或使用截图查看静态画面

## 视频流架构

```
┌─────────────────┐         ┌─────────────────┐         ┌─────────────────┐
│   Rust 客户端    │ ──────> │   服务端缓存     │ ──────> │   Web 前端      │
│  (教室设备)      │  推送    │  (内存/30秒)    │  播放    │  (管理系统)      │
│                 │  JPEG   │                 │  FLV流  │                 │
└─────────────────┘         └─────────────────┘         └─────────────────┘
       │                           │                           │
       │  POST /client/            │  GET /admin-api/          │  flv.js
       │  /inspection/video/push   │  /erp/inspection/         │  播放
       │                           │  video/stream/xxx         │
```

## 项目结构

```
screenshot-client/
├── src/                          # React 前端源码
│   ├── App.tsx                   # 主应用组件
│   ├── components/               # UI 组件
│   │   ├── Dashboard.tsx         # 仪表盘组件
│   │   ├── DeviceInfoCard.tsx    # 设备信息卡片
│   │   ├── SoftwareUsageList.tsx # 软件使用列表
│   │   └── VideoPreview.tsx      # 视频预览
│   ├── contexts/                 # React Context
│   └── index.css                 # 样式文件
├── src-tauri/                    # Rust 后端源码
│   ├── src/
│   │   ├── lib.rs                # 核心逻辑（视频推送、心跳、注册、软件监控）
│   │   ├── database.rs           # SQLite 数据持久化
│   │   ├── main.rs               # 入口文件
│   │   └── monitor/              # 软件监控模块
│   │       ├── mod.rs
│   │       ├── process_monitor.rs    # 进程监控
│   │       ├── session_manager.rs    # 会话管理
│   │       ├── sync_scheduler.rs     # 同步调度
│   │       └── windows_api.rs        # Windows API 封装
│   ├── Cargo.toml                # Rust 依赖
│   └── tauri.conf.json           # Tauri 配置
├── docs/                         # 设计文档
│   └── api-docs/
│       ├── rust-client-video-api.md      # 视频流 API 文档
│       └── 软件使用数据推送接口文档.md    # 软件监控 API 文档
└── README.md                     # 本文件
```

## 更新日志

### 2026-03-10

- **新增软件使用监控功能**
  - 实时监听软件启动和停止事件
  - 自动推送软件使用数据到服务端
  - 支持启动时全量推送当前运行软件
  - 本地 SQLite 数据库存储和断网补传
- 修复软件推送数据结构，确保 deptId/classId 正确传递
- 新增系统托盘后台运行模式
- 优化设备注册流程和错误提示

### 2026-03-05

- 新增实时视频流推送功能（2 帧/秒）
- 新增设备自动注册流程
- 新增自动登录功能
- 新增心跳保活机制
- 新增定时截图上传（每 5 分钟）
- 优化图片压缩（≤100KB）
- 支持摄像头和屏幕截图双模式

## 许可证

MIT License
