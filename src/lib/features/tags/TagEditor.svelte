<script lang="ts">
  import { commands, type EntityRef, type Tag } from "$lib/api/bindings";
  import { call } from "$lib/api/client";
  import { errorMessage } from "$lib/i18n/errors";
  import TagInput from "$lib/ui/TagInput.svelte";
  import { toast } from "$lib/ui/toast.svelte";
  import { nextTagColor } from "./palette";

  // Edits the tags of one entity by name; unknown names become new tags.
  let {
    owner,
    label,
    tagIds = $bindable([]),
  }: { owner: EntityRef; label: string; tagIds?: string[] } = $props();

  let tags = $state<Tag[]>([]);

  let names = $derived(tagIds.flatMap((id) => tags.find((t) => t.id === id)?.name ?? []));

  $effect(() => {
    call(commands.listTags()).then(
      (all) => (tags = all),
      (err) => toast.show({ kind: "error", message: errorMessage(err) }),
    );
  });

  async function setNames(next: string[]) {
    try {
      const ids: string[] = [];
      for (const name of next) {
        let tag = tags.find((t) => t.name.toLocaleLowerCase() === name.toLocaleLowerCase());
        if (!tag) {
          tag = await call(commands.createTag({ name, color: nextTagColor(tags.length) }));
          tags = [...tags, tag];
        }
        ids.push(tag.id);
      }
      await call(commands.setTags(owner, ids));
      tagIds = ids;
    } catch (err) {
      toast.show({ kind: "error", message: errorMessage(err) });
    }
  }
</script>

<TagInput
  {label}
  suggestions={tags.map((t) => t.name)}
  bind:value={() => names, (next) => void setNames(next)}
/>
