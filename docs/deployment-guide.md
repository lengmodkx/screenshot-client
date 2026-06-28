# 智能黑板截图客户端 - 部署文档

> 版本：v1.4.2
> 适用对象：运维工程师 / 系统集成商
> 平台：Windows 10 / Windows 11（64 位）

---

## 一、产品概述

**智能黑板截图客户端**（ScreenshotClient）是面向内蒙古自治区中小学实验室管理平台开发的桌面代理程序。它部署在学校智慧黑板/多媒体设备上，承担以下职责：

| 功能 | 说明 | 频率 |
|------|------|------|
| 实时视频流推送 | 将教室画面（摄像头或屏幕）推送到服务端 | 2 帧/秒 |
| 定时截图上传 | 上传静态画面供后台巡视 | 每 5 分钟 |
| 软件使用监控 | 监控设备上运行的软件，记录启停事件 | 实时 |
| 设备心跳保活 | 维持设备在线状态 | 每 30 秒 |
| 设备自动注册 | 首次登录时注册设备到平台 | 一次性 |

**技术栈**：Tauri 2.x + React 19 + TypeScript + Rust + Tailwind CSS 4
**包体积**：约 10 MB（远小于 Electron 的 150MB+）

---

## 二、部署架构

```
┌──────────────────────────────────────────────────────────────┐
│                     学校智慧黑板 / 多媒体设备                   │
│                                                              │
│   ┌──────────────────────────────────────────────────────┐  │
│   │           智能黑板截图客户端 (ScreenshotClient)         │  │
│   │                                                      │  │
│   │  ┌─────────────┐    ┌─────────────┐    ┌───────────┐ │  │
│   │  │ React 前端   │◄──►│ Tauri 桥接   │◄──►│ Rust 后端 │ │  │
│   │  │ (UI/交互)    │    │             │    │ (采集/上传)│ │  │
│   │  └─────────────┘    └─────────────┘    └─────┬─────┘ │  │
│   │                                              │       │  │
│   │  ┌───────────────────────────────────────────┐       │  │
│   │  │ 模块: 视频推送 | 截图 | 软件监控 | 心跳      │       │  │
│   │  └───────────────────────────────────────────┘       │  │
│   └──────────────────────────┬───────────────────────────┘  │
│                              │ HTTPS/HTTP                    │
└──────────────────────────────┼───────────────────────────────┘
                               │
                               ▼
┌──────────────────────────────────────────────────────────────┐
│          内蒙古中小学实验室管理平台 (Java 后端)                │
│                                                              │
│   API 地址示例：http://172.16.10.11:48080                   │
│                                                              │
│   ┌──────────────┐ ┌──────────────┐ ┌──────────────────┐    │
│   │ 登录认证      │ │ 视频流缓存    │ │ 文件存储 (OSS)   │    │
│   │ /login       │ │ (内存/30秒)  │ │ infra_file 表     │    │
│   └──────────────┘ └──────────────┘ └──────────────────┘    │
└──────────────────────────────────────────────────────────────┘
```

---

## 三、部署前准备

### 3.1 硬件要求

| 项目 | 最低配置 | 推荐配置 |
|------|----------|----------|
| CPU | Intel i3 / 同档 | Intel i5 及以上 |
| 内存 | 4 GB | 8 GB 及以上 |
| 硬盘 | 500 MB 可用空间 | 2 GB 可用空间 |
| 摄像头 | USB 摄像头或内置摄像头（可选） | 720P 及以上 |
| 网络 | 10 Mbps 局域网 | 100 Mbps 可访问外网 |

> **注**：未接摄像头的设备会自动降级为「屏幕截图模式」，不影响核心功能。

### 3.2 软件要求

| 软件 | 版本要求 | 用途 |
|------|----------|------|
| Windows 10/11 | 64 位（1903+） | 操作系统 |
| WebView2 Runtime | 最新版 | Tauri 运行时（Win11 自带，Win10 需手动安装） |
| .NET Framework | 4.7.2+ | 通常系统自带 |
| VC++ Runtime | 2015-2022 | 通常系统自带 |

