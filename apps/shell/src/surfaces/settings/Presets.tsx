// The presets page of the settings screen.
//
// A preset is the whole config under a name. What makes that safe to press is
// everything around it:
//
// **Applying shows what it would change first.** end4-pC merges the file and
// tells you afterwards, which is fine while a preset is something you saved an
// hour ago and not fine for one somebody sent you — it rewrites a couple of
// hundred settings and there is no list of which. So Apply opens the list,
// every row can be unticked, and the button says how many it will write.
//
// **And it can be taken back.** The config a preset replaced is kept for one
// press. Nothing else in this shell changes that much in one click.
//
// The rules live in `bw-core::preset` under tests: what a name may be, which
// paths are safe to write, and what happens to a key this build has never
// heard of. Nothing here re-decides any of that.

import { useCallback, useEffect, useMemo, useState } from "react";
import type { Comparison, PresetSummary } from "@bw/core";
import { Command, configSchema } from "@bw/core";
import { backend } from "../../shell/backend";
import { actions } from "../../shell/store";
import { describeError } from "../../shell/errors";
import { Button, Dialog, Placeholder, Symbol } from "../../widgets";
import { tr } from "../../i18n";
import { contrast } from "./diff";
import "./presets.css";

/** What the settings rows call a path, so the confirm list says the same.
 *
 * The section is part of it here where it is not on the settings page: there,
 * a heading says which section you are reading; in a flat list of changes,
 * "Height" on its own could be four different settings. */
const LABELS = new Map(
  configSchema.map((field) => [
    field.path,
    [field.section, field.group, field.label].filter(Boolean).join(" · "),
  ]),
);

/** A value as a row shows it — an empty one still needs to occupy space. */
function shown(value: string): string {
  return value === "" ? "—" : value;
}

interface Pending {
  name: string;
  comparison: Comparison;
}

