// Shared helpers for the art-gen tools. Standard library only.

import { readFileSync } from "node:fs";

// RFC 4180 CSV: quoted fields, doubled quotes, commas and newlines inside quotes.
export function parseCsv(text) {
  const rows = [];
  let row = [];
  let field = "";
  let quoted = false;
  for (let i = 0; i < text.length; i += 1) {
    const ch = text[i];
    if (quoted) {
      if (ch === '"' && text[i + 1] === '"') {
        field += '"';
        i += 1;
      } else if (ch === '"') {
        quoted = false;
      } else {
        field += ch;
      }
    } else if (ch === '"') {
      quoted = true;
    } else if (ch === ",") {
      row.push(field);
      field = "";
    } else if (ch === "\n" || ch === "\r") {
      if (ch === "\r" && text[i + 1] === "\n") i += 1;
      row.push(field);
      field = "";
      if (row.some((cell) => cell !== "")) rows.push(row);
      row = [];
    } else {
      field += ch;
    }
  }
  if (field !== "" || row.length > 0) {
    row.push(field);
    if (row.some((cell) => cell !== "")) rows.push(row);
  }
  const [header, ...body] = rows;
  return body.map((cells) => Object.fromEntries(header.map((name, at) => [name, cells[at] ?? ""])));
}

export function readChecklist(path) {
  return parseCsv(readFileSync(path, "utf8"));
}

// "2×1" (or "2x1") to [2, 1].
export function pair(text) {
  const match = /^\s*(\d+)\s*[×x]\s*(\d+)\s*$/.exec(text);
  if (match === null) throw new Error(`not a WxH pair: ${JSON.stringify(text)}`);
  return [Number(match[1]), Number(match[2])];
}

// The same slot grammar as Slopify's `slices/admission/substitute.ts`: `{{ name }}`, one pass,
// a name with no value is left as written.
const slot = /\{\{([^{}\n]*)\}\}/g;
export function render(body, values) {
  return body.replace(slot, (whole, inside) => {
    const name = inside.trim();
    return Object.hasOwn(values, name) ? values[name] : whole;
  });
}
export function slotsOf(body) {
  const names = [];
  for (const match of body.matchAll(slot)) {
    const name = match[1].trim();
    if (name !== "" && !names.includes(name)) names.push(name);
  }
  return names;
}

// Shell-style glob over asset ids: `*` any run, `?` one character. Comma-separated alternatives.
export function globMatcher(pattern) {
  const parts = pattern
    .split(",")
    .map((p) => p.trim())
    .filter(Boolean)
    .map(
      (p) =>
        new RegExp(
          `^${p
            .split("")
            .map((ch) => (ch === "*" ? ".*" : ch === "?" ? "." : ch.replace(/[.+^${}()|[\]\\]/g, "\\$&")))
            .join("")}$`,
        ),
    );
  return (id) => parts.some((re) => re.test(id));
}