### 3.3 网络要求

客户端需要访问以下地址（请根据实际情况调整）：

| 用途 | 地址 | 协议 |
|------|------|------|
| API 服务 | `http://<server-ip>:48080` | HTTP/HTTPS |
| 视频流查看 | `http://<server-ip>:48080/admin-api/erp/inspection/video/...` | HTTP/HTTPS |

**防火墙要求**：出站方向允许 48080（或对应后端）端口。

### 3.4 账号准备

请向平台管理员获取：

- **租户名称**（tenantName）：如 `neimenggu`
- **登录账号**（username）：如 `xcyxadmin`
- **登录密码**（password）：初始化密码

---

## 四、安装部署

### 4.1 方案 A：MSI / NSIS 安装包安装（推荐生产环境）

#### 步骤 1：获取安装包

构建产物路径（构建后自动生成）：

```
src-tauri/target/release/bundle/
├── msi/                       # MSI 安装包（推荐域控环境）
│   └── ScreenshotClient_1.4.2_x64_en-US.msi
└── nsis/                      # NSIS 安装包（推荐单台安装）
    └── ScreenshotClient_1.4.2_x64-setup.exe
```

#### 步骤 2：安装

**NSIS 方式（推荐）**：

1. 将 `ScreenshotClient_1.4.2_x64-setup.exe` 拷贝到目标设备
2. 双击运行安装包
3. 选择安装目录（默认 `C:\Program Files\ScreenshotClient`）
4. 勾选「创建桌面快捷方式」
5. 点击「安装」，等待完成
6. 点击「完成」退出安装向导

**MSI 方式（域控/批量部署）**：

```cmd
msiexec /i ScreenshotClient_1.4.2_x64_en-US.msi /qn ^
  INSTALLDIR="C:\Program Files\ScreenshotClient" ^
  ALLUSERS=1
```

或通过组策略（GPO）批量下发。

#### 步骤 3：验证安装

- 开始菜单出现「截图客户端」
- 桌面有快捷方式
- 安装目录下有 `screenshot-client.exe`

---

### 4.2 方案 B：绿色版直接运行（测试/快速验证）

#### 步骤 1：定位可执行文件

```
src-tauri/target/release/screenshot-client.exe
```

#### 步骤 2：创建桌面快捷方式（可选）

右键 → 发送到 → 桌面快捷方式。

#### 步骤 3：首次启动

双击 `screenshot-client.exe` 运行（首次启动会自动生成 `%APPDATA%\ScreenshotClient\` 配置目录）。

---

### 4.3 方案 C：从源码构建（定制化场景）

#### 步骤 1：安装构建工具

```powershell
# 1. 安装 Node.js (建议 22.12 LTS)
# 官网下载：https://nodejs.org/

# 2. 安装 Rust (建议 1.70+)
# 官网下载：https://rustup.rs/

# 3. 安装 Microsoft C++ Build Tools
# 官网下载：https://visualstudio.microsoft.com/visual-cpp-build-tools/
# 必须勾选："使用 C++ 的桌面开发" 工作负载
```

#### 步骤 2：克隆代码并构建

```bash
# 克隆仓库
git clone https://github.com/lengmodkx/screenshot-client.git
cd screenshot-client

# 安装前端依赖
npm install

# 构建生产版本（包含前端 + Rust + 打包安装包）
npm run tauri build
```

构建完成后安装包位于 `src-tauri/target/release/bundle/`。

---

## 五、首次配置

### 5.1 启动客户端

双击桌面「截图客户端」快捷方式，首次进入登录界面。

### 5.2 配置 API 地址（如需修改）

默认 API 地址：`http://172.16.10.11:48080`

如需修改为生产/测试环境地址：

**方法 1：登录界面直接输入**

在登录页面的「API 地址」输入框修改后登录。

**方法 2：修改配置文件**

配置文件路径：`%APPDATA%\ScreenshotClient\config.json`