export function Presets() {
  const [presets, setPresets] = useState<PresetSummary[] | null>(null);
  const [thumbs, setThumbs] = useState<Record<string, string>>({});
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [problem, setProblem] = useState<string | null>(null);
  const [undoable, setUndoable] = useState(false);
  const [pending, setPending] = useState<Pending | null>(null);
  const [chosen, setChosen] = useState<Set<string>>(new Set());
  const [removing, setRemoving] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      setPresets(await actions.presets());
      setUndoable(await actions.hasPresetUndo());
    } catch (error) {
      setProblem(describeError(error));
      setPresets([]);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  // A card without its wallpaper is a grey box with a name on it, and the
  // wallpaper is most of what tells two presets apart at a glance.
  useEffect(() => {
    if (!presets) return;
    let live = true;
    void (async () => {
      const found = await Promise.all(
        presets
          .filter((preset) => preset.wallpaper)
          .map(async (preset) => {
            const thumb = await backend()
              .invoke<string>(Command.ThumbnailFor, { path: preset.wallpaper })
              // A preset from another machine names a picture this one has
              // not got. The card falls back to its placeholder.
              .catch(() => "");
            return [
              preset.name,
              thumb ? backend().assetUrl(thumb) : "",
            ] as const;
          }),
      );
      if (live) setThumbs(Object.fromEntries(found));
    })();
    return () => {
      live = false;
    };
  }, [presets]);

  // Case-insensitively, because these are file names: `dark` and `Dark` cannot
  // both exist, and the button has to say Replace rather than Save for either.
  const taken = useMemo(
    () =>
      (presets ?? []).find(
        (preset) => preset.name.toLowerCase() === name.trim().toLowerCase(),
      ),
    [presets, name],
  );

  const save = useCallback(async () => {
    try {
      setPresets(
        await actions.savePreset(name.trim(), description, Boolean(taken)),
      );
      setName("");
      setDescription("");
      setProblem(null);
    } catch (error) {
      setProblem(describeError(error));
    }
  }, [name, description, taken]);

  const open = useCallback(async (preset: PresetSummary) => {
    try {
      const comparison = await actions.comparePreset(preset.name);
      setChosen(new Set(comparison.changes.map((change) => change.path)));
      setPending({ name: preset.name, comparison });
      setProblem(null);
    } catch (error) {
      setProblem(describeError(error));
    }
  }, []);

  const apply = useCallback(async () => {
    if (!pending) return;
    try {
      await actions.applyPreset(pending.name, [...chosen]);
      setPending(null);
      setUndoable(await actions.hasPresetUndo());
      setProblem(null);
    } catch (error) {
      setProblem(describeError(error));
    }
  }, [pending, chosen]);

  const undo = useCallback(async () => {
    try {
      await actions.undoPreset();
      setUndoable(false);
      setProblem(null);
    } catch (error) {
      setProblem(describeError(error));
    }
  }, []);

  const remove = useCallback(async (target: string) => {
    try {
      setPresets(await actions.removePreset(target));
      setRemoving(null);
      setProblem(null);
    } catch (error) {
      setProblem(describeError(error));
    }
  }, []);

  return (
    <div className="bw-presets">
      <p className="bw-presets-lead">
        {tr(
          "A preset is every setting on this screen, saved under a name. Applying one shows what it would change before it writes anything.",
        )}
      </p>

      <div className="bw-presets-save">
        <input
          type="text"
          value={name}
          maxLength={64}
          placeholder={tr("Name")}
          aria-label={tr("Preset name")}
          onChange={(event) => setName(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter" && name.trim()) void save();
          }}
        />
        <input
          type="text"
          value={description}
          placeholder={tr("Description (optional)")}
          aria-label={tr("Preset description")}
          onChange={(event) => setDescription(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter" && name.trim()) void save();
          }}
        />
        <Button
          variant="filled"
          icon="save"
          disabled={!name.trim()}
          onClick={() => void save()}
        >
          {/* Saying Replace rather than asking afterwards: the name is already
              on screen, so the button can simply tell the truth about what
              pressing it does. */}
          {taken ? tr("Replace") : tr("Save")}
        </Button>
      </div>

      {undoable ? (
        <div className="bw-presets-undo">
          <Symbol name="history" size={18} />
          <span>{tr("A preset replaced your settings.")}</span>
          <Button icon="undo" onClick={() => void undo()}>
            {tr("Undo")}
          </Button>
        </div>
      ) : null}

      {problem ? <p className="bw-settings-problem">{problem}</p> : null}

      {presets === null ? null : presets.length === 0 ? (
        <Placeholder icon="bookmarks" text={tr("No presets yet")} />
      ) : (
        <div className="bw-presets-grid">
          {presets.map((preset) => (
            <article key={preset.name} className="bw-preset">
              <header>
                <span className="bw-preset-initial" aria-hidden="true">
                  {preset.name.slice(0, 1).toUpperCase()}
                </span>
                <div>
                  <h3>{preset.name}</h3>
                  <p title={preset.problem ?? preset.description}>
                    {preset.problem
                      ? preset.problem
                      : preset.description || tr("Saved preset")}
                  </p>
                </div>
              </header>

              <div className="bw-preset-shot" data-broken={!!preset.problem}>
                {thumbs[preset.name] ? (
                  <img src={thumbs[preset.name]} alt="" loading="lazy" />
                ) : (
                  <Symbol
                    name={preset.problem ? "error" : "wallpaper"}
                    size={32}
                    color="var(--on-surface-variant)"
                  />
                )}
              </div>

              {removing === preset.name ? (
                <footer>
                  <span className="bw-preset-ask">{tr("Delete it?")}</span>
                  <Button variant="text" onClick={() => setRemoving(null)}>
                    {tr("Keep")}
                  </Button>
                  <Button
                    variant="filled"
                    onClick={() => void remove(preset.name)}
                  >
                    {tr("Delete")}
                  </Button>
                </footer>
              ) : (
                <footer>
                  {/* Deleting a preset removes a file and there is no undo for
                      that, so it asks — unlike Apply, which has one. */}
                  <Button
                    variant="text"
                    icon="delete"
                    aria-label={`${tr("Delete")} ${preset.name}`}
                    onClick={() => setRemoving(preset.name)}
                  >
                    {tr("Delete")}
                  </Button>
                  <Button
                    variant="filled"
                    icon="check"
                    disabled={!!preset.problem}
                    aria-label={`${tr("Apply")} ${preset.name}`}
                    onClick={() => void open(preset)}
                  >
                    {tr("Apply")}
                  </Button>
                </footer>
              )}
            </article>
          ))}
        </div>
      )}

      {pending ? (
        <ConfirmApply
          pending={pending}
          chosen={chosen}
          onChoose={setChosen}
          onDismiss={() => setPending(null)}
          onApply={() => void apply()}
        />
      ) : null}
    </div>
  );
}

