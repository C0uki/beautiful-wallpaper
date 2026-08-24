// The toast stack.
//
// Follows the original's behaviour rather than Windows': toasts group by the
// application that sent them, they can be swiped away, and the neighbours
// follow the drag a little so the stack feels like one object.
//
// Only the shell's own notifications reach this today. Reading other
// applications' notifications needs package identity, which arrives with the
// MSIX sparse package in a later phase — so the store this reads from is
// deliberately source-agnostic and will not need reshaping then.

import { useEffect, useMemo, useRef, useState } from "react";
import { IconButton, Symbol } from "../../widgets";
import { formatAge } from "../../lib/format";
import { tr } from "../../i18n";
import { actions, useShell } from "../../shell/store";
import type { Notification } from "@bw/core";
import "./toasts.css";

/** Drag further than this and letting go dismisses. */
const DISMISS_THRESHOLD = 70;

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

/** One toast, draggable sideways to dismiss. */
function Toast({
  notification,
  extra,
  onDismiss,
  onDragDistance,
  neighbourOffset,
}: {
  notification: Notification;
  /** How many more from the same application are stacked behind this one. */
  extra: number;
  onDismiss: () => void;
  onDragDistance: (distance: number) => void;
  neighbourOffset: number;
}) {
  const [distance, setDistance] = useState(0);
  const [leaving, setLeaving] = useState(false);
  const start = useRef<number | null>(null);
  const now = useShell((state) => state.now);

  const offset = start.current === null ? neighbourOffset : distance;

  const finish = () => {
    if (start.current === null) return;
    start.current = null;
    onDragDistance(0);

    if (Math.abs(distance) > DISMISS_THRESHOLD) {
      // Let it fly out before it is actually removed, so the list does not
      // snap closed under the pointer.
      setLeaving(true);
      setDistance(distance > 0 ? 400 : -400);
      window.setTimeout(onDismiss, 180);
      return;
    }
    setDistance(0);
  };

  return (
    <div
      className="bw-toast"
      data-urgency={notification.urgency}
      data-leaving={leaving}
      style={{
        transform: `translateX(${offset}px)`,
        opacity: leaving ? 0 : 1 - Math.min(Math.abs(offset) / 260, 0.55),
        transition: start.current === null ? undefined : "none",
      }}
      onPointerDown={(event) => {
        start.current = event.clientX;
        (event.target as Element).setPointerCapture?.(event.pointerId);
      }}
      onPointerMove={(event) => {
        if (start.current === null) return;
        const moved = event.clientX - start.current;
        setDistance(moved);
        onDragDistance(moved);
      }}
      onPointerUp={finish}
      onPointerCancel={finish}
    >
      <div className="bw-toast-icon">
        <Symbol
          name={notification.image ? "image" : "notifications"}
          size={20}
          filled
        />
      </div>

      <div className="bw-toast-text">
        <div className="bw-toast-heading">
          <span className="bw-toast-app">{notification.appName}</span>
          <span className="bw-toast-age">
            {formatAge(notification.time, now.getTime() / 1000)}
          </span>
        </div>
        <span className="bw-toast-summary">{notification.summary}</span>
        {notification.body ? (
          <span className="bw-toast-body">{notification.body}</span>
        ) : null}
        {extra > 0 ? (
          <span className="bw-toast-more">
            {tr("+%1 more").replace("%1", String(extra))}
          </span>
        ) : null}
      </div>

      <IconButton
        icon="close"
        size={30}
        label={tr("Dismiss")}
        onClick={(event) => {
          event.stopPropagation();
          onDismiss();
        }}
      />
    </div>
  );
}

export function Toasts() {
  const notifications = useShell((state) => state.notifications);
  const config = useShell((state) => state.config.notifications);
  const [dragged, setDragged] = useState<{
    appName: string;
    distance: number;
  } | null>(null);
  const [expired, setExpired] = useState<number[]>([]);

  const groups = useMemo(() => groupByApp(notifications), [notifications]);

  // Toasts leave on their own; the notification itself stays in the history.
  useEffect(() => {
    if (config.doNotDisturb) return;

    const timers = notifications
      .filter((notification) => notification.urgency !== "critical")
      .filter((notification) => !expired.includes(notification.id))
      .map((notification) =>
        window.setTimeout(
          () => setExpired((previous) => [...previous, notification.id]),
          config.timeout,
        ),
      );
    return () => timers.forEach(window.clearTimeout);
    // `expired` is deliberately not a dependency: adding to it would restart
    // every other toast's timer.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [notifications, config.timeout, config.doNotDisturb]);

  if (config.doNotDisturb) return null;

  const visible = groups
    .map((group) => ({
      ...group,
      notifications: group.notifications.filter(
        (notification) => !expired.includes(notification.id),
      ),
    }))
    .filter((group) => group.notifications.length > 0)
    .slice(0, config.maxVisible);

  const fromBottom = config.position.startsWith("bottom");

  return (
    <div className="bw-toasts" data-bottom={fromBottom}>
      {visible.map((group) => {
        const [newest, ...rest] = group.notifications;
        if (!newest) return null;
        const dragging = dragged?.appName === group.appName;

        return (
          <Toast
            key={newest.id}
            notification={newest}
            extra={rest.length}
            neighbourOffset={dragging ? 0 : (dragged?.distance ?? 0) * 0.15}
            onDragDistance={(distance) =>
              setDragged({ appName: group.appName, distance })
            }
            onDismiss={() => void actions.dismissNotification(newest.id)}
          />
        );
      })}
    </div>
  );
}