```json
{
  "api_url": "http://your-server-ip:48080",
  "account_username": "",
  "account_password": "",
  "tenant_name": "",
  "device_code": "DEV_XXXXXXXX",
  "device_name": "",
  "school_class_id": 0,
  "device_id": 0,
  "is_registered": false,
  "capture_mode": "camera",
  "camera_resolution": "1080p"
}
```

> 配置文件修改后必须重启客户端才能生效。

### 5.3 登录

1. 输入 **租户名称**（如 `neimenggu`）
2. 输入 **账号**
3. 输入 **密码**
4. 点击「登录」

### 5.4 设备注册（首次登录后自动进入）

1. 登录成功后，自动跳转设备注册页面
2. 选择 **设备类型**（智能黑板 / 电子大屏）
3. 选择 **所属班级**
4. 输入 **设备名称**（建议按教室命名，如 "三年一班智慧黑板"）
5. 点击「注册」

注册成功后自动进入主仪表盘。

---

## 六、进阶配置

### 6.1 开机自启动

#### 方法 1：安装包选项（推荐）

NSIS 安装包安装时勾选「开机自动启动」，默认会将快捷方式放入启动文件夹。

#### 方法 2：手动配置

将快捷方式放入启动文件夹：

```
shell:startup
```

#### 方法 3：注册表方式（域控）

```reg
Windows Registry Editor Version 5.00

[HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run]
"ScreenshotClient"="\"C:\\Program Files\\ScreenshotClient\\screenshot-client.exe\" --minimized"
```

> 加 `--minimized` 参数启动后自动最小化到托盘，不弹窗。

### 6.2 采集模式选择

客户端支持两种采集模式：

| 模式 | 适用场景 | 优势 | 限制 |
|------|----------|------|------|
| `camera` 摄像头模式 | 默认推荐 | 真实教室画面 | 需要摄像头硬件 |
| `screen` 屏幕截图模式 | 无摄像头设备 | 不需要额外硬件 | 仅截取屏幕内容 |

切换方法：修改 `config.json` 中的 `capture_mode` 字段，重启生效。

### 6.3 摄像头分辨率调整

可选值：`480p` / `720p` / `1080p`（默认）

修改 `config.json`：

```json
{
  "camera_resolution": "720p"
}
```

> 分辨率越高画面越清晰，但占用带宽越多。**推荐 720p**（平衡清晰度与带宽）。

### 6.4 防火墙放行

如果客户端无法连接服务器，请检查 Windows Defender 防火墙：

```powershell
# 允许客户端程序通过防火墙（以管理员身份运行）
New-NetFirewallRule -DisplayName "ScreenshotClient" `
  -Direction Outbound `
  -Program "C:\Program Files\ScreenshotClient\screenshot-client.exe" `
  -Action Allow
