# 智能黑板截图客户端设计方案

## 需求概述

开发一款Windows桌面客户端，安装在学校智能黑板/多媒体设备上，用于定时截图并上传到管理后台或保存到本地。

## 功能需求

### 核心功能
1. **定时截图**：每10秒截取屏幕（可配置间隔）
2. **云端上传**：登录后带token上传到管理项目API
3. **本地保存**：无网络或选择本地模式时，保存到指定文件夹
4. **自动清理**：本地保存时自动清理7天前的图片
5. **登录认证**：支持账号密码登录获取token

### 工作流程
```
启动 → 检测网络 → 有网络且已登录 → 上传云端
                → 未登录 → 登录 → 上传云端
                → 无网络 → 存本地
```

### 用户设置
- 截图间隔（默认10秒）
- 存储模式（云端/本地）
- 本地保存路径
- 账号登录

## 技术选型

| 技术 | 选择 | 理由 |
|------|------|------|
| 桌面框架 | Tauri 2.x | 轻量(5-10MB)、性能好、内存占用低 |
| 前端 | React + TypeScript | 成熟稳定，生态丰富 |
| UI | Tailwind CSS | 快速开发 |
| 截屏 | Rust screenshots crate | 成熟稳定 |
| HTTP | Rust reqwest | 异步高性能 |
| 存储 | Tauri fs API + 本地JSON | 轻量无需数据库 |

## API设计

基于现有的管理项目API：

### 登录
```
POST /api/login
  Body: { username, password }
  Response: { token, expires }
```

### 上传截图
```
POST /api/screenshot/upload
  Header: Authorization: Bearer <token>
  Body: (multipart) image file
  Response: { success, url }
```

## 模块设计

| 模块 | 职责 |
|------|------|
| 截屏模块 | 定时全屏截图，返回图片数据 |
| 上传模块 | 检测网络状态，带token上传，失败处理 |
| 本地存储模块 | 保存图片到本地，7天自动清理 |
| 登录模块 | 账号密码认证，token存储和刷新 |
| 设置模块 | 配置管理，UI交互 |

## 文件目录结构

```
src-tauri/
  src/
    main.rs          # 入口
    screenshot.rs    # 截屏逻辑
    uploader.rs      # 上传逻辑
    storage.rs       # 本地存储
    config.rs        # 配置管理

src/
  App.tsx            # 主应用
  components/        # UI组件
  hooks/             # React Hooks
  services/          # 前端服务
```

## 后续步骤

1. 初始化Tauri项目
2. 实现截屏功能
3. 实现登录和token管理
4. 实现上传模块
5. 实现本地存储和清理
6. 打包发布exe
