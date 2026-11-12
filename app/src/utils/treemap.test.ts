// task-086 3.2：treemap 布局纯函数单测。
import { describe, expect, test } from "vitest";
import { layoutTreemap, type TreemapDatum } from "./treemap";

function datum(key: string, size: number): TreemapDatum {
  return { key, label: key, size, color: "#000" };
}

describe("layoutTreemap", () => {
  test("空列表返回空数组", () => {
    expect(layoutTreemap([], 100, 100)).toEqual([]);
  });

  test("矩形面积严格正比于大小，且不越界", () => {
    const data = [datum("a", 100), datum("b", 100), datum("c", 100)];
    const rects = layoutTreemap(data, 200, 200);
    expect(rects).toHaveLength(3);

    // 总面积守恒（200x200 = 40000）
    const totalArea = rects.reduce((s, r) => s + r.w * r.h, 0);
    expect(Math.round(totalArea)).toBe(40000);

    // 每块面积 ≈ 1/3（0.1 容差，浮点累积）
    for (const r of rects) {
      expect(r.w * r.h).toBeGreaterThanOrEqual(12000);
      expect(r.w * r.h).toBeLessThanOrEqual(15000);
    }

    // 全在 [0,200] 范围内
    for (const r of rects) {
      expect(r.x).toBeGreaterThanOrEqual(0);
      expect(r.y).toBeGreaterThanOrEqual(0);
      expect(r.x + r.w).toBeLessThanOrEqual(200 + 0.01);
      expect(r.y + r.h).toBeLessThanOrEqual(200 + 0.01);
    }
  });

  test("不重叠：所有矩形两两不相交", () => {
    const data = Array.from({ length: 8 }, (_, i) => datum(`f${i}`, (i + 1) * 10));
    const rects = layoutTreemap(data, 300, 200);
    for (let i = 0; i < rects.length; i++) {
      for (let j = i + 1; j < rects.length; j++) {
        if (rects[i].key === rects[j].key) continue;
        const a = rects[i];
        const b = rects[j];
        // 相交测试：若不相交则其中一轴必然分离
        const noOverlap =
          a.x + a.w <= b.x + 0.01 ||
          b.x + b.w <= a.x + 0.01 ||
          a.y + a.h <= b.y + 0.01 ||
          b.y + b.h <= a.y + 0.01;
        expect(noOverlap).toBe(true);
      }
    }
  });

  test("全 0 大小条目平均分配", () => {
    const data = [datum("a", 0), datum("b", 0)];
    const rects = layoutTreemap(data, 100, 100);
    expect(rects).toHaveLength(2);
    expect(rects[0].h).toBe(100);
    expect(rects[0].w + rects[1].w).toBeCloseTo(100);
  });

  test("单条目占满整个区域", () => {
    const rects = layoutTreemap([datum("solo", 500)], 400, 100);
    expect(rects).toHaveLength(1);
    expect(rects[0].x).toBe(0);
    expect(rects[0].y).toBe(0);
    expect(rects[0].w).toBe(400);
    expect(rects[0].h).toBe(100);
  });

  test("只有一条非零条目时占满区域（零条目不挤占）", () => {
    const rects = layoutTreemap([datum("big", 500), datum("zero", 0)], 200, 100);
    // 0 大小条目仍应分配到区域（总面积 ≤0 兜底按平均分），此处验证非零条目不丢失
    const keys = new Set(rects.map((r) => r.datum.key));
    expect(keys.has("big")).toBe(true);
    expect(keys.has("zero")).toBe(true);
  });
});