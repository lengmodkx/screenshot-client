# Rust 客户端视频推送接口文档

## 一、概述

本文档定义 Rust 客户端需要实现的接口规范，用于将教室设备的视频流推送到服务端。

## 二、接口清单

| 序号 | 接口路径 | 方法 | 说明 |
|------|----------|------|------|
| 1 | /client/inspection/login | POST | 登录认证 |
| 2 | /client/inspection/register | POST | 设备注册 |
| 3 | /client/inspection/video/push | POST | 推送视频帧 |
| 4 | /client/inspection/heartbeat | POST | 设备心跳 |
| 5 | /client/inspection/uploadScreenshot | POST | 上传截图 |

> **说明**：
> - 1-5 号接口为 **Rust 客户端需要实现** 的接口
> - 以下 6-7 号接口为 **系统端（Web）使用**，Rust 团队了解即可：
>
> | 6 | /admin-api/erp/inspection/video/url | GET | 获取视频流信息 |
> | 7 | /admin-api/erp/inspection/video/stream/{deviceCode} | GET | FLV 视频流播放 |

## 三、接口详情

### 3.1 登录认证

**接口地址**：`POST /client/inspection/login`

**请求头**：
```
Content-Type: application/json
```

**请求参数**：
```json
{
  "tenantName": "租户名称",
  "username": "账号",
  "password": "密码"
}
```

**返回**：
```json
{
  "code": 200,
  "data": {
    "userId": 142,
    "username": "admin",
    "deptId": 1,
    "accessToken": "xxx",
    "refreshToken": "xxx",
    "expiresTime": "2026-03-05T10:00:00"
  },
  "msg": "success"
}
```

**说明**：
- 登录成功后保存 `accessToken`，后续接口需要使用
- token 有效期默认2小时，过期后需要重新登录

---

### 3.2 设备注册

**接口地址**：`POST /client/inspection/register`

**请求头**：
```
Content-Type: application/x-www-form-urlencoded
Authorization: Bearer {accessToken}
```

**请求参数**：

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| deviceName | String | 是 | 设备名称（如：一年级一班黑板） |
| deviceCode | String | 是 | 设备编码（唯一标识，如：DEV_C4C6E66D3600） |
| deviceType | Integer | 是 | 设备类型：1-电子大屏，2-黑板 |
| ipAddress | String | 是 | 设备 IP 地址 |
| classroomId | Integer | 否 | 教室 ID |
| registerType | Integer | 是 | 注册类型：0-后台注册，1-自动注册 |

**返回**：
```json
{
  "code": 200,
  "data": {
    "id": 123,
    "deviceName": "一年级一班黑板",
    "deviceCode": "DEV_C4C6E66D3600"
  },
  "msg": "success"
}
```

**说明**：
- 设备注册成功后才能推送视频帧
- 如果设备已注册（根据 deviceCode + ipAddress + deviceName 判断），会返回已有设备信息
- 建议在程序启动时先调用注册接口

---

### 3.3 推送视频帧（核心接口）

**接口地址**：`POST /client/inspection/video/push`

**请求头**：
```
Content-Type: multipart/form-data
Authorization: Bearer {accessToken}
```

**请求参数**：

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| deviceCode | String | 是 | 设备编码（唯一标识） |
| data | Blob/Base64 | 是 | 视频帧数据（Base64编码的JPEG图片） |

**Base64 编码示例（Rust）**：

```rust
use base64::Engine;

fn encode_frame_to_base64(frame_data: &[u8]) -> String {
    use base64::engine::general_purpose::STANDARD;
    STANDARD.encode(frame_data)
}
```

**请求示例**：

```rust
// 1. 采集摄像头（使用摄像头库获取帧数据）
let frame_data = capture_camera_frame()?;

// 2. 编码为JPEG
let jpeg_data = encode_to_jpeg(&frame_data)?;

// 3. 转换为Base64
let base64_data = encode_frame_to_base64(&jpeg_data);

// 4. 发送请求
let client = reqwest::Client::new();
let form = reqwest::multipart::Form::new()
    .text("deviceCode", "DEV_001")
    .text("data", base64_data);

client.post("http://server:48080/client/inspection/video/push")
    .header("Authorization", format!("Bearer {}", token))
    .multipart(form)
    .send()?;
```

**返回**：
```json
{
  "code": 200,
  "data": true,
  "msg": "success"
}
```

**推送频率建议**：
- 每秒推送 1-5 帧（建议 1-2 帧即可，过高的帧率会增加带宽和服务器压力）
- 根据网络情况调整，网络差时降低帧率
- 使用循环持续推送，不要等待响应后再发下一帧
- **重要**：图片大小建议控制在 100KB 以内，过大的图片会导致延迟和内存问题

