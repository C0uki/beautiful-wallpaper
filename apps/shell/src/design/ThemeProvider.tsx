// Applies the derived tokens as CSS custom properties.
//
// Components bind to variables rather than to React state, so a wallpaper change
// repaints the whole shell without re-rendering the tree.

import {
  applyCssVariables,
  deriveTokens,
  type DerivedTokens,
} from "@bw/tokens";
import {
  Fragment,
  createContext,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { setLocale } from "../i18n";
import { connect, useShell } from "../shell/store";
import "./global.css";

const TokensContext = createContext<DerivedTokens | null>(null);

/** The derived tokens, for the rare case JS needs a colour value. */
export function useTokens(): DerivedTokens {
  const tokens = useContext(TokensContext);
  if (!tokens) throw new Error("useTokens must be used inside <ThemeProvider>");
  return tokens;
}

export function ThemeProvider({ children }: { children: ReactNode }) {
  const theme = useShell((state) => state.theme);
  const appearance = useShell((state) => state.config.appearance);
  const requestedLocale = useShell((state) => state.config.language.ui);
  const [locale, setActiveLocale] = useState(() => setLocale("auto"));

  useEffect(() => {
    void connect();
  }, []);

  // `tr()` reads a module-level dictionary, so swapping it does not by itself
  // invalidate anything React has rendered. Keying the subtree on the resolved
  // locale is what makes a language change take effect — at the cost of
  // remounting the surface, which is fine for something users change once.
  useEffect(() => {
    setActiveLocale(setLocale(requestedLocale));
  }, [requestedLocale]);

  const tokens = useMemo(
    () =>
      theme
        ? deriveTokens(theme, {
            transparencyEnabled: appearance.transparency.enable,
            extraTransparency: appearance.transparency.extra,
          })
        : null,
    [theme, appearance.transparency.enable, appearance.transparency.extra],
  );

  useEffect(() => {
    if (!tokens) return;
    applyCssVariables(document.documentElement, tokens);
  }, [tokens]);

  useEffect(() => {
    const root = document.documentElement;
    root.style.setProperty("--font-main", appearance.fonts.main);
    root.style.setProperty("--font-title", appearance.fonts.title);
    root.style.setProperty("--font-mono", appearance.fonts.monospace);
    root.style.setProperty("--font-size", `${appearance.fonts.pixelSize}px`);
    root.style.setProperty(
      "--rounding-scale",
      String(appearance.roundingScale),
    );
  }, [appearance.fonts, appearance.roundingScale]);

  // Until the first theme arrives there is nothing sensible to paint; rendering
  // children against unset variables would flash the wrong colours.
  if (!tokens) return null;

  return (
    <TokensContext.Provider value={tokens}>
      <Fragment key={locale}>{children}</Fragment>
    </TokensContext.Provider>
  );
}
