// Now playing, from the Windows media session.
//
// The bar has a compact version of this; here there is room for artwork and
// the position, so it gets both.

import { IconButton, Symbol } from "../../widgets";
import { tr } from "../../i18n";
import { actions, useShell } from "../../shell/store";

function formatPosition(seconds: number): string {
  const total = Math.max(0, Math.floor(seconds));
  const minutes = Math.floor(total / 60);
  return `${minutes}:${String(total % 60).padStart(2, "0")}`;
}

export function MediaCard() {
  const media = useShell((state) => state.media);

  // No session at all is the common case on a fresh desktop; an empty card
  // would just be a hole in the column.
  if (!media || !media.title) return null;

  const progress =
    media.duration > 0
      ? Math.min(1, Math.max(0, media.position / media.duration))
      : 0;

  return (
    <div className="bw-card bw-media-card">
      <div className="bw-media-art">
        {media.artwork ? (
          <img src={media.artwork} alt="" />
        ) : (
          <Symbol name="music_note" size={28} />
        )}
      </div>

      <div className="bw-media-body">
        <span className="bw-media-title">{media.title}</span>
        <span className="bw-media-artist">{media.artist || media.source}</span>

        {media.duration > 0 ? (
          <div className="bw-media-progress">
            <div style={{ width: `${progress * 100}%` }} />
          </div>
        ) : null}

        <div className="bw-media-row">
          <span className="bw-media-time">
            {media.duration > 0
              ? `${formatPosition(media.position)} / ${formatPosition(media.duration)}`
              : ""}
          </span>
          <div className="bw-media-buttons">
            <IconButton
              icon="skip_previous"
              size={32}
              label={tr("Previous")}
              onClick={() => void actions.mediaCommand("previous")}
            />
            <IconButton
              icon={media.playing ? "pause" : "play_arrow"}
              size={38}
              label={media.playing ? tr("Pause") : tr("Play")}
              onClick={() => void actions.mediaCommand("playPause")}
            />
            <IconButton
              icon="skip_next"
              size={32}
              label={tr("Next")}
              onClick={() => void actions.mediaCommand("next")}
            />
          </div>
        </div>
      </div>
    </div>
  );
}