---

### 3.4 设备心跳

**接口地址**：`POST /client/inspection/heartbeat`

**请求头**：
```
Authorization: Bearer {accessToken}
Content-Type: application/x-www-form-urlencoded
```

**请求参数**：

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| deviceCode | String | 是 | 设备编码 |

**返回**：
```json
{
  "code": 200,
  "data": true,
  "msg": "success"
}
```

**心跳频率建议**：每30秒发送一次

---

### 3.5 上传截图

**接口地址**：`POST /client/inspection/uploadScreenshot`

**请求头**：
```
Content-Type: multipart/form-data
Authorization: Bearer {accessToken}
```

**请求参数**：

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| deviceCode | String | 是 | 设备编码 |
| file | File | 是 | 截图文件（JPEG格式） |

**返回**：
```json
{
  "code": 200,
  "data": "http://server/screenshot/xxx.jpg",
  "msg": "success"
}
```

**说明**：
- 截图用于后台查看设备当前画面
- 建议每 5-10 分钟上传一次截图
- 截图分辨率建议 1280x720 或更低

---

## 四、系统端接口（供参考）

以下接口由 **Web 前端/管理系统** 调用，Rust 客户端团队了解即可：

### 4.1 获取视频流信息

**接口地址**：`GET /admin-api/erp/inspection/video/url`

**说明**：Web 前端调用此接口获取视频流地址和在线状态

**请求参数**：

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| deviceCode | String | 是 | 设备编码 |

**返回**：
```json
{
  "code": 200,
  "data": {
    "deviceCode": "DEV_C4C6E66D3600",
    "deviceName": "一年级一班黑板",
    "streamUrl": "/admin-api/erp/inspection/video/stream/DEV_C4C6E66D3600",
    "status": 1,
    "lastPushTime": "2026-03-05T14:30:00"
  },
  "msg": "success"
}
```

**字段说明**：
- `status`: 0-离线，1-在线
- `streamUrl`: FLV 视频流地址
- `lastPushTime`: 最后收到视频帧的时间

---

### 4.2 FLV 视频流播放

**接口地址**：`GET /admin-api/erp/inspection/video/stream/{deviceCode}`

**说明**：
- 这是 FLV 视频流的播放地址
- Web 前端使用 [flv.js](https://github.com/Bilibili/flv.js) 播放器播放
- 数据格式为 FLV 封装的 JPEG 帧序列（简化版，非标准 FLV）
- **延迟**：约 3-10 秒

**请求方式**：
- Web 前端通过 `<video>` 标签或 flv.js 播放器连接此地址
- 响应为流式数据，非一次性返回

---

## 五、完整业务流程

```rust
fn main() {
    // 1. 登录获取token
    let token = login("租户名称", "账号", "密码");

    // 2. 启动心跳线程（每30秒）
    let token_clone = token.clone();
    std::thread::spawn(move || {
        heartbeat_loop("DEV_001", &token_clone);
    });

    // 3. 主循环：采集并推送视频帧（每秒1-5帧）
    loop {
        // 采集摄像头
        let frame = capture_camera_frame().unwrap();

        // 编码为JPEG
        let jpeg = encode_to_jpeg(&frame).unwrap();

        // 推送视频帧
        push_video_frame("DEV_001", &jpeg, &token).unwrap();

        // 等待一段时间后继续（根据帧率调整）
        std::thread::sleep(std::time::Duration::from_millis(200)); // 5fps
    }
}

fn login(tenant: &str, username: &str, password: &str) -> String {
    // 实现登录逻辑，返回accessToken
}

fn heartbeat_loop(device_code: &str, token: &str) {
    loop {
        std::thread::sleep(std::time::Duration::from_secs(30));
        send_heartbeat(device_code, token).unwrap();
    }
}
```

---

## 六、视频流架构说明

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
       │                           │                           │
       └──────────────────────────>│  实时转发视频帧            │<──────────────────────────┘
```

**数据流说明**：
1. Rust 客户端持续推送 JPEG 图片到服务端缓存（ConcurrentHashMap）
2. 缓存有效期 30 秒，过期自动清理
3. Web 前端通过 FLV 接口拉取视频流（实际上是 JPEG 帧序列）
4. 延迟约 3-10 秒（取决于推送频率和网络状况）

---

## 七、错误处理

### 5.1 常见错误码

| code | msg | 说明 | 处理方式 |
|------|-----|------|----------|
| 401 | 未授权 | token过期或无效 | 重新登录获取新token |
| 500 | 服务器错误 | 服务端异常 | 记录日志，稍后重试 |
| 901 | 多租户错误 | 租户不存在 | 检查租户名称是否正确 |
| 1002003002 | 设备已注册 | 相同设备信息已存在 | 忽略或使用已有设备 |
| 1002003000 | 设备不存在 | 设备编码错误 | 检查设备编码 |

### 7.2 错误处理策略

1. **网络异常**：记录日志，继续下一次推送，不要阻塞
2. **401 错误**：自动重新登录，拿到新token后继续推送
3. **连接失败**：指数退避重试（1s, 2s, 4s, 8s... 最大30s）
4. **设备已注册错误（1002003002）**：直接使用返回的设备信息，继续后续操作

---

## 八、技术依赖（Rust）

建议使用以下库：

```toml
[dependencies]
reqwest = { version = "0.11", features = ["multipart"] }
base64 = "0.21"
# 摄像头采集（根据实际使用的摄像头库选择）
# nokhwa = "0.10"  # 跨平台摄像头库
# opencv = "0.71"  # OpenCV绑定
# image = "0.24"   # 图像处理
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }
```

---

## 九、测试建议

1. **单元测试**：单独测试 Base64 编码、请求组装
2. **集成测试**：先使用 Postman/curl 测试接口连同性
3. **压力测试**：模拟多设备同时推送视频帧

**curl 测试命令**：

```bash
# ========== Rust 客户端接口 ==========

