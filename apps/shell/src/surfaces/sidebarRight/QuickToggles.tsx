// The quick toggles, in either of the original's two styles.
//
// `classic` is a row of small circular buttons; `android` is a grid of wide
// tiles with a label, a state line and — where one exists — a chevron opening
// a dialog. The grid also has an edit mode for enabling, disabling, resizing
// and reordering tiles, which is what most of the code below is about.

import { Symbol, useRipple } from "../../widgets";
import { tr } from "../../i18n";
import { actions, useShell } from "../../shell/store";
import {
  useVisibleToggles,
  type DetailDialog,
  type ToggleDefinition,
} from "./toggles";

interface Props {
  editing: boolean;
  onOpenDialog: (dialog: DetailDialog) => void;
}

export function QuickToggles({ editing, onOpenDialog }: Props) {
  const style = useShell((state) => state.config.sidebar.quickToggles.style);
  const enabled = useShell((state) => state.config.sidebar.quickToggles.enable);

  if (!enabled) return null;
  return style === "classic" ? (
    <ClassicToggles onOpenDialog={onOpenDialog} />
  ) : (
    <AndroidToggles editing={editing} onOpenDialog={onOpenDialog} />
  );
}

/** A row of small round buttons — the original's `classic` panel. */
function ClassicToggles({
  onOpenDialog,
}: {
  onOpenDialog: (dialog: DetailDialog) => void;
}) {
  const shell = useShell();
  const toggles = useVisibleToggles();

  return (
    <div className="bw-card bw-toggles-classic">
      {toggles.map((toggle) => {
        const on = toggle.state(shell) ?? false;
        return (
          <button
            key={toggle.id}
            type="button"
            className="bw-toggle-round"
            data-on={on}
            aria-pressed={on}
            aria-label={toggle.label()}
            title={toggle.label()}
            onClick={() => toggle.toggle(shell)}
            onContextMenu={(event) => {
              if (!toggle.detail) return;
              // Right-click is the classic style's only route to a dialog;
              // there is no room on the button for a chevron.
              event.preventDefault();
              onOpenDialog(toggle.detail);
            }}
          >
            <Symbol
              name={on ? (toggle.iconOn ?? toggle.icon) : toggle.icon}
              size={20}
              filled={on}
            />
          </button>
        );
      })}
    </div>
  );
}

/** The grid of tiles — the original's `android` panel. */
function AndroidToggles({
  editing,
  onOpenDialog,
}: {
  editing: boolean;
  onOpenDialog: (dialog: DetailDialog) => void;
}) {
  const shell = useShell();
  const toggles = useVisibleToggles();
  const layout = shell.persistent.sidebar.quickToggles;

  // In edit mode every toggle is shown, including the disabled ones — they are
  // what the user is there to switch back on.
  const shown = editing
    ? [
        ...toggles,
        ...layout
          .filter((slot) => !slot.enabled)
          .map((slot) => toggles.find((toggle) => toggle.id === slot.id))
          .filter((toggle): toggle is ToggleDefinition => Boolean(toggle)),
      ]
    : toggles;

  const slotFor = (id: string) => layout.find((slot) => slot.id === id);

  /** Writes the whole layout back, materialising it on first edit. */
  const writeLayout = (
    change: (
      slots: Array<{ id: string; enabled: boolean; wide: boolean }>,
    ) => Array<{ id: string; enabled: boolean; wide: boolean }>,
  ) => {
    const current =
      layout.length > 0
        ? layout.map((slot) => ({ ...slot }))
        : toggles.map((toggle) => ({
            id: toggle.id,
            enabled: true,
            wide: false,
          }));
    void actions.setPersistentValue("sidebar.quickToggles", change(current));
  };

  return (
    <div className="bw-toggles-grid">
      {shown.map((toggle, index) => {
        const slot = slotFor(toggle.id);
        const on = toggle.state(shell) ?? false;
        const disabled = editing && slot?.enabled === false;

        return (
          <Tile
            key={toggle.id}
            toggle={toggle}
            on={on}
            wide={slot?.wide ?? false}
            editing={editing}
            dimmed={disabled}
            detailText={toggle.detailText?.(shell)}
            onActivate={() => {
              if (!editing) {
                toggle.toggle(shell);
                return;
              }
              writeLayout((slots) =>
                slots.map((entry) =>
                  entry.id === toggle.id
                    ? { ...entry, enabled: !entry.enabled }
                    : entry,
                ),
              );
            }}
            onResize={() =>
              writeLayout((slots) =>
                slots.map((entry) =>
                  entry.id === toggle.id
                    ? { ...entry, wide: !entry.wide }
                    : entry,
                ),
              )
            }
            onMove={(direction) =>
              writeLayout((slots) => {
                const from = slots.findIndex((entry) => entry.id === toggle.id);
                const to = from + direction;
                if (from < 0 || to < 0 || to >= slots.length) return slots;
                const next = [...slots];
                const [moved] = next.splice(from, 1);
                next.splice(to, 0, moved!);
                return next;
              })
            }
            canMoveUp={index > 0}
            canMoveDown={index < shown.length - 1}
            onDetail={
              toggle.detail
                ? () => onOpenDialog(toggle.detail as DetailDialog)
                : undefined
            }
          />
        );
      })}
    </div>
  );
}

