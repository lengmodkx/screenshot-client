# 巡课客户端接口文档

## 一、认证接口

### 1. 客户端登录
**接口地址：** `POST /client/inspection/login`

**请求头：**
```
Content-Type: application/json
```

**请求参数：**
```json
{
  "tenantName": "租户名称",
  "username": "账号",
  "password": "密码"
}
```

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| tenantName | String | 是 | 租户名称（如：测试租户） |
| username | String | 是 | 登录账号 |
| password | String | 是 | 登录密码 |

**返回：**
```json
{
  "code": 0,
  "msg": "success",
  "data": {
    "userId": 142,
    "username": "admin",
    "deptId": 1,
    "deptName": "XX学校",
    "accessToken": "xxx",
    "refreshToken": "xxx",
    "expiresTime": "2026-03-05T10:00:00"
  }
}
```

---

## 二、设备管理接口

### 2. 设备注册
**接口地址：** `POST /client/inspection/register`

**请求头：**
```
Authorization: Bearer {accessToken}
Content-Type: application/x-www-form-urlencoded
```

**请求参数：**
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| deviceCode | String | 是 | 设备编码（唯一标识） |
| deviceName | String | 是 | 设备名称 |
| deptId | Long | 否 | 部门ID（登录后获取） |

**返回：**
```json
{
  "code": 0,
  "msg": "success",
  "data": {
    "id": 1,
    "deviceName": "一年级一班黑板",
    "deviceCode": "DEVICE_001",
    "deviceType": 1,
    "deptId": 1,
    "status": 1,
    "registerType": 1
  }
}
```

---

### 3. 设备心跳
**接口地址：** `POST /client/inspection/heartbeat`

**请求头：**
```
Authorization: Bearer {accessToken}
```

**请求参数：**
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| deviceCode | String | 是 | 设备编码 |

**返回：**
```json
{
  "code": 0,
  "msg": "success",
  "data": true
}
```

---

### 4. 上传截图
**接口地址：** `POST /client/inspection/screenshot/upload`

**请求头：**
```
Authorization: Bearer {accessToken}
Content-Type: multipart/form-data
```

**请求参数：**
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| deviceCode | String | 是 | 设备编码 |
| file | File | 是 | 截图文件 |

**返回：**
```json
{
  "code": 0,
  "msg": "success",
  "data": true
}
```

---

## 三、前端巡课接口

### 5. 获取在线设备列表
**接口地址：** `GET /client/inspection/device/online-list`

**请求头：**
```
Authorization: Bearer {accessToken}
```

**返回：**
```json
{
  "code": 0,
  "msg": "success",
  "data": [
    {
      "id": 1,
      "deviceName": "一年级一班黑板",
      "deviceCode": "DEVICE_001",
      "deviceType": 1,
      "classroomName": "一年级一班",
      "status": 1,
      "lastHeartbeat": "2026-03-04 15:00:00",
      "screenshotUrl": "/uploads/screenshots/DEVICE_001.jpg"
    }
  ]
}
```

---

### 6. 获取最新截图
**接口地址：** `GET /client/inspection/screenshot/{deviceCode}`

**请求头：**
```
Authorization: Bearer {accessToken}
```

**返回：**
```json
{
  "code": 0,
  "msg": "success",
  "data": {
    "deviceCode": "DEVICE_001",
    "screenshotUrl": "/uploads/screenshots/DEVICE_001.jpg",
    "screenshotTime": "2026-03-04 15:00:00"
  }
}
```

---

### 7. 开始查看（记录日志）
**接口地址：** `POST /client/inspection/view/start`

**请求头：**
```
Authorization: Bearer {accessToken}
Content-Type: application/json
```

**请求参数：**
```json
{
  "deviceId": 1,
  "viewType": 2
}
```

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| deviceId | Long | 是 | 设备ID |
| viewType | Integer | 是 | 查看类型：1-实时视频，2-截图查看 |

**返回：**
```json
{
  "code": 0,
  "msg": "success",
  "data": 1  // 日志ID
}
```

---

### 8. 结束查看（记录日志）
**接口地址：** `POST /client/inspection/view/end`

**请求头：**
```
Authorization: Bearer {accessToken}
```

**请求参数：**
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| logId | Long | 是 | 开始查看时返回的日志ID |

