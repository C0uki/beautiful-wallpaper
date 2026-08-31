// Picking a region of the screen.
//
// What is drawn here is not the screen: it is a copy of the screen taken
// before this window appeared. That is what keeps the overlay out of its own
// screenshot, and it means the selection is made against what the user was
// looking at rather than against whatever has changed since.
//
// The consequence is that everything is measured against the image rather
// than the viewport. The frame is in physical pixels and this window is in CSS
// ones, so the drag has to be scaled — and the scale is taken from the image's
// own dimensions against its layout size, not from `devicePixelRatio`, which
// disagrees often enough to matter.

import { useCallback, useEffect, useRef, useState } from "react";
import type { CaptureFrame, CaptureOutcome } from "@bw/core";
import { Event } from "@bw/core";
import { IconButton, Symbol } from "../../widgets";
import { tr } from "../../i18n";
import { actions, connect, useShell } from "../../shell/store";
import { describeError } from "../../shell/errors";
import { backend } from "../../shell/backend";
import "./regionSelect.css";

/** Below this a drag is a mis-click, not a selection. Mirrors `bw-core`. */
const MIN_SELECTION = 8;

interface Drag {
  from: { x: number; y: number };
  to: { x: number; y: number };
}

export function RegionSelect() {
  const ready = useShell((state) => state.ready);
  const open = useShell((state) => state.states.regionSelectOpen);

  const [frame, setFrame] = useState<CaptureFrame | null>(null);
  const [drag, setDrag] = useState<Drag | null>(null);
  // Kept apart from `drag` so the box stays on screen once the button is
  // released: after a reading, which region was read is worth seeing.
  const [dragging, setDragging] = useState(false);
  const [outcome, setOutcome] = useState<CaptureOutcome | null>(null);
  const [working, setWorking] = useState(false);
  const image = useRef<HTMLImageElement>(null);

  useEffect(() => {
    void connect();
    // The frame arrives on its own channel rather than through the store: it
    // is one payload for one surface, and no other window has any use for it.
    void backend().listen<CaptureFrame>(Event.Capture, (next) => {
      setFrame(next);
      setDrag(null);
      setDragging(false);
      setOutcome(null);
      setWorking(false);
    });
  }, []);

  const cancel = useCallback(() => {
    setDrag(null);
    setDragging(false);
    setOutcome(null);
    void actions.cancelCapture();
  }, []);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") cancel();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [cancel]);

  const finish = useCallback(
    async (selection: Drag) => {
      const element = image.current;
      const shot = frame;
      if (!element || !shot) return;

      // Measured, not assumed: the ratio between the frame's real pixels and
      // the size it is being drawn at is the only scale that is certainly
      // right, whatever the window reports about itself.
      const scale = shot.width / element.clientWidth;

      const region = {
        x: Math.min(selection.from.x, selection.to.x),
        y: Math.min(selection.from.y, selection.to.y),
        width: Math.abs(selection.to.x - selection.from.x),
        height: Math.abs(selection.to.y - selection.from.y),
      };
      if (region.width < MIN_SELECTION || region.height < MIN_SELECTION) {
        cancel();
        return;
      }
      setDrag(selection);

      setWorking(true);
      try {
        setOutcome(await actions.finishCapture(region, scale));
      } catch (error) {
        setOutcome({
          saved: null,
          text: null,
          translated: null,
          problem: describeError(error),
        });
      } finally {
        setWorking(false);
      }
    },
    [cancel, frame],
  );

  if (!ready || !frame) return null;
  // The window is hidden by the backend, but a stale frame would flash on the
  // way back in if it were still drawn while closed.
  if (!open) return null;

  const box = drag ? rectangle(drag) : null;

  return (
    <div className="bw-region">
      <img
        ref={image}
        className="bw-region-frame"
        src={backend().assetUrl(frame.image)}
        alt=""
        draggable={false}
        onMouseDown={(event) => {
          if (outcome || working) return;
          const at = pointIn(event);
          setDrag({ from: at, to: at });
          setDragging(true);
        }}
        onMouseMove={(event) => {
          if (!dragging || !drag) return;
          setDrag({ from: drag.from, to: pointIn(event) });
        }}
        onMouseUp={() => {
          if (!dragging || !drag) return;
          setDragging(false);
          void finish(drag);
        }}
      />

      {/* Four panels rather than one box with a hole in it: a hole needs
          either a clip path or a huge shadow, and both fight the pointer. */}
      {box ? (
        <>
          <div
            className="bw-region-shade"
            style={{ inset: `0 0 auto 0`, height: box.top }}
          />
          <div
            className="bw-region-shade"
            style={{ top: box.top + box.height, left: 0, right: 0, bottom: 0 }}
          />
          <div
            className="bw-region-shade"
            style={{
              top: box.top,
              left: 0,
              width: box.left,
              height: box.height,
            }}
          />
          <div
            className="bw-region-shade"
            style={{
              top: box.top,
              left: box.left + box.width,
              right: 0,
              height: box.height,
            }}
          />
          <div
            className="bw-region-box"
            style={{
              top: box.top,
              left: box.left,
              width: box.width,
              height: box.height,
            }}
          >
            <span className="bw-region-size">
              {Math.round(box.width)} × {Math.round(box.height)}
            </span>
          </div>
        </>
      ) : (
        !outcome &&
        !working && (
          <div className="bw-region-shade bw-region-hint">
            <p>
              <Symbol name="crop_free" size={20} />
              {tr("Drag to choose a region, or press Escape")}
            </p>
          </div>
        )
      )}

      {working ? (
        <div className="bw-region-panel">
          <p className="bw-region-status">{tr("Reading…")}</p>
        </div>
      ) : null}

      {outcome ? <Result outcome={outcome} onClose={cancel} /> : null}
    </div>
  );
}

