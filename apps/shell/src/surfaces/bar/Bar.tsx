// The bar surface.
//
// On Windows this window reserves its edge through SHAppBarMessage, so maximised
// windows keep clear of it. The layout comes from `config.bar.left/center/right`,
// the same three-slot arrangement the original uses.

import { useShell } from "../../shell/store";
import { BAR_WIDGETS } from "./widgets";
import "./bar.css";

function Slot({ names, justify }: { names: string[]; justify: string }) {
  return (
    <div className="bw-bar-slot" style={{ justifyContent: justify }}>
      {names.map((name, index) => {
        const Widget = BAR_WIDGETS[name];
        if (!Widget) return null;
        return <Widget key={`${name}-${index}`} />;
      })}
    </div>
  );
}

export function Bar() {
  const bar = useShell((state) => state.config.bar);

  return (
    <div
      className="bw-bar"
      data-style={bar.style}
      data-vertical={bar.vertical}
      data-bottom={bar.bottom}
      style={{ ["--bar-height" as string]: `${bar.height}px` }}
    >
      <Slot names={bar.left} justify="flex-start" />
      <Slot names={bar.center} justify="center" />
      <Slot names={bar.right} justify="flex-end" />
    </div>
  );
}
