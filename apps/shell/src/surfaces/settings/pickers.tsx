// Picking a colour or a font instead of typing one.
//
// Colours are a row of wells, as the Mac's accent colour is: the ones worth
// offering, and a last well that opens the system colour picker for anything
// else. Fonts are a list of the families actually installed, each drawn in
// itself, so a name is chosen by what it looks like.

import { useMemo } from "react";
import { Symbol } from "../../widgets";
import { tr } from "../../i18n";

export interface Swatch {
  /** What is written to the config. */
  value: string;
  /** What the well is painted with. */
  paint: string;
  /** English, translated when drawn. */
  label: string;
}

export function ColorPicker({
  value,
  swatches,
  onChange,
}: {
  value: string;
  swatches: Swatch[];
  onChange: (value: string) => void;
}) {
  const current = value.trim().toLowerCase();
  const custom = !swatches.some(
    (swatch) => swatch.value.toLowerCase() === current,
  );
  return (
    <div className="bw-color-picker" role="radiogroup">
      {swatches.map((swatch) => (
        <button
          key={swatch.value}
          type="button"
          role="radio"
          aria-checked={swatch.value.toLowerCase() === current}
          aria-label={tr(swatch.label)}
          title={tr(swatch.label)}
          className="bw-color-well"
          style={{ background: swatch.paint }}
          onClick={() => onChange(swatch.value)}
        >
          {swatch.value === "" ? <Symbol name="wallpaper" size={14} /> : null}
        </button>
      ))}
      {/* Anything else: the colour input is the system's own picker, laid
          over a rainbow well. Shows the chosen colour once there is one. */}
      <label
        className="bw-color-well bw-color-custom"
        role="radio"
        aria-checked={custom}
        title={tr("Other colour…")}
        style={
          custom && /^#[0-9a-f]{6}$/.test(current)
            ? { background: current }
            : undefined
        }
      >
        <input
          type="color"
          aria-label={tr("Other colour…")}
          value={/^#[0-9a-f]{6}$/.test(current) ? current : "#4f8cff"}
          onChange={(event) => onChange(event.target.value)}
        />
      </label>
    </div>
  );
}

/** Families worth offering, by what they are for. Only the installed ones
 *  are listed. ponytail: a curated list checked against what renders; ask
 *  DirectWrite for every installed family if a font a user has is missing. */
const FAMILIES = {
  sans: [
    "Segoe UI Variable Text",
    "Segoe UI Variable Display",
    "Segoe UI",
    "Aptos",
    "Yu Gothic UI",
    "Yu Gothic",
    "Meiryo UI",
    "Meiryo",
    "BIZ UDPGothic",
    "MS UI Gothic",
    "Noto Sans JP",
    "Noto Sans",
    "Inter",
    "Arial",
    "Calibri",
    "Verdana",
    "Tahoma",
    "Trebuchet MS",
    "Bahnschrift",
    "Corbel",
    "Candara",
  ],
  mono: [
    "Cascadia Code",
    "Cascadia Mono",
    "Consolas",
    "JetBrains Mono",
    "Fira Code",
    "Source Code Pro",
    "BIZ UDGothic",
    "MS Gothic",
    "Lucida Console",
    "Courier New",
  ],
};

const GENERIC = { sans: "sans-serif", mono: "monospace" };

/** Whether a family is installed: text set in it measures differently from
 *  the same text in each fallback it could otherwise land on. */
function installed(family: string): boolean {
  const context = document.createElement("canvas").getContext("2d");
  if (!context) return true;
  const sample = "mmmmmmmmmmlli1WQ@あ漢";
  return ["monospace", "serif", "sans-serif"].some((fallback) => {
    context.font = `32px ${fallback}`;
    const bare = context.measureText(sample).width;
    context.font = `32px "${family}", ${fallback}`;
    return context.measureText(sample).width !== bare;
  });
}

/** The first family in a stack, without quotes. */
export function familyOf(stack: string): string {
  return (stack.split(",")[0] ?? "").trim().replace(/^["']|["']$/g, "");
}

export function FontPicker({
  value,
  kind,
  onChange,
}: {
  value: string;
  kind: keyof typeof FAMILIES;
  onChange: (value: string) => void;
}) {
  const families = useMemo(() => FAMILIES[kind].filter(installed), [kind]);
  const chosen = familyOf(value);
  // A family typed into the file before, or one this list does not know,
  // stays selectable rather than being shown as something else.
  const listed = families.includes(chosen) ? families : [chosen, ...families];

  return (
    <select
      value={chosen}
      style={{ fontFamily: `"${chosen}", ${GENERIC[kind]}` }}
      // Written as a stack ending in the generic family, so a machine
      // without it still gets a font of the right kind.
      onChange={(event) => onChange(`${event.target.value}, ${GENERIC[kind]}`)}
    >
      {listed.map((family) => (
        <option
          key={family}
          value={family}
          style={{ fontFamily: `"${family}", ${GENERIC[kind]}` }}
        >
          {family}
        </option>
      ))}
    </select>
  );
}
