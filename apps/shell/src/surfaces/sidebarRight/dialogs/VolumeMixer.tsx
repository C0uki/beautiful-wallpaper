// Per-application volume.
//
// Sessions that have stopped playing are dimmed rather than removed: a video
// ending should not make a slider vanish from under the pointer.

import {
  Dialog,
  Placeholder,
  ScrollArea,
  Slider,
  Symbol,
} from "../../../widgets";
import { tr } from "../../../i18n";
import { backend } from "../../../shell/backend";
import { actions, useShell } from "../../../shell/store";

export function VolumeMixer({ onDismiss }: { onDismiss: () => void }) {
  const sessions = useShell((state) => state.sessions);

  return (
    <Dialog title={tr("Volume mixer")} icon="tune" onDismiss={onDismiss}>
      {sessions.length === 0 ? (
        <Placeholder icon="volume_off" text={tr("Nothing is playing")} />
      ) : (
        <ScrollArea className="bw-mixer">
          {sessions.map((session) => (
            <div
              key={session.id}
              className="bw-mixer-row"
              data-inactive={!session.active}
            >
              <button
                type="button"
                className="bw-mixer-icon"
                aria-label={
                  session.muted ? tr("Click to unmute") : tr("Click to mute")
                }
                onClick={() =>
                  void actions.setSessionMuted(session.id, !session.muted)
                }
              >
                {session.icon ? (
                  <img
                    src={backend().assetUrl(session.icon)}
                    alt=""
                    data-muted={session.muted}
                  />
                ) : (
                  <Symbol name="apps" size={22} />
                )}
                {session.muted ? (
                  <span className="bw-mixer-muted">
                    <Symbol name="volume_off" size={16} />
                  </span>
                ) : null}
              </button>

              <div className="bw-mixer-body">
                <span className="bw-mixer-name">{session.name}</span>
                <Slider
                  label={session.name}
                  value={session.muted ? 0 : session.percent}
                  onChange={(percent) =>
                    void actions.setSessionVolume(session.id, percent)
                  }
                />
              </div>
            </div>
          ))}
        </ScrollArea>
      )}
    </Dialog>
  );
}
