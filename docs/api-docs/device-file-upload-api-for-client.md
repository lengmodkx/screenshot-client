# 巡课设备文件一键上传接口文档（客户端专用）

> 接口用途：供截图客户端等外部系统调用，上传文件后自动完成：OSS 存储 → `infra_file` 文件表落库 → `erp_inspection_device_file` 设备关联表落库。

---

## 一、接口基本信息

| 项目 | 说明 |
| :--- | :--- |
| **接口地址** | `POST /admin-api/erp/inspection-device/file/upload-direct` |
| **请求协议** | HTTP / HTTPS |
| **认证方式** | `Authorization: Bearer {accessToken}` |
| **Content-Type** | `multipart/form-data` |

---

## 二、请求参数

### 2.1 请求头（Header）

| 参数名 | 必填 | 示例值 | 说明 |
| :--- | :--- | :--- | :--- |
| `Authorization` | 是 | `Bearer eyJ0eXAiOiJKV1Q...` | 用户登录凭证，必须在请求头中携带 |
| `Content-Type` | 否 | `multipart/form-data; boundary=----WebKitFormBoundary7MA4YWxkTrZu0gW` | 使用 `multipart/form-data`，一般由 HTTP 客户端自动设置 |

### 2.2 请求体（Body）

`multipart/form-data` 格式，包含以下字段：

| 字段名 | 类型 | 必填 | 说明 |
| :--- | :--- | :--- | :--- |
| `deviceId` | `Long` / `String` | **是** | 设备编号（对应 `erp_inspection_device.id`），文件将关联到该设备 |
| `file` | `File` / `Binary` | **是** | 文件二进制流，支持任意格式（图片、PDF、视频等） |

---

## 三、请求示例

### 3.1 cURL 示例

```bash
curl -X POST "https://{your-domain}/admin-api/erp/inspection-device/file/upload-direct" \
  -H "Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9..." \
  -F "deviceId=100" \
  -F "file=@/path/to/screenshot.png"
```

### 3.2 Java (OkHttp) 示例

```java
import okhttp3.*;

import java.io.File;
import java.io.IOException;

public class FileUploadExample {

    public static void main(String[] args) throws IOException {
        String url = "https://{your-domain}/admin-api/erp/inspection-device/file/upload-direct";
        String token = "Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...";

        File file = new File("/path/to/screenshot.png");

        RequestBody fileBody = RequestBody.create(MediaType.parse("image/png"), file);

        RequestBody requestBody = new MultipartBody.Builder()
                .setType(MultipartBody.FORM)
                .addFormDataPart("deviceId", "100")
                .addFormDataPart("file", file.getName(), fileBody)
                .build();

        Request request = new Request.Builder()
                .url(url)
                .header("Authorization", token)
                .post(requestBody)
                .build();

        OkHttpClient client = new OkHttpClient();
        try (Response response = client.newCall(request).execute()) {
            System.out.println(response.body().string());
        }
    }
}
```

### 3.3 Python (requests) 示例

```python
import requests

url = "https://{your-domain}/admin-api/erp/inspection-device/file/upload-direct"
headers = {
    "Authorization": "Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9..."
}
files = {
    "file": open("/path/to/screenshot.png", "rb")
}
data = {
    "deviceId": "100"
}

response = requests.post(url, headers=headers, data=data, files=files)
print(response.json())
```

### 3.4 JavaScript / Axios 示例

```javascript
const formData = new FormData();
formData.append('deviceId', '100');
formData.append('file', fileInput.files[0]);

axios.post('/admin-api/erp/inspection-device/file/upload-direct', formData, {
  headers: {
    'Authorization': 'Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...',
    'Content-Type': 'multipart/form-data'
  }
}).then(res => {
  console.log(res.data);
}).catch(err => {
  console.error(err);
});
```

---

## 四、响应结果

### 4.1 通用响应格式

所有接口统一返回 `CommonResult<T>` 包装：

```json
{
  "code": 0,      // 状态码：0 表示成功，非 0 表示失败
  "data": { },    // 业务数据（成功时返回）
  "msg": ""       // 提示信息（失败时返回错误原因）
}
```

