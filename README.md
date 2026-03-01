# 智能黑板截图客户端

一款用于学校智能黑板/多媒体设备的定时截图工具，支持云端上传和本地保存两种模式。

## 功能特性

- **定时截图**：按设定的时间间隔自动截取屏幕（默认10秒）
- **双模式存储**：
  - 云端上传：登录后自动上传到管理平台
  - 本地保存：离线时自动保存到本地磁盘
- **自动清理**：本地保存模式下自动清理7天前的旧截图
- **网络检测**：实时检测网络状态，自动切换上传/保存模式
- **配置灵活**：可自定义截图间隔、存储路径、API地址等

## 技术栈

- **Tauri 2.x** - 轻量级桌面应用框架
- **React** - 前端框架
- **TypeScript** - 类型安全
- **Rust** - 后端截屏和文件处理

## 环境要求

- Node.js 18+
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
- `screenshot-client.exe` - 直接运行的exe文件
- `bundle/nsis/` - NSIS安装包
- `bundle/msi/` - MSI安装包

## 配置说明

配置文件位于 `%APPDATA%/ScreenshotClient/config.json`

| 配置项 | 说明 | 默认值 |
|--------|------|--------|
| interval | 截图间隔（秒） | 10 |
| mode | 存储模式（cloud/local） | local |
| local_path | 本地保存路径 | 图片/Screenshots |
| api_url | API服务器地址 | http://localhost:3000 |
| retention_days | 本地保留天数 | 7 |

## API接口

客户端需要管理后台提供以下接口：

### 登录
```
POST /api/login
Body: { username, password }
Response: { token }
```

### 上传截图
```
POST /api/screenshot/upload
Header: Authorization: Bearer <token>
Body: (multipart) image file
```

### 健康检查（可选）
```
GET /api/health
```

## 使用说明

1. 首次运行默认使用本地保存模式
2. 点击"开始"按钮启动定时截图
3. 如需云端上传：
   - 在设置中切换到"云端上传"模式
   - 点击"登录"输入账号密码
4. 截图默认保存在 `图片/Screenshots` 文件夹

## 项目结构

```
screenshot-client/
├── src/                    # React前端源码
│   ├── App.tsx            # 主应用组件
│   └── App.css           # 样式文件
├── src-tauri/             # Rust后端源码
│   ├── src/
│   │   ├── lib.rs        # 核心逻辑
│   │   └── main.rs       # 入口文件
│   ├── Cargo.toml        # Rust依赖
│   └── tauri.conf.json   # Tauri配置
└── docs/                  # 设计文档
```

## 许可证

MIT License