```

### 6.5 代理设置（特殊网络环境）

如果客户端网络使用代理访问外网，可能导致 502 错误。**解决方法**：

**方法 1：配置 NO_PROXY 环境变量**

```cmd
setx NO_PROXY "172.16.10.11,localhost,127.0.0.1"
```

**方法 2：在代码中禁用代理**（如需修改源码）

Rust 端 reqwest 默认走系统代理，可在 `Client::builder()` 中调用 `.no_proxy()`。

---

## 七、批量部署建议

### 7.1 域控 + GPO 部署

1. 将 MSI 安装包放到网络共享：`\\fileserver\share\ScreenshotClient\`
2. 打开组策略管理控制台（GPMC）
3. 创建/编辑 GPO → 计算机配置 → 策略 → 软件设置 → 软件安装
4. 新建 → 程序包 → 选择 MSI 文件 → 分配
5. 链接到对应 OU，下次重启自动安装

### 7.2 静默安装脚本示例

```bat
@echo off
REM 静默安装脚本
msiexec /i "\\fileserver\share\ScreenshotClient\ScreenshotClient_1.4.2_x64_en-US.msi" /qn /norestart
echo 安装完成
```

通过计划任务或开机脚本推送到所有设备。

### 7.3 配置预置（避免逐台配置）

可以通过组策略首选项 (GPP) 推送配置文件：

```
%APPDATA%\ScreenshotClient\config.json
```

使用环境变量 `%USERNAME%` 等动态生成设备编码或账号。

---

## 八、卸载

### 8.1 通过控制面板

设置 → 应用 → 已安装的应用 → 找到「截图客户端」→ 卸载。

### 8.2 静默卸载（MSI）

```cmd
msiexec /x {ProductCode} /qn
```

> ProductCode 可通过以下命令查询：

```powershell
Get-WmiObject -Class Win32_Product | Where-Object {$_.Name -like "*Screenshot*"} | Select-Object Name, IdentifyingNumber
```

### 8.3 清理残留文件（可选）

卸载后如需彻底清理：

```cmd
rd /s /q "%APPDATA%\ScreenshotClient"
rd /s /q "%LOCALAPPDATA%\ScreenshotClient"
```

---

## 九、常见部署问题排查

### Q1：双击 exe 闪退

**可能原因**：
- 缺少 WebView2 Runtime（Win10 常见）
- 缺少 VC++ Runtime

**解决方法**：

1. 安装 WebView2 Runtime：https://developer.microsoft.com/microsoft-edge/webview2/
2. 安装 VC++ Redistributable：https://aka.ms/vs/17/release/vc_redist.x64.exe

### Q2：登录提示「网络异常」

**排查步骤**：

1. 浏览器访问 `http://<server-ip>:48080/client/inspection/login`，确认 API 可达
2. 检查防火墙是否放行 48080 端口
3. 检查代理设置（参考 6.5 节）

### Q3：客户端能登录但推流失败

**排查步骤**：

1. 检查摄像头是否被其他程序占用
2. 切换为屏幕截图模式测试（修改 `capture_mode` 为 `screen`）
3. 查看客户端日志（在客户端窗口底部状态栏）

### Q4：开机自启后窗口不弹出

这是正常行为——默认开机自启进入托盘模式。如需弹窗，请删除 `--minimized` 启动参数。

### Q5：截图上传失败但视频流正常

**排查步骤**：

1. 确认设备已注册（`config.json` 中 `is_registered: true` 且 `device_id` 非 0）
2. 重新执行设备注册流程

### Q6：磁盘空间持续增长

客户端会在 `Pictures\Screenshots\` 目录缓存截图。已实现的清理机制：

- 后端 15 天自动清理定时任务（推荐）
- 客户端本地启动时清理 15 天前文件（兜底）

如需立即释放空间：

```cmd
rd /s /q "%USERPROFILE%\Pictures\Screenshots"
```

---

## 十、附录

### 10.1 关键文件路径

| 路径 | 用途 |
|------|------|
| `%APPDATA%\ScreenshotClient\config.json` | 主配置文件 |
| `%APPDATA%\ScreenshotClient\screenshot-client.db` | SQLite 本地数据库（软件监控数据） |
| `%USERPROFILE%\Pictures\Screenshots\` | 本地截图缓存目录 |
| `C:\Program Files\ScreenshotClient\` | 默认安装目录（MSI/NSIS 安装） |

### 10.2 关键端口

| 端口 | 用途 | 方向 |
|------|------|------|
| 48080 | 后端 API 服务（默认值，可配置） | 出站 |
| 1420 | 开发模式 Vite 服务（仅开发用） | 本地 |

### 10.3 版本信息查询

- 关于窗口：客户端主界面 → 帮助 → 关于
- 配置文件：`config.json` 中无版本字段，从 `tauri.conf.json` 的 `version` 字段读取

### 10.4 技术支持

| 渠道 | 联系方式 |
|------|----------|
| 项目仓库 | https://github.com/lengmodkx/screenshot-client |
| 问题反馈 | GitHub Issues |
| 平台 API 文档 | `docs/api-docs/rust-client-video-api.md` |

---

**文档版本**：v1.0
**对应客户端版本**：v1.4.2
**最后更新**：2026-06-28
