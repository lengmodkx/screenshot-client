# 截图上传与巡视功能设计方案

## 一、现状梳理

当前项目架构：
- **Tauri 桌面客户端**：负责截图、视频流推送、软件监控
- **后端服务**：外部 Java 后端（API 地址可配置，默认 `http://172.16.10.11:48080`）
- **Web 管理端**：外部前端项目（路径 `/inspection/hc/inspection/device/index`）

现有截图逻辑：
1. **实时视频推送**：摄像头模式 500ms/次，屏幕截图模式 1000ms/次 → `POST /client/inspection/video/push`
2. **静态截图上传**：每 5 分钟上传一次 → `POST /client/inspection/uploadScreenshot`，文件名固定为 `screenshot.jpg`

---

## 二、整体架构设计（基于新版一键上传接口）

```
┌─────────────────┐      每分钟截图       ┌─────────────────────────────┐
│  Tauri 客户端    │ ───────────────────► │   Java 后端                  │
│  (Rust + React) │  POST /admin-api/    │  (SpringBoot + Yudao)        │
│                 │  erp/inspection-     │                              │
│                 │  device/file/        │  ① 接收文件 → OSS 存储        │
│                 │  upload-direct       │  ② infra_file 落库           │
└─────────────────┘                      │  ③ erp_inspection_device_file │
                                         │    关联表落库                 │
                                         └────────┬────────────────────┘
                                                  │
                              查询设备文件列表      │ 15天清理
┌─────────────────┐  GET /admin-api/erp/inspection-device/file/page  │ (定时任务)
│   Web 管理端     │ ◄─────────────────────────────────────────────────┘
│  (Vue/React)    │
└─────────────────┘
```

客户端专用上传接口文档：`docs/api-docs/device-file-upload-api-for-client.md`

---

## 三、Tauri 客户端改造（已完成）

### 3.1 前端：修改截图上传间隔（1分钟）

文件：`src/App.tsx`

```tsx
// 截图上传：每1分钟上传一次（用于后台查看静态画面）
uploadScreenshotFile();
screenshotTimerRef.current = window.setInterval(() => {
  uploadScreenshotFile();
}, 60 * 1000); // 1分钟
```

### 3.2 前端：增加上传防重锁

避免 1 分钟间隔内因网络延迟导致并发上传：

```tsx
const isUploadingScreenshot = useRef(false);

const uploadScreenshotFile = async () => {
  if (!config || !isLoggedIn || !config.is_registered || !isOnline) return;
  if (isUploadingScreenshot.current) return;

  const imageData = currentImageRef.current;
  if (!imageData) {
    console.log("No image available for screenshot upload");
    return;
  }

  isUploadingScreenshot.current = true;
  try {
    const url = await invoke<string>("upload_screenshot_file", { imageData });
    console.log("Screenshot uploaded:", url);
    setStatusMessage(`截图已上传: ${new Date().toLocaleTimeString()}`);
  } catch (e) {
    console.error("Screenshot upload failed:", e);
  } finally {
    isUploadingScreenshot.current = false;
  }
};
```

### 3.3 Rust 端：改造为客户端专用一键上传接口

文件：`src-tauri/src/lib.rs` → `upload_screenshot_file`

使用 **巡课设备文件一键上传接口**，一次性完成 OSS 存储 → `infra_file` 落库 → `erp_inspection_device_file` 关联落库。

```rust
let upload_url = format!("{}/admin-api/erp/inspection-device/file/upload-direct", config.api_url);

let form = multipart::Form::new()
    .text("deviceId", device_id.to_string())
    .part("file", part);

let response = client
    .post(&upload_url)
    .header("Authorization", format!("Bearer {}", token))
    .multipart(form)
    .send()
    .await?;
```

- **URL**：`POST /admin-api/erp/inspection-device/file/upload-direct`
- **参数**：`deviceId`（设备编号，来自 `config.device_id`）+ `file`（JPEG 文件二进制）
- **返回**：`fileUrl`（可直接访问的截图 URL）
- **后端自动完成**：OSS 上传 → `infra_file` 表落库 → `erp_inspection_device_file` 关联表落库

### 3.4 Rust 端：新增本地截图清理命令（兜底）

如果后端不负责清理，客户端本地也应定时清理缓存的截图文件：

