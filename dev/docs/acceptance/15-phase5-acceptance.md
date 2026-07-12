# Phase 5 演出验收

## 启动

```bash
cargo run --release -- dev projects/test-project
```

点击 **START**，在选择界面选择 **Phase 5 演出**。

## 按步骤验收

1. `intro` 以黑屏、居中文字依次显示 `PHASE 5` 与“演出控制器”；`-hold` 下只能由点击推进。
2. 背景用约 0.65 秒噪声 dissolve 入场；动画时长不随刷新率变化。
3. `filmMode` 显示固定 16:9 内的上下黑边；`wait:800` 后自动关闭，期间脚本不提前显示后续内容。
4. 左侧立绘按 transition rule 从左侧淡入，随后依次 shake、前后缩放、blur；每个阻塞动画完成后才进入下一项。
5. filter 阶段立绘略暗、对比更高、降低饱和并局部模糊；清除 filter 后立即恢复，背景和 UI 不受影响。
6. 三个小立绘从左到右使用 add、multiply、screen；它们与背景合成结果应明显不同，透明边缘保持正常。
7. 背景依次显示 old-film、glitch、RGB 分离效果；临时/复杂命令与预制动画使用相同的时间曲线和清理规则。
8. 背景切换依次使用 wipe 与 dissolve：wipe 不应压缩整张图，dissolve 应是细颗粒渐显而非普通 alpha fade。
9. rain 粒子稳定下落，切换 sakura 时旧粒子同帧释放；`pixiInit` 后全部清空。
10. 最后的 exit rule 淡出立绘并自然返回主场景；存档/读档后不应留下失去所有权的粒子或阻塞状态。

## 通过标准

- `-next` 只取消脚本阻塞，不取消效果生命周期；没有 `-next` 的定时演出严格等待完成。
- 普通 alpha 图片继续走 Sprite 快路径，只有 filter/特殊 blend/材质转场进入专用 Material2D 管线。
- 同一目标只有一个预制动画所有者，新效果替换旧效果，结束后恢复基础 transform 或执行明确退出。
- 粒子数固定有界，不加载 JavaScript/PixiJS，不在稳态效果关闭后维持连续刷新。
- intro、film、wipe、dissolve 与粒子均锁定设计视口，不随窗口宽高比拉伸 UI 布局。