/** What came back, and a way out. */
function Result({
  outcome,
  onClose,
}: {
  outcome: CaptureOutcome;
  onClose: () => void;
}) {
  const nothing =
    !outcome.problem && !outcome.text && !outcome.translated && !outcome.saved;

  return (
    <div className="bw-region-panel">
      <div className="bw-region-panel-head">
        <span>{tr("Selection")}</span>
        <IconButton icon="close" size={30} label="Close" onClick={onClose} />
      </div>

      {outcome.problem ? (
        <p className="bw-region-problem">{outcome.problem}</p>
      ) : null}

      {/* An empty region is an outcome, not a failure: saying so is the
          difference between "there was no text there" and a silent panel. */}
      {nothing ? (
        <p className="bw-region-status">{tr("No text in that region")}</p>
      ) : null}

      {outcome.text ? (
        <Passage label={tr("Recognised")} text={outcome.text} />
      ) : null}
      {outcome.translated ? (
        <Passage label={tr("Translation")} text={outcome.translated} />
      ) : null}
    </div>
  );
}

function Passage({ label, text }: { label: string; text: string }) {
  const [copied, setCopied] = useState(false);

  return (
    <section className="bw-region-passage">
      <header>
        <span>{label}</span>
        <IconButton
          icon={copied ? "check" : "content_copy"}
          size={28}
          label={copied ? "Copied" : "Copy"}
          onClick={() => {
            void navigator.clipboard
              .writeText(text)
              .then(() => setCopied(true))
              // Clipboard access can be refused, and the text is on screen
              // either way; the button simply does not claim it worked.
              .catch(() => setCopied(false));
          }}
        />
      </header>
      <p>{text}</p>
    </section>
  );
}

/** Where the pointer is, in the image's own layout coordinates. */
function pointIn(event: React.MouseEvent<HTMLImageElement>) {
  const bounds = event.currentTarget.getBoundingClientRect();
  return { x: event.clientX - bounds.left, y: event.clientY - bounds.top };
}

/** A drag as a box, whichever way round it was made. */
function rectangle(drag: Drag) {
  return {
    left: Math.min(drag.from.x, drag.to.x),
    top: Math.min(drag.from.y, drag.to.y),
    width: Math.abs(drag.to.x - drag.from.x),
    height: Math.abs(drag.to.y - drag.from.y),
  };
}
