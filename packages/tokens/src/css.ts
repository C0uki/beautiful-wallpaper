// Turning derived tokens into CSS custom properties.
//
// Every surface is its own webview, so each one applies the same variables to its
// own `:root`. Binding components to variables rather than to JS values means a
// theme change repaints without React re-rendering the tree.

import type { DerivedTokens, LayerColors } from "./derive";
import { motion, shape } from "./motion";

function layerVariables(
  prefix: string,
  layer: LayerColors,
): Record<string, string> {
  return {
    [`${prefix}`]: layer.base,
    [`${prefix}-hover`]: layer.hover,
    [`${prefix}-active`]: layer.active,
    [`${prefix}-disabled`]: layer.disabled,
    [`${prefix}-border`]: layer.border,
    [`${prefix}-on`]: layer.on,
  };
}

/** The full variable set, as a plain object. */
export function cssVariables(tokens: DerivedTokens): Record<string, string> {
  const variables: Record<string, string> = {};

  for (const [name, value] of Object.entries(tokens.m3)) {
    // `surfaceContainerHigh` → `--m3-surface-container-high`
    const kebab = name.replace(
      /[A-Z0-9]+/g,
      (part) => `-${part.toLowerCase()}`,
    );
    variables[`--m3-${kebab}`] = value;
  }

  tokens.layer.forEach((layer, index) => {
    Object.assign(variables, layerVariables(`--layer${index}`, layer));
  });

  Object.assign(variables, layerVariables("--primary", tokens.primary));
  Object.assign(variables, layerVariables("--secondary", tokens.secondary));
  Object.assign(variables, layerVariables("--tertiary", tokens.tertiary));
  Object.assign(variables, layerVariables("--error", tokens.error));
  Object.assign(variables, layerVariables("--success", tokens.success));

  variables["--on-surface"] = tokens.onSurface;
  variables["--on-surface-variant"] = tokens.onSurfaceVariant;
  variables["--outline"] = tokens.outline;
  variables["--outline-variant"] = tokens.outlineVariant;
  variables["--scrim"] = tokens.scrim;
  variables["--shadow"] = tokens.shadow;
  variables["--background-transparency"] = String(
    tokens.backgroundTransparency,
  );

  tokens.terminal.forEach((color, index) => {
    variables[`--term-${index}`] = color;
  });

  for (const [name, value] of Object.entries(motion.easing)) {
    variables[`--ease-${name}`] = value;
  }
  for (const [name, value] of Object.entries(motion.duration)) {
    variables[`--duration-${name}`] = `${value}ms`;
  }
  for (const [name, value] of Object.entries(shape.rounding)) {
    variables[`--rounding-${name}`] = `${value}px`;
  }

  return variables;
}

/** Applies the variables to an element, usually `document.documentElement`. */
export function applyCssVariables(
  target: HTMLElement,
  tokens: DerivedTokens,
): void {
  const variables = cssVariables(tokens);
  for (const [name, value] of Object.entries(variables)) {
    target.style.setProperty(name, value);
  }
  target.dataset["mode"] = tokens.mode;
  target.style.colorScheme = tokens.mode;
}

/** The same variables as a stylesheet, for server-side or static rendering. */
export function cssVariablesText(
  tokens: DerivedTokens,
  selector = ":root",
): string {
  const body = Object.entries(cssVariables(tokens))
    .map(([name, value]) => `  ${name}: ${value};`)
    .join("\n");
  return `${selector} {\n${body}\n}\n`;
}
