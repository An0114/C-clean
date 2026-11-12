// task-086 3.2：矩形树图（treemap）的纯布局函数。
//
// 为什么单列文件：BigFiles.tsx 已很大，且 treemap 的「矩形递归分割」是纯函数，
// 拆出来能被单测直接覆盖（前端门禁要求 ≥224 passed）。组件只负责把布局结果
// 渲染成 SVG `<rect>`。
//
// 算法：自顶向下**等量二等分递归**（squarified 的简化版）。
// - 把条目按大小降序，按「累计面积接近一半」切出左右/上下两块，交替方向；
// - 深度优先递归，直到子区域只剩一个条目或面积过小；
// - 无需排序输入，本函数内部排序（降维：调用方传什么顺序都行）。
//
// 为什么用二等分而不是网格：
// - 文件夹大小分布极不均（一个 Downloads 可能顶十几个小目录），网格会让小目录
//   挤成一格、看不出相对大小；等量分割保证矩形面积严格正比于大小；
// - 代码简单、可预测，单测能稳定断言坐标不重叠、总面积守恒。

/** 一条可入图的数据（调用方把 `FolderStat` 映射到这里）。 */
export interface TreemapDatum {
  /** 文件夹路径（唯一键 + hover 显示）。 */
  key: string;
  /** 标签（显示在矩形内的短名字）。 */
  label: string;
  /** 子树大小（字节）。 */
  size: number;
  /** 矩形填充色。 */
  color: string;
}

/** 一个已布局的矩形。 */
export interface TreemapRect {
  datum: TreemapDatum;
  x: number;
  y: number;
  w: number;
  h: number;
  /** 深度（用于决定字号 / 是否画边框）。 */
  depth: number;
}

/** 面积过小就不再分割的阈值：矩形短边小到这个值就塞不下字了。 */
const MIN_SIDE = 3;

/**
 * 对条目做矩形树图布局。
 *
 * `data` 顺序无关（内部按 size 降序）；`width/height` 是布局区域尺寸。
 * 返回的矩形面积严格正比于各自 `size`，坐标范围在 `[0,width]×[0,height]`。
 * 传入空列表返回空数组。
 */
export function layoutTreemap(data: TreemapDatum[], width: number, height: number): TreemapRect[] {
  if (data.length === 0) return [];
  const sorted = [...data].sort((a, b) => b.size - a.size);
  const total = sorted.reduce((s, d) => s + d.size, 0);
  if (total <= 0) {
    // 全 0 大小（空目录）：平均分给每个格子，保证都有矩形可看。
    return sorted.map((d, i) => {
      const w = width / sorted.length;
      return { datum: d, x: i * w, y: 0, w, h: height, depth: 0 };
    });
  }
  const out: TreemapRect[] = [];
  split(sorted, 0, 0, width, height, total, 0, out);
  return out;
}

/**
 * 递归二分割。`sum` 是 `items` 累计大小；`rect` 是当前区域。
 * `splitArea` = 第一条的累计目标面积（≈ 一半，实际按比例定位分界）。
 */
function split(
  items: TreemapDatum[],
  x: number,
  y: number,
  w: number,
  h: number,
  total: number,
  depth: number,
  out: TreemapRect[]
): void {
  if (items.length === 0) return;
  if (items.length === 1 || w < MIN_SIDE * 2 || h < MIN_SIDE * 2) {
    out.push({ datum: items[0], x, y, w, h, depth });
    return;
  }

  // 找「累计面积到一半」的切点：让左右两半面积尽量接近。
  let acc = 0;
  let cut = 1; // 至少第一条在左边
  const half = total / 2;
  for (let i = 0; i < items.length; i++) {
    const prev = acc;
    acc += items[i].size;
    if (acc >= half) {
      // 选更接近一半的那一侧（prev vs acc）
      cut = half - prev < acc - half ? i : i + 1;
      break;
    }
  }
  cut = Math.max(1, Math.min(cut, items.length - 1));

  const left = items.slice(0, cut);
  const right = items.slice(cut);
  const leftSize = left.reduce((s, d) => s + d.size, 0);
  const rightSize = right.reduce((s, d) => s + d.size, 0);

  if (w >= h) {
    // 横向切：左 / 右两块，分界位置与面积比成正比
    const frac = total > 0 ? leftSize / total : 0.5;
    const leftW = w * frac;
    split(left, x, y, leftW, h, leftSize, depth + 1, out);
    split(right, x + leftW, y, w - leftW, h, rightSize, depth + 1, out);
  } else {
    // 纵向切：上 / 下两块
    const frac = total > 0 ? leftSize / total : 0.5;
    const topH = h * frac;
    split(left, x, y, w, topH, leftSize, depth + 1, out);
    split(right, x, y + topH, w, h - topH, rightSize, depth + 1, out);
  }
}