<script lang="ts" module>
  import type { Attachment } from "svelte/attachments";

  /** Props the trigger element must spread: `<Button {...trigger}>`. */
  export interface TooltipTriggerProps {
    "aria-describedby"?: string;
    onpointerenter: () => void;
    onpointerleave: () => void;
    onfocus: () => void;
    onblur: () => void;
    onkeydown: (event: KeyboardEvent) => void;
    [key: symbol]: Attachment<HTMLElement>;
  }
</script>

<script lang="ts">
  import type { Snippet } from "svelte";
  import { createAttachmentKey } from "svelte/attachments";
  import type { Placement } from "@floating-ui/dom";
  import { floating, portal } from "./overlay";

  let {
    text,
    delay = 500,
    placement = "top",
    children,
  }: {
    text: string;
    delay?: number;
    placement?: Placement;
    children: Snippet<[TooltipTriggerProps]>;
  } = $props();

  const id = $props.id();
  let anchor = $state<HTMLElement>();
  let visible = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;

  function show() {
    clearTimeout(timer);
    timer = setTimeout(() => (visible = true), delay);
  }

  function hide() {
    clearTimeout(timer);
    visible = false;
  }

  $effect(() => () => clearTimeout(timer));

  let triggerProps: TooltipTriggerProps = $derived({
    "aria-describedby": visible ? id : undefined,
    onpointerenter: show,
    onpointerleave: hide,
    onfocus: show,
    onblur: hide,
    onkeydown: (event: KeyboardEvent) => {
      if (event.key === "Escape") hide();
    },
    [createAttachmentKey()]: (node: HTMLElement) => {
      anchor = node;
    },
  });
</script>

{@render children(triggerProps)}

{#if visible}
  <div
    {@attach portal}
    {@attach floating(() => anchor, placement)}
    {id}
    role="tooltip"
    class="pointer-events-none z-50 max-w-xs rounded-[var(--radius)] bg-fg px-2 py-1 text-xs text-bg"
  >
    {text}
  </div>
{/if}
