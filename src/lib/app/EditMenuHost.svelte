<script lang="ts">
  import { m } from "$lib/paraglide/messages";
  import MenuList from "$lib/ui/MenuList.svelte";
  import type { MenuEntry } from "$lib/ui/menu";
  import { clickOutside, floating, portal } from "$lib/ui/overlay";
  import { closeEditMenu, currentEditMenu } from "./edit-menu.svelte";

  let request = $derived(currentEditMenu());

  function isReadOnly(el: HTMLElement) {
    if (el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement) {
      return el.readOnly || el.disabled;
    }
    return el.isContentEditable === false;
  }

  let items: MenuEntry[] = $derived.by(() => {
    const readOnly = request ? isReadOnly(request.target) : true;
    return [
      { id: "cut", label: m.ui_cut(), disabled: readOnly },
      { id: "copy", label: m.ui_copy() },
      { id: "paste", label: m.ui_paste(), disabled: readOnly },
      { separator: true },
      { id: "selectAll", label: m.ui_select_all() },
    ];
  });

  async function run(command: string) {
    const target = request?.target;
    if (!target) return;
    target.focus();
    if (command === "selectAll" && "select" in target) {
      (target as HTMLInputElement).select();
    } else if (command === "paste") {
      const text = await navigator.clipboard?.readText?.().catch(() => null);
      if (text != null) document.execCommand("insertText", false, text);
    } else {
      document.execCommand(command);
    }
  }
</script>

{#if request}
  {@const at = request}
  <div
    {@attach portal}
    {@attach floating(() => ({ x: at.x, y: at.y }))}
    {@attach clickOutside(closeEditMenu)}
    class="z-50"
  >
    <MenuList
      {items}
      onSelect={(id) => void run(id)}
      onClose={(reason) => {
        const target = at.target; // `at` turns null once the menu closes
        closeEditMenu();
        if (reason !== "outside") target.focus();
      }}
    />
  </div>
{/if}
