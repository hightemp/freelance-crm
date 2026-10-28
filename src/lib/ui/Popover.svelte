<script lang="ts" module>
  import type { Attachment } from "svelte/attachments";

  export interface PopoverTriggerProps {
    "aria-haspopup": "dialog";
    "aria-expanded": boolean;
    "aria-controls": string | undefined;
    onclick: () => void;
    [key: symbol]: Attachment<HTMLElement>;
  }
</script>

<script lang="ts">
  import type { Snippet } from "svelte";
  import { createAttachmentKey } from "svelte/attachments";
  import type { Placement } from "@floating-ui/dom";
  import { clickOutside, floating, portal } from "./overlay";

  let {
    label,
    open = $bindable(false),
    placement = "bottom-start",
    trigger,
    children,
  }: {
    /** Accessible name of the popover panel. */
    label: string;
    open?: boolean;
    placement?: Placement;
    trigger: Snippet<[PopoverTriggerProps]>;
    children: Snippet;
  } = $props();

  const id = $props.id();
  let anchor = $state<HTMLElement>();

  function close(returnFocus: boolean) {
    open = false;
    if (returnFocus) anchor?.focus();
  }

  let triggerProps: PopoverTriggerProps = $derived({
    "aria-haspopup": "dialog",
    "aria-expanded": open,
    "aria-controls": open ? id : undefined,
    onclick: () => (open = !open),
    [createAttachmentKey()]: (node: HTMLElement) => {
      anchor = node;
    },
  });
</script>

{@render trigger(triggerProps)}

{#if open}
  <div
    {@attach portal}
    {@attach floating(() => anchor, placement)}
    {@attach clickOutside(
      () => close(false),
      () => [anchor],
    )}
    {id}
    role="dialog"
    aria-label={label}
    tabindex="-1"
    onkeydown={(e) => {
      if (e.key === "Escape") {
        e.stopPropagation();
        close(true);
      }
    }}
    class="z-40 rounded-[var(--radius)] border border-border bg-bg p-3 shadow-lg"
  >
    {@render children()}
  </div>
{/if}
