// The controls every surface shares, drawn as macOS draws them.
//
// A card, the three buttons (the accent-filled default, the bezel push
// button and the borderless one), an icon button, a progress ring, a
// segmented control, a search field, a slider, a list row and a scroll area,
// plus the tab strip, switch, check box, sheet and empty-state placeholder.
// Pressing darkens a control rather than spreading a ripple across it, as
// on a Mac.

import {
  useEffect,
  type ButtonHTMLAttributes,
  type CSSProperties,
  type HTMLAttributes,
  type ReactNode,
} from "react";
import { Symbol } from "./Symbol";
import RubberSegment from "./reactbits/RubberSegment";
import SpringCheck from "./reactbits/SpringCheck";
import SquishSwitch from "./reactbits/SquishSwitch";
import "./widgets.css";

export { Symbol };
export type { SymbolProps } from "./Symbol";

export interface CardProps extends HTMLAttributes<HTMLDivElement> {
  children?: ReactNode;
  /** Let the wallpaper show through, by the amount the palette derives. */
  translucent?: boolean;
  elevation?: 0 | 1 | 2;
}

export function Card({
  children,
  translucent = true,
  elevation = 1,
  ...rest
}: CardProps) {
  return (
    <div
      {...rest}
      className={rest.className ? `bw-card ${rest.className}` : "bw-card"}
      data-translucent={translucent}
      data-elevation={elevation}
    >
      {children}
    </div>
  );
}

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "filled" | "tonal" | "text";
  icon?: string;
  active?: boolean;
}

export function Button({
  variant = "tonal",
  icon,
  active = false,
  children,
  ...rest
}: ButtonProps) {
  return (
    <button
      {...rest}
      type="button"
      className="bw-button"
      data-variant={variant}
      data-active={active}
    >
      {icon ? <Symbol name={icon} size={16} filled={active} /> : null}
      {children}
    </button>
  );
}

export interface IconButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  icon: string;
  size?: number;
  active?: boolean;
  label: string;
}

export function IconButton({
  icon,
  size = 40,
  active = false,
  label,
  ...rest
}: IconButtonProps) {
  return (
    <button
      {...rest}
      type="button"
      title={label}
      aria-label={label}
      aria-pressed={active}
      className="bw-icon-button"
      data-active={active}
      style={{ width: size, height: size, position: "relative", ...rest.style }}
    >
      <Symbol name={icon} size={Math.round(size * 0.5)} filled={active} />
    </button>
  );
}

export interface ProgressRingProps {
  value: number;
  size?: number;
  thickness?: number;
  color?: string;
  trackColor?: string;
}

export function ProgressRing({
  value,
  size = 44,
  thickness = 4,
  color = "var(--primary)",
  trackColor = "var(--layer3)",
}: ProgressRingProps) {
  const radius = (size - thickness) / 2;
  const circumference = 2 * Math.PI * radius;
  const clamped = Math.min(100, Math.max(0, value));

  return (
    <svg className="bw-ring" width={size} height={size} aria-hidden="true">
      <circle
        cx={size / 2}
        cy={size / 2}
        r={radius}
        stroke={trackColor}
        strokeWidth={thickness}
      />
      <circle
        cx={size / 2}
        cy={size / 2}
        r={radius}
        stroke={color}
        strokeWidth={thickness}
        strokeDasharray={circumference}
        strokeDashoffset={circumference * (1 - clamped / 100)}
      />
    </svg>
  );
}

export interface SegmentedProps<T extends string> {
  options: Array<{ value: T; label: string; icon?: string }>;
  value: T;
  onChange: (value: T) => void;
}

