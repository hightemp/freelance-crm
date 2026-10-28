<script lang="ts" module>
  import type { Attachment } from "svelte/attachments";

  export interface MenuTriggerProps {
    "aria-haspopup": "menu";
    "aria-expanded": boolean;
    "aria-controls": string | undefined;
    onclick: () => void;
    onkeydown: (event: KeyboardEvent) => void;
    [key: symbol]: Attachment<HTMLElement>;
  }
</script>

<script lang="ts">
  import type { Snippet } from "svelte";
  import { createAttachmentKey } from "svelte/attachments";
  import type { Placement } from "@floating-ui/dom";
  import MenuList, { type CloseReason } from "./MenuList.svelte";
  import type { MenuEntry } from "./menu";
  import { clickOutside, floating, portal } from "./overlay";

  let {
    items,
    onSelect,
    placement = "bottom-start",
    trigger,
  }: {
    items: MenuEntry[];
    onSelect: (id: string) => void;
    placement?: Placement;
    trigger: Snippet<[MenuTriggerProps]>;
  } = $props();

  const id = $props.id();
  let open = $state(false);
  let anchor = $state<HTMLElement>();

  function close(reason: CloseReason) {
    open = false;
    if (reason !== "outside") anchor?.focus();
  }

  let triggerProps: MenuTriggerProps = $derived({
    "aria-haspopup": "menu",
    "aria-expanded": open,
    "aria-controls": open ? id : undefined,
    onclick: () => (open = !open),
    onkeydown: (event: KeyboardEvent) => {
      if (event.key === "ArrowDown" || event.key === "ArrowUp") {
        event.preventDefault();
        open = true;
      }
    },
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
      () => close("outside"),
      () => [anchor],
    )}
    class="z-50"
  >
    <MenuList {id} {items} {onSelect} onClose={close} />
  </div>
{/if}
