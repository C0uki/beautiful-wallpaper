// The notification history.
//
// The toasts show what has just arrived; this shows everything still held,
// grouped by application, with the count and the two controls the original
// puts under the list: silence, and clear all.

import { useState } from "react";
import { IconButton, Placeholder, ScrollArea, Symbol } from "../../widgets";
import { formatAge } from "../../lib/format";
import { tr } from "../../i18n";
import { actions, useShell } from "../../shell/store";
import type { Notification } from "@bw/core";

function groupByApp(notifications: Notification[]) {
  const groups: Array<{ appName: string; items: Notification[] }> = [];
  for (const notification of notifications) {
    const existing = groups.find(
      (group) => group.appName === notification.appName,
    );
    if (existing) existing.items.push(notification);
    else groups.push({ appName: notification.appName, items: [notification] });
  }
  return groups;
}

export function NotificationCentre() {
  const notifications = useShell((state) => state.notifications);
  const doNotDisturb = useShell(
    (state) => state.config.notifications.doNotDisturb,
  );
  const now = useShell((state) => state.now);
  const [collapsed, setCollapsed] = useState<string[]>([]);

  const groups = groupByApp(notifications);

  return (
    <div className="bw-card bw-notification-centre">
      {notifications.length === 0 ? (
        <Placeholder icon="notifications_active" text={tr("Nothing")} />
      ) : (
        <ScrollArea className="bw-notification-list">
          {groups.map((group) => {
            const isCollapsed = collapsed.includes(group.appName);
            const shown = isCollapsed ? group.items.slice(0, 1) : group.items;

            return (
              <section key={group.appName} className="bw-notification-group">
                <header>
                  <button
                    type="button"
                    onClick={() =>
                      setCollapsed((current) =>
                        current.includes(group.appName)
                          ? current.filter((name) => name !== group.appName)
                          : [...current, group.appName],
                      )
                    }
                    aria-expanded={!isCollapsed}
                  >
                    <Symbol
                      name={isCollapsed ? "expand_more" : "expand_less"}
                      size={18}
                    />
                    <span>{group.appName}</span>
                    {group.items.length > 1 ? (
                      <span className="bw-notification-count">
                        {group.items.length}
                      </span>
                    ) : null}
                  </button>
                </header>

                {shown.map((notification) => (
                  <article
                    key={notification.id}
                    className="bw-notification"
                    data-urgency={notification.urgency}
                  >
                    <div className="bw-notification-text">
                      <div className="bw-notification-heading">
                        <span className="bw-notification-summary">
                          {notification.summary}
                        </span>
                        <span className="bw-notification-age">
                          {formatAge(notification.time, now.getTime() / 1000)}
                        </span>
                      </div>
                      {notification.body ? (
                        <span className="bw-notification-body">
                          {notification.body}
                        </span>
                      ) : null}
                    </div>
                    <IconButton
                      icon="close"
                      size={28}
                      label={tr("Dismiss")}
                      onClick={() =>
                        void actions.dismissNotification(notification.id)
                      }
                    />
                  </article>
                ))}
              </section>
            );
          })}
        </ScrollArea>
      )}

      <footer className="bw-notification-actions">
        <button
          type="button"
          className="bw-notification-action"
          data-on={doNotDisturb}
          aria-pressed={doNotDisturb}
          aria-label={tr("Do not disturb")}
          onClick={() =>
            void actions.setConfigValue(
              "notifications.doNotDisturb",
              !doNotDisturb,
            )
          }
        >
          <Symbol name="notifications_paused" size={18} filled={doNotDisturb} />
        </button>

        <span className="bw-notification-total">
          {tr("%1 notifications").replace("%1", String(notifications.length))}
        </span>

        <button
          type="button"
          className="bw-notification-action"
          aria-label={tr("Clear all")}
          disabled={notifications.length === 0}
          onClick={() => void actions.clearNotifications()}
        >
          <Symbol name="delete_sweep" size={18} />
        </button>
      </footer>
    </div>
  );
}