### 4.2 成功响应示例（HTTP 200）

```json
{
  "code": 0,
  "data": {
    "id": 15,
    "deviceId": 100,
    "fileId": 2048,
    "fileName": "screenshot.png",
    "fileUrl": "https://oss.example.com/inspection/device/20250414/screenshot_1713072000000.png",
    "fileType": "image/png",
    "fileSize": 102400,
    "createTime": "2025-04-14T10:30:00"
  },
  "msg": ""
}
```

### 4.3 成功响应字段说明

| 字段名 | 类型 | 说明 |
| :--- | :--- | :--- |
| `id` | `Long` | 设备文件关联记录编号（`erp_inspection_device_file.id`） |
| `deviceId` | `Long` | 设备编号 |
| `fileId` | `Long` | 文件表编号（`infra_file.id`） |
| `fileName` | `String` | 原始文件名 |
| `fileUrl` | `String` | 文件访问 URL，可直接用于下载或展示 |
| `fileType` | `String` | 文件 MIME 类型 |
| `fileSize` | `Integer` | 文件大小，单位：字节（Byte） |
| `createTime` | `String` | 创建时间，ISO 8601 格式 |

### 4.4 失败响应示例

#### 设备不存在

```json
{
  "code": 1001002004,
  "data": null,
  "msg": "巡课设备不存在"
}
```

#### Token 无效或已过期

```json
{
  "code": 401,
  "data": null,
  "msg": "账号未登录"
}
```

#### 无操作权限

```json
{
  "code": 403,
  "data": null,
  "msg": "没有该操作权限"
}
```

#### 参数校验失败（缺少 deviceId 或 file）

```json
{
  "code": 100,
  "data": null,
  "msg": "请求参数不正确: Required request parameter 'deviceId' is not present"
}
```

---

## 五、上传后数据流向说明

调用本接口后，后端会依次完成以下 3 步操作：

```
┌─────────────────┐
│  1. 接收文件流   │
│  MultipartFile  │
└────────┬────────┘
         │
         ▼
┌─────────────────────────────────────────┐
│  2. 上传至 OSS / 本地存储 / S3 等        │
│  并写入 infra_file 文件表                │
│  返回文件访问 URL                        │
└────────┬────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────┐
│  3. 写入 erp_inspection_device_file      │
│  设备文件关联表                          │
│  建立 deviceId ↔ fileId 关联关系         │
└─────────────────────────────────────────┘
```

- **文件存储路径**：默认会按日期分目录，文件路径示例：`inspection/device/20250414/screenshot_1713072000000.png`
- **文件去重**：如果文件名相同，后端会自动追加时间戳后缀，避免覆盖已有文件。

---

## 六、对接注意事项

1. **必须携带有效 Token**
   - 本接口属于管理后台接口，需在请求头中传入 `Authorization: Bearer {accessToken}`。
   - 截图客户端需要先完成登录，获取 Access Token。

2. **Content-Type 不要手动写死**
   - 使用 `multipart/form-data` 时，`boundary` 分隔符必须由 HTTP 客户端库自动生成。不要手动写死 `Content-Type: multipart/form-data`，否则可能导致后端解析失败。

3. **deviceId 必须是有效的设备编号**
   - 如果传入的 `deviceId` 在 `erp_inspection_device` 表中不存在，会返回 "巡课设备不存在" 错误。

4. **文件大小限制**
   - 具体限制取决于服务端 Spring Boot 的 `spring.servlet.multipart.max-file-size` 和 `spring.servlet.multipart.max-request-size` 配置。
   - 默认通常是 **10MB** 或 **100MB**，如需上传大文件（如视频），请提前确认服务端配置。

5. **支持任意文件类型**
   - 本接口不对文件后缀做限制，图片、PDF、Word、视频等均可上传。但建议截图客户端只上传 `png`、`jpg`、`jpeg` 等常见图片格式。

6. **并发上传建议**
   - 如果截图客户端需要高频上传（如每秒一张截图），建议做本地队列 + 失败重试机制，避免网络抖动导致数据丢失。
