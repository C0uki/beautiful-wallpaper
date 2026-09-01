// CPU, memory and swap, with a little history.
//
// The readings arrive on the same event the bar and the desktop widget use;
// what is different here is the graph, so this keeps a rolling buffer of what
// has come through. It starts empty and fills up — a graph that invented its
// own past would be lying about a machine nobody has watched yet.

import { useEffect, useState } from "react";
import { tr } from "../../../i18n";
import { Symbol } from "../../../widgets";
import { useShell } from "../../../shell/store";

/** How many samples the graph holds. At the default poll, about two minutes. */
const HISTORY = 60;

type Series = "cpu" | "memory" | "swap";

const SERIES: { id: Series; icon: string; label: () => string }[] = [
  { id: "cpu", icon: "planner_review", label: () => tr("CPU") },
  { id: "memory", icon: "memory", label: () => tr("RAM") },
  { id: "swap", icon: "swap_horiz", label: () => tr("Swap") },
];

export function ResourcesWidget({ interactive }: { interactive: boolean }) {
  const resources = useShell((state) => state.resources);
  const [series, setSeries] = useState<Series>("cpu");
  const [history, setHistory] = useState<Record<Series, number[]>>({
    cpu: [],
    memory: [],
    swap: [],
  });

  useEffect(() => {
    if (!resources) return;
    setHistory((previous) => ({
      cpu: [...previous.cpu, resources.cpu].slice(-HISTORY),
      memory: [...previous.memory, resources.memory].slice(-HISTORY),
      swap: [...previous.swap, resources.swap].slice(-HISTORY),
    }));
  }, [resources]);

  const values = history[series];
  const latest = values.at(-1) ?? 0;

  return (
    <div className="bw-overlay-resources">
      <div className="bw-overlay-resources-tabs" role="tablist">
        {SERIES.map((entry) => (
          <button
            key={entry.id}
            type="button"
            role="tab"
            aria-selected={entry.id === series}
            className={entry.id === series ? "selected" : ""}
            disabled={!interactive}
            onClick={() => setSeries(entry.id)}
          >
            <Symbol name={entry.icon} size={18} />
            <span>{entry.label()}</span>
          </button>
        ))}
      </div>

      <div className="bw-overlay-resources-body">
        <div className="bw-overlay-resources-readout">
          <strong>{latest.toFixed(1)}%</strong>
          <span>{tr("now")}</span>
        </div>
        <Graph values={values} />
      </div>
    </div>
  );
}

/** The history as an area, oldest on the left. */
function Graph({ values }: { values: number[] }) {
  if (values.length < 2) {
    return (
      <div className="bw-overlay-graph empty">
        <span>{tr("Collecting…")}</span>
      </div>
    );
  }

  // A fixed 0–100 scale rather than one fitted to the data: a graph that
  // rescales itself makes four per cent of CPU look like a crisis.
  const step = 100 / (values.length - 1);
  const points = values
    .map((value, index) => `${index * step},${100 - value}`)
    .join(" ");

  return (
    <svg
      className="bw-overlay-graph"
      viewBox="0 0 100 100"
      preserveAspectRatio="none"
      aria-hidden="true"
    >
      <polygon
        points={`0,100 ${points} 100,100`}
        fill="var(--primary)"
        opacity="0.25"
      />
      <polyline
        points={points}
        fill="none"
        stroke="var(--primary)"
        strokeWidth="1.5"
        vectorEffect="non-scaling-stroke"
      />
    </svg>
  );
}