# 1. 登录
curl -X POST http://localhost:48080/client/inspection/login \
  -H "Content-Type: application/json" \
  -d '{"tenantName":"测试租户","username":"admin","password":"123456"}'

# 2. 设备注册
curl -X POST http://localhost:48080/client/inspection/register \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -d "deviceName=一年级一班黑板" \
  -d "deviceCode=DEV_001" \
  -d "deviceType=2" \
  -d "ipAddress=192.168.1.100" \
  -d "classroomId=1" \
  -d "registerType=1"

# 3. 推送视频帧（假设token已获取）
curl -X POST http://localhost:48080/client/inspection/video/push \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -F "deviceCode=DEV_001" \
  -F "data=$(base64 -w0 /path/to/image.jpg)"

# 4. 发送心跳
curl -X POST http://localhost:48080/client/inspection/heartbeat \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -d "deviceCode=DEV_001"

# 5. 上传截图
curl -X POST http://localhost:48080/client/inspection/uploadScreenshot \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -F "deviceCode=DEV_001" \
  -F "file=@/path/to/screenshot.jpg"

# ========== 系统端接口（供参考） ==========

# 6. 获取视频流信息（需要登录后的 admin 用户 token）
curl -X GET "http://localhost:48080/admin-api/erp/inspection/video/url?deviceCode=DEV_001" \
  -H "Authorization: Bearer ADMIN_TOKEN"

# 7. FLV 视频流（使用播放器或浏览器打开）
# 注意：这是流式接口，直接用 curl 会下载二进制数据
# 建议使用 flv.js 播放器或 VLC 播放
open "http://localhost:48080/admin-api/erp/inspection/video/stream/DEV_001"
```

---

## 十、注意事项

### Rust 客户端开发要点

1. **Base64 编码**：
   - 确保图片数据是有效的 Base64 字符串
   - **不要包含前缀**（如 `data:image/jpeg;base64,`）
   - 只保留纯 Base64 编码内容

2. **JPEG 格式**：
   - 只支持 JPEG 格式的图片
   - 建议使用 640x480 或 1280x720 分辨率
   - 单张图片大小控制在 **100KB 以内**

3. **设备编码**：
   - 确保每个设备的编码唯一
   - 建议使用硬件唯一标识（如 MAC 地址、UUID）
   - 格式示例：`DEV_` + 设备唯一标识

4. **推送频率**：
   - 建议每秒 1-2 帧（足以满足监控需求）
   - 过高的帧率会增加带宽和服务器压力
   - 网络不稳定时自动降低帧率

5. **错误处理**：
   - 推送失败时不要阻塞，继续下一次推送
   - 401 错误时自动重新登录
   - 设备已注册错误直接使用返回的设备信息

6. **资源管理**：
   - 程序启动时：登录 → 注册设备 → 启动心跳 → 开始推送视频
   - 程序退出时：停止推送线程 → 释放摄像头资源 → 可选：发送离线心跳

### 与系统端的协作

- Rust 客户端推送的视频帧存储在服务端内存中（30秒缓存）
- Web 前端通过 FLV 接口拉取视频流播放
- 如果 30 秒内没有收到视频帧，系统会显示设备离线
- 截图功能用于后台查看设备静态画面，与视频流是独立的

---

**文档版本**：2026-03-05
**适用系统**：危化品及实验室服务管理平台 - 巡课系统
