// The tabbed group at the foot of the sidebar.
//
// Which tab is open and whether the group is collapsed are runtime state, not
// configuration, so they live in the state store — which means they survive a
// restart without ending up in a config file people share.

import { IconButton, Tabs } from "../../widgets";
import { tr } from "../../i18n";
import { actions, useShell } from "../../shell/store";
import { Calendar } from "./Calendar";
import { TodoList } from "./TodoList";
import { Timer } from "./Timer";

const TABS = [
  { id: "calendar", icon: "calendar_month", label: () => tr("Calendar") },
  { id: "todo", icon: "done_outline", label: () => tr("To Do") },
  { id: "timer", icon: "schedule", label: () => tr("Timer") },
];

export function BottomGroup() {
  const group = useShell((state) => state.persistent.sidebar.bottomGroup);
  const todos = useShell((state) => state.todos);
  const now = useShell((state) => state.now);

  const index = Math.min(group.tab, TABS.length - 1);
  const active = TABS[index]!;

  const setCollapsed = (collapsed: boolean) =>
    void actions.setPersistentValue("sidebar.bottomGroup.collapsed", collapsed);

  if (group.collapsed) {
    const remaining = todos.filter((todo) => !todo.done).length;
    return (
      <div className="bw-card bw-bottom-group bw-bottom-group-collapsed">
        <IconButton
          icon="keyboard_arrow_up"
          size={34}
          label={tr("Expand")}
          onClick={() => setCollapsed(false)}
        />
        <span>
          {tr("%1   •   %2 tasks")
            .replace(
              "%1",
              now.toLocaleDateString(undefined, {
                weekday: "short",
                day: "numeric",
                month: "short",
              }),
            )
            .replace("%2", String(remaining))}
        </span>
      </div>
    );
  }

  return (
    <div className="bw-card bw-bottom-group">
      <div className="bw-bottom-group-head">
        <Tabs
          tabs={TABS.map((tab) => ({
            id: tab.id,
            label: tab.label(),
            icon: tab.icon,
          }))}
          active={active.id}
          onSelect={(id) =>
            void actions.setPersistentValue(
              "sidebar.bottomGroup.tab",
              TABS.findIndex((tab) => tab.id === id),
            )
          }
        />
        <IconButton
          icon="keyboard_arrow_down"
          size={34}
          label={tr("Collapse")}
          onClick={() => setCollapsed(true)}
        />
      </div>

      <div className="bw-bottom-group-body">
        {active.id === "calendar" ? <Calendar /> : null}
        {active.id === "todo" ? <TodoList /> : null}
        {active.id === "timer" ? <Timer /> : null}
      </div>
    </div>
  );
}
