// The toast stack.
//
// Follows the original's behaviour rather than Windows': toasts group by the
// application that sent them and can be swiped away. Each is React Bits'
// Swipe Toast, with a fuse along its foot for the time it has left.
//
// Only the shell's own notifications reach this today. Reading other
// applications' notifications needs package identity, which arrives with the
// MSIX sparse package in a later phase — so the store this reads from is
// deliberately source-agnostic and will not need reshaping then.

import { useEffect, useMemo, useState } from "react";
import { Symbol } from "../../widgets";
import SwipeToast from "../../widgets/reactbits/SwipeToast";
import { formatAge } from "../../lib/format";
import { tr } from "../../i18n";
import { actions, useShell } from "../../shell/store";
import type { Notification } from "@bw/core";
import "./toasts.css";

/**
 * How much of a toast's time on screen is left, in milliseconds.
 *
 * Measured from when the notification was posted rather than from when this
 * page happened to see it. The two are the same for anything arriving live,
 * and very different at startup: the history is kept on disk and the shell
 * posts its own notifications while the surfaces are still being built, so
 * without this a page would greet its first render with every toast of every
 * past session — and hold any `critical` one there for good, because those are
 * never given a timer.
 *
 * Zero or less means it belongs in the history and was never news.
 */
export function toastLife(
  notification: Pick<Notification, "time">,
  timeout: number,
  now: number = Date.now(),
): number {
  return timeout - (now - notification.time * 1000);
}

/** Notifications from one application, newest first. */
interface Group {
  appName: string;
  notifications: Notification[];
}

function groupByApp(notifications: Notification[]): Group[] {
  const groups: Group[] = [];
  for (const notification of notifications) {
    const existing = groups.find(
      (group) => group.appName === notification.appName,
    );
    if (existing) {
      existing.notifications.push(notification);
    } else {
      groups.push({
        appName: notification.appName,
        notifications: [notification],
      });
    }
  }
  return groups;
}

/** One toast: swiped sideways or closed to dismiss, gone by itself when its
 *  fuse burns down. */
function Toast({
  notification,
  extra,
  timeout,
  onExpire,
}: {
  notification: Notification;
  /** How many more from the same application are stacked behind this one. */
  extra: number;
  timeout: number;
  onExpire: () => void;
}) {
  const now = useShell((state) => state.now);
  // Taken once: the fuse is lit with whatever was left when the toast came
  // up, and a duration that changed every second would light it again.
  const [life] = useState(() => toastLife(notification, timeout));
  const critical = notification.urgency === "critical";

  return (
    <SwipeToast
      inline
      closeButton
      width={360}
      radius={16}
      background={critical ? "var(--error-container)" : "var(--layer1)"}
      color="var(--on-surface)"
      fuseColor={critical ? "var(--error)" : "var(--primary)"}
      // A critical one waits to be dismissed.
      duration={critical ? 0 : Math.max(0, life)}
      icon={
        <Symbol
          name={notification.image ? "image" : "notifications"}
          size={20}
          filled
        />
      }
      title={notification.summary}
      description={
        <span className="bw-toast-text">
          <span className="bw-toast-heading">
            <span className="bw-toast-app">{notification.appName}</span>
            <span className="bw-toast-age">
              {formatAge(notification.time, now.getTime() / 1000)}
            </span>
          </span>
          {notification.body ? (
            <span className="bw-toast-body">{notification.body}</span>
          ) : null}
          {extra > 0 ? (
            <span className="bw-toast-more">
              {tr("+%1 more").replace("%1", String(extra))}
            </span>
          ) : null}
        </span>
      }
      onClose={(reason) => {
        // Burnt down: off the screen, still in the history. Swiped or
        // closed: dismissed. Its own `open` is never used, so nothing closes
        // it any other way.
        if (reason === "timeout") onExpire();
        else if (reason !== "programmatic")
          void actions.dismissNotification(notification.id);
      }}
    />
  );
}

export function Toasts() {
  const notifications = useShell((state) => state.notifications);
  const config = useShell((state) => state.config.notifications);
  const [expired, setExpired] = useState<number[]>([]);

  const groups = useMemo(() => groupByApp(notifications), [notifications]);

  // Toasts leave on their own; the notification itself stays in the history.
  useEffect(() => {
    if (config.doNotDisturb) return;

    const now = Date.now();
    const fresh = notifications.filter(
      (notification) => !expired.includes(notification.id),
    );

    // Already over before this page ever saw it. The history outlives the
    // process, so without this every notification of every past session would
    // toast at once the moment the page connects — and a `critical` one, which
    // is never given a timer, would then sit on the screen for good.
    const stale = fresh
      .filter(
        (notification) => toastLife(notification, config.timeout, now) <= 0,
      )
      .map((notification) => notification.id);
    if (stale.length > 0) {
      setExpired((previous) => [...previous, ...stale]);
    }

    // `expired` is deliberately not a dependency: this only has to look again
    // when something new arrives.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [notifications, config.timeout, config.doNotDisturb]);

  const visible = groups
    .map((group) => ({
      ...group,
      notifications: group.notifications.filter(
        (notification) => !expired.includes(notification.id),
      ),
    }))
    .filter((group) => group.notifications.length > 0)
    .slice(0, config.maxVisible);

  // This window is a quarter of the screen and is not click-through, so while
  // it is on screen with nothing on it every click in that rectangle lands
  // here and goes no further — `pointer-events: none` cannot pass a click to
  // another window. Only the page knows whether a toast is up, so the page is
  // what parks the window off screen when none is.
  const showing = !config.doNotDisturb && visible.length > 0;
  useEffect(() => {
    void actions.setSurfaceRevealed("notifications", showing);
  }, [showing]);

  if (config.doNotDisturb) return null;

  const fromBottom = config.position.startsWith("bottom");

  return (
    <div className="bw-toasts" data-bottom={fromBottom}>
      {visible.map((group) => {
        const [newest, ...rest] = group.notifications;
        if (!newest) return null;

        return (
          <Toast
            key={newest.id}
            notification={newest}
            extra={rest.length}
            timeout={config.timeout}
            onExpire={() => setExpired((previous) => [...previous, newest.id])}
          />
        );
      })}
    </div>
  );
}