```rust
#[tauri::command]
async fn cleanup_local_screenshots(retention_days: i64) -> Result<u32, String> {
    let save_dir = dirs::picture_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Screenshots");
    
    if !save_dir.exists() {
        return Ok(0);
    }

    let cutoff = Local::now() - chrono::Duration::days(retention_days);
    let mut deleted = 0u32;

    for entry in fs::read_dir(&save_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let metadata = entry.metadata().map_err(|e| e.to_string())?;
        
        if metadata.is_file() {
            let modified = metadata.modified()
                .map_err(|e| e.to_string())?;
            let modified_dt = chrono::DateTime::<Local>::from(modified);
            
            if modified_dt < cutoff {
                fs::remove_file(entry.path()).map_err(|e| e.to_string())?;
                deleted += 1;
            }
        }
    }

    Ok(deleted)
}
```

---

## 四、后端 API 设计（Java/SpringBoot）

后端已有 **客户端专用一键上传接口**：

```http
POST /admin-api/erp/inspection-device/file/upload-direct
```

该接口已经自动完成了：
- 文件上传至 OSS/本地存储/S3
- `infra_file` 文件表落库
- `erp_inspection_device_file` 设备关联表落库

因此后端只需补充 **2 个能力**：

### 4.1 设备文件分页查询接口（供 Web 端"巡视"使用）

如果后端已经提供 `erp_inspection_device_file` 相关的分页查询接口，Web 端直接调用即可。如果没有，可参考以下设计：

```java
@GetMapping("/admin-api/erp/inspection-device/file/page")
@PreAuthorize("@ss.hasPermission('erp:inspection-device:query')")
public CommonResult<PageResult<InspectionDeviceFileRespVO>> getDeviceFilePage(
        @Valid InspectionDeviceFilePageReqVO pageReqVO) {
    return CommonResult.success(deviceFileService.getDeviceFilePage(pageReqVO));
}
```

**请求参数（Query）**：

| 参数名 | 类型 | 必填 | 说明 |
| :--- | :--- | :--- | :--- |
| `deviceId` | Long | 是 | 设备编号 |
| `pageNo` | Integer | 是 | 页码，从 1 开始 |
| `pageSize` | Integer | 是 | 每页条数，建议 20 |
| `createTime` | String[] | 否 | 创建时间范围 `[开始, 结束]` |

**响应字段**：

| 字段名 | 说明 |
| :--- | :--- |
| `id` | 设备文件关联记录编号 |
| `deviceId` | 设备编号 |
| `fileId` | 文件表编号 |
| `fileName` | 原始文件名 |
| `fileUrl` | 文件访问 URL |
| `fileType` | MIME 类型 |
| `fileSize` | 文件大小 |
| `createTime` | 创建时间 |

### 4.2 15 天自动清理定时任务

利用已有的 `erp_inspection_device_file` + `infra_file` 表进行清理：

```java
@Component
@Slf4j
public class DeviceFileCleanupJob {

    @Autowired
    private ErpInspectionDeviceFileMapper deviceFileMapper;
    @Autowired
    private InfraFileService fileService;

    @Scheduled(cron = "0 0 2 * * ?") // 每天凌晨 2 点执行
    public void cleanup() {
        LocalDateTime cutoffTime = LocalDateTime.now().minusDays(15);
        
        // 1. 查询 15 天前的设备截图关联记录
        List<ErpInspectionDeviceFileDO> oldList = deviceFileMapper.selectList(
            new LambdaQueryWrapper<ErpInspectionDeviceFileDO>()
                .le(ErpInspectionDeviceFileDO::getCreateTime, cutoffTime)
        );
        
        if (CollUtil.isEmpty(oldList)) {
            log.info("[DeviceFileCleanup] 没有需要清理的过期截图文件");
            return;
        }
        
        Set<Long> fileIds = oldList.stream()
            .map(ErpInspectionDeviceFileDO::getFileId)
            .collect(Collectors.toSet());
        
        // 2. 调用 Yudao 的文件删除接口清理物理文件
        for (Long fileId : fileIds) {
            try {
                fileService.deleteFile(fileId);
            } catch (Exception e) {
                log.error("[DeviceFileCleanup] 删除文件失败, fileId={}: {}", fileId, e.getMessage());
            }
        }
        
        // 3. 删除设备文件关联记录
        int deleted = deviceFileMapper.delete(
            new LambdaQueryWrapper<ErpInspectionDeviceFileDO>()
                .le(ErpInspectionDeviceFileDO::getCreateTime, cutoffTime)
        );
        
        log.info("[DeviceFileCleanup] 清理完成: 删除文件 {} 个, 删除关联记录 {} 条", fileIds.size(), deleted);
    }
}
```

