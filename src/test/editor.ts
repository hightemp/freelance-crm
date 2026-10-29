// CodeMirror cannot be typed into with user-event under jsdom (no layout),
// so tests edit through the editor API, which runs the same update path.
import { EditorView } from "@codemirror/view";

function viewOf(textbox: HTMLElement): EditorView {
  const dom = textbox.closest<HTMLElement>(".cm-editor");
  const view = dom && EditorView.findFromDOM(dom);
  if (!view) throw new Error("not a CodeMirror editor");
  return view;
}

/** Appends text at the end of the document, like typing it. */
export function typeInEditor(textbox: HTMLElement, text: string): void {
  const view = viewOf(textbox);
  view.dispatch({ changes: { from: view.state.doc.length, insert: text } });
}

export function replaceInEditor(textbox: HTMLElement, text: string): void {
  const view = viewOf(textbox);
  view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: text } });
}

export function editorText(textbox: HTMLElement): string {
  return viewOf(textbox).state.doc.toString();
}
