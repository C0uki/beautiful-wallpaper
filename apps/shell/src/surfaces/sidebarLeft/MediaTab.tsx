// The media tab.
//
// The right sidebar has a compact card; here there is a whole panel, so the
// artwork gets room and the position bar is seekable-looking, with a
// spectrum of the output under it while something plays, and the lyrics
// under that — from lrclib.net, following the song when they are timed.

import { useEffect, useRef, useState } from "react";
import type { Lyrics, MediaState } from "@bw/core";
import { IconButton, Placeholder, Symbol } from "../../widgets";
import { formatClock } from "../../lib/format";
import { tr } from "../../i18n";
import { actions, useShell } from "../../shell/store";
import { backend } from "../../shell/backend";
import { Visualizer } from "../../widgets/Visualizer";

/** The playing track's lyrics, with the line being sung kept in view. */
function MediaLyrics({ media }: { media: MediaState }) {
  const track = `${media.artist}\u0000${media.title}`;
  const [found, setFound] = useState<{
    track: string;
    lyrics: Lyrics | null;
  }>();
  const box = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let current = true;
    void actions
      .getLyrics(media)
      .catch(() => null)
      .then((lyrics) => {
        if (current) setFound({ track, lyrics });
      });
    return () => {
      current = false;
    };
    // Asked once a track: the position changes every second and is not a
    // new song.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [track]);

  const lyrics = found?.track === track ? found.lyrics : null;
  const lines = lyrics?.lines ?? [];

  // The position arrives once a second, and the lines are timed to the
  // hundredth: between reports the song goes on, so the time since the last
  // one is added. No more than a report and a half, in case they stop.
  const heard = useRef({ position: media.position, at: performance.now() });
  if (heard.current.position !== media.position) {
    heard.current = { position: media.position, at: performance.now() };
  }
  const [, tick] = useState(0);
  const timed = lines.length > 0;
  useEffect(() => {
    if (!media.playing || !timed) return;
    const timer = window.setInterval(() => tick((count) => count + 1), 200);
    return () => window.clearInterval(timer);
  }, [media.playing, timed]);
  const position = media.playing
    ? heard.current.position +
      Math.min((performance.now() - heard.current.at) / 1000, 1.5)
    : media.position;

  const sung = lines.reduce(
    (last, line, index) => (line.time <= position ? index : last),
    -1,
  );

  useEffect(() => {
    const scroller = box.current;
    const line = scroller?.children[sung] as HTMLElement | undefined;
    if (!scroller || !line) return;
    scroller.scrollTo({
      top: line.offsetTop - (scroller.clientHeight - line.clientHeight) / 2,
      behavior: "smooth",
    });
  }, [sung]);

  if (!lyrics) return null;
  return (
    <div className="bw-media-tab-lyrics" ref={box} aria-label={tr("Lyrics")}>
      {lines.length ? (
        lines.map((line, index) => (
          <p key={index} data-sung={index === sung}>
            {line.text || "♪"}
          </p>
        ))
      ) : (
        <p className="bw-media-tab-plain">{lyrics.plain}</p>
      )}
    </div>
  );
}

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
          <img src={backend().assetUrl(media.artwork)} alt="" />
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

      <MediaLyrics media={media} />
    </div>
  );
}