/** The list of what Apply would write, before it writes any of it. */
function ConfirmApply({
  pending,
  chosen,
  onChoose,
  onDismiss,
  onApply,
}: {
  pending: Pending;
  chosen: Set<string>;
  onChoose: (chosen: Set<string>) => void;
  onDismiss: () => void;
  onApply: () => void;
}) {
  const { changes, unknown } = pending.comparison;
  const all = changes.length > 0 && chosen.size === changes.length;

  const toggle = (path: string) => {
    const next = new Set(chosen);
    if (!next.delete(path)) next.add(path);
    onChoose(next);
  };

  return (
    <Dialog
      title={pending.name}
      icon="swap_horiz"
      onDismiss={onDismiss}
      footer={
        <>
          <Button variant="text" onClick={onDismiss}>
            {changes.length === 0 ? tr("Close") : tr("Cancel")}
          </Button>
          {changes.length === 0 ? null : (
            <Button
              variant="filled"
              disabled={chosen.size === 0}
              onClick={onApply}
            >
              {/* The count is on the button, so nobody has to scroll back up
                  to find out how much it is about to change. */}
              {tr("Change %1 settings").replace("%1", String(chosen.size))}
            </Button>
          )}
        </>
      }
    >
      {changes.length === 0 ? (
        // Worth its own sentence: an Apply that would write nothing is
        // otherwise a button that does nothing, which reads as broken.
        <p className="bw-presets-same">
          {tr("This preset matches your settings already.")}
        </p>
      ) : (
        <>
          <div className="bw-presets-all">
            <span>
              {tr("%1 settings would change").replace(
                "%1",
                String(changes.length),
              )}
            </span>
            <Button
              variant="text"
              onClick={() =>
                onChoose(
                  all
                    ? new Set()
                    : new Set(changes.map((change) => change.path)),
                )
              }
            >
              {all ? tr("Clear all") : tr("Select all")}
            </Button>
          </div>

          <ul className="bw-presets-changes">
            {changes.map((change) => {
              // Trimmed to where the two sides stop agreeing, and titled with
              // the whole of each: a row whose two ends read identically says
              // nothing about what would change.
              const [from, to] = contrast(change.from, change.to);
              return (
                <li key={change.path}>
                  <label>
                    <input
                      type="checkbox"
                      checked={chosen.has(change.path)}
                      onChange={() => toggle(change.path)}
                    />
                    <span className="bw-presets-what">
                      <strong>{LABELS.get(change.path) ?? change.path}</strong>
                      <code>{change.path}</code>
                    </span>
                    <span className="bw-presets-move">
                      <del title={change.from}>{shown(from)}</del>
                      <Symbol name="arrow_forward" size={14} />
                      <ins title={change.to}>{shown(to)}</ins>
                    </span>
                  </label>
                </li>
              );
            })}
          </ul>
        </>
      )}

      {unknown.length > 0 ? (
        // A preset written by a newer build. Writing these would produce a
        // config this one refuses to read, so they are reported and skipped —
        // saying so, because "it applied and that part did nothing" is
        // indistinguishable from a bug.
        <div className="bw-presets-unknown">
          <p>
            <Symbol name="info" size={16} />
            {tr(
              "%1 settings in this preset do not exist in this build",
            ).replace("%1", String(unknown.length))}
          </p>
          <ul>
            {unknown.map((path) => (
              <li key={path}>
                <code>{path}</code>
              </li>
            ))}
          </ul>
        </div>
      ) : null}
    </Dialog>
  );
}