function Tile({
  toggle,
  on,
  wide,
  editing,
  dimmed,
  detailText,
  onActivate,
  onResize,
  onMove,
  canMoveUp,
  canMoveDown,
  onDetail,
}: {
  toggle: ToggleDefinition;
  on: boolean;
  wide: boolean;
  editing: boolean;
  dimmed: boolean;
  detailText: string | undefined;
  onActivate: () => void;
  onResize: () => void;
  onMove: (direction: -1 | 1) => void;
  canMoveUp: boolean;
  canMoveDown: boolean;
  onDetail?: (() => void) | undefined;
}) {
  const ripple = useRipple();

  return (
    <div
      className="bw-toggle-tile"
      data-on={on}
      data-wide={wide}
      data-dimmed={dimmed}
      data-editing={editing}
    >
      <button
        type="button"
        className="bw-toggle-tile-main"
        aria-pressed={editing ? undefined : on}
        aria-label={toggle.label()}
        onPointerDown={ripple.spawn}
        onClick={onActivate}
      >
        <Symbol
          name={on ? (toggle.iconOn ?? toggle.icon) : toggle.icon}
          size={22}
          filled={on}
        />
        <span className="bw-toggle-tile-text">
          <span className="bw-toggle-tile-label">{toggle.label()}</span>
          {detailText ? (
            <span className="bw-toggle-tile-detail">{detailText}</span>
          ) : null}
        </span>
        {ripple.layer}
      </button>

      {editing ? (
        <div className="bw-toggle-tile-edit">
          <button
            type="button"
            aria-label={tr("Move up")}
            disabled={!canMoveUp}
            onClick={() => onMove(-1)}
          >
            <Symbol name="keyboard_arrow_up" size={16} />
          </button>
          <button
            type="button"
            aria-label={tr("Move down")}
            disabled={!canMoveDown}
            onClick={() => onMove(1)}
          >
            <Symbol name="keyboard_arrow_down" size={16} />
          </button>
          <button type="button" aria-label={tr("Resize")} onClick={onResize}>
            <Symbol
              name={wide ? "close_fullscreen" : "open_in_full"}
              size={16}
            />
          </button>
        </div>
      ) : onDetail ? (
        <button
          type="button"
          className="bw-toggle-tile-detail-button"
          aria-label={tr("More")}
          onClick={onDetail}
        >
          <Symbol name="chevron_right" size={18} />
        </button>
      ) : null}
    </div>
  );
}
