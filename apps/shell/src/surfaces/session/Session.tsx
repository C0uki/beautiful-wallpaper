// The way out of the session.
//
// Six buttons at most, and the machine decides how many of those it can
// actually do — the list arrives already filtered and already ordered, so
// nothing here reasons about what a given button means.
//
// It does reason about the keyboard, because that is where the danger is. The
// screen opens under a key, Enter is one keystroke further, and two of these
// buttons close every program the user has open. So the caret starts on
// something recoverable, and if there is nothing recoverable on offer it
// starts nowhere at all.

import { useCallback, useEffect, useState } from "react";
import type { SessionAction } from "@bw/core";
import { Symbol } from "../../widgets";
import { tr } from "../../i18n";
import { actions, connect, useShell } from "../../shell/store";
import { describeError } from "../../shell/errors";
import "./session.css";

/** What each button says, in the user's language. */
function label(action: SessionAction): string {
  switch (action) {
    case "lock":
      return tr("Lock");
    case "sleep":
      return tr("Sleep");
    case "hibernate":
      return tr("Hibernate");
    case "logOut":
      return tr("Log out");
    case "restart":
      return tr("Restart");
    case "shutDown":
      return tr("Shut down");
  }
}

/** Which glyph, mirroring `SessionAction::symbol` in `bw-core`. */
function symbol(action: SessionAction): string {
  switch (action) {
    case "lock":
      return "lock";
    case "sleep":
      return "bedtime";
    case "hibernate":
      return "ac_unit";
    case "logOut":
      return "logout";
    case "restart":
      return "restart_alt";
    case "shutDown":
      return "power_settings_new";
  }
}

/** Whether taking this closes the user's programs. Mirrors `bw-core`. */
function endsTheSession(action: SessionAction): boolean {
  return action === "logOut" || action === "restart" || action === "shutDown";
}

export function Session() {
  const ready = useShell((state) => state.ready);
  const open = useShell((state) => state.states.sessionOpen);
  const enabled = useShell((state) => state.config.session.enable);

  const [available, setAvailable] = useState<SessionAction[]>([]);
  const [focused, setFocused] = useState<number | null>(null);
  const [problem, setProblem] = useState<string | null>(null);

  useEffect(() => {
    void connect();
  }, []);

  // Re-asked on every open rather than once: a laptop that had no hibernation
  // file when the shell started may have one now.
  useEffect(() => {
    if (!open) return;
    setProblem(null);
    void actions
      .sessionActions()
      .then((found) => {
        setAvailable(found);
        // Never on something that ends the session — the same rule the
        // backend's `initial_focus` states, and for the same reason.
        const safe = found.findIndex((action) => !endsTheSession(action));
        setFocused(safe < 0 ? null : safe);
      })
      .catch(() => setAvailable([]));
  }, [open]);

  const close = useCallback(() => actions.setState("sessionOpen", false), []);

  const take = useCallback(async (action: SessionAction) => {
    try {
      await actions.runSessionAction(action);
    } catch (error) {
      // Shutting down can be refused outright — a managed account, another
      // user logged in — and the screen closing on a refusal would leave
      // the user with a machine that simply did not switch off.
      setProblem(describeError(error));
      await actions.setState("sessionOpen", true);
    }
  }, []);

  const onKeyDown = useCallback(
    (event: React.KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        void close();
        return;
      }
      if (!available.length) return;

      if (event.key === "ArrowRight" || event.key === "ArrowLeft") {
        event.preventDefault();
        const step = event.key === "ArrowRight" ? 1 : -1;
        setFocused((index) => {
          const from = index ?? (step > 0 ? -1 : 0);
          return (from + step + available.length) % available.length;
        });
        return;
      }
      if (event.key === "Enter" || event.key === " ") {
        if (focused === null) return;
        event.preventDefault();
        const action = available[focused];
        if (action) void take(action);
      }
    },
    [available, close, focused, take],
  );

  if (!ready || !enabled || !open) return null;

  return (
    <div
      className="bw-session"
      role="dialog"
      aria-label={tr("Session")}
      tabIndex={-1}
      // Autofocus so the arrow keys work without a click; the caret's starting
      // position is decided above, not by the browser.
      ref={(element) => element?.focus()}
      onKeyDown={onKeyDown}
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) void close();
      }}
    >
      <div className="bw-session-panel">
        {problem ? <p className="bw-session-problem">{problem}</p> : null}

        {available.length ? (
          <ul className="bw-session-actions">
            {available.map((action, index) => (
              <li key={action}>
                <button
                  type="button"
                  className={[
                    "bw-session-action",
                    endsTheSession(action) ? "ends" : "",
                    index === focused ? "focused" : "",
                  ]
                    .filter(Boolean)
                    .join(" ")}
                  onMouseEnter={() => setFocused(index)}
                  onClick={() => void take(action)}
                >
                  <Symbol name={symbol(action)} size={34} />
                  <span>{label(action)}</span>
                </button>
              </li>
            ))}
          </ul>
        ) : (
          <p className="bw-session-problem">
            {tr("This machine offers no way out of the session")}
          </p>
        )}

        <p className="bw-session-hint">{tr("Escape closes this")}</p>
      </div>
    </div>
  );
}
