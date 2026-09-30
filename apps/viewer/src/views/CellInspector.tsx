import type { CellSample } from "@arda";
import { describeCell } from "../format.ts";

export function CellInspector({ cell, onClose }: { cell: CellSample; onClose: () => void }) {
  return (
    <aside className="inspector" aria-label="Cell inspector" data-testid="inspector">
      <div className="inspector-head">
        <h2>
          Cell {cell.gx}, {cell.gy}
        </h2>
        <button type="button" onClick={onClose} aria-label="Close inspector">
          ×
        </button>
      </div>
      {describeCell(cell).map((section) => (
        <section key={section.title}>
          <h3>{section.title}</h3>
          <dl>
            {section.rows.map((r) => (
              <div className="row" key={r.label}>
                <dt>{r.label}</dt>
                <dd>{r.value}</dd>
              </div>
            ))}
          </dl>
        </section>
      ))}
      <p className="inspector-links">
        <a href={`#/area?ax=${cell.ax}&ay=${cell.ay}`}>Open area {cell.ax},{cell.ay}</a>
        <br />
        <a href={`#/cell?gx=${cell.gx}&gy=${cell.gy}`} data-testid="open-tactical">
          Open tactical map here
        </a>
      </p>
      <details>
        <summary>Raw CellSample JSON</summary>
        <pre>{JSON.stringify(cell, null, 2)}</pre>
      </details>
    </aside>
  );
}