export function Segmented<T extends string>({
  options,
  value,
  onChange,
}: SegmentedProps<T>) {
  // React Bits' Rubber Segment, as a Mac's segmented control: a grey track
  // and a raised, lighter thumb under the chosen label — not the accent.
  // The thumb still follows a drag.
  return (
    <RubberSegment
      items={options.map((option) => ({
        value: option.value,
        label: option.label,
        ...(option.icon
          ? { icon: <Symbol name={option.icon} size={16} /> }
          : {}),
      }))}
      value={value}
      onChange={(next) => onChange(next as T)}
      size="sm"
      radius={7}
      trackColor="var(--layer2)"
      thumbColor="var(--bw-raised)"
      textColor="var(--on-surface)"
      activeTextColor="var(--on-surface)"
    />
  );
}

export interface SearchFieldProps {
  value: string;
  placeholder?: string;
  onChange: (value: string) => void;
  onSubmit?: (value: string) => void;
}

export function SearchField({
  value,
  placeholder,
  onChange,
  onSubmit,
}: SearchFieldProps) {
  return (
    <label className="bw-field">
      <Symbol name="search" size={18} />
      <input
        value={value}
        placeholder={placeholder}
        onChange={(event) => onChange(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter") onSubmit?.(value);
        }}
      />
      {value ? (
        <IconButton
          icon="close"
          size={26}
          label="Clear search"
          onClick={() => onChange("")}
        />
      ) : null}
    </label>
  );
}

/** A value the user can drag, for volume and brightness. */
export interface SliderProps {
  value: number;
  min?: number;
  max?: number;
  step?: number;
  label: string;
  icon?: string;
  disabled?: boolean;
  onChange: (value: number) => void;
}

export function Slider({
  value,
  min = 0,
  max = 100,
  step = 1,
  label,
  icon,
  disabled = false,
  onChange,
}: SliderProps) {
  // The browser's own range input, drawn as a Mac's slider: a thin track
  // filled with the accent up to a round white knob. Native, so the arrow
  // keys, Page Up and Home already work.
  const clamped = Math.min(Math.max(value, min), max);
  const fill = max > min ? ((clamped - min) / (max - min)) * 100 : 0;
  return (
    <div className="bw-slider" data-disabled={disabled}>
      {icon ? <Symbol name={icon} size={16} /> : null}
      <input
        type="range"
        className="bw-range"
        value={clamped}
        min={min}
        max={max}
        step={step}
        disabled={disabled}
        aria-label={label}
        style={{ "--fill": `${fill}%` } as CSSProperties}
        onChange={(event) => onChange(Number(event.target.value))}
      />
    </div>
  );
}

/** One row of a list: an icon, a title with optional detail, a trailing slot. */
export interface ListRowProps {
  icon?: string;
  title: string;
  detail?: string;
  trailing?: ReactNode;
  selected?: boolean;
  onClick?: () => void;
}

export function ListRow({
  icon,
  title,
  detail,
  trailing,
  selected,
  onClick,
}: ListRowProps) {
  const interactive = Boolean(onClick);

  return (
    <div
      className="bw-list-row"
      data-selected={selected}
      data-interactive={interactive}
      role={interactive ? "button" : undefined}
      tabIndex={interactive ? 0 : undefined}
      onClick={onClick}
      onKeyDown={
        interactive
          ? (event) => {
              if (event.key === "Enter" || event.key === " ") {
                event.preventDefault();
                onClick?.();
              }
            }
          : undefined
      }
    >
      {icon ? <Symbol name={icon} size={18} /> : null}
      <div className="bw-list-row-text">
        <span className="bw-list-row-title">{title}</span>
        {detail ? <span className="bw-list-row-detail">{detail}</span> : null}
      </div>
      {trailing}
    </div>
  );
}

/** A scrolling region that keeps its scrollbar out of the layout. */
export function ScrollArea({
  children,
  style,
  ...rest
}: HTMLAttributes<HTMLDivElement> & { children: ReactNode }) {
  return (
    <div className="bw-scroll-area" style={style} {...rest}>
      {children}
    </div>
  );
}

/** A tab strip, drawn as a Mac draws one: a segmented control. */
export interface TabsProps {
  tabs: Array<{ id: string; label: string; icon?: string }>;
  active: string;
  onSelect: (id: string) => void;
}

