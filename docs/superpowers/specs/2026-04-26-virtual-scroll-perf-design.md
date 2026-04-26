# 前端列表性能优化（虚拟滚动加固 + 渲染性能修复）设计文档

- **作者**：Aone Copilot × 蝉雨
- **日期**：2026-04-26
- **版本**：v1.0
- **状态**：Approved

## 1. 背景

Smart Clipboard 的主列表 `ClipboardList.vue` 已实现自建虚拟滚动（pageSize=50、scrollTop 驱动），但经代码审查发现 3 个高优先级问题会在 10,000+ 条记录下导致严重性能退化：

1. **EntryCard.vue 标签加载风暴**：每个卡片独立调用 `get_entry_tags` IPC，大列表时产生 N 次重复 IPC 调用
2. **ClipboardList.vue ResizeObserver 内存泄漏**：虚拟项离开可视区时 ResizeObserver 未被 disconnect，组件卸载时只 disconnect 未 clear Map 引用
3. **clipboardStore.ts 虚拟滚动高度更新**：`measuredItemHeights` 使用对象展开式更新 `{ ...old, [key]: height }`，每次单条高度变化触发所有依赖该 ref 的 computed 全量重算

## 2. 目标

- **修复标签 IPC 风暴**：改为批量预加载模式，可见条目标签一次性获取
- **修复 ResizeObserver 泄漏**：虚拟项离开可视区时立即 disconnect 并清理引用
- **修复高度更新导致的全量重渲染**：改用 Map + shallowRef + triggerRef 实现细粒度响应式

**非目标**：

- 不引入 `@tanstack/vue-virtual` 等外部虚拟滚动库（当前自建方案已满足需求，仅需加固）
- 不改变后端 IPC 接口签名
- 不改变标签数据流方向（仍由 store 统一管理）

## 3. 问题详析与方案

### 3.1 EntryCard.vue 标签加载风暴

**现状**：

```typescript
// EntryCard.vue
watch(
  () => props.entry.id,
  () => loadEntryTags(),
  { immediate: true },
);

async function loadEntryTags() {
  const cached = store.entryTagsMap[props.entry.id];
  if (cached) {
    entryTags.value = cached;
    return;
  }
  entryTags.value = await invoke<Tag[]>('get_entry_tags', { entryId: props.entry.id });
  store.setEntryTags(props.entry.id, entryTags.value);
}
```

虽然有 `entryTagsMap` 缓存，但初次加载时 50 个可见卡片会各自发起独立 IPC 调用。

**方案**：

1. 在 `clipboardStore` 新增 `batchLoadEntryTags(entryIds: number[])` 方法，过滤已缓存的 ID 后批量调用 `get_entry_tags`
2. 在 `ClipboardList.vue` 中 watch `visibleItems`，对可见条目中未缓存标签的 entry 执行批量预加载
3. `EntryCard.vue` 移除独立的 `loadEntryTags()` 调用，改为纯计算属性从 `store.entryTagsMap` 读取

### 3.2 ClipboardList.vue ResizeObserver 内存泄漏

**现状**：

```typescript
const resizeObservers = new Map<string, ResizeObserver>();

function measureVirtualItem(key, el) {
  // 创建 ResizeObserver 并加入 Map，但虚拟项离开可视区时不会清理
  if (!resizeObservers.has(key)) {
    const ro = new ResizeObserver(() => { ... });
    ro.observe(target);
    resizeObservers.set(key, ro);
  }
}

onUnmounted(() => {
  for (const ro of resizeObservers.values()) ro.disconnect();
  resizeObservers.clear(); // ✅ 组件卸载时清理，但运行期间 Map 只增不减
});
```

**方案**：

1. watch `visibleItems` 的 key 集合，对离开可视区的 key 执行 `ro.disconnect()` 并从 Map 中删除
2. 组件卸载时的清理逻辑保持不变作为兜底

### 3.3 clipboardStore.ts 展开式更新

**现状**：

```typescript
const measuredItemHeights = ref<Record<string, number>>({});

function setVirtualItemHeight(key: string, height: number) {
  if (measuredItemHeights.value[key] === height) return;
  measuredItemHeights.value = { ...measuredItemHeights.value, [key]: height };
  // ↑ 每次创建全新对象，所有依赖 measuredItemHeights 的 computed 全量重算
}
```

**方案**：
将 `measuredItemHeights` 改为 `shallowRef<Map<string, number>>`，更新时直接 `map.set(key, height)` + `triggerRef()` 手动触发响应式。同理处理 `entryTagsMap`。

## 4. 不改动清单

- **不改** 后端 Rust IPC 命令签名
- **不改** `ClipboardEntry` / `Tag` 类型定义
- **不改** 虚拟滚动的核心算法（scrollTop 驱动、OVERSCAN、layoutItems computed）
- **不改** 批量操作（多选、批量删除等）逻辑
- **不改** `pageSize` 和加载更多策略

## 5. 风险与回退

- `shallowRef + triggerRef` 方案需确保所有读取方都能正确感知变化，通过单元测试覆盖
- 批量标签预加载依赖现有 `get_entry_tags` IPC 逐条调用（暂无批量 IPC），通过 `Promise.all` 并发但受限于可见条目数（通常 < 20），性能可接受
