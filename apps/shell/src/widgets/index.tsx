// Material 3 primitives shared by every surface.
//
// The set mirrors the widgets end4-pC reaches for most often: a card, the three
// button variants, an icon button with a ripple, a progress bar and ring, a
// segmented control, a search field, a slider, a list row and a scroll area.

import {
  useCallback,
  useRef,
  useState,
  type ButtonHTMLAttributes,
  type CSSProperties,
  type HTMLAttributes,
  type ReactNode,
} from "react";
import { Symbol } from "./Symbol";
import "./widgets.css";

export { Symbol };
export type { SymbolProps } from "./Symbol";

/** Expanding-circle feedback on press, as Material specifies. */
export function useRipple() {
  const [ripples, setRipples] = useState<
    Array<{ id: number; style: CSSProperties }>
  >([]);
  const nextId = useRef(0);

  const spawn = useCallback((event: React.MouseEvent<HTMLElement>) => {
    const target = event.currentTarget;
    const box = target.getBoundingClientRect();
    // Large enough to cover the element from wherever it was clicked.
    const size = Math.hypot(box.width, box.height) * 2;
    const id = nextId.current++;

    setRipples((current) => [
      ...current,
      {
        id,
        style: {
          left: event.clientX - box.left - size / 2,
          top: event.clientY - box.top - size / 2,
          width: size,
          height: size,
        },
      },
    ]);

    window.setTimeout(() => {
      setRipples((current) => current.filter((ripple) => ripple.id !== id));
    }, 450);
  }, []);

  const layer = ripples.map((ripple) => (
    <span key={ripple.id} className="bw-ripple" style={ripple.style} />
  ));

  return { spawn, layer };
}

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
  onClick,
  ...rest
}: ButtonProps) {
  const { spawn, layer } = useRipple();
  return (
    <button
      {...rest}
      type="button"
      className="bw-button"
      data-variant={variant}
      data-active={active}
      onClick={(event) => {
        spawn(event);
        onClick?.(event);
      }}
    >
      {icon ? <Symbol name={icon} size={18} filled={active} /> : null}
      {children}
      {layer}
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
  onClick,
  ...rest
}: IconButtonProps) {
  const { spawn, layer } = useRipple();
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
      onClick={(event) => {
        spawn(event);
        onClick?.(event);
      }}
    >
      <Symbol name={icon} size={Math.round(size * 0.55)} filled={active} />
      {layer}
    </button>
  );
}

export function ProgressBar({ value }: { value: number }) {
  const clamped = Math.min(100, Math.max(0, value));
  return (
    <div
      className="bw-progress"
      role="progressbar"
      aria-valuenow={Math.round(clamped)}
      aria-valuemin={0}
      aria-valuemax={100}
    >
      <i style={{ width: `${clamped}%` }} />
    </div>
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
  return (
    <div className="bw-segmented" role="tablist">
      {options.map((option) => (
        <Button
          key={option.value}
          role="tab"
          aria-selected={option.value === value}
          variant={option.value === value ? "filled" : "text"}
          {...(option.icon ? { icon: option.icon } : {})}
          onClick={() => onChange(option.value)}
        >
          {option.label}
        </Button>
      ))}
    </div>
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
  const clamped = Math.min(Math.max(value, min), max);
  const fraction = max === min ? 0 : (clamped - min) / (max - min);

  return (
    <div className="bw-slider" data-disabled={disabled}>
      {icon ? <Symbol name={icon} size={18} /> : null}
      <div className="bw-slider-track">
        <div
          className="bw-slider-fill"
          style={{ width: `${fraction * 100}%` }}
        />
        {/* The native input carries the interaction and the accessibility;
            the painted track above is decoration sitting under it. */}
        <input
          type="range"
          min={min}
          max={max}
          step={step}
          value={clamped}
          disabled={disabled}
          aria-label={label}
          onChange={(event) => onChange(Number(event.target.value))}
        />
      </div>
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
  const ripple = useRipple();
  const interactive = Boolean(onClick);

  return (
    <div
      className="bw-list-row"
      data-selected={selected}
      data-interactive={interactive}
      role={interactive ? "button" : undefined}
      tabIndex={interactive ? 0 : undefined}
      onPointerDown={interactive ? ripple.spawn : undefined}
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
      {icon ? <Symbol name={icon} size={20} /> : null}
      <div className="bw-list-row-text">
        <span className="bw-list-row-title">{title}</span>
        {detail ? <span className="bw-list-row-detail">{detail}</span> : null}
      </div>
      {trailing}
      {interactive ? ripple.layer : null}
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
