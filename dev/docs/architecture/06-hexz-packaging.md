# Hexz 打包与挂载

> 状态：已集成 `maincoretech/hexz_k`。Crabgal 不再定义私有容器协议。

## 边界

- `hexz_k::ResourcePack` 负责标准 `.hxz` 的索引、校验、解压、解密信息和随机读取。
- `hexz-ops` 负责 zstd 与 AES-256-GCM 分块打包；Crabgal 不复制 magic、header、block 或 CRC 语义。
- `crabgal-loader::adapter::asset::hexz` 只负责配置适配、安全路径检查和 loader mount。
- Hexz 不进入 core、ECS、UI 或脚本解析。

## 打包

```bash
cargo run --release --features hexz-pack -- pack <project> <output.hxz>
```

`hexz-ops` 是开发工具 feature，不进入默认发布引擎二进制。默认使用 64 KiB block、zstd 和
AES-256-GCM 分块加密；加密路径由 Hexz 顺序生成唯一 nonce。文件排除交给 Hexz 标准的 `.gitignore`、
`.ignore` 或 `.hexzignore`；项目必须排除 `saves/` 与生成缓存。

默认编译期资源密钥只用于防止资源被直接解压，属于弱保护而不是 DRM。发行方可在构建打包工具和
引擎时同时设置 `CRABGAL_HEXZ_PASSWORD` 覆盖默认值；客户端内置密钥始终可能被逆向获得。

## 读取

1. 使用 `ResourcePackOptions::memory_constrained()` 打开，限制解压 block cache。
2. 归档与 O(1) clone 的索引句柄在整个游戏生命周期内保持打开。
3. 配置和脚本通过统一 `ContentMount` 按需读取，不写入临时目录。
4. 图片、音频和字体由 Bevy `AssetReader` 打开 `ResourceFile`，按 loader 请求流式读取。
5. reader 支持 seek，解码器无需先复制完整文件；entry 名仍经过相对路径安全检查。

运行归档不会创建 staging、ready marker 或明文资源缓存。完整项目包暴露 `assets/` 与
`scripts/`；纯资源包只暴露 asset root。
