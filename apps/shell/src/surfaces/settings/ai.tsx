// The AI page's own controls: a key for each service, and a model chosen from
// the ones the service itself lists — so a model released tomorrow can be
// picked without a new version of the shell.
//
// The keys never go into the config. They are sent once to the Windows
// credential manager and not shown again; a row says only whether one is
// there.

import { useEffect, useState } from "react";
import type { AiModel, AiProvider } from "@bw/core";
import { Button } from "../../widgets";
import { tr } from "../../i18n";
import { actions, useShell } from "../../shell/store";
import { describeError } from "../../shell/errors";

const SERVICES: { provider: AiProvider; name: string }[] = [
  { provider: "anthropic", name: "Claude (Anthropic)" },
  { provider: "gemini", name: "Gemini (Google)" },
  { provider: "openai", name: "ChatGPT (OpenAI)" },
];

export function ApiKeys() {
  return (
    <section className="bw-settings-card">
      <h2>{tr("API keys")}</h2>
      <div className="bw-settings-card-rows">
        {SERVICES.map((service) => (
          <KeyRow key={service.provider} {...service} />
        ))}
      </div>
    </section>
  );
}

function KeyRow({ provider, name }: { provider: AiProvider; name: string }) {
  const changes = useShell((state) => state.aiKeyChanges);
  const [saved, setSaved] = useState(false);
  const [draft, setDraft] = useState("");
  const [problem, setProblem] = useState<string | null>(null);

  useEffect(() => {
    void actions.hasAiKeyFor(provider).then(setSaved);
  }, [provider, changes]);

  const save = async (key: string) => {
    try {
      await actions.setAiKey(provider, key);
      setDraft("");
      setProblem(null);
    } catch (error) {
      setProblem(describeError(error));
    }
  };

  return (
    <div className="bw-settings-row">
      <div className="bw-settings-label">
        <span>{name}</span>
        <em>{problem ?? (saved ? tr("Saved") : tr("Not set"))}</em>
      </div>
      <div className="bw-settings-control bw-api-key">
        <input
          type="password"
          value={draft}
          placeholder={saved ? "••••••••" : tr("Paste the key")}
          aria-label={tr("API key for %1").replace("%1", name)}
          autoComplete="off"
          spellCheck={false}
          onChange={(event) => setDraft(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter" && draft.trim()) void save(draft);
          }}
        />
        <Button disabled={!draft.trim()} onClick={() => void save(draft)}>
          {tr("Save")}
        </Button>
        {saved ? (
          <Button variant="text" onClick={() => void save("")}>
            {tr("Remove")}
          </Button>
        ) : null}
      </div>
    </div>
  );
}

export function ModelPicker({
  provider,
  value,
  onChange,
}: {
  provider: AiProvider;
  value: string;
  onChange: (value: string) => void;
}) {
  const changes = useShell((state) => state.aiKeyChanges);
  const [models, setModels] = useState<AiModel[]>([]);
  const [problem, setProblem] = useState<unknown>(null);

  useEffect(() => {
    let live = true;
    setModels([]);
    actions.listAiModels(provider).then(
      (list) => {
        if (!live) return;
        setModels(list);
        setProblem(null);
      },
      (error: unknown) => {
        if (live) setProblem(error);
      },
    );
    return () => {
      live = false;
    };
  }, [provider, changes]);

  // Until there is a list — no key yet, or the service out of reach — the
  // model is typed, as it always was.
  if (models.length === 0) {
    return (
      <div className="bw-model-picker">
        <input
          type="text"
          value={value}
          spellCheck={false}
          onChange={(event) => onChange(event.target.value)}
        />
        {problem ? (
          <small>
            {problem === "noKey"
              ? tr("Save this service's key to choose from its models.")
              : tr("The list of models could not be loaded.")}
          </small>
        ) : null}
      </div>
    );
  }

  // A model the service no longer lists stays selected, said as such, rather
  // than silently becoming the first in the list.
  const listed = models.some((model) => model.id === value);
  return (
    <select value={value} onChange={(event) => onChange(event.target.value)}>
      {listed ? null : (
        <option value={value}>
          {tr("%1 (not offered)").replace("%1", value)}
        </option>
      )}
      {models.map((model) => (
        <option key={model.id} value={model.id}>
          {model.name && model.name !== model.id
            ? `${model.name} (${model.id})`
            : model.id}
        </option>
      ))}
    </select>
  );
}
