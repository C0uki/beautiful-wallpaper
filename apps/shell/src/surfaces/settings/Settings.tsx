// The settings screen.
//
// Every row on it is generated from the Rust schema, so a config key added in
// any future change has a control here without anybody touching this file.
// What this file does is decide how a row is *drawn* — a toggle, a slider, a
// choice — and how two hundred of them are made findable.
//
// Findable is the harder half. A generated form is complete by construction
// and unusable if that is all it is, so there is a search across every page
// and every page is grouped by the schema's own nesting.

import { useCallback, useEffect, useMemo, useState } from "react";
import type { Field } from "@bw/core";
import { configSchema } from "@bw/core";
import { IconButton, SearchField, Slider, Switch, Symbol } from "../../widgets";
import { tr } from "../../i18n";
import { actions, connect, useShell } from "../../shell/store";
import { describeError } from "../../shell/errors";
import { OVERRIDES } from "./overrides";
import { PAGES, pageFor } from "./pages";
import { Presets } from "./Presets";
import "./settings.css";

/** Reads a dotted path out of the config. */
function valueAt(config: unknown, path: string): unknown {
  return path
    .split(".")
    .reduce<unknown>(
      (node, key) =>
        node && typeof node === "object"
          ? (node as Record<string, unknown>)[key]
          : undefined,
      config,
    );
}

/** Everything a search term should match: the label, the group, the path. */
function matches(field: Field, term: string): boolean {
  if (!term) return true;
  const haystack = `${field.label} ${field.group} ${field.path}`.toLowerCase();
  return term
    .toLowerCase()
    .split(/\s+/)
    .filter(Boolean)
    .every((word) => haystack.includes(word));
}

