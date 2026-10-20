// The AI chat tab.
//
// The original supports Gemini, OpenAI and Mistral through three strategy
// objects; this talks to one API, so the shape is simpler and the streaming
// is the interesting part. What it keeps from the original: the summarised
// reasoning gets its own collapsible pane rather than being spliced into the
// answer, searches and their sources are shown, and files can be attached.
// The reasoning is React Bits' Thought Line and the input its Prompt Bar.

import { useEffect, useRef, useState } from "react";
import { IconButton, Placeholder, Symbol } from "../../widgets";
import PromptBar from "../../widgets/reactbits/PromptBar";
import ThoughtLine from "../../widgets/reactbits/ThoughtLine";
import { tr } from "../../i18n";
import { actions, useShell } from "../../shell/store";
import { Markdown } from "./Markdown";
import type { AiError, ChatMessage } from "@bw/core";
import "./chat.css";

/** The model's reasoning: breathing while it thinks, folded away once the
 *  answer begins, one step to a paragraph. */
function Thinking({ text, working }: { text: string; working: boolean }) {
  return (
    <ThoughtLine
      className="bw-chat-thinking"
      label={tr("Thinking…")}
      doneLabel={tr("Reasoning")}
      steps={text
        .split(/\n\s*\n/)
        .map((step) => step.trim())
        .filter(Boolean)}
      working={working}
      fontSize={13}
      color="var(--on-surface-variant)"
      glyphColor="var(--primary)"
    />
  );
}

function Turn({
  message,
  streaming,
}: {
  message: ChatMessage;
  streaming: boolean;
}) {
  const isUser = message.role === "user";

  return (
    <article className="bw-chat-turn" data-role={message.role}>
      {message.attachments.length > 0 ? (
        <div className="bw-chat-attachments">
          {message.attachments.map((name) => (
            <span key={name}>
              <Symbol name="attach_file" size={14} />
              {name}
            </span>
          ))}
        </div>
      ) : null}

      {!isUser && message.thinking ? (
        // Thinking until the answer starts to arrive.
        <Thinking
          text={message.thinking}
          working={streaming && !message.content}
        />
      ) : null}

      {message.searches.map((query) => (
        <div key={query} className="bw-chat-search">
          <Symbol name="travel_explore" size={14} />
          <span>{query}</span>
        </div>
      ))}

      <div className="bw-chat-body">
        {isUser ? (
          // A user's own text is shown verbatim: rendering it as Markdown
          // would mangle anything they pasted, code included.
          <p className="bw-chat-plain">{message.content}</p>
        ) : (
          <Markdown>{message.content}</Markdown>
        )}
        {streaming && !isUser ? <span className="bw-chat-cursor" /> : null}
      </div>

      {message.sources.length > 0 ? (
        <div className="bw-chat-sources">
          {message.sources.map((source) => (
            <a
              key={source.url}
              href={source.url}
              title={source.url}
              onClick={(event) => {
                event.preventDefault();
                void actions.openUrl(source.url);
              }}
            >
              <Symbol name="link" size={12} />
              {source.title}
            </a>
          ))}
        </div>
      ) : null}

      {message.answeredBy ? (
        <span className="bw-chat-fallback">
          {tr("Answered by %1").replace("%1", message.answeredBy)}
        </span>
      ) : null}
    </article>
  );
}

/** Why no reply came, said so the user knows whether waiting, the key or
 *  the settings is the answer. */
function failureMessage(error: AiError | null): string {
  switch (error) {
    case "noKey":
      return tr("No API key is saved for this service.");
    case "badKey":
      return tr("That API key was rejected. Check it in settings.");
    case "rateLimited":
      return tr(
        "The service is busy, or this key's limit has been reached. Try again later.",
      );
    case "refused":
      return tr("The model would not answer that.");
    case "unavailable":
      return tr("Could not reach the API.");
    default:
      // A reply that ended with nothing in it, or one from before a restart.
      return tr("No reply came back.");
  }
}

export function Chat() {
  const chat = useShell((state) => state.chat);
  const streaming = useShell((state) => state.chatStreaming);
  const hasKey = useShell((state) => state.hasAiKey);
  const failure = useShell((state) => state.chatError);

  // What a send that failed gave back, for the bar to start again with; the
  // count makes it a new bar each time.
  const [restored, setRestored] = useState({
    text: "",
    files: [] as string[],
    count: 0,
  });
  const [error, setError] = useState<string | null>(null);
  const bottom = useRef<HTMLDivElement>(null);
  const pinned = useRef(true);

  // Follow the reply as it streams, but only while the user is already at the
  // bottom — yanking the view back while they are reading further up is worse
  // than not following at all.
  useEffect(() => {
    if (pinned.current) bottom.current?.scrollIntoView({ block: "end" });
  }, [chat, streaming]);

  if (!hasKey) {
    return (
      <Placeholder
        icon="key"
        text={tr(
          "Add an API key under AI and translation in settings to use the chat.",
        )}
      />
    );
  }

  const send = async (text: string, files: string[]) => {
    if ((!text && files.length === 0) || streaming) return;
    setError(null);
    pinned.current = true;

    try {
      await actions.sendChat(text, files);
    } catch (reason) {
      setError(String(reason));
      // Put the text back so it is not lost to a failed send.
      setRestored((last) => ({ text, files, count: last.count + 1 }));
    }
  };

  const last = chat.at(-1);
  const failed =
    last?.role === "assistant" && !streaming && !last.content.trim();

  return (
    <div className="bw-chat">
      <header className="bw-chat-head">
        <span>{tr("Intelligence")}</span>
        <IconButton
          icon="delete_sweep"
          size={30}
          label={tr("Clear the conversation")}
          disabled={chat.length === 0 || streaming}
          onClick={() => void actions.clearChat()}
        />
      </header>

      <div
        className="bw-chat-log"
        onScroll={(event) => {
          const box = event.currentTarget;
          pinned.current =
            box.scrollHeight - box.scrollTop - box.clientHeight < 40;
        }}
      >
        {chat.length === 0 ? (
          <Placeholder icon="neurology" text={tr("Ask something")} />
        ) : (
          chat.map((message, index) => (
            <Turn
              key={message.id}
              message={message}
              streaming={streaming && index === chat.length - 1}
            />
          ))
        )}
        <div ref={bottom} />
      </div>

      {failed ? (
        <div className="bw-chat-failed">
          <span>{failureMessage(failure)}</span>
          <button type="button" onClick={() => void actions.retryChat()}>
            {tr("Try again")}
          </button>
        </div>
      ) : null}

      {error ? <div className="bw-chat-error">{error}</div> : null}

      <PromptBar
        key={restored.count}
        className="bw-chat-prompt"
        defaultValue={restored.text}
        defaultAttachments={restored.files}
        placeholder={tr("Ask something")}
        // Files are the only thing the plus offers: no models, no effort,
        // no slash commands and no dictation here.
        sources={[
          {
            key: "files",
            name: tr("Attach a file"),
            icon: <Symbol name="attach_file" size={15} />,
            attach: true,
          },
        ]}
        commands={[]}
        models={[]}
        efforts={[]}
        busy={streaming}
        // As wide as the tab: the bar is the smaller of this and 100%.
        width={10000}
        background="var(--layer2)"
        color="var(--on-surface)"
        menuBackground="var(--layer3)"
        sparkColor="var(--primary)"
        onAttach={() => actions.pickFiles()}
        onSend={(text, detail) => void send(text, detail.attachments)}
      />
    </div>
  );
}
