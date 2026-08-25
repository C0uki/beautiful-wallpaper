// The AI chat tab.
//
// The original supports Gemini, OpenAI and Mistral through three strategy
// objects; this talks to one API, so the shape is simpler and the streaming
// is the interesting part. What it keeps from the original: the summarised
// reasoning gets its own collapsible pane rather than being spliced into the
// answer, searches and their sources are shown, and files can be attached.

import { useEffect, useRef, useState } from "react";
import { IconButton, Placeholder, Symbol } from "../../widgets";
import { tr } from "../../i18n";
import { actions, useShell } from "../../shell/store";
import { Markdown } from "./Markdown";
import type { ChatMessage } from "@bw/core";
import "./chat.css";

/** The model's reasoning, folded away by default. */
function Thinking({ text }: { text: string }) {
  const [open, setOpen] = useState(false);

  return (
    <div className="bw-chat-thinking" data-open={open}>
      <button type="button" onClick={() => setOpen((value) => !value)}>
        <Symbol name={open ? "expand_less" : "expand_more"} size={16} />
        <span>{tr("Reasoning")}</span>
      </button>
      {open ? <p>{text}</p> : null}
    </div>
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
        <Thinking text={message.thinking} />
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

export function Chat() {
  const chat = useShell((state) => state.chat);
  const streaming = useShell((state) => state.chatStreaming);
  const hasKey = useShell((state) => state.hasAiKey);

  const [draft, setDraft] = useState("");
  const [attachments, setAttachments] = useState<string[]>([]);
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
        text={tr("Add an Anthropic API key in settings to use the chat.")}
      />
    );
  }

  const send = async () => {
    const text = draft.trim();
    if ((!text && attachments.length === 0) || streaming) return;

    setDraft("");
    const files = attachments;
    setAttachments([]);
    setError(null);
    pinned.current = true;

    try {
      await actions.sendChat(text, files);
    } catch (reason) {
      setError(String(reason));
      // Put the text back so it is not lost to a failed send.
      setDraft(text);
      setAttachments(files);
    }
  };

  const attach = async () => {
    const picked = await actions.pickFiles();
    if (picked.length > 0) setAttachments((current) => [...current, ...picked]);
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
          <span>{tr("No reply came back.")}</span>
          <button type="button" onClick={() => void actions.retryChat()}>
            {tr("Try again")}
          </button>
        </div>
      ) : null}

      {error ? <div className="bw-chat-error">{error}</div> : null}

      {attachments.length > 0 ? (
        <div className="bw-chat-pending">
          {attachments.map((path) => (
            <button
              key={path}
              type="button"
              aria-label={tr("Remove")}
              onClick={() =>
                setAttachments((current) =>
                  current.filter((other) => other !== path),
                )
              }
            >
              <Symbol name="attach_file" size={13} />
              {path.split(/[\\/]/).pop()}
              <Symbol name="close" size={13} />
            </button>
          ))}
        </div>
      ) : null}

      <div className="bw-chat-input">
        <IconButton
          icon="attach_file"
          size={34}
          label={tr("Attach a file")}
          disabled={streaming}
          onClick={() => void attach()}
        />
        <textarea
          value={draft}
          rows={1}
          placeholder={tr("Ask something")}
          aria-label={tr("Message")}
          disabled={streaming}
          onChange={(event) => setDraft(event.target.value)}
          onKeyDown={(event) => {
            // Enter sends; Shift+Enter is a newline, as every chat does.
            if (event.key === "Enter" && !event.shiftKey) {
              event.preventDefault();
              void send();
            }
          }}
        />
        <IconButton
          icon={streaming ? "hourglass_empty" : "send"}
          size={34}
          label={tr("Send")}
          disabled={streaming || (!draft.trim() && attachments.length === 0)}
          onClick={() => void send()}
        />
      </div>
    </div>
  );
}
