// The wallpaper picker.
//
// A port of end4-pC's `modules/ii/wallpaperSelector`: source tabs for the local
// folder and the three online providers, a search field, breadcrumb navigation
// with history, and a light/dark toggle. Left-click applies, right-click on an
// online result downloads without applying.

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  Command,
  type Entry,
  type Provider,
  type WallpaperItem,
  type WallpaperPage,
} from "@bw/core";
import { backend } from "../../shell/backend";
import { actions, useShell } from "../../shell/store";
import {
  Button,
  Card,
  IconButton,
  SearchField,
  Segmented,
  Symbol,
} from "../../widgets";
import { tr } from "../../i18n";

type Source = "local" | Provider;

const SOURCES: Array<{ value: Source; label: string }> = [
  { value: "local", label: "Local" },
  { value: "wallhaven", label: "Wallhaven" },
  { value: "unsplash", label: "Unsplash" },
  { value: "pexels", label: "Pexels" },
];

/** Directory navigation with back/forward, as `FolderListModelWithHistory` does. */
function useDirectoryHistory(initial: string) {
  const [stack, setStack] = useState<string[]>([initial]);
  const [index, setIndex] = useState(0);

  const current = stack[index] ?? initial;

  const go = useCallback(
    (path: string) => {
      setStack((previous) => [...previous.slice(0, index + 1), path]);
      setIndex((previous) => previous + 1);
    },
    [index],
  );

  return {
    current,
    go,
    back: () => setIndex((previous) => Math.max(0, previous - 1)),
    forward: () =>
      setIndex((previous) => Math.min(stack.length - 1, previous + 1)),
    canGoBack: index > 0,
    canGoForward: index < stack.length - 1,
    reset: (path: string) => {
      setStack([path]);
      setIndex(0);
    },
  };
}

function parentOf(path: string): string | null {
  const normalized = path.replace(/[\\/]+$/, "");
  const cut = Math.max(
    normalized.lastIndexOf("/"),
    normalized.lastIndexOf("\\"),
  );
  // Stop at a drive root (`C:`), which has no meaningful parent.
  if (cut <= 2) return null;
  return normalized.slice(0, cut);
}

function Tile({
  label,
  thumb,
  icon,
  selected,
  onActivate,
  onSecondary,
  caption,
}: {
  label: string;
  thumb?: string;
  icon?: string;
  selected?: boolean;
  onActivate: () => void;
  onSecondary?: () => void;
  caption?: string;
}) {
  return (
    <button
      type="button"
      onClick={onActivate}
      onContextMenu={(event) => {
        if (!onSecondary) return;
        event.preventDefault();
        onSecondary();
      }}
      title={label}
      style={{
        display: "block",
        width: "100%",
        textAlign: "left",
        borderRadius: 18,
        overflow: "hidden",
        background: "var(--layer3)",
        outline: selected ? "3px solid var(--primary)" : "none",
        outlineOffset: -3,
        transition: "transform var(--duration-fast) var(--ease-spatial-fast)",
      }}
    >
      <div
        style={{
          position: "relative",
          aspectRatio: "16 / 10",
          background: "var(--layer4)",
          display: "grid",
          placeItems: "center",
        }}
      >
        {thumb ? (
          <img
            src={thumb}
            alt=""
            loading="lazy"
            style={{ width: "100%", height: "100%", objectFit: "cover" }}
          />
        ) : (
          <Symbol
            name={icon ?? "image"}
            size={34}
            color="var(--on-surface-variant)"
          />
        )}
      </div>
      <div style={{ padding: "8px 10px" }}>
        <div
          style={{
            fontSize: "0.86em",
            overflow: "hidden",
            textOverflow: "ellipsis",
            whiteSpace: "nowrap",
          }}
        >
          {label}
        </div>
        {caption ? (
          <div
            style={{
              fontSize: "0.76em",
              color: "var(--on-surface-variant)",
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
            }}
          >
            {caption}
          </div>
        ) : null}
      </div>
    </button>
  );
}

