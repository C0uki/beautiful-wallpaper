// The settings screen.
//
// Every row on it is generated from the Rust schema, so a config key added in
// any future change has a control here without anybody touching this file.
// What this file does is decide how a row is *drawn* — a toggle, a slider, a
// choice — and how two hundred of them are made findable.
//
// Findable is the harder half. A generated form is complete by construction
// and unusable if that is all it is, so there is a search across every page,
// the pages are laid out by what the user is looking for (see pages.ts), and
// each page is drawn as the Mac's System Settings draws one: a coloured tile
// per page, and rows gathered into cards.

import { useCallback, useEffect, useMemo, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import type { Field } from "@bw/core";
import { configSchema } from "@bw/core";
import {
  Check,
  IconButton,
  SearchField,
  Slider,
  Switch,
  Symbol,
} from "../../widgets";
import { tr } from "../../i18n";
import { actions, connect, useShell } from "../../shell/store";
import { describeError } from "../../shell/errors";
import { OVERRIDES } from "./overrides";
import { PAGES, orderOn, pageFor } from "./pages";
import { Presets } from "./Presets";
import { Shortcuts } from "./Shortcuts";
import { ColorPicker, FontPicker, Typed } from "./pickers";
import { ApiKeys, ModelPicker } from "./ai";
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

/** The label a row is shown with: the curated one where the mechanical one
 *  reads wrongly. */
function labelOf(field: Field): string {
  return OVERRIDES[field.path]?.label?.() ?? tr(field.label);
}

/** Everything a search term should match: the label, the sentence under it,
 * the group, the path — as shown and in English, so a term from the docs
 * still finds its row. */
function matches(field: Field, term: string): boolean {
  if (!term) return true;
  const hint = OVERRIDES[field.path]?.hint?.() ?? "";
  const haystack =
    `${labelOf(field)} ${field.label} ${hint} ${tr(field.group)} ${field.group} ${field.path}`.toLowerCase();
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
  // keystroke is the one thing here that could feel slow. A page shows its
  // rows in the order it claims them; the sort is stable, so the schema's own
  // order holds within each claim.
  const byPage = useMemo(() => {
    const found = new Map<string, Field[]>();
    for (const field of configSchema) {
      const home = pageFor(field);
      if (!home) continue;
      found.set(home.id, [...(found.get(home.id) ?? []), field]);
    }
    for (const [id, fields] of found) {
      const page = PAGES.find((entry) => entry.id === id)!;
      fields.sort((a, b) => orderOn(page, a) - orderOn(page, b));
    }
    return found;
  }, []);

  const searching = term.trim().length > 0;
  // While searching the page list stops meaning anything — the answer is
  // wherever it is — so every page's rows are shown together.
  const shown = useMemo(() => {
    const fields = searching
      ? configSchema.filter((field) => pageFor(field))
      : (byPage.get(page) ?? []);
    return fields.filter((field) => matches(field, term));
  }, [byPage, page, searching, term]);

  // At the foot of the page list, where a Mac's settings keep it too.
  const [version, setVersion] = useState("");
  useEffect(() => {
    getVersion().then(setVersion, () => setVersion(""));
  }, []);

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

  // Escape closes it, as it does every other overlay (#232) — but not while
  // the IME is converting, where Escape takes the conversion back. Recording
  // a shortcut takes its Escape first (`Shortcuts.tsx`), and a preset's
  // dialog closes on its own. Whatever box has the focus is left first, so a
  // value still being typed is written, not lost.
  useEffect(() => {
    if (!open) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== "Escape" || event.isComposing) return;
      if (document.querySelector(".bw-dialog")) return;
      (document.activeElement as HTMLElement | null)?.blur();
      close();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, close]);

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
          {PAGES.map((entry, index) => (
            <button
              key={entry.id}
              type="button"
              className={!searching && entry.id === page ? "selected" : ""}
              // A gap before the first page of each group, as on a Mac.
              data-starts-group={
                index > 0 && PAGES[index - 1]!.group !== entry.group
              }
              onClick={() => {
                setTerm("");
                setPage(entry.id);
              }}
            >
              <span
                className="bw-settings-tile"
                style={{ background: entry.tint }}
              >
                <Symbol name={entry.icon} size={16} filled />
              </span>
              <span>{entry.title()}</span>
            </button>
          ))}
          {version ? (
            <p className="bw-settings-version">{tr("Version %1", version)}</p>
          ) : null}
        </nav>

        <div className="bw-settings-body">
          <header className="bw-settings-head">
            <div>
              <h1>
                {searching
                  ? tr("%1 settings match").replace("%1", String(shown.length))
                  : (current?.title() ?? "")}
              </h1>
              {!searching && current ? <p>{current.summary()}</p> : null}
            </div>
            <IconButton
              icon="close"
              size={32}
              label={tr("Close")}
              onClick={close}
            />
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
            ) : !searching && current?.custom === "shortcuts" ? (
              <Shortcuts config={config} onSet={set} />
            ) : shown.length ? (
              <>
                {/* The keys are not settings — they never enter the config —
                    so the AI page draws them above its rows. */}
                {!searching && current?.id === "ai" ? <ApiKeys /> : null}
                <Rows
                  fields={shown}
                  config={config}
                  onSet={set}
                  grouped={!searching}
                />
              </>
            ) : (
              <p className="bw-settings-empty">{tr("Nothing matched")}</p>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

/** The rows, gathered into cards: a new card wherever the group changes,
 *  titled by the group, or by the section when the rows are not in one. A
 *  search is one card, since its rows come from everywhere. */
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
  const cards: { key: string; title: string; fields: Field[] }[] = [];
  for (const field of fields) {
    const key = grouped ? `${field.section}/${field.group}` : "found";
    const last = cards[cards.length - 1];
    if (last?.key === key) last.fields.push(field);
    else
      cards.push({
        key,
        title: grouped ? tr(field.group || field.section) : "",
        fields: [field],
      });
  }

  return (
    <>
      {cards.map((card) => (
        <section key={card.key} className="bw-settings-card">
          {card.title ? <h2>{card.title}</h2> : null}
          <div className="bw-settings-card-rows">
            {card.fields.map((field) => (
              <Row
                key={field.path}
                field={field}
                config={config}
                onSet={onSet}
              />
            ))}
          </div>
        </section>
      ))}
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
      {/* The dotted path is what `bw config set` takes; it is kept on the
          label's tooltip, and a search still finds it. */}
      <div className="bw-settings-label" title={field.path}>
        <span>{labelOf(field)}</span>
        {override?.hint ? <em>{override.hint()}</em> : null}
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
  // A colour or a font is picked rather than typed: the schema only knows
  // either is a string.
  if (override?.color) {
    return (
      <ColorPicker
        value={typeof value === "string" ? value : (override.unset ?? "")}
        swatches={override.color}
        onChange={(next) => onSet(field.path, next)}
      />
    );
  }
  if (override?.model) {
    return (
      <ModelPicker
        provider={override.model}
        value={typeof value === "string" ? value : ""}
        onChange={(next) => onSet(field.path, next)}
      />
    );
  }
  if (override?.font) {
    return (
      <FontPicker
        value={typeof value === "string" ? value : ""}
        kind={override.font}
        onChange={(next) => onSet(field.path, next)}
      />
    );
  }

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
          label={labelOf(field)}
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
              label={labelOf(field)}
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
        <Typed
          value={typeof value === "string" ? value : ""}
          onCommit={(next) => onSet(field.path, next)}
        />
      );

    case "textList": {
      const list = Array.isArray(value) ? (value as string[]) : [];
      // Picked from what the shell can use, rather than typed (#229). One
      // written into the file by hand is still listed, so it can be undone.
      if (override?.options) {
        return (
          <div className="bw-settings-options">
            {[...new Set([...override.options, ...list])].map((option) => (
              // Named beside the box rather than through `label`, which a
              // ticked box strikes through as a done to-do.
              <label key={option}>
                <Check
                  ariaLabel={option}
                  checked={list.includes(option)}
                  onChange={(on) =>
                    onSet(
                      field.path,
                      on
                        ? [...list, option]
                        : list.filter((entry) => entry !== option),
                    )
                  }
                />
                <span>{option}</span>
              </label>
            ))}
          </div>
        );
      }
      return (
        <Typed
          multiline
          value={list.join("\n")}
          onCommit={(next) =>
            onSet(
              field.path,
              next.split("\n").filter((line) => line.length > 0),
            )
          }
        />
      );
    }

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