**返回：**
```json
{
  "code": 0,
  "msg": "success",
  "data": true
}
```

---

## 四、管理端接口（后台管理用）

### 9. 获取设备分页列表
**接口地址：** `GET /admin-api/erp/inspection-device/page`

**请求头：**
```
Authorization: Bearer {accessToken}
```

**请求参数：**
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| pageNo | Integer | 否 | 页码，默认1 |
| pageSize | Integer | 否 | 每页条数，默认10 |
| deviceName | String | 否 | 设备名称（模糊搜索） |
| deviceCode | String | 否 | 设备编码（模糊搜索） |
| status | Integer | 否 | 在线状态：0-离线，1-在线 |

**返回：**
```json
{
  "code": 0,
  "msg": "success",
  "data": {
    "list": [...],
    "total": 10
  }
}
```

---

### 10. 获取日志分页列表
**接口地址：** `GET /admin-api/erp/inspection-log/page`

**请求头：**
```
Authorization: Bearer {accessToken}
```

**请求参数：**
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| pageNo | Integer | 否 | 页码，默认1 |
| pageSize | Integer | 否 | 每页条数，默认10 |
| deviceName | String | 否 | 设备名称 |
| userName | String | 否 | 查看人姓名 |
| startTime | String | 否 | 开始时间 |
| endTime | String | 否 | 结束时间 |

---

## 五、班级管理接口

### 11. 获取班级分页列表
**接口地址：** `GET /admin-api/hc/school-class/page`

**请求头：**
```
Authorization: Bearer {accessToken}
```

**请求参数：**
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| pageNo | Integer | 否 | 页码，默认1 |
| pageSize | Integer | 否 | 每页条数，默认10 |
| className | String | 否 | 班级名称（模糊搜索） |
| grade | String | 否 | 年级 |

**返回：**
```json
{
  "code": 0,
  "msg": "success",
  "data": {
    "list": [
      {
        "id": 1,
        "classCode": "CLASS_001",
        "schoolId": 1,
        "grade": "一年级",
        "className": "一年级一班",
        "studentCount": 45,
        "classroom": "教学楼A101",
        "headTeacher": "张三",
        "status": 1
      }
    ],
    "total": 10
  }
}
```

---

### 12. 获取班级详情
**接口地址：** `GET /admin-api/hc/school-class/get?id={id}`

**请求头：**
```
Authorization: Bearer {accessToken}
```

**返回：**
```json
{
  "code": 0,
  "msg": "success",
  "data": {
    "id": 1,
    "classCode": "CLASS_001",
    "schoolId": 1,
    "grade": "一年级",
    "className": "一年级一班",
    "studentCount": 45,
    "classroom": "教学楼A101",
    "headTeacher": "张三",
    "status": 1
  }
}
```

---

### 13. 班级下拉列表
**接口地址：** `GET /admin-api/hc/school-class/simple-list`

**请求头：**
```
Authorization: Bearer {accessToken}
```

**返回：**
```json
{
  "code": 0,
  "msg": "success",
  "data": [
    {
      "id": 1,
      "className": "一年级一班"
    },
    {
      "id": 2,
      "className": "一年级二班"
    }
  ]
}
```

---

## 六、枚举值说明

| 字段 | 值 | 说明 |
|------|------|------|
| deviceType（设备类型） | 1 | 智能黑板 |
| | 2 | 智能多媒体 |
| | 3 | 其他 |
| status（在线状态） | 0 | 离线 |
| | 1 | 在线 |
| registerType（注册方式） | 1 | 主动注册 |
| | 2 | 手动添加 |
| viewType（查看类型） | 1 | 实时视频 |
| | 2 | 截图查看 |

---

## 七、客户端使用流程

1. **登录获取Token**
   - 调用 `POST /client/inspection/login`
   - 传入 `tenantName`（租户名称）、`username`、`password`
   - 保存返回的 `accessToken` 和 `deptId`

2. **注册设备**
   - 调用 `POST /client/inspection/register`
   - 传入 deviceCode、deviceName、deptId

3. **保持心跳**
   - 每隔30秒调用 `POST /client/inspection/heartbeat`
   - 传入 deviceCode

4. **上传截图**
   - 调用 `POST /client/inspection/screenshot/upload`
   - 传入 deviceCode 和截图文件
