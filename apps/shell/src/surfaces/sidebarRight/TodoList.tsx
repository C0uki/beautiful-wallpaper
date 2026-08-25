// The To Do tab.
//
// Unfinished first, then finished, each group in the order the user put them
// in. Reordering across that boundary would be confusing, so the list is only
// sorted for display and the store keeps the real order.

import { useState } from "react";
import { IconButton, Placeholder, ScrollArea, Symbol } from "../../widgets";
import { tr } from "../../i18n";
import { actions, useShell } from "../../shell/store";

export function TodoList() {
  const todos = useShell((state) => state.todos);
  const [draft, setDraft] = useState("");

  const unfinished = todos.filter((todo) => !todo.done);
  const finished = todos.filter((todo) => todo.done);

  const submit = () => {
    const content = draft.trim();
    if (!content) return;
    setDraft("");
    void actions.addTodo(content);
  };

  return (
    <div className="bw-todo">
      <div className="bw-todo-input">
        <input
          value={draft}
          placeholder={tr("Add a task")}
          aria-label={tr("Add a task")}
          onChange={(event) => setDraft(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") submit();
          }}
        />
        <IconButton
          icon="add"
          size={32}
          label={tr("Add")}
          disabled={draft.trim().length === 0}
          onClick={submit}
        />
      </div>

      {todos.length === 0 ? (
        <Placeholder icon="done_outline" text={tr("Nothing to do")} />
      ) : (
        <ScrollArea className="bw-todo-list">
          {[...unfinished, ...finished].map((todo) => (
            <div key={todo.id} className="bw-todo-item" data-done={todo.done}>
              <button
                type="button"
                className="bw-todo-check"
                role="checkbox"
                aria-checked={todo.done}
                aria-label={todo.content}
                onClick={() => void actions.setTodoDone(todo.id, !todo.done)}
              >
                <Symbol
                  name={todo.done ? "check_box" : "check_box_outline_blank"}
                  size={20}
                  filled={todo.done}
                />
              </button>
              <span className="bw-todo-content">{todo.content}</span>
              <IconButton
                icon="close"
                size={28}
                label={tr("Delete")}
                onClick={() => void actions.removeTodo(todo.id)}
              />
            </div>
          ))}
        </ScrollArea>
      )}

      {finished.length > 0 ? (
        <footer className="bw-todo-footer">
          <button type="button" onClick={() => void actions.clearDoneTodos()}>
            {tr("Clear finished")}
          </button>
        </footer>
      ) : null}
    </div>
  );
}
