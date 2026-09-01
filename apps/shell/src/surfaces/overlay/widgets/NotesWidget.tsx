// A sticky note.
//
// Saved as the user stops typing rather than on every keystroke: the text goes
// through the persistent store to disk, and a write per character would be a
// write per character.

import { useEffect, useRef, useState } from "react";
import { tr } from "../../../i18n";
import { actions, useShell } from "../../../shell/store";

/** Long enough that a pause is a pause, short enough to survive a crash. */
const SAVE_DELAY = 600;

export function NotesWidget({ editable }: { editable: boolean }) {
  const stored = useShell((state) => state.persistent.overlay.notesText);
  const [text, setText] = useState(stored);
  const pending = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Follows the store while the user is not the one changing it — another
  // window, or a config reload, is allowed to win.
  useEffect(() => {
    setText((current) => (pending.current ? current : stored));
  }, [stored]);

  useEffect(
    () => () => {
      if (pending.current) clearTimeout(pending.current);
    },
    [],
  );

  const change = (next: string) => {
    setText(next);
    if (pending.current) clearTimeout(pending.current);
    pending.current = setTimeout(() => {
      pending.current = null;
      void actions.setPersistentValue("overlay.notesText", next);
    }, SAVE_DELAY);
  };

  return (
    <textarea
      className="bw-overlay-notes"
      value={text}
      readOnly={!editable}
      spellCheck={false}
      placeholder={tr("Anything worth keeping for the next five minutes")}
      onChange={(event) => change(event.target.value)}
    />
  );
}
