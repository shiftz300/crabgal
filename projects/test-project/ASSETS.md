# Test asset manifest

这些资源只服务于引擎验收，刻意采用清晰边缘、明显色区和短时长，便于定位渲染错误。

| 路径 | 用途 | 来源 |
|---|---|---|
| `assets/background/calibration_day.webp` | 日景、滤镜、转场、裁切校准 | imagegen 生成后缩放为 1920×1080、WebP 编码 |
| `assets/background/calibration_sunset.webp` | 暗景、混合、粒子可读性校准 | imagegen 生成后缩放为 1920×1080、WebP 编码 |
| `assets/figure/aya_smile.webp` | 主立绘、Alpha/transform | imagegen 纯绿幕生成，chroma key 去背，lossless WebP |
| `assets/figure/aya_thinking.webp` | 表情替换、mini avatar、blend | 基于同一角色编辑生成，chroma key 去背，lossless WebP |
| `content/shared/audio/calibration_bgm.opus` | BGM/循环/滑动条 | WebGAL K 原素材《葉ノ舞》，从 WebM/Opus 无损换封装为 Ogg Opus |
| `content/shared/audio/calibration_voice.opus` | voice/重播/Backlog | WebGAL 原 `v16.wav`，转为双声道 Ogg Opus |
| `content/shared/audio/calibration_effect.opus` | 一次与循环音效 | WebGAL K 原 `click.webm`，无损换封装为 Ogg Opus |
| `assets/video/calibration_pan.mp4` | 视频时钟/跳过/主动停止 | sunset 背景的 5 秒缓慢推镜，H.264/AAC |

## Image prompts

背景提示词：

> 1920x1080 visual novel test background, empty modern seaside school rooftop at clear daytime, clean Japanese anime background art, crisp architectural edges, strong readable zones of blue sky, white walls, teal sea and warm wood, symmetrical perspective, no people, no text, no logos, restrained high-quality cel painted style

> 1920x1080 visual novel test background, empty Japanese seaside school art room at sunset, large windows showing orange sky and dark blue sea, clean anime background art, crisp vertical and horizontal edges, distinct warm and cool color regions, uncluttered foreground for character sprites, no people, no text, no logos

立绘提示词：

> Full body Japanese anime visual novel character sprite, one cheerful schoolgirl facing camera, simple navy sailor uniform with teal ribbon, hands relaxed at sides, clean cel shading, crisp thick silhouette, centered, entire body and shoes visible, no crop, no props, no text, uniform flat pure chroma green #00FF00 background, no shadow, no green clothing, no green reflections

第二表情只要求保持角色、服装和构图不变，并改为闭嘴思考姿势。
