// The first-run screen.
//
// The original's welcome app is one scrolling page of preferences, and on
// Linux that is the right shape: the shell works the moment it starts, and the
// page is an invitation. Here, three of the things it would offer may not work
// at all until somebody looks, and none of them announce themselves:
//
// - no wallpaper has been chosen, so the palette has nothing to generate from;
// - the workspaces widget needs a tiling window manager Windows does not ship,
//   and without one it is permanently empty, which reads as a bug;
// - Windows may have refused any of the keyboard shortcuts, and a refused key
//   is indistinguishable from a key that does nothing.
//
// So this is a wizard, and each step establishes one of them and reports what
// it found. Every step can be skipped — none of this is a form — but a step
// that found something wrong says so rather than letting the user walk past
// it without knowing.

import { useCallback, useEffect, useMemo, useState } from "react";
import type { KeyStatus } from "@bw/core";
import { Command } from "@bw/core";
import { backend } from "../../shell/backend";
import { actions, connect, useShell } from "../../shell/store";
import { describeError } from "../../shell/errors";
import { Button, IconButton, Switch, Symbol } from "../../widgets";
import { availableLocales, tr } from "../../i18n";
import { resumeAt, STEPS } from "./steps";
import "./wizard.css";

/** The four places a bar can go, as the two booleans that put it there. */
const PLACES = [
  { id: "top", icon: "arrow_upward", bottom: false, vertical: false },
  { id: "bottom", icon: "arrow_downward", bottom: true, vertical: false },
  { id: "left", icon: "arrow_back", bottom: false, vertical: true },
  { id: "right", icon: "arrow_forward", bottom: true, vertical: true },
] as const;

const STYLES = ["m3", "hug", "float", "islands"] as const;

