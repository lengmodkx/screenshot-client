# 文件存储接口文档

> 基于芋道（Yudao）框架 `yudao-module-infra` 模块整理，供外部项目调用参考。

---

## 一、基础信息

| 项           | 说明                                                         |
| ------------ | ------------------------------------------------------------ |
| 模块路径     | `/infra/file`                                                |
| 完整 BaseURL | `http://{host}:{port}/admin-api/infra/file`（管理后台）<br>`http://{host}:{port}/api/infra/file`（App 端，视网关配置而定） |
| 认证方式     | `Authorization: Bearer {accessToken}`                        |
| 通用响应格式 | `CommonResult<T>`，统一包装为 `{ "code": 0, "data": T, "msg": "" }` |

---

## 二、上传模式说明

本项目支持 **两种** 文件上传模式：

| 模式 | 适用场景 | 说明 |
| :--- | :--- | :--- |
| **模式一：后端上传** | 小文件、内部系统对接 | 直接将文件以 `multipart/form-data` 提交到后端，后端转存至配置的存储器（本地 / OSS / S3 等），返回文件的访问 URL。 |
| **模式二：前端直传** | 大文件、移动端、H5 | ① 调用 `presigned-url` 获取预签名上传地址；<br>② 前端/客户端直接 PUT/POST 到对象存储（阿里云 OSS、七牛云等）；<br>③ 上传成功后调用 `create` 将文件信息落库，拿到文件编号。 |

---

## 三、模式一：后端上传（最简单）

### 3.1 管理后台 - 上传文件

**接口地址**

```http
POST /infra/file/upload
```

**权限要求**

- 需携带有效登录 Token（`Authorization: Bearer {token}`）
- 后台默认需要 `infra:file:upload` 或相关功能权限（视具体版本配置）

**请求方式**

`multipart/form-data`

**请求参数**

| 参数名 | 类型 | 必填 | 说明 |
| :--- | :--- | :--- | :--- |
| `file` | File | 是 | 文件附件 |
| `directory` | String | 否 | 文件目录，如 `avatar`、`contract`。注意：**不允许包含 `..`、`/`、`\`** |

**请求示例（cURL）**

```bash
curl -X POST "http://localhost:48080/admin-api/infra/file/upload" \
  -H "Authorization: Bearer eyJ0eXAiOiJKV1Q..." \
  -F "file=@/Users/demo/Desktop/test.png" \
  -F "directory=avatar"
```

**成功响应**

```json
{
  "code": 0,
  "data": "https://test.yudao.iocoder.cn/avatar/xxxxx.png",
  "msg": ""
}
```

- `data`：文件的可访问 URL。

---

### 3.2 App 端 - 上传文件

**接口地址**

```http
POST /infra/file/upload
```

**权限要求**

- `@PermitAll`，**无需登录 Token** 即可调用（视业务需要可改成需登录）。

**请求参数 / 响应格式**

与 **3.1 管理后台上传文件** 完全一致。

---

## 四、模式二：前端直传（推荐大文件）

### 4.1 获取文件预签名地址（上传）

**接口地址**

```http
GET /infra/file/presigned-url
```

**权限要求**

- 管理后台：需登录 Token
- App 端：`@PermitAll`（无 Token 要求）

**请求参数（Query）**

| 参数名 | 类型 | 必填 | 说明 |
| :--- | :--- | :--- | :--- |
| `name` | String | 是 | 原始文件名，如 `test.png` |
| `directory` | String | 否 | 文件目录，如 `avatar` |

**请求示例**

```bash
curl -X GET "http://localhost:48080/admin-api/infra/file/presigned-url?name=test.png&directory=avatar" \
  -H "Authorization: Bearer eyJ0eXAiOiJKV1Q..."
```

**成功响应**

```json
{
  "code": 0,
  "data": {
    "configId": 11,
    "uploadUrl": "https://s3.cn-south-1.qiniucs.com/.../test.png?X-Amz-Algorithm=...",
    "url": "https://test.yudao.iocoder.cn/avatar/test.png",
    "path": "avatar/test.png"
  },
  "msg": ""
}
```

**字段说明**

| 字段名 | 说明 |
| :--- | :--- |
| `configId` | 文件配置编号，后续 `create` 接口需要 |
| `uploadUrl` | 预签名的直传地址，前端应将文件 **直接 PUT/POST** 到该 URL |
| `url` | 文件上传成功后的访问地址 |
| `path` | 文件在存储器中的相对路径，后续 `create` 接口需要 |

### 4.2 直传到对象存储

拿到 `uploadUrl` 后，客户端直接上传文件：

```bash
curl -X PUT "{uploadUrl}" \
  -H "Content-Type: image/png" \
  --data-binary @/Users/demo/Desktop/test.png
