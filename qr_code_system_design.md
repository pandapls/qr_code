# QR Code Generator 系统设计文档

本文档基于 `Design a QR code Generator` PDF 截图内容整理，并补充为一份更适合 Rust 大型应用练习的详细设计稿。

这份题目的本质不是“生成一张二维码图片”这么简单，而是一个：

- URL Mapping 服务
- QR Code 资源生成服务
- Redirect 服务
- 读多写少的高并发在线系统

---

## 1. 题目背景

QR code generator 是一种工具，可以接收输入数据，例如：

- URL
- email
- unique ID

然后将输入转换为 QR code 图像，也就是由黑白方块组成的二维矩阵，供相机或 App 扫描。

在这道题里，系统的主要输入是 `URL`，主要业务目标是：

1. 生成一个可被扫描的二维码
2. 用户能够管理这个二维码
3. 扫码后重定向到原始 URL

---

## 2. 功能性需求

根据 PDF 截图，功能需求包括：

1. 用户可以上传指定网址，PDF 中给出的假设是 ASCII，最长 20 字符
2. 服务会返回生成的 QR code
3. 用户可以管理自己创建过的 QR Code
4. 当用户扫描 QR code 时，系统会 redirect 到原始 URL

从 API 设计与后续流程图可以进一步展开为：

1. 创建 QR Code
2. 获取 QR Code 图片
3. 编辑 QR Code
4. 删除 QR Code
5. 通过 `qr_token` 获取原始 URL
6. 扫描二维码后通过短地址重定向到真实 URL

---

## 3. 非功能性需求

PDF 截图里明确提到以下非功能性要求：

- High Availability：服务需要 `24/7` 可用
- Redirection 延迟极低：目标 `< 100ms`
- 支持 `10 亿 QR codes`
- 支持 `1 亿用户`

由这些约束可以推导出：

- 数据必须持久化，不能只放内存
- `qr_token` 必须可快速查询
- redirect 链路必须比创建链路更优先优化
- 系统应当支持水平扩展
- 热点内容需要缓存或 CDN

---

## 4. API 设计

以下内容按照 PDF 中展示的 API 整理，并补充说明。

### 4.1 Create a QR Code

`POST /v1/qr_code`

请求体：

```json
{
  "url": "https://example.com"
}
```

响应体：

```json
{
  "qr_token": "abc123"
}
```

说明：

- 用户提交原始 URL
- 服务端验证 URL 合法性
- 生成全局唯一的 `qr_token`
- 将映射关系写入数据库
- 返回 token，客户端可继续拿 token 获取图片或管理资源

### 4.2 Get QR Code image

`GET /v1/qr_code_image/:qr_token?dimension={dimension}&color={color}&border={border}`

响应体：

```json
{
  "image_location": "https://cdn.example.com/qrs/abc123.svg"
}
```

说明：

- 根据 `qr_token` 获取二维码图片
- query parameters 用于控制图片规格：
  - `dimension`
  - `color`
  - `border`
- 服务端可以动态生成图片，也可以返回静态资源地址

### 4.3 Edit a QR Code

`PUT /v1/qr_code/:qr_token`

请求体：

```json
{
  "url": "https://example.com/updated"
}
```

说明：

- 修改 `qr_token` 对应的原始 URL
- 修改后，二维码中编码的短地址不变
- 但后续扫描会跳到新的 URL

### 4.4 Delete a QR Code

`DELETE /v1/qr_code/:qr_token`

说明：

- 删除指定二维码对应的映射关系
- 删除后该 token 不再可用

### 4.5 Get the original URL

`GET /v1/qr_code/:qr_token`

响应体：

```json
{
  "url": "https://example.com"
}
```

说明：

- 直接按 token 查询原始 URL
- 这个接口本质上也是对数据库的点查

---

## 5. 高层设计

PDF 中给出的高层架构非常清晰：

```text
Client
  <-> API Gateway
  <-> QR Code Service
  <-> Database
```

数据库旁边列出的核心表是 `QrCodes`，字段包括：

- `id`
- `user_id`
- `qr_token`
- `url`
- `created_at`

这张图已经隐含说明了两个关键事实：

1. 这是一个持久化系统，不是纯内存工具
2. `qr_token -> url` 映射是整个系统的核心数据

---

## 6. 核心业务流程

### 6.1 QR Code Creation / Edit Flow

根据 PDF 中的流程说明：

1. 用户调用 `POST /v1/qr_code`，在 request body 里传入 `url`
2. 服务先验证 URL 是否合法
3. 如果合法，系统生成一个全局唯一 token，也就是 `qr_token`
4. 在 `QrCodes` 表中创建一条记录
5. 服务返回包含 `qr_token` 的响应

PDF 还强调了一点：

- `qr_token` 必须在所有用户之间全局唯一
- 唯一性由数据库 schema 保证

编辑流程与创建流程类似：

1. 用户调用 `PUT /v1/qr_code/:qr_token`
2. 服务查找记录
3. 更新 URL
4. 保留原 token 不变