export function WallpaperSelector() {
  const config = useShell((state) => state.config);
  const currentWallpaper = useShell((state) => state.wallpaper.path);
  const mode = useShell((state) => state.theme?.mode ?? "dark");

  const [source, setSource] = useState<Source>("local");
  const [search, setSearch] = useState("");
  const [entries, setEntries] = useState<Entry[]>([]);
  const [thumbs, setThumbs] = useState<Record<string, string>>({});
  const [online, setOnline] = useState<WallpaperPage | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const startDirectory =
    config.wallpaperSelector.userPath || "C:/Users/you/Pictures/Wallpapers";
  const history = useDirectoryHistory(startDirectory);
  const requestId = useRef(0);

  // Local listing.
  useEffect(() => {
    if (source !== "local") return;
    const id = ++requestId.current;
    setLoading(true);
    setError(null);

    void (async () => {
      try {
        const listed = await backend().invoke<Entry[]>(Command.ListWallpapers, {
          path: history.current,
        });
        if (id !== requestId.current) return;
        setEntries(listed);

        const previews = await Promise.all(
          listed
            .filter((entry) => !entry.isDirectory)
            .map(async (entry) => {
              const thumb = await backend()
                .invoke<string>(Command.ThumbnailFor, { path: entry.path })
                .catch(() => "");
              return [entry.path, thumb] as const;
            }),
        );
        if (id !== requestId.current) return;
        setThumbs(Object.fromEntries(previews));
      } catch (caught) {
        if (id === requestId.current) setError(String(caught));
      } finally {
        if (id === requestId.current) setLoading(false);
      }
    })();
  }, [source, history.current]);

  // Online search.
  const fetchOnline = useCallback(
    async (page: number, append: boolean) => {
      const id = ++requestId.current;
      setLoading(true);
      setError(null);
      try {
        const result = await backend().invoke<WallpaperPage>(
          Command.SearchOnlineWallpapers,
          {
            provider: source,
            search,
            page,
            resolution: config.wallpaperSelector.online.resolution,
            purity: config.wallpaperSelector.online.purity,
            category: config.wallpaperSelector.online.category,
          },
        );
        if (id !== requestId.current) return;
        setOnline((previous) =>
          append && previous
            ? { ...result, items: [...previous.items, ...result.items] }
            : result,
        );
      } catch (caught) {
        if (id === requestId.current) setError(String(caught));
      } finally {
        if (id === requestId.current) setLoading(false);
      }
    },
    [source, search, config.wallpaperSelector.online],
  );

  useEffect(() => {
    if (source === "local") return;
    setOnline(null);
    void fetchOnline(1, false);
  }, [source, fetchOnline]);

  const filtered = useMemo(() => {
    if (!search) return entries;
    const needle = search.toLowerCase();
    return entries.filter((entry) => entry.name.toLowerCase().includes(needle));
  }, [entries, search]);

  const applyLocal = (entry: Entry) => {
    if (entry.isDirectory) {
      history.go(entry.path);
      setSearch("");
      return;
    }
    void actions.applyWallpaper(entry.path);
    if (config.wallpaperSelector.closeAfterSelection) {
      void actions.setState("wallpaperSelectorOpen", false);
    }
  };

  const applyOnline = async (item: WallpaperItem, applyAfter: boolean) => {
    setLoading(true);
    try {
      const saved = await backend().invoke<string>(Command.DownloadWallpaper, {
        url: item.full,
        provider: item.provider,
        downloadLocation: item.downloadLocation,
      });
      if (applyAfter) await actions.applyWallpaper(saved);
    } catch (caught) {
      setError(String(caught));
    } finally {
      setLoading(false);
    }
  };

  const columns = Math.max(2, config.wallpaperSelector.columns);
  const parent = parentOf(history.current);

  return (
    <Card
      elevation={2}
      style={{
        display: "flex",
        flexDirection: "column",
        gap: 12,
        width: "100%",
        height: "100%",
        padding: 16,
        borderRadius: 26,
      }}
    >
      <header
        style={{
          display: "flex",
          alignItems: "center",
          gap: 10,
          flexWrap: "wrap",
        }}
      >
        <Segmented
          options={SOURCES.map((entry) => ({
            ...entry,
            label: tr(entry.label),
          }))}
          value={source}
          onChange={setSource}
        />
        <div style={{ flex: 1, minWidth: 180 }}>
          {config.wallpaperSelector.showSearchbar ? (
            <SearchField
              value={search}
              placeholder={tr("Search wallpapers")}
              onChange={setSearch}
              onSubmit={() => {
                if (source !== "local") void fetchOnline(1, false);
              }}
            />
          ) : null}
        </div>
        <IconButton
          icon={mode === "dark" ? "dark_mode" : "light_mode"}
          label={mode === "dark" ? tr("Dark") : tr("Light")}
          onClick={() =>
            void actions.setMode(mode === "dark" ? "light" : "dark")
          }
        />
        <IconButton
          icon="shuffle"
          label={tr("Random")}
          onClick={() => void actions.randomWallpaper()}
        />
        <IconButton
          icon="close"
          label={tr("Close")}
          onClick={() => void actions.setState("wallpaperSelectorOpen", false)}
        />
      </header>

      {source === "local" ? (
        <nav style={{ display: "flex", alignItems: "center", gap: 4 }}>
          <IconButton
            icon="arrow_back"
            size={32}
            label={tr("Back")}
            disabled={!history.canGoBack}
            onClick={history.back}
          />
          <IconButton
            icon="arrow_forward"
            size={32}
            label={tr("Forward")}
            disabled={!history.canGoForward}
            onClick={history.forward}
          />
          <IconButton
            icon="arrow_upward"
            size={32}
            label={tr("Up")}
            disabled={parent === null}
            onClick={() => parent && history.go(parent)}
          />
          <span
            style={{
              marginLeft: 6,
              color: "var(--on-surface-variant)",
              fontSize: "0.86em",
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
            }}
          >
            {history.current}
          </span>
        </nav>
      ) : null}

      {error ? (
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: 8,
            padding: "10px 12px",
            borderRadius: 14,
            background: "var(--error-container, var(--layer3))",
            color: "var(--m3-on-error-container, var(--on-surface))",
            fontSize: "0.88em",
          }}
        >
          <Symbol name="error" size={18} />
          {error}
        </div>
      ) : null}

      <div style={{ flex: 1, overflowY: "auto", overflowX: "hidden" }}>
        <div
          style={{
            display: "grid",
            gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))`,
            gap: 12,
            paddingRight: 4,
          }}
        >
          {source === "local"
            ? filtered.map((entry) => (
                <Tile
                  key={entry.path}
                  label={entry.name}
                  {...(entry.isDirectory
                    ? {}
                    : { thumb: thumbs[entry.path] ?? "" })}
                  icon={entry.isDirectory ? "folder" : "image"}
                  selected={entry.path === currentWallpaper}
                  onActivate={() => applyLocal(entry)}
                />
              ))
            : (online?.items ?? []).map((item) => (
                <Tile
                  key={item.id}
                  label={item.title || item.id}
                  thumb={item.thumb}
                  caption={
                    item.author
                      ? tr(
                          "%1 by %2",
                          `${item.width}×${item.height}`,
                          item.author,
                        )
                      : ""
                  }
                  onActivate={() => void applyOnline(item, true)}
                  onSecondary={() => void applyOnline(item, false)}
                />
              ))}
        </div>

        {!loading && source === "local" && filtered.length === 0 ? (
          <p
            style={{
              color: "var(--on-surface-variant)",
              padding: 24,
              textAlign: "center",
            }}
          >
            {search
              ? tr("Nothing matched %1", search)
              : tr("No wallpapers here")}
          </p>
        ) : null}

        {source !== "local" && online && online.page < online.totalPages ? (
          <div
            style={{ display: "flex", justifyContent: "center", padding: 16 }}
          >
            <Button
              icon="autorenew"
              disabled={loading}
              onClick={() => void fetchOnline(online.page + 1, true)}
            >
              {loading ? tr("Loading…") : tr("Load more")}
            </Button>
          </div>
        ) : null}
      </div>

      <footer
        style={{
          display: "flex",
          alignItems: "center",
          gap: 8,
          color: "var(--on-surface-variant)",
          fontSize: "0.8em",
        }}
      >
        {loading ? <Symbol name="autorenew" size={16} /> : null}
        {source !== "local" && online
          ? tr("Page %1 of %2", online.page, online.totalPages)
          : tr(
              "%1 wallpapers",
              filtered.filter((entry) => !entry.isDirectory).length,
            )}
      </footer>
    </Card>
  );
}
