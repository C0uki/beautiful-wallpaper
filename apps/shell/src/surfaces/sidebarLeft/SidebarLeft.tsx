// The left sidebar.
//
// The original carries four tabs here — AI chat, translator, media and a booru
// browser. Two are built; the chat and the booru are later work. The tab strip
// is driven by which are enabled, so it copes with one, two, or none.

import { useEffect, useState } from "react";
import { Placeholder, ScrollArea, Symbol } from "../../widgets";
import { tr } from "../../i18n";
import { actions, connectSidebarLeft, useShell } from "../../shell/store";
import { Chat } from "./Chat";
import { MediaTab } from "./MediaTab";
import { Translator } from "./Translator";
import "../panel.css";
import "./sidebarLeft.css";

interface Tab {
  id: string;
  icon: string;
  label: string;
  content: () => React.ReactNode;
}

export function SidebarLeft() {
  const config = useShell((state) => state.config.sidebar.left);
  const policies = useShell((state) => state.config.policies);
  const open = useShell((state) => state.states.sidebarLeftOpen);
  const ready = useShell((state) => state.ready);
  const [active, setActive] = useState<string | null>(null);

  useEffect(() => {
    void connectSidebarLeft();
  }, []);

  // Escape closes, as every other overlay does.
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape")
        void actions.setState("sidebarLeftOpen", false);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  if (!ready) return null;

  const tabs: Tab[] = [
    // The original gates this on `policies.ai`; 0 means off entirely.
    ...(policies.ai !== 0
      ? [
          {
            id: "chat",
            icon: "neurology",
            label: tr("Intelligence"),
            content: () => <Chat />,
          },
        ]
      : []),
    ...(config.translator.enable
      ? [
          {
            id: "translator",
            icon: "translate",
            label: tr("Translator"),
            content: () => <Translator />,
          },
        ]
      : []),
    ...(config.media.enable
      ? [
          {
            id: "media",
            icon: "music_note",
            label: tr("Media"),
            content: () => <MediaTab />,
          },
        ]
      : []),
  ];

  // `active` is only a preference; the tab it names may have been switched off
  // since, so the list decides what is actually shown.
  const current = tabs.find((tab) => tab.id === active) ?? tabs[0];

  return (
    <div className="bw-sidebar bw-sidebar-left" data-open={open}>
      {tabs.length === 0 ? (
        <Placeholder icon="inbox" text={tr("Enjoy your empty sidebar…")} />
      ) : (
        <>
          <nav className="bw-sidebar-left-tabs" role="tablist">
            {tabs.map((tab) => (
              <button
                key={tab.id}
                type="button"
                role="tab"
                aria-selected={tab.id === current?.id}
                data-active={tab.id === current?.id}
                aria-label={tab.label}
                title={tab.label}
                onClick={() => setActive(tab.id)}
              >
                <Symbol
                  name={tab.icon}
                  size={22}
                  filled={tab.id === current?.id}
                />
              </button>
            ))}
          </nav>

          <ScrollArea className="bw-sidebar-left-body">
            {current?.content()}
          </ScrollArea>
        </>
      )}
    </div>
  );
}