export function Wizard() {
  const ready = useShell((state) => state.ready);
  const open = useShell((state) => state.states.wizardOpen);
  const config = useShell((state) => state.config);

  const [step, setStep] = useState(0);
  const [problem, setProblem] = useState<string | null>(null);
  const [keys, setKeys] = useState<KeyStatus[] | null>(null);
  // `undefined` is "not looked yet"; `null` is "looked, found nothing".
  const [manager, setManager] = useState<string | null | undefined>(undefined);
  const [thumb, setThumb] = useState("");

  // The persisted state is asked for directly rather than read off the store:
  // only `connectSidebar` fetches it, and paying for the radio and audio-session
  // enumeration to find out which step this screen was on would be absurd.
  // Read once — the step is written on every move, and reading it back would
  // fight the click that caused it.
  useEffect(() => {
    void (async () => {
      await connect();
      try {
        const state = await actions.refreshPersistent();
        setStep(resumeAt(state.firstRun.step));
      } catch {
        // Starting over is the right failure: it is the first run.
      }
    })();
  }, []);

  const current = STEPS[step];

  const go = useCallback((next: number) => {
    const clamped = resumeAt(next);
    setStep(clamped);
    // Remembered as it happens, so closing the window half-way and coming
    // back resumes rather than starting over.
    void actions.setPersistentValue("firstRun.step", clamped).catch(() => {});
  }, []);

  const set = useCallback(async (path: string, value: unknown) => {
    try {
      await actions.setConfigValue(path, value);
      setProblem(null);
    } catch (error) {
      setProblem(describeError(error));
    }
  }, []);

  const finish = useCallback(async () => {
    try {
      // Marked done before the window is hidden: if the two are the other way
      // round and the shell is killed in between, the screen comes back.
      await actions.setPersistentValue("firstRun.done", true);
    } catch (error) {
      setProblem(describeError(error));
    }
    void actions.setState("wizardOpen", false);
  }, []);

  // Both of these ask the backend something it can only answer at runtime, and
  // both are worth asking again when the step is opened rather than once at
  // startup: the user may have installed a window manager, or changed a key.
  useEffect(() => {
    if (!open || current?.id !== "keys") return;
    void actions
      .keyReport()
      .then(setKeys)
      .catch((error) => setProblem(describeError(error)));
  }, [open, current?.id]);

  useEffect(() => {
    if (!open || current?.id !== "windows") return;
    setManager(undefined);
    void actions
      .detectWindowManager()
      .then(setManager)
      .catch(() => setManager(null));
  }, [open, current?.id]);

  const wallpaper = config.background.wallpaperPath;
  useEffect(() => {
    if (!wallpaper) {
      setThumb("");
      return;
    }
    let live = true;
    void backend()
      .invoke<string>(Command.ThumbnailFor, { path: wallpaper })
      .then((found) => {
        if (live) setThumb(found ? backend().assetUrl(found) : "");
      })
      .catch(() => {
        if (live) setThumb("");
      });
    return () => {
      live = false;
    };
  }, [wallpaper]);

  const troubled = useMemo(
    () =>
      (keys ?? []).filter(
        (key) => key.refused || key.takenByWindows || key.sharedWith.length > 0,
      ),
    [keys],
  );

  if (!ready || !open || !current) return null;
  const last = step === STEPS.length - 1;

  return (
    <div className="bw-wizard" role="dialog" aria-label={tr("First run")}>
      <div className="bw-wizard-panel">
        <nav className="bw-wizard-rail" aria-label={tr("Steps")}>
          <h1>beautiful-wallpaper</h1>
          <ol>
            {STEPS.map((entry, index) => (
              <li
                key={entry.id}
                className={
                  index === step ? "current" : index < step ? "done" : ""
                }
              >
                <button type="button" onClick={() => go(index)}>
                  <Symbol
                    name={index < step ? "check" : entry.icon}
                    size={18}
                  />
                  <span>{entry.title()}</span>
                </button>
              </li>
            ))}
          </ol>
          {/* Always available, on every step. None of this is a form, and a
              first-run screen that cannot be dismissed is a hostage. */}
          <Button variant="text" onClick={() => void finish()}>
            {tr("Skip the rest")}
          </Button>
        </nav>

        <div className="bw-wizard-body">
          <header>
            <h2>
              <Symbol name={current.icon} size={26} />
              {current.title()}
            </h2>
            <p>{current.blurb()}</p>
          </header>

          {problem ? <p className="bw-wizard-problem">{problem}</p> : null}

          <div className="bw-wizard-content">
            {current.id === "welcome" ? (
              <>
                <Field label={tr("Language")}>
                  <select
                    value={config.language.ui}
                    aria-label={tr("Language")}
                    onChange={(event) =>
                      void set("language.ui", event.target.value)
                    }
                  >
                    <option value="auto">{tr("Match Windows")}</option>
                    {availableLocales().map((locale) => (
                      <option key={locale} value={locale}>
                        {locale}
                      </option>
                    ))}
                  </select>
                </Field>
                <Field label={tr("Light or dark")}>
                  <div className="bw-wizard-choices">
                    {(["light", "dark"] as const).map((mode) => (
                      <button
                        key={mode}
                        type="button"
                        className={
                          config.appearance.palette.mode === mode
                            ? "selected"
                            : ""
                        }
                        onClick={() => void actions.setMode(mode)}
                      >
                        <Symbol
                          name={mode === "light" ? "light_mode" : "dark_mode"}
                          size={22}
                        />
                        <span>
                          {mode === "light" ? tr("Light") : tr("Dark")}
                        </span>
                      </button>
                    ))}
                  </div>
                </Field>
              </>
            ) : null}

            {current.id === "wallpaper" ? (
              <div className="bw-wizard-wallpaper">
                <div className="bw-wizard-shot">
                  {thumb ? (
                    <img src={thumb} alt="" />
                  ) : (
                    <Symbol
                      name="wallpaper"
                      size={40}
                      color="var(--on-surface-variant)"
                    />
                  )}
                </div>
                <div>
                  <p className="bw-wizard-said">
                    {wallpaper
                      ? wallpaper
                      : tr(
                          "Nothing chosen yet, so the palette is the default one.",
                        )}
                  </p>
                  <Button
                    variant="filled"
                    icon="image"
                    onClick={() => {
                      void (async () => {
                        try {
                          const [picked] = await actions.pickFiles();
                          if (picked) await actions.applyWallpaper(picked);
                          setProblem(null);
                        } catch (error) {
                          setProblem(describeError(error));
                        }
                      })();
                    }}
                  >
                    {tr("Choose an image")}
                  </Button>
                </div>
              </div>
            ) : null}

            {current.id === "bar" ? (
              <>
                <Field label={tr("Where it sits")}>
                  <div className="bw-wizard-choices">
                    {PLACES.map((place) => (
                      <button
                        key={place.id}
                        type="button"
                        aria-label={place.id}
                        className={
                          config.bar.bottom === place.bottom &&
                          config.bar.vertical === place.vertical
                            ? "selected"
                            : ""
                        }
                        onClick={() => {
                          void set("bar.bottom", place.bottom);
                          void set("bar.vertical", place.vertical);
                        }}
                      >
                        <Symbol name={place.icon} size={22} />
                      </button>
                    ))}
                  </div>
                </Field>
                <Field label={tr("Shape")}>
                  <div className="bw-wizard-choices">
                    {STYLES.map((style) => (
                      <button
                        key={style}
                        type="button"
                        className={config.bar.style === style ? "selected" : ""}
                        onClick={() => void set("bar.style", style)}
                      >
                        <span>{style}</span>
                      </button>
                    ))}
                  </div>
                </Field>
                <Field label={tr("Hide the Windows taskbar")}>
                  <Switch
                    checked={config.windows.hideSystemTaskbar}
                    label={tr("Hide the Windows taskbar")}
                    onChange={(next) => {
                      void set("windows.hideSystemTaskbar", next);
                      // Hiding the taskbar without the dock leaves no way to
                      // reach a minimised window at all.
                      if (next && !config.dock.enable) {
                        void set("dock.enable", true);
                      }
                    }}
                  />
                </Field>
                <Field label={tr("Show the dock")}>
                  <Switch
                    checked={config.dock.enable}
                    label={tr("Show the dock")}
                    onChange={(next) => void set("dock.enable", next)}
                  />
                </Field>
              </>
            ) : null}

            {current.id === "windows" ? (
              <WindowManagerStep
                manager={manager}
                bar={config.bar}
                onDrop={() => {
                  // Taking the widget out rather than leaving an empty space
                  // that looks like a bar that failed to draw.
                  for (const slot of ["left", "center", "right"] as const) {
                    const kept = config.bar[slot].filter(
                      (widget) => widget !== "workspaces",
                    );
                    if (kept.length !== config.bar[slot].length) {
                      void set(`bar.${slot}`, kept);
                    }
                  }
                }}
              />
            ) : null}

            {current.id === "keys" ? (
              <KeysStep
                keys={keys}
                troubled={troubled}
                onUse={async (key) => {
                  if (!key.suggestion) return;
                  try {
                    await actions.setConfigValue(
                      `keybinds.${key.binding}`,
                      key.suggestion,
                    );
                    setKeys(await actions.retryKeys());
                    setProblem(null);
                  } catch (error) {
                    setProblem(describeError(error));
                  }
                }}
                onRetry={async () => {
                  try {
                    setKeys(await actions.retryKeys());
                  } catch (error) {
                    setProblem(describeError(error));
                  }
                }}
              />
            ) : null}

            {current.id === "done" ? (
              <ul className="bw-wizard-done">
                <li>
                  <Symbol name="settings" size={18} />
                  {tr("Everything else is in Settings, on %1.").replace(
                    "%1",
                    config.keybinds.settings,
                  )}
                </li>
                <li>
                  <Symbol name="browse" size={18} />
                  {tr("The launcher is on %1.").replace(
                    "%1",
                    config.keybinds.overview,
                  )}
                </li>
                <li>
                  <Symbol name="bookmarks" size={18} />
                  {tr(
                    "Once it looks the way you want, save it as a preset in Settings.",
                  )}
                </li>
                <li>
                  <Symbol name="waving_hand" size={18} />
                  {tr("This screen comes back with `bw wizard open`.")}
                </li>
              </ul>
            ) : null}
          </div>

          <footer className="bw-wizard-nav">
            <IconButton
              icon="arrow_back"
              size={40}
              label={tr("Back")}
              disabled={step === 0}
              onClick={() => go(step - 1)}
            />
            <span className="bw-wizard-count">
              {`${step + 1} / ${STEPS.length}`}
            </span>
            {last ? (
              <Button
                variant="filled"
                icon="check"
                onClick={() => void finish()}
              >
                {tr("Done")}
              </Button>
            ) : (
              <Button
                variant="filled"
                icon="arrow_forward"
                onClick={() => go(step + 1)}
              >
                {tr("Next")}
              </Button>
            )}
          </footer>
        </div>
      </div>
    </div>
  );
}

