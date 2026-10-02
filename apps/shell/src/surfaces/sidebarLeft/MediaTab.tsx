// The media tab.
//
// The right sidebar has a compact card; here there is a whole panel, so the
// artwork gets room and the position bar is seekable-looking, with a
// spectrum of the output under it while something plays. The original also
// shows fetched lyrics, which came from an external script.

import { IconButton, Placeholder, Symbol } from "../../widgets";
import { formatClock } from "../../lib/format";
import { tr } from "../../i18n";
import { actions, useShell } from "../../shell/store";
import { Visualizer } from "../../widgets/Visualizer";

export function MediaTab() {
  const media = useShell((state) => state.media);

  if (!media?.title) {
    return <Placeholder icon="music_note" text={tr("Nothing is playing")} />;
  }

  const progress =
    media.duration > 0
      ? Math.min(1, Math.max(0, media.position / media.duration))
      : 0;

  return (
    <div className="bw-media-tab">
      <div className="bw-media-tab-art">
        {media.artwork ? (
          <img src={media.artwork} alt="" />
        ) : (
          <Symbol name="album" size={64} />
        )}
      </div>

      <div className="bw-media-tab-text">
        <span className="bw-media-tab-title">{media.title}</span>
        <span className="bw-media-tab-artist">{media.artist}</span>
        {media.album ? (
          <span className="bw-media-tab-album">{media.album}</span>
        ) : null}
      </div>

      {media.playing ? (
        <Visualizer bars={48} className="bw-media-tab-visualizer" />
      ) : null}

      {media.duration > 0 ? (
        <div className="bw-media-tab-progress">
          <div className="bw-media-tab-bar">
            <div style={{ width: `${progress * 100}%` }} />
          </div>
          <div className="bw-media-tab-times">
            <span>{formatClock(media.position)}</span>
            <span>{formatClock(media.duration)}</span>
          </div>
        </div>
      ) : null}

      <div className="bw-media-tab-buttons">
        <IconButton
          icon="skip_previous"
          size={40}
          label={tr("Previous")}
          onClick={() => void actions.mediaCommand("previous")}
        />
        <IconButton
          icon={media.playing ? "pause" : "play_arrow"}
          size={52}
          label={media.playing ? tr("Pause") : tr("Play")}
          onClick={() => void actions.mediaCommand("playPause")}
        />
        <IconButton
          icon="skip_next"
          size={40}
          label={tr("Next")}
          onClick={() => void actions.mediaCommand("next")}
        />
      </div>

      {media.source ? (
        <span className="bw-media-tab-source">{media.source}</span>
      ) : null}
    </div>
  );
}
