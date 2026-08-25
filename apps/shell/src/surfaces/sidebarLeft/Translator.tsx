// The translator tab.
//
// The original shells out to `trans` (translate-shell), a Bash script wrapping
// Google Translate. Neither Bash nor that script exists on Windows, so this
// goes through the Anthropic API instead — which also means the key and the
// client are already in place for the chat tab that comes later.

import { useEffect, useRef, useState } from "react";
import { IconButton, Placeholder, Symbol } from "../../widgets";
import { tr } from "../../i18n";
import { actions, useShell } from "../../shell/store";
import type { AiError } from "@bw/core";

/** The languages the picker offers. `auto` is only valid as a source. */
const LANGUAGES: Array<{ code: string; label: string }> = [
  { code: "auto", label: "Detect" },
  { code: "en", label: "English" },
  { code: "ja", label: "日本語" },
  { code: "zh", label: "中文" },
  { code: "ko", label: "한국어" },
  { code: "es", label: "Español" },
  { code: "fr", label: "Français" },
  { code: "de", label: "Deutsch" },
  { code: "pt", label: "Português" },
  { code: "ru", label: "Русский" },
];

/** What to tell the user about a failure. */
export function errorMessage(error: AiError): string {
  switch (error) {
    case "noKey":
      return tr("Add an Anthropic API key in settings to use the translator.");
    case "badKey":
      return tr("That API key was rejected. Check it in settings.");
    case "rateLimited":
      return tr("Rate limited. Try again in a moment.");
    case "refused":
      return tr("The model would not translate that.");
    default:
      return tr("Could not reach the API.");
  }
}

export function Translator() {
  const config = useShell((state) => state.config.sidebar.left.translator);
  const hasKey = useShell((state) => state.hasAiKey);

  const [source, setSource] = useState("");
  const [result, setResult] = useState("");
  const [error, setError] = useState<AiError | null>(null);
  const [busy, setBusy] = useState(false);
  const [from, setFrom] = useState(config.from);
  const [to, setTo] = useState(config.to);

  // Every request costs money, so the text is sent once the user stops typing
  // rather than on each keystroke.
  const pending = useRef<number | null>(null);
  useEffect(() => {
    if (pending.current !== null) window.clearTimeout(pending.current);
    if (!source.trim() || !hasKey) {
      setResult("");
      return;
    }

    pending.current = window.setTimeout(() => {
      setBusy(true);
      void actions
        .translate(source, from, to)
        .then((outcome) => {
          setResult(outcome.text);
          setError(outcome.error);
        })
        .finally(() => setBusy(false));
    }, config.delay);

    return () => {
      if (pending.current !== null) window.clearTimeout(pending.current);
    };
  }, [source, from, to, config.delay, hasKey]);

  if (!hasKey) {
    return (
      <Placeholder
        icon="key"
        text={tr("Add an Anthropic API key in settings to use the translator.")}
      />
    );
  }

  const swap = () => {
    // Detect has no meaning as a target, so swapping out of it keeps the
    // target and just stops detecting.
    const nextFrom = to;
    const nextTo = from === "auto" ? config.to : from;
    setFrom(nextFrom);
    setTo(nextTo);
    setSource(result);
    setResult(source);
  };

  return (
    <div className="bw-translator">
      <div className="bw-translator-languages">
        <select
          value={from}
          aria-label={tr("Translate from")}
          onChange={(event) => setFrom(event.target.value)}
        >
          {LANGUAGES.map((language) => (
            <option key={language.code} value={language.code}>
              {language.label}
            </option>
          ))}
        </select>

        <IconButton
          icon="swap_horiz"
          size={32}
          label={tr("Swap languages")}
          onClick={swap}
        />

        <select
          value={to}
          aria-label={tr("Translate to")}
          onChange={(event) => setTo(event.target.value)}
        >
          {LANGUAGES.filter((language) => language.code !== "auto").map(
            (language) => (
              <option key={language.code} value={language.code}>
                {language.label}
              </option>
            ),
          )}
        </select>
      </div>

      <textarea
        className="bw-translator-input"
        value={source}
        placeholder={tr("Type something to translate")}
        aria-label={tr("Text to translate")}
        onChange={(event) => setSource(event.target.value)}
      />

      <div className="bw-translator-output" data-busy={busy}>
        {error ? (
          <span className="bw-translator-error">{errorMessage(error)}</span>
        ) : (
          <span>{result}</span>
        )}
        {busy ? (
          <Symbol
            name="autorenew"
            size={16}
            className="bw-translator-spinner"
          />
        ) : null}
      </div>

      <div className="bw-translator-actions">
        <button
          type="button"
          disabled={!result}
          onClick={() => void navigator.clipboard?.writeText(result)}
        >
          <Symbol name="content_copy" size={16} />
          {tr("Copy")}
        </button>
      </div>
    </div>
  );
}
