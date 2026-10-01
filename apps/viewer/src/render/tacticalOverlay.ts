// Overlay shapes in tactical square units, drawn by TacticalMap as SVG.
// Building them is pure so it can be tested without a DOM.

import type { Token } from "../api/scene.ts";
import { wallLine, wallsAround, type RulesSidecarDto, type TacticalLayoutDto } from "../api/tactical.ts";

export interface SquareRef {
  sx: number;
  sy: number;
}

export type OverlayShape =
  | {
      kind: "rect";
      x: number;
      y: number;
      w: number;
      h: number;
      fill?: string;
      /** Fill with the diagonal hatch pattern instead of a flat colour. */
      hatch?: boolean;
      stroke?: string;
      /** Screen pixels, independent of zoom. */
      strokeWidth?: number;
    }
  | { kind: "line"; x1: number; y1: number; x2: number; y2: number; stroke: string; strokeWidth: number };

export interface RulesToggles {
  difficult: boolean;
  water: boolean;
  cover: boolean;
}

export const COVER_COLOURS = { half: "#f5c542", three_quarters: "#f5862f", total: "#e5484d" } as const;

/** Fill for a rules water depth: wading (1-4 ft) is light, swimming (5+ ft) dark. */
export function waterFill(depthFt: number): string | null {
  if (depthFt <= 0) return null;
  return depthFt >= 5 ? "rgba(20, 80, 210, 0.55)" : "rgba(80, 170, 255, 0.38)";
}

/** Shapes for the enabled rules layers. */
export function rulesOverlay(rules: Pick<RulesSidecarDto, "width" | "squares">, on: RulesToggles): OverlayShape[] {
  const out: OverlayShape[] = [];
  rules.squares.forEach((cell, i) => {
    const x = i % rules.width;
    const y = Math.floor(i / rules.width);
    if (on.water && cell.water_depth_ft !== undefined) {
      const fill = waterFill(cell.water_depth_ft);
      if (fill) out.push({ kind: "rect", x, y, w: 1, h: 1, fill });
    }
    if (on.difficult && cell.difficult === true) {
      out.push({ kind: "rect", x, y, w: 1, h: 1, hatch: true });
    }
    if (on.cover && cell.cover !== undefined && cell.cover !== "none") {
      out.push({ kind: "rect", x: x + 0.12, y: y + 0.12, w: 0.76, h: 0.76, stroke: COVER_COLOURS[cell.cover], strokeWidth: 2 });
    }
  });
  return out;
}

/** Marks for the pointer layer: the pinned square outline and the shown square's walls. */
export function pointerMarks(layout: TacticalLayoutDto, shown: SquareRef | null, pinned: SquareRef | null): OverlayShape[] {
  const out: OverlayShape[] = [];
  if (pinned) out.push({ kind: "rect", x: pinned.sx, y: pinned.sy, w: 1, h: 1, stroke: "#ff3b30", strokeWidth: 2 });
  if (shown) {
    if (!pinned || pinned.sx !== shown.sx || pinned.sy !== shown.sy) {
      out.push({ kind: "rect", x: shown.sx, y: shown.sy, w: 1, h: 1, stroke: "rgba(255,255,255,0.9)", strokeWidth: 1.5 });
    }
    for (const { wall } of wallsAround(layout, shown.sx, shown.sy)) {
      const [x1, y1, x2, y2] = wallLine(wall);
      out.push({ kind: "line", x1, y1, x2, y2, stroke: wall.kind === "run" ? "#ff9f0a" : "#30d158", strokeWidth: 5 });
    }
  }
  return out;
}

/** Orange outlines on squares whose ground was edited. */
export function editMarks(edited: Iterable<SquareRef>): OverlayShape[] {
  return [...edited].map(({ sx, sy }) => ({ kind: "rect", x: sx + 0.06, y: sy + 0.06, w: 0.88, h: 0.88, stroke: "#ff9f0a", strokeWidth: 2 }));
}

/** NPC token discs (square markers): workers red, residents blue. */
export function tokenMarks(tokens: readonly Token[]): OverlayShape[] {
  return tokens.map((t) => ({
    kind: "rect",
    x: t.x + 0.18,
    y: t.y + 0.18,
    w: 0.64,
    h: 0.64,
    fill: t.kind === "worker" ? "#e5484d" : "#3e63dd",
    stroke: "#ffffff",
    strokeWidth: 2,
  }));
}