export function Settings() {
  const ready = useShell((state) => state.ready);
  const open = useShell((state) => state.states.settingsOpen);
  const config = useShell((state) => state.config);

  const [page, setPage] = useState(PAGES[0]!.id);
  const [term, setTerm] = useState("");
  const [problem, setProblem] = useState<string | null>(null);

  useEffect(() => {
    void connect();
  }, []);

  useEffect(() => {
    if (open) {
      setTerm("");
      setProblem(null);
    }
  }, [open]);

  // Grouped once rather than per render: two hundred rows filtered on every
  // keystroke is the one thing here that could feel slow.
  const byPage = useMemo(() => {
    const found = new Map<string, Field[]>();
    for (const field of configSchema) {
      const home = pageFor(field.section);
      if (!home) continue;
      found.set(home.id, [...(found.get(home.id) ?? []), field]);
    }
    return found;
  }, []);

  const searching = term.trim().length > 0;
  // While searching the page list stops meaning anything — the answer is
  // wherever it is — so every page's rows are shown together.
  const shown = useMemo(() => {
    const fields = searching
      ? configSchema.filter((field) => pageFor(field.section))
      : (byPage.get(page) ?? []);
    return fields.filter((field) => matches(field, term));
  }, [byPage, page, searching, term]);

  const set = useCallback(async (path: string, value: unknown) => {
    try {
      await actions.setConfigValue(path, value);
      setProblem(null);
    } catch (error) {
      // A refused write is worth saying: the file may be read-only, or the
      // value may not be one the schema accepts.
      setProblem(describeError(error));
    }
  }, []);

  const close = useCallback(
    () => void actions.setState("settingsOpen", false),
    [],
  );

  if (!ready || !open) return null;

  const current = PAGES.find((found) => found.id === page);

  return (
    <div
      className="bw-settings"
      role="dialog"
      aria-label={tr("Settings")}
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) close();
      }}
    >
      <div className="bw-settings-panel">
        <nav className="bw-settings-pages" aria-label={tr("Settings pages")}>
          <div className="bw-settings-search">
            <SearchField
              value={term}
              placeholder={tr("Search every setting")}
              onChange={setTerm}
            />
          </div>
          {PAGES.map((entry) => (
            <button
              key={entry.id}
              type="button"
              className={!searching && entry.id === page ? "selected" : ""}
              onClick={() => {
                setTerm("");
                setPage(entry.id);
              }}
            >
              <Symbol name={entry.icon} size={20} />
              <span>{entry.title()}</span>
            </button>
          ))}
        </nav>

        <div className="bw-settings-body">
          <header className="bw-settings-head">
            <h1>
              {searching
                ? tr("%1 settings match").replace("%1", String(shown.length))
                : (current?.title() ?? "")}
            </h1>
            <IconButton icon="close" size={32} label="Close" onClick={close} />
          </header>

          {problem ? <p className="bw-settings-problem">{problem}</p> : null}
          {!searching && current?.caution ? (
            <p className="bw-settings-caution">
              <Symbol name="warning" size={18} />
              {current.caution()}
            </p>
          ) : null}

          <div className="bw-settings-rows">
            {/* A page that is not a list of settings draws itself. Presets are
                the whole config under a name rather than one key each, so
                there is nothing for the generated form to generate. Searching
                still puts the rows back: the answer is wherever it is. */}
            {!searching && current?.custom === "presets" ? (
              <Presets />
            ) : shown.length ? (
              <Rows
                fields={shown}
                config={config}
                onSet={set}
                grouped={!searching}
              />
            ) : (
              <p className="bw-settings-empty">{tr("Nothing matched")}</p>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

/** The rows, with a heading whenever the group changes. */
function Rows({
  fields,
  config,
  onSet,
  grouped,
}: {
  fields: Field[];
  config: unknown;
  onSet: (path: string, value: unknown) => void;
  grouped: boolean;
}) {
  let lastGroup: string | null = null;
  let lastSection: string | null = null;

  return (
    <>
      {fields.map((field) => {
        const headings: React.ReactNode[] = [];
        if (grouped && field.section !== lastSection) {
          lastSection = field.section;
          lastGroup = null;
          headings.push(
            <h2 key={`s-${field.section}`} className="bw-settings-section">
              {field.section}
            </h2>,
          );
        }
        if (grouped && field.group !== lastGroup) {
          lastGroup = field.group;
          if (field.group) {
            headings.push(
              <h3
                key={`g-${field.section}-${field.group}`}
                className="bw-settings-group"
              >
                {field.group}
              </h3>,
            );
          }
        }

        return (
          <div key={field.path} className="bw-settings-block">
            {headings}
            <Row field={field} config={config} onSet={onSet} />
          </div>
        );
      })}
    </>
  );
}

function Row({
  field,
  config,
  onSet,
}: {
  field: Field;
  config: unknown;
  onSet: (path: string, value: unknown) => void;
}) {
  const override = OVERRIDES[field.path];
  const value = valueAt(config, field.path);

  return (
    <div className="bw-settings-row">
      <div className="bw-settings-label">
        <span>{field.label}</span>
        {override?.hint ? <em>{override.hint()}</em> : null}
        <code>{field.path}</code>
      </div>
      <div className="bw-settings-control">
        <Control
          field={field}
          value={value}
          override={override}
          onSet={onSet}
        />
      </div>
    </div>
  );
}

function Control({
  field,
  value,
  override,
  onSet,
}: {
  field: Field;
  value: unknown;
  override: (typeof OVERRIDES)[string] | undefined;
  onSet: (path: string, value: unknown) => void;
}) {
  // A curated choice wins over the generated kind: the schema knows a bar
  // style is a string, not that it is one of four.
  if (override?.choices) {
    return (
      <select
        value={String(value ?? "")}
        onChange={(event) =>
          onSet(
            field.path,
            field.kind === "integer"
              ? Number(event.target.value)
              : event.target.value,
          )
        }
      >
        {override.choices.map((choice) => (
          <option key={choice.value} value={choice.value}>
            {choice.label()}
          </option>
        ))}
      </select>
    );
  }

  switch (field.kind) {
    case "toggle":
      return (
        <Switch
          checked={Boolean(value)}
          label={field.label}
          onChange={(next) => onSet(field.path, next)}
        />
      );

    case "integer":
    case "decimal": {
      const current = typeof value === "number" ? value : 0;
      if (override?.range) {
        return (
          <div className="bw-settings-slider">
            <Slider
              value={current}
              min={override.range.min}
              max={override.range.max}
              step={override.range.step}
              label={field.label}
              onChange={(next) => onSet(field.path, next)}
            />
            <span>{current}</span>
          </div>
        );
      }
      return (
        <input
          type="number"
          value={current}
          step={field.kind === "decimal" ? 0.01 : 1}
          onChange={(event) => {
            const next = Number(event.target.value);
            // A half-typed number is not a value to write: "-" and "" both
            // parse to something the schema would reject.
            if (Number.isFinite(next)) onSet(field.path, next);
          }}
        />
      );
    }

    case "text":
      return (
        <input
          type="text"
          value={typeof value === "string" ? value : ""}
          onChange={(event) => onSet(field.path, event.target.value)}
        />
      );

    case "textList":
      return (
        <textarea
          rows={Math.min(6, Math.max(2, (value as string[])?.length ?? 2))}
          value={Array.isArray(value) ? value.join("\n") : ""}
          spellCheck={false}
          onChange={(event) =>
            onSet(
              field.path,
              event.target.value.split("\n").filter((line) => line.length > 0),
            )
          }
        />
      );

    case "unsupported":
      // Said rather than hidden: only a row that admits it exists tells the
      // user the setting is there to be edited in the file.
      return (
        <span className="bw-settings-unsupported">
          {tr("Edit this one in config.json")}
        </span>
      );
  }
}
