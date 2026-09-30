// Parser for the `ARDACOLS` v1 binary area layout (`/v1/area/{ax}/{ay}/cells?format=bin`).
// See crates/arda-server/API.md. Columns are wrapped as zero-copy typed-array views.

import type { AreaColumns, Legend } from "@arda";

export type ColumnName = keyof AreaColumns;
export type ColumnArray = Uint8Array | Uint32Array | Float32Array;

export interface BinAreaColumns {
  contract_version: number;
  ax: number;
  ay: number;
  gx0: number;
  gy0: number;
  width: number;
  height: number;
  /** Columns by contract name. f32 columns use NaN for null. */
  columns: Partial<Record<ColumnName, ColumnArray>>;
}

const MAGIC = "ARDACOLS";
export const ARDACOLS_LAYOUT_VERSION = 1;
const DTYPE_U8 = 1;
const DTYPE_U32 = 2;
const DTYPE_F32 = 3;

export function parseAreaColumnsBin(buf: ArrayBuffer): BinAreaColumns {
  if (buf.byteLength < 48) throw new Error("ARDACOLS: truncated header");
  const dv = new DataView(buf);
  const bytes = new Uint8Array(buf);
  const magic = String.fromCharCode(...bytes.subarray(0, 8));
  if (magic !== MAGIC) throw new Error(`ARDACOLS: bad magic ${JSON.stringify(magic)}`);
  const layout = dv.getUint32(8, true);
  if (layout !== ARDACOLS_LAYOUT_VERSION) throw new Error(`ARDACOLS: unknown layout version ${layout}`);

  const head = {
    contract_version: dv.getUint32(12, true),
    ax: dv.getInt32(16, true),
    ay: dv.getInt32(20, true),
    gx0: dv.getUint32(24, true),
    gy0: dv.getUint32(28, true),
    width: dv.getUint32(32, true),
    height: dv.getUint32(36, true),
  };
  const count = dv.getUint32(40, true);
  const dataOffset = dv.getUint32(44, true);
  if (dataOffset % 4 !== 0) throw new Error("ARDACOLS: data offset not 4-aligned");

  const n = head.width * head.height;
  const columns: Partial<Record<ColumnName, ColumnArray>> = {};
  let p = 48;
  let at = dataOffset;
  for (let i = 0; i < count; i++) {
    if (p >= dataOffset) throw new Error("ARDACOLS: column table overruns data offset");
    const len = dv.getUint8(p);
    const name = String.fromCharCode(...bytes.subarray(p + 1, p + 1 + len));
    const dtype = dv.getUint8(p + 1 + len);
    p += 2 + len;
    const size = dtype === DTYPE_U8 ? n : n * 4;
    if (at + size > buf.byteLength) throw new Error(`ARDACOLS: column ${name} truncated`);
    let arr: ColumnArray;
    switch (dtype) {
      case DTYPE_U8:
        arr = new Uint8Array(buf, at, n);
        break;
      case DTYPE_U32:
        arr = at % 4 === 0 ? new Uint32Array(buf, at, n) : new Uint32Array(buf.slice(at, at + size));
        break;
      case DTYPE_F32:
        arr = at % 4 === 0 ? new Float32Array(buf, at, n) : new Float32Array(buf.slice(at, at + size));
        break;
      default:
        throw new Error(`ARDACOLS: column ${name} has unknown dtype ${dtype}`);
    }
    columns[name as ColumnName] = arr;
    // Columns follow each other with no gaps. With 512 x 512 areas every
    // column stays 4-byte aligned, so the views above are zero-copy.
    at += size;
  }
  return { ...head, columns };
}

/**
 * Enum code tables for the binary form, which carries no legend. They match
 * the JSON body's `legend` and the order documented in API.md.
 */
export const BIN_LEGEND: Legend = {
  terrain: ["sea", "land", "lake"],
  cover: ["bare", "grass", "scrub", "forest", "marsh", "rock", "ice"],
  road: ["none", "track", "road", "highway"],
};
