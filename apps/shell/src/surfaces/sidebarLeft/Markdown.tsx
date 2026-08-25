// Rendering a reply.
//
// react-markdown with GFM and highlight.js, which is what the original's
// message blocks amount to. Two things it does that a bare renderer does not:
// every fenced block gets a copy button, and links open in the user's browser
// rather than navigating the surface — a webview that follows a link has
// replaced the shell's UI with a web page and there is no way back.

import { useState } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import rehypeHighlight from "rehype-highlight";
import { Symbol } from "../../widgets";
import { tr } from "../../i18n";
import { backend } from "../../shell/backend";
import "highlight.js/styles/github-dark.css";
import "./markdown.css";

function CodeBlock({ children }: { children: React.ReactNode }) {
  const [copied, setCopied] = useState(false);

  const copy = () => {
    // The rendered text is what the user sees, so it is what gets copied —
    // reaching back into the AST would miss the highlighter's transformations.
    const text = (
      document.activeElement?.closest(".bw-code")?.querySelector("code")
        ?.textContent ?? ""
    ).replace(/\n$/, "");
    void navigator.clipboard?.writeText(text);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1400);
  };

  return (
    <div className="bw-code">
      <button
        type="button"
        className="bw-code-copy"
        aria-label={tr("Copy")}
        onClick={copy}
      >
        <Symbol name={copied ? "check" : "content_copy"} size={14} />
      </button>
      <pre>{children}</pre>
    </div>
  );
}

export function Markdown({ children }: { children: string }) {
  return (
    <div className="bw-markdown">
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        rehypePlugins={[rehypeHighlight]}
        components={{
          pre: ({ children }) => <CodeBlock>{children}</CodeBlock>,
          a: ({ href, children }) => (
            <a
              href={href}
              onClick={(event) => {
                // Never navigate the surface itself.
                event.preventDefault();
                if (href) {
                  void backend().invoke("plugin:opener|open_url", {
                    url: href,
                  });
                }
              }}
            >
              {children}
            </a>
          ),
        }}
      >
        {children}
      </ReactMarkdown>
    </div>
  );
}