```

> 具体是 `PUT` 还是 `POST`，取决于当前启用的文件存储客户端（OSS / S3 / 七牛等），一般预签名 URL 已经带好了方法签名。

### 4.3 创建文件记录（落库）

前端直传成功后，必须调用该接口将文件信息写入数据库，否则系统中查不到该文件。

**接口地址**

```http
POST /infra/file/create
```

**权限要求**

- 管理后台：需登录 Token
- App 端：`@PermitAll`

**请求方式**

`application/json`

**请求参数**

| 参数名 | 类型 | 必填 | 说明 |
| :--- | :--- | :--- | :--- |
| `configId` | Long | 是 | 预签名接口返回的 `configId` |
| `path` | String | 是 | 预签名接口返回的 `path` |
| `name` | String | 是 | 原始文件名 |
| `url` | String | 是 | 预签名接口返回的 `url` |
| `type` | String | 否 | 文件 MIME 类型，如 `image/png` |
| `size` | Integer | 是 | 文件大小（字节） |

**请求示例**

```bash
curl -X POST "http://localhost:48080/admin-api/infra/file/create" \
  -H "Authorization: Bearer eyJ0eXAiOiJKV1Q..." \
  -H "Content-Type: application/json" \
  -d '{
    "configId": 11,
    "path": "avatar/test.png",
    "name": "test.png",
    "url": "https://test.yudao.iocoder.cn/avatar/test.png",
    "type": "image/png",
    "size": 20480
  }'
```

**成功响应**

```json
{
  "code": 0,
  "data": 1024,
  "msg": ""
}
```

- `data`：创建成功的文件编号（`id`），业务表可存储该 ID 或 `url`。

---

## 五、文件下载

**接口地址**

```http
GET /infra/file/{configId}/get/{path}
```

**权限要求**

`@PermitAll`，**无需 Token**，支持租户隔离 `@TenantIgnore`。

**请求参数**

- `configId`：文件配置编号
- `path`：文件路径（需 URL Encode，支持多级路径）

**请求示例**

```bash
curl -O "http://localhost:48080/admin-api/infra/file/11/get/avatar/test.png"
```

> 如果 `path` 包含中文或特殊字符，请先做 URL 编码。

---

## 六、管理后台 - 文件管理（补充）

### 6.1 文件分页列表

**接口地址**

```http
GET /infra/file/page
```

**权限要求**

`@PreAuthorize("@ss.hasPermission('infra:file:query')")`

**请求参数（Query）**

| 参数名 | 类型 | 必填 | 说明 |
| :--- | :--- | :--- | :--- |
| `pageNo` | Integer | 是 | 页码，从 1 开始 |
| `pageSize` | Integer | 是 | 每页条数 |
| `path` | String | 否 | 文件路径，模糊查询 |
| `type` | String | 否 | 文件 MIME 类型，模糊查询 |

**响应字段**

| 字段名 | 说明 |
| :--- | :--- |
| `id` | 文件编号 |
| `configId` | 配置编号 |
| `path` | 文件路径 |
| `name` | 原文件名 |
| `url` | 访问 URL |
| `type` | MIME 类型 |
| `size` | 文件大小 |
| `createTime` | 创建时间 |

### 6.2 删除文件

**接口地址**

```http
DELETE /infra/file/delete?id={id}
```

**权限要求**

`infra:file:delete`

### 6.3 批量删除文件

**接口地址**

```http
DELETE /infra/file/delete-list?ids={id1},{id2}
```

**权限要求**

`infra:file:delete`

---

## 七、对接建议

1. **内部系统 / 小文件**：优先使用 **模式一**（`POST /infra/file/upload`），一次请求即可完成上传+落库。
2. **移动端 / 大文件 / 带宽敏感**：优先使用 **模式二**（`presigned-url` + 直传 + `create`），减少服务端带宽压力。
3. **注意目录校验**：`directory` 字段后端会校验，不能包含 `..`、`/`、`\`，建议只传纯目录名，如 `avatar`、`contract`、`2024/report`（虽然代码里限制了 `/`，但实际业务中如果需要多级目录，建议咨询后端调整校验规则）。
4. **Token 有效期**：管理后台接口需要携带 `Authorization: Bearer {token}`，Token 过期后需重新登录或刷新。
