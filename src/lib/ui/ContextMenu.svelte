<script lang="ts">
  import type { Snippet } from "svelte";
  import MenuList, { type CloseReason } from "./MenuList.svelte";
  import type { MenuEntry } from "./menu";
  import { clickOutside, floating, portal, type Anchor } from "./overlay";

  let {
    items,
    onSelect,
    children,
  }: { items: MenuEntry[]; onSelect: (id: string) => void; children: Snippet } = $props();

  let anchor = $state<Anchor>();
  let returnFocus: HTMLElement | null = null;

  function openAt(at: Anchor) {
    returnFocus = document.activeElement as HTMLElement | null;
    anchor = at;
  }

  function close(reason: CloseReason) {
    anchor = undefined;
    if (reason !== "outside") returnFocus?.focus();
  }

  function oncontextmenu(event: MouseEvent) {
    event.preventDefault();
    openAt({ x: event.clientX, y: event.clientY });
  }

  function onkeydown(event: KeyboardEvent) {
    if (event.key === "ContextMenu" || (event.key === "F10" && event.shiftKey)) {
      event.preventDefault();
      openAt(event.target as Element);
    }
  }
</script>

<!-- Wrapper only listens for events bubbling from its focusable children. -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="contents" {oncontextmenu} {onkeydown}>
  {@render children()}
</div>

{#if anchor}
  <div
    {@attach portal}
    {@attach floating(() => anchor)}
    {@attach clickOutside(() => close("outside"))}
    class="z-50"
  >
    <MenuList {items} {onSelect} onClose={close} />
  </div>
{/if}