function Field({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <div className="bw-wizard-field">
      <span>{label}</span>
      <div>{children}</div>
    </div>
  );
}

/** What the shell found when it asked for a window manager. */
function WindowManagerStep({
  manager,
  bar,
  onDrop,
}: {
  manager: string | null | undefined;
  bar: { left: string[]; center: string[]; right: string[] };
  onDrop: () => void;
}) {
  const onBar = [...bar.left, ...bar.center, ...bar.right].includes(
    "workspaces",
  );

  if (manager === undefined) {
    return <p className="bw-wizard-said">{tr("Looking…")}</p>;
  }

  if (manager) {
    return (
      <p className="bw-wizard-found" data-good="true">
        <Symbol name="check_circle" size={20} />
        {tr("%1 is running, so the workspaces widget will work.").replace(
          "%1",
          manager,
        )}
      </p>
    );
  }

  return (
    <>
      <p className="bw-wizard-found">
        <Symbol name="info" size={20} />
        {/* Said plainly, because the alternative is a widget that is empty
            forever and looks like a bar that failed to draw. Only GlazeWM can
            be detected: its IPC is what this build reads. */}
        {tr(
          "No GlazeWM found. The workspaces widget has nothing to show without a tiling window manager, and this build can only talk to GlazeWM.",
        )}
      </p>
      {onBar ? (
        <Button icon="visibility_off" onClick={onDrop}>
          {tr("Take it off the bar")}
        </Button>
      ) : (
        <p className="bw-wizard-said">{tr("It is not on the bar anyway.")}</p>
      )}
    </>
  );
}