> 如果底层存储（如 MinIO）已经配置了 15 天的 Lifecycle Policy，可以只做步骤 3（删数据库记录），物理文件让存储层自动清理。

---

## 五、Web 管理端"巡视"功能设计

### 5.1 页面改造点

在 `/inspection/hc/inspection/device/index` 页面：

1. **设备列表行操作**："巡视"按钮点击后弹出抽屉/弹窗
2. **弹窗内容**：
   - 顶部：设备名称 + 日期选择器（默认最近 1 天）
   - 中部：按时间倒序排列的截图缩略图网格/时间轴
   - 底部：分页控件

### 5.2 前端组件伪代码

```vue
<template>
  <a-drawer :visible="visible" title="设备截图巡视" width="900" @close="close">
    <div class="screenshot-header">
      <span>{{ currentDevice.deviceName }}</span>
      <a-range-picker v-model="dateRange" @change="loadFiles" />
    </div>
    
    <div v-if="fileList.length > 0" class="screenshot-grid">
      <div v-for="item in fileList" :key="item.id" class="screenshot-item">
        <img 
          :src="item.fileUrl" 
          @click="previewImage(item.fileUrl)"
          class="screenshot-thumb"
        />
        <p class="screenshot-time">{{ item.createTime }}</p>
      </div>
    </div>
    
    <a-empty v-else description="暂无截图记录" />
    
    <a-pagination 
      v-model:current="pageNo" 
      :total="total" 
      :pageSize="pageSize"
      @change="loadFiles"
    />
  </a-drawer>
</template>

<script setup>
const visible = ref(false);
const currentDevice = ref({});
const fileList = ref([]);
const dateRange = ref([dayjs().subtract(1, 'day'), dayjs()]);
const pageNo = ref(1);
const pageSize = 20;

const openDrawer = (device) => {
  currentDevice.value = device;
  visible.value = true;
  loadFiles();
};

const loadFiles = async () => {
  const res = await http.get('/admin-api/erp/inspection-device/file/page', {
    params: {
      deviceId: currentDevice.value.id,
      pageNo: pageNo.value,
      pageSize: pageSize,
      createTime: dateRange.value 
        ? [dateRange.value[0].format('YYYY-MM-DD HH:mm:ss'), dateRange.value[1].format('YYYY-MM-DD HH:mm:ss')]
        : undefined
    }
  });
  fileList.value = res.data.list;
  total.value = res.data.total;
};
</script>
```

### 5.3 截图预览优化

- **缩略图**：如果 OSS 支持图片处理参数（如阿里云 OSS `?x-oss-process=image/resize,w_200`），可在列表中展示缩略图
- **大图预览**：点击缩略图后使用 `Image.preview()` 或 LightBox 展示原图

---

## 六、推荐实现顺序

| 优先级 | 任务 | 负责方 | 说明 |
|--------|------|--------|------|
| P0 | Tauri 端改为 1 分钟上传 | 客户端 | **已完成**：`App.tsx` + `lib.rs` 已改 |
| P0 | 后端确认一键上传接口可用 | Java 后端 | 确保 `POST /admin-api/erp/inspection-device/file/upload-direct` 正常 |
| P1 | 后端提供设备文件分页查询 | Java 后端 | 查询 `erp_inspection_device_file` 表 |
| P1 | Web 管理端"巡视"弹窗 | Web 前端 | 展示截图时间轴/网格 |
| P1 | 15 天清理定时任务 | Java 后端 | 删除过期 `infra_file` + `erp_inspection_device_file` |
| P2 | Tauri 本地缓存清理 | 客户端 | 每天启动时清理本地 15 天前文件 |

---

## 七、关键注意点

1. **截图频率与存储成本**：1 分钟一张截图，单设备每天 1440 张。若每张 100KB，则每天约 140MB，15 天约 2.1GB。需要评估 OSS/MinIO 容量和费用。
2. **压缩策略**：当前 Tauri 端已将 JPEG 压缩到 100KB 以内，建议保持。
3. **并发控制**：前端已加 `isUploadingScreenshot` 锁，避免 1 分钟间隔内因网络延迟导致并发上传。
4. **deviceId 来源**：Tauri 端上传时使用 `config.device_id`，该值在设备注册后由后端返回并持久化到本地配置文件中。如果 `device_id` 为空，上传会报错"设备未注册"。
5. **失败重试**：当前接口在 Token 过期时会自动重登后重试，但断网时不会自动重试。后续如需增强可靠性，可在 Rust 端增加本地队列或指数退避重试。