export function Tabs({ tabs, active, onSelect }: TabsProps) {
  return (
    <div className="bw-tabs" role="tablist">
      {tabs.map((tab) => (
        <button
          key={tab.id}
          type="button"
          role="tab"
          className="bw-tab"
          aria-selected={tab.id === active}
          data-active={tab.id === active}
          onClick={() => onSelect(tab.id)}
        >
          {tab.icon ? <Symbol name={tab.icon} size={16} /> : null}
          <span>{tab.label}</span>
        </button>
      ))}
    </div>
  );
}

/** A switch, for a toggle that is a setting rather than an action. */
export interface SwitchProps {
  checked: boolean;
  label: string;
  disabled?: boolean;
  onChange: (checked: boolean) => void;
}

export function Switch({ checked, label, disabled, onChange }: SwitchProps) {
  // React Bits' Squish Switch at a Mac's size and colours: a white knob on a
  // grey track that turns the accent when on. Without the squish — a Mac's
  // knob slides rather than stretches — but it can still be dragged across.
  return (
    <SquishSwitch
      checked={checked}
      onChange={onChange}
      disabled={disabled ?? false}
      ariaLabel={label}
      width={38}
      height={22}
      radius={11}
      stretch={0}
      hoverScale={1}
      trackColor="var(--layer4)"
      trackOnColor="var(--primary)"
      thumbColor="#ffffff"
      thumbOnColor="#ffffff"
    />
  );
}

/** A Mac's check box: a small rounded square, filled with the accent and a
 *  white tick when on. React Bits' Spring Check, with `label` struck through
 *  once it is ticked. */
export interface CheckProps {
  checked: boolean;
  label?: ReactNode;
  /** For a box with no visible label. */
  ariaLabel?: string;
  onChange: (checked: boolean) => void;
}

export function Check({ checked, label, ariaLabel, onChange }: CheckProps) {
  return (
    <SpringCheck
      checked={checked}
      onChange={onChange}
      label={label ?? ""}
      {...(ariaLabel ? { ariaLabel } : {})}
      boxSize={16}
      boxRadius={4}
      bounce={0}
      fontSize={13}
      color="var(--on-surface)"
      fillColor="var(--primary)"
      checkColor="var(--primary-on)"
    />
  );
}

/**
 * A sheet: a dialog drawn inside its own surface rather than in a new
 * window, sliding down from the top of the panel as a Mac's sheet slides
 * from its window's title bar.
 *
 * The original opens these as Quickshell popups over the sidebar. A separate
 * Win32 window per dialog would need its own capability entry, its own
 * layering and its own focus handling for something that only ever appears
 * over one panel — so they are drawn in the panel instead.
 */
export interface DialogProps {
  title: string;
  icon?: string;
  onDismiss: () => void;
  children: ReactNode;
  footer?: ReactNode;
}

export function Dialog({
  title,
  icon,
  onDismiss,
  children,
  footer,
}: DialogProps) {
  // Escape closes, as it does everywhere else in Windows.
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onDismiss();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onDismiss]);

  return (
    <div className="bw-dialog-scrim" onClick={onDismiss}>
      <div
        className="bw-dialog"
        role="dialog"
        aria-modal="true"
        aria-label={title}
        // Clicks inside must not reach the scrim, or interacting with the
        // dialog would close it.
        onClick={(event) => event.stopPropagation()}
      >
        <header className="bw-dialog-header">
          {icon ? <Symbol name={icon} size={22} /> : null}
          <h2>{title}</h2>
          <IconButton
            icon="close"
            size={32}
            label="Close"
            onClick={onDismiss}
          />
        </header>
        <div className="bw-dialog-body">{children}</div>
        {footer ? <footer className="bw-dialog-footer">{footer}</footer> : null}
      </div>
    </div>
  );
}

/** What a list shows when it has nothing in it. */
export function Placeholder({ icon, text }: { icon: string; text: string }) {
  return (
    <div className="bw-placeholder">
      <Symbol name={icon} size={40} />
      <span>{text}</span>
    </div>
  );
}
