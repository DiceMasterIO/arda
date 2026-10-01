import { describe, expect, it } from "vitest";
import { ARDACOLS_LAYOUT_VERSION, parseAreaColumnsBin } from "./areaBin.ts";

/** Builds a tiny ARDACOLS buffer the way crates/arda-server/src/columnar.rs does. */
function build(width: number, height: number, cols: [string, 1 | 2 | 3, number[]][]): ArrayBuffer {
  const n = width * height;
  const table = cols.reduce((a, [name]) => a + 2 + name.length, 0);
  const dataOffset = Math.ceil((48 + table) / 4) * 4;
  const data = cols.reduce((a, [, d]) => a + n * (d === 1 ? 1 : 4), 0);
  const buf = new ArrayBuffer(dataOffset + data);
  const dv = new DataView(buf);
  const bytes = new Uint8Array(buf);
  bytes.set(new TextEncoder().encode("ARDACOLS"), 0);
  dv.setUint32(8, ARDACOLS_LAYOUT_VERSION, true);
  dv.setUint32(12, 3, true);
  dv.setInt32(16, 1, true);
  dv.setInt32(20, 2, true);
  dv.setUint32(24, 512, true);
  dv.setUint32(28, 1024, true);
  dv.setUint32(32, width, true);
  dv.setUint32(36, height, true);
  dv.setUint32(40, cols.length, true);
  dv.setUint32(44, dataOffset, true);
  let p = 48;
  for (const [name, dtype] of cols) {
    dv.setUint8(p, name.length);
    bytes.set(new TextEncoder().encode(name), p + 1);
    dv.setUint8(p + 1 + name.length, dtype);
    p += 2 + name.length;
  }
  let at = dataOffset;
  for (const [, dtype, values] of cols) {
    values.forEach((v, i) => {
      if (dtype === 1) dv.setUint8(at + i, v);
      else if (dtype === 2) dv.setUint32(at + i * 4, v, true);
      else dv.setFloat32(at + i * 4, v, true);
    });
    at += n * (dtype === 1 ? 1 : 4);
  }
  return buf;
}

describe("parseAreaColumnsBin", () => {
  it("reads the header and typed columns", () => {
    const buf = build(2, 2, [
      ["height_m", 3, [1.5, -2, 100, NaN]],
      ["terrain", 1, [0, 1, 1, 2]],
      ["river_segment", 2, [0, 7, 7, 4000000000]],
    ]);
    const a = parseAreaColumnsBin(buf);
    expect(a).toMatchObject({ contract_version: 3, ax: 1, ay: 2, gx0: 512, gy0: 1024, width: 2, height: 2 });
    expect(a.columns.height_m).toBeInstanceOf(Float32Array);
    expect(Array.from(a.columns.height_m ?? []).slice(0, 3)).toEqual([1.5, -2, 100]);
    expect(Number.isNaN(a.columns.height_m?.[3])).toBe(true);
    expect(Array.from(a.columns.terrain ?? [])).toEqual([0, 1, 1, 2]);
    expect(a.columns.river_segment).toBeInstanceOf(Uint32Array);
    expect(a.columns.river_segment?.[3]).toBe(4000000000);
  });

  it("reads the society columns of layout 2", () => {
    const buf = build(2, 1, [
      ["land_use", 1, [0, 9]],
      ["realm_id", 2, [0, 3]],
    ]);
    const a = parseAreaColumnsBin(buf);
    expect(Array.from(a.columns.land_use ?? [])).toEqual([0, 9]);
    expect(a.columns.realm_id).toBeInstanceOf(Uint32Array);
    expect(Array.from(a.columns.realm_id ?? [])).toEqual([0, 3]);
  });

  it("copies f32 columns that follow an unaligned u8 column", () => {
    const buf = build(3, 1, [
      ["terrain", 1, [0, 1, 2]],
      ["moisture", 3, [0.25, 0.5, 1]],
    ]);
    expect(Array.from(parseAreaColumnsBin(buf).columns.moisture ?? [])).toEqual([0.25, 0.5, 1]);
  });

  it("rejects bad magic, layout versions and truncation", () => {
    const good = build(2, 2, [["terrain", 1, [0, 0, 0, 0]]]);
    const badMagic = good.slice(0);
    new Uint8Array(badMagic)[0] = 0x58;
    expect(() => parseAreaColumnsBin(badMagic)).toThrow(/magic/);
    const badLayout = good.slice(0);
    new DataView(badLayout).setUint32(8, 1, true);
    expect(() => parseAreaColumnsBin(badLayout)).toThrow(/layout version/);
    expect(() => parseAreaColumnsBin(good.slice(0, good.byteLength - 1))).toThrow(/truncated/);
    expect(() => parseAreaColumnsBin(new ArrayBuffer(10))).toThrow(/header/);
  });
});