这也是为什么二维码内部应编码短地址，而不是直接编码原始 URL。

### 6.2 QR Code Retrieval Flow

PDF 中给出的图片获取流程是：

1. 用户调用  
   `GET /v1/qr_code_image/:qr_token?dimension=300&color=000000&border=10`
2. 通过 query parameters 指定图片规格
3. 服务依据 token 生成 QR code 图像
4. 然后回传 `image resource location`

这里的设计含义是：

- 图片层与映射层可以解耦
- 图片可以是动态生成
- 也可以生成后存到对象存储或 CDN

### 6.3 Redirect Flow

PDF 中明确说明二维码内部嵌入的 URL 会是类似：

`https://myqrcode.com/qr_token`

用户扫描后：

1. 浏览器访问带 `qr_token` 的短地址
2. backend 查找 `QrCodes` table
3. 返回 HTTP redirect 到原始 URL

PDF 还专门讨论了 `301` 和 `302`：

- `301 Permanent Redirect`
  - 浏览器会缓存结果
  - 未来可能绕过我们的服务

- `302 Temporary Redirect`
  - 浏览器不会长期缓存
  - 每次都能经过我们的 server

PDF 给出的选择是：

- 使用 `302`

原因是：

- 二维码拥有者可能删除或修改映射关系
- 我们希望每次扫描都取到最新状态

---

## 7. 数据模型设计

### 7.1 QrCodes 表

根据 PDF 图中的字段，建议设计为：

```sql
CREATE TABLE qr_codes (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    qr_token VARCHAR(32) NOT NULL UNIQUE,
    url TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);
```

如果先不做用户系统，可以先把 `user_id` 改成可空：

```sql
CREATE TABLE qr_codes (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NULL,
    qr_token VARCHAR(32) NOT NULL UNIQUE,
    url TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);
```

### 7.2 为什么必须有数据库

从 PDF 的设计可以直接推出数据库是必需的，因为它要承担：

- 保存二维码与 URL 的映射关系
- 支持 CRUD
- 保证 `qr_token` 全局唯一
- 为 redirect 查询提供权威数据源
- 为缓存 miss 提供回源能力

如果只放在内存里，会有这些问题：

- 服务重启后数据丢失
- 无法支持多实例共享数据
- 无法用 UNIQUE 约束处理 token collision
- 无法支撑 10 亿级记录

---

## 8. 如何生成唯一 token

PDF 在 Deep Dives 中给出了一套非常典型的设计思路。

### 8.1 基本思路

我们需要足够的 entropy 来尽量保证生成的 token 唯一。

一个可行方案是：

1. 对输入做哈希，例如 `SHA-256`
2. 得到固定长度的二进制输出
3. 再把结果编码成更短、可传播的文本
4. 截取前 N 位作为最终 token

### 8.2 为什么不能只用纯哈希

PDF 提醒了一个关键问题：

- 纯哈希是 deterministic 的
- 同一个长网址，永远映射到同一个短码

如果不希望同一个输入总是得到同一个 token，可以在输入中加入：

- `secret`
- `nonce`

这样每次生成的结果都可能不同。

### 8.3 Base62

PDF 提到可以用 `Base62` 来把二进制结果转换成更短的文本。

Base62 字符集合是：

`0-9A-Za-z`

它的优点：

- 比十六进制更短
- URL 友好
- 人类更容易复制传播

### 8.4 Collision 处理

即便 key space 很大，collision 仍然可能发生。

PDF 给出的解决思路是：

1. 在数据库中把 `qr_token` 设为 `UNIQUE`
2. 插入时如果冲突，数据库会报错
3. 服务端重新生成一个 token 再重试

这说明：

- 唯一性最终应由数据库兜底
- 业务层只是在尽量降低冲突概率

---

## 9. 如何保证 redirect 足够快

PDF 这一部分主要讲了三件事：

- Indexing
- Caching
- CDN

### 9.1 Indexing

为了避免 full table scan，需要在数据库里为 `qr_token` 建索引。

```sql
CREATE UNIQUE INDEX idx_qr_codes_qr_token ON qr_codes(qr_token);
```

如果 `qr_token` 就是主查询键，那么它甚至可以直接承担主键或唯一索引的职责。

索引的意义在于：

- 避免每次 redirect 都做全表扫描
- 将按 token 查找降为高效点查

### 9.2 Caching

PDF 明确指出这个系统是 `read-heavy`：

- `write:read ≈ 1:100`

并给了一个流量估算例子：

- `100,000,000 users * 5 redirects = 500,000,000 redirects/day`
- `500,000,000 / 86,400 ≈ 5,787 redirects/second`

因此不能让每次读取都直接打到数据库。

推荐做法：

1. 读取时先查 cache
2. cache hit 则直接返回
3. cache miss 才访问 DB
4. 写入时更新 DB
5. 删除或修改时做 cache invalidation

PDF 提到两种缓存方案：

- 每台 server instance 的 local cache
  - 实现简单
  - 但 hit rate 低

- 独立 distributed cache
  - 例如 `Redis` / `Memcached`
  - hit rate 高
  - 但系统复杂度更高

### 9.3 CDN

PDF 中把 CDN 讨论得很清楚。

适合放到 CDN 的内容有：

- QR code 图片
- 热门 QR Code 对应的静态资源
- 甚至在某些设计里，热点 redirect 结果也可以被边缘节点加速

这样做的好处：

- 用户从最近的 edge 节点获取内容
- 降低跨洲或跨地区传输延迟
- 减少回源压力
- 大幅降低整体 latency

PDF 还指出：

- QR code image 本身很适合作为 static data 放在 CDN

---

## 10. 系统扩展能力

### 10.1 Stateless Service

PDF 在 Scaling the System 部分建议：

- Server 采用 stateless 设计

意思是：

- request 处理完成后不保留业务状态
- 状态放在数据库、缓存、对象存储里

这样做的好处是：

- 更容易水平扩容
- 可以通过增加 instance 数量应对流量增长

### 10.2 Database Scaling

PDF 对数据库容量给了一个粗略估算：

- 假设有 `10 亿` 条数据
- 每条约 `200 bytes`
- 总量大约 `200GB`

它的结论是：

- 对可预见的未来，单个 DB instance 可能已经足够承载主存储

但为了 fault tolerance，可以继续演进为：

- 一个 write replica / primary
- 多个 read replica

当写主库挂掉时，可以把某个 read replica 提升为新的 write replica。

### 10.3 定期清理

PDF 还提到：

- 长时间没有被点击的 URL 可以考虑定期清理
- 可以先通知用户即将删除
- 然后通过 cron jobs 扫描数据库并移除过期记录

这属于生命周期治理能力，不是 MVP 必需，但在真实系统中很常见。

---

## 11. 这道题的系统本质

把 PDF 通读后，可以把它总结成一句话：

`QR Code Generator = QR 图片生成 + URL 映射存储 + 低延迟 redirect`

它不是一个单纯前端工具，而是一个偏基础设施风格的在线服务。

真正的核心难点不是“如何画二维码”，而是：

1. 如何生成唯一 `qr_token`
2. 如何存储和查询大规模映射关系
3. 如何把 redirect 做到足够快
4. 如何支撑高读流量
5. 如何在修改和删除场景下保持最新跳转结果

---

## 12. 对当前 Rust 项目的落地建议

针对你现在这个 Rust 练习项目，最合理的路线是：

### 12.1 第一阶段

先保留内存版仓储，目的是练：

- 模块拆分
- trait 抽象
- service 层
- axum API

### 12.2 第二阶段

按 PDF 的真实设计，把仓储切换到数据库：

- 数据库优先选 `PostgreSQL`
- Rust 里优先选 `sqlx`

### 12.3 第三阶段

继续接近 PDF 的生产设计：

- 为 `qr_token` 建唯一索引
- 增加 Redis 缓存
- 二维码图片落对象存储或动态 SVG
- 热点图片接 CDN
- redirect 使用 `302`

---

## 13. 推荐的 Rust 模块落地方式

为了与你当前项目结构对齐，建议保留：

```text
src/
  main.rs
  api.rs
  domain.rs
  service.rs
  repository.rs
  error.rs
```

职责映射如下：

- `domain.rs`
  - `QrCode`
  - `CreateQrCodeRequest`
  - `UpdateQrCodeRequest`
  - `QrCodeResponse`

- `repository.rs`
  - `QrCodeRepository trait`
  - `InMemoryQrCodeRepository`
  - 后续新增 `PostgresQrCodeRepository`

- `service.rs`
  - URL 校验
  - token 生成
  - collision 重试
  - CRUD 编排
  - redirect 查询

- `api.rs`
  - `POST /v1/qr_code`
  - `GET /v1/qr_code/:qr_token`
  - `PUT /v1/qr_code/:qr_token`
  - `DELETE /v1/qr_code/:qr_token`
  - `GET /v1/qr_code_image/:qr_token`
  - 或额外暴露 `GET /:qr_token` 处理实际跳转

- `main.rs`
  - 初始化 repository
  - 初始化 service
  - 启动 axum

---

## 14. 文档结论

通过 PDF 截图可以明确确认：

1. 这个系统最终应该接数据库
2. `qr_token` 的全局唯一性应由数据库约束保证
3. redirect 的低延迟依赖索引、缓存和 CDN
4. 服务应保持 stateless，方便水平扩展
5. `302 redirect` 比 `301` 更适合这个题目的可编辑二维码场景

如果把这道题做成一个完整的 Rust 学习项目，最好的节奏不是一开始就堆满基础设施，而是：

1. 先用内存仓储练清楚分层
2. 再换 PostgreSQL
3. 再加 Redis
4. 最后再补 CDN、对象存储和清理策略

这样既对齐 PDF 的设计，也适合你的学习节奏。
