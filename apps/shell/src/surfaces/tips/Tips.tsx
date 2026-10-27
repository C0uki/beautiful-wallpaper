// The tips.
//
// Laid out as Apple's Tips app is: collections down the left, and the chosen
// one's tips as cards on the right, each saying one thing the shell can do and,
// where it can, offering to open it. Nothing here changes a setting; it only
// says where things are.

import { useCallback, useEffect, useState } from "react";
import type { StateFlagName } from "@bw/core";
import { Button, IconButton, Symbol } from "../../widgets";
import { tr } from "../../i18n";
import { actions, connect, useShell } from "../../shell/store";
import { capsOf } from "../settings/chord";
import { NAMES, type Binding } from "../settings/Shortcuts";
import { COLLECTIONS, KEYBOARD, type Tip } from "./content";
import "./tips.css";

export function Tips() {
  const ready = useShell((state) => state.ready);
  const open = useShell((state) => state.states.tipsOpen);
  const keybinds = useShell((state) => state.config.keybinds);
  const [chosen, setChosen] = useState(COLLECTIONS[0]!.id);

  useEffect(() => {
    void connect();
  }, []);

  const close = useCallback(() => void actions.setState("tipsOpen", false), []);

  // What it opens first, and only then put away: closed, this window is
  // destroyed, and a request it had not sent yet went with it.
  const go = useCallback(async (flag: StateFlagName) => {
    await actions.setState(flag, true);
    await actions.setState("tipsOpen", false);
  }, []);

  if (!ready || !open) return null;

  const collection = COLLECTIONS.find((entry) => entry.id === chosen)!;

  return (
    <div
      className="bw-tips"
      role="dialog"
      aria-label={tr("Tips")}
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) close();
      }}
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.preventDefault();
          close();
        }
      }}
    >
      <div className="bw-tips-panel">
        <nav className="bw-tips-collections" aria-label={tr("Tips")}>
          {COLLECTIONS.map((entry) => (
            <button
              key={entry.id}
              type="button"
              className={entry.id === chosen ? "selected" : ""}
              onClick={() => setChosen(entry.id)}
            >
              <span className="bw-tips-tile" style={{ background: entry.tint }}>
                <Symbol name={entry.icon} size={16} filled />
              </span>
              <span>{tr(entry.title)}</span>
            </button>
          ))}
        </nav>

        <div className="bw-tips-body">
          <header className="bw-tips-head">
            <div>
              <h1>{tr(collection.title)}</h1>
              <p>{tr(collection.summary)}</p>
            </div>
            <IconButton
              icon="close"
              size={32}
              label={tr("Close")}
              onClick={close}
              autoFocus
            />
          </header>

          {collection.id === KEYBOARD ? (
            <Keys
              keybinds={keybinds}
              onSettings={() => void go("settingsOpen")}
            />
          ) : (
            <ul className="bw-tips-cards">
              {collection.tips.map((tip) => (
                <Card key={tip.title} tip={tip} onOpen={go} />
              ))}
            </ul>
          )}
        </div>
      </div>
    </div>
  );
}

function Card({
  tip,
  onOpen,
}: {
  tip: Tip;
  onOpen: (flag: StateFlagName) => void;
}) {
  return (
    <li className="bw-tips-card">
      <span className="bw-tips-icon">
        <Symbol name={tip.icon} size={22} />
      </span>
      <h2>{tr(tip.title)}</h2>
      <p>{tr(tip.body)}</p>
      {tip.code ? <Code text={tip.code} /> : null}
      {tip.open ? (
        <Button variant="tonal" onClick={() => onOpen(tip.open!.flag)}>
          {tr(tip.open.label)}
        </Button>
      ) : null}
    </li>
  );
}

/** A command, with a button that copies it. */
function Code({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <div className="bw-tips-code">
      <code>{text}</code>
      <IconButton
        icon={copied ? "check" : "content_copy"}
        size={28}
        label={tr("Copy")}
        onClick={() =>
          void navigator.clipboard.writeText(text).then(() => setCopied(true))
        }
      />
    </div>
  );
}

/** The keyboard's tips: the keys the config has now, so they cannot go stale. */
function Keys({
  keybinds,
  onSettings,
}: {
  keybinds: Record<string, unknown>;
  onSettings: () => void;
}) {
  const bindings = Object.keys(NAMES) as Binding[];
  return (
    <div className="bw-tips-keys">
      {keybinds["enable"] === false ? (
        <p className="bw-tips-note">
          {tr("Keyboard shortcuts are switched off in Settings.")}
        </p>
      ) : null}
      <ul>
        {bindings.map((binding) => {
          const chord = String(keybinds[binding] ?? "");
          return (
            <li key={binding}>
              <span>{tr(NAMES[binding])}</span>
              <span className="bw-tips-chord">
                {chord ? (
                  capsOf(chord).map((cap, index) => (
                    <kbd key={index}>{cap}</kbd>
                  ))
                ) : (
                  <span>{tr("Off")}</span>
                )}
              </span>
            </li>
          );
        })}
      </ul>
      <Button variant="tonal" onClick={onSettings}>
        {tr("Change them in Settings")}
      </Button>
    </div>
  );
}