/** Every shortcut, and the three different ways one can be broken. */
function KeysStep({
  keys,
  troubled,
  onUse,
  onRetry,
}: {
  keys: KeyStatus[] | null;
  troubled: KeyStatus[];
  onUse: (key: KeyStatus) => Promise<void>;
  onRetry: () => Promise<void>;
}) {
  if (!keys) return <p className="bw-wizard-said">{tr("Looking…")}</p>;

  if (troubled.length === 0) {
    return (
      <p className="bw-wizard-found" data-good="true">
        <Symbol name="check_circle" size={20} />
        {tr("Windows gave up all %1 of them.").replace(
          "%1",
          String(keys.length),
        )}
      </p>
    );
  }

  return (
    <>
      <p className="bw-wizard-found">
        <Symbol name="warning" size={20} />
        {tr("%1 of %2 need a different combination.")
          .replace("%1", String(troubled.length))
          .replace("%2", String(keys.length))}
      </p>
      <ul className="bw-wizard-keys">
        {troubled.map((key) => (
          <li key={key.binding}>
            <div>
              <strong>{key.binding}</strong>
              <kbd>{key.chord}</kbd>
            </div>
            <p>{explain(key)}</p>
            {key.suggestion ? (
              <Button icon="swap_horiz" onClick={() => void onUse(key)}>
                {tr("Use %1").replace("%1", key.suggestion)}
              </Button>
            ) : (
              <span className="bw-wizard-said">
                {tr("Pick one yourself in Settings.")}
              </span>
            )}
          </li>
        ))}
      </ul>
      <Button variant="text" icon="refresh" onClick={() => void onRetry()}>
        {tr("Try them again")}
      </Button>
    </>
  );
}

/** Why this key is a problem — three different reasons, said differently. */
function explain(key: KeyStatus): string {
  if (key.sharedWith.length > 0) {
    // The one nothing refuses: both register as far as the config is
    // concerned, and only one of them can ever fire.
    return tr("Also used by %1, and only one of them can work.").replace(
      "%1",
      key.sharedWith.join(", "),
    );
  }
  if (key.takenByWindows) {
    return tr("Windows uses this for %1.").replace("%1", key.takenByWindows);
  }
  return tr("Windows would not hand this one over.");
}
