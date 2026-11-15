import { openUrl } from "@tauri-apps/plugin-opener";
import Markdown from "react-markdown";

/**
 * Renders trade notes written in Markdown.
 * - react-markdown never renders raw HTML from the text, so a pasted
 *   <script> or <img onerror> can't run inside the app.
 * - Links open in the system browser; following them inside the app window
 *   would replace TradeLedger itself.
 */
export default function Notes({ text }: { text: string }) {
  if (text.trim() === "") {
    return <p className="hint">No notes yet.</p>;
  }
  return (
    <div className="notes-view">
      <Markdown
        components={{
          a: ({ href, children }) => (
            <a
              href={href}
              onClick={(e) => {
                e.preventDefault();
                if (href && /^https?:\/\//i.test(href)) void openUrl(href);
              }}
            >
              {children}
            </a>
          ),
        }}
      >
        {text}
      </Markdown>
    </div>
  );
}
