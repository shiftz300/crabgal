# 存档与回退（混合 Siglus + Ren'Py）

## 混合方案

| 类型 | 粒度 | 存储 | 借鉴 |
|------|------|------|------|
| Rollback（回退） | 每步自动 | 内存，~2.2KB/步，128 步上限 | Ren'Py |
| Save（存档） | 手动触发 | 磁盘，~2KB/条 | Siglus + Suika2 |

## Rollback 结构

```rust
struct RollbackEntry {
    vm_snapshot: VmState,       // ~200B
    dirty_objects: Vec<(u64, Vec<u8>)>,  // 被修改的对象状态（bincode）
    audio_state: AudioState,
}

struct RollbackLog {
    entries: VecDeque<RollbackEntry>,  // 最多 128 步
}
```

每步开销：200B VM + ~2KB dirty objects = ~2.2KB
128 步 = ~280KB（完全可常驻）

## RevertableObject（Ren'Py 模式，Rust 实现）

```rust
// Ren'Py 的 mutation 追踪在 Rust 中更简单：
// 每次 .update() 结束时自动检查 dirty 标记
impl Displayable for Sprite {
    fn update(&mut self, st: f64, at: f64) {
        self.x = at * 100.0;   // setter 自动设 dirty = true
        self.dirty = true;
    }
}

// commit 时只序列化 dirty 对象
fn commit_rollback(log: &mut RollbackLog, vm: &Vm, objects: &[Box<dyn Displayable>]) {
    let dirty: Vec<_> = objects.iter()
        .filter(|o| o.is_dirty())
        .map(|o| (o.id(), bincode::serialize(&o.state()).unwrap()))
        .collect();
    log.push(RollbackEntry { vm_snapshot: vm.snapshot(), dirty_objects: dirty, ... });
}
```

## 存档二进制格式

```
SaveFile {
    magic: [u8; 4],          // "NSVE"
    version: u16,             // 向前兼容
    timestamp: u64,
    preview: [u8; 4096],      // 缩略图 PNG
    vm_state: VmState,        // ~200B
    display_state: Vec<u8>,   // 所有对象状态
    audio_state: AudioState,
    rollback_log: Option<Vec<RollbackEntry>>, // 可选，加载后支持回退
}
```

## 回退流程

```
用户按滚轮向上：
  rollback_log.pop() → RollbackEntry
    → vm.restore(entry.vm_snapshot)
    → objects[i].restore(entry.dirty_objects[i])
    → audio.restore(entry.audio_state)
    → 继续执行
```
