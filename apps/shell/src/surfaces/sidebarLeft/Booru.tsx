// The image-board browser.
//
// A tag search over a handful of boards, in a masonry grid, with each result
// settable as the wallpaper. The safe-rating filter is applied by the backend
// as part of the query rather than here — a filter in the UI is one forgotten
// branch away from showing what it was meant to hide, and it would spend the
// request either way.
//
// Whether this tab exists at all is `policies.weeb`, which ships at 0.

import { useCallback, useEffect, useRef, useState } from "react";
import {
  Button,
  IconButton,
  Placeholder,
  SearchField,
  Symbol,
} from "../../widgets";
import { tr } from "../../i18n";
import { actions, useShell } from "../../shell/store";
import type { BooruItem } from "@bw/core";
import "./booru.css";

/** Boards, and what each is good for. Wording follows the original's. */
const BOARDS = [
  {
    id: "safebooru",
    label: "Safebooru",
    note: () => tr("Safe-rated work only"),
  },
  {
    id: "yandere",
    label: "yande.re",
    note: () => tr("All-rounder, good quality"),
  },
  { id: "konachan", label: "Konachan", note: () => tr("Desktop wallpapers") },
  {
    id: "danbooru",
    label: "Danbooru",
    note: () => tr("The largest, quality varies"),
  },
  {
    id: "gelbooru",
    label: "Gelbooru",
    note: () => tr("The largest, quality varies"),
  },
];

export function Booru() {
  const settings = useShell((state) => state.config.sidebar.left.booru);

  const [tags, setTags] = useState("");
  const [items, setItems] = useState<BooruItem[]>([]);
  const [page, setPage] = useState(1);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [applied, setApplied] = useState<string | null>(null);

  // Only the newest search may write results. Without this, a slow first page
  // can land after a faster second one and overwrite it.
  const generation = useRef(0);

  const search = useCallback(
    async (wanted: number, append: boolean) => {
      const mine = ++generation.current;
      setBusy(true);
      setError(null);

      try {
        const result = await actions.searchBooru(tags, wanted);
        if (mine !== generation.current) return;
        setItems((current) =>
          append ? [...current, ...result.items] : result.items,
        );
        setPage(wanted);
      } catch (reason) {
        if (mine === generation.current) setError(String(reason));
      } finally {
        if (mine === generation.current) setBusy(false);
      }
    },
    [tags],
  );

  // The first page loads on open so the tab is not an empty box.
  useEffect(() => {
    void search(1, false);
    // Deliberately once: re-running on every `tags` keystroke would search
    // per character. The field searches on Enter instead.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [settings.provider]);

  const apply = async (item: BooruItem) => {
    setApplied(item.id);
    try {
      const path = await actions.downloadWallpaper(
        item.file,
        settings.provider,
      );
      await actions.applyWallpaper(path);
    } finally {
      window.setTimeout(() => setApplied(null), 1200);
    }
  };

  return (
    <div className="bw-booru">
      <div className="bw-booru-controls">
        <SearchField
          value={tags}
          placeholder={tr("Tags")}
          onChange={setTags}
          onSubmit={() => void search(1, false)}
        />
        <select
          value={settings.provider}
          aria-label={tr("Board")}
          onChange={(event) =>
            void actions.setConfigValue(
              "sidebar.left.booru.provider",
              event.target.value,
            )
          }
        >
          {BOARDS.map((board) => (
            <option key={board.id} value={board.id} title={board.note()}>
              {board.label}
            </option>
          ))}
        </select>
      </div>

      <p className="bw-booru-note">
        {settings.allowAdult
          ? tr("The safe-rating filter is off, from settings.")
          : tr("Showing safe-rated work only.")}
      </p>

      {error ? (
        <div className="bw-booru-error">
          <span>{error}</span>
          <Button variant="text" onClick={() => void search(page, false)}>
            {tr("Try again")}
          </Button>
        </div>
      ) : null}

      {items.length === 0 && !busy ? (
        <Placeholder icon="image_search" text={tr("Nothing found")} />
      ) : (
        <div className="bw-booru-grid">
          {items.map((item) => (
            <figure
              key={`${item.id}-${item.preview}`}
              className="bw-booru-item"
            >
              <img
                src={item.preview}
                alt={item.tags}
                loading="lazy"
                // A board's own aspect ratio, so the grid does not reflow as
                // each thumbnail finishes loading.
                style={{
                  aspectRatio:
                    item.width && item.height
                      ? `${item.width} / ${item.height}`
                      : undefined,
                }}
              />
              <figcaption>
                <IconButton
                  icon={applied === item.id ? "check" : "wallpaper"}
                  size={30}
                  label={tr("Set as wallpaper")}
                  onClick={() => void apply(item)}
                />
                <IconButton
                  icon="open_in_new"
                  size={30}
                  label={tr("Open on the board")}
                  onClick={() => void actions.openUrl(item.pageUrl)}
                />
              </figcaption>
            </figure>
          ))}
        </div>
      )}

      <div className="bw-booru-more">
        {busy ? (
          <Symbol name="autorenew" size={18} className="bw-booru-spinner" />
        ) : items.length > 0 ? (
          <Button variant="tonal" onClick={() => void search(page + 1, true)}>
            {tr("Load more")}
          </Button>
        ) : null}
      </div>
    </div>
  );
}
