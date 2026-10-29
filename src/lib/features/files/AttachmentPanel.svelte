<script lang="ts">
  import { untrack } from "svelte";
  import Download from "@lucide/svelte/icons/download";
  import ExternalLink from "@lucide/svelte/icons/external-link";
  import FileIcon from "@lucide/svelte/icons/file";
  import FolderOpen from "@lucide/svelte/icons/folder-open";
  import LayoutGrid from "@lucide/svelte/icons/layout-grid";
  import List from "@lucide/svelte/icons/list";
  import Lock from "@lucide/svelte/icons/lock";
  import Trash2 from "@lucide/svelte/icons/trash-2";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import { commands, type Attachment, type EntityRef } from "$lib/api/bindings";
  import { call } from "$lib/api/client";
  import { errorMessage } from "$lib/i18n/errors";
  import { formatDate, formatFileSize } from "$lib/i18n/format";
  import { i18n } from "$lib/i18n/locale.svelte";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import Checkbox from "$lib/ui/Checkbox.svelte";
  import { confirm } from "$lib/ui/dialogs.svelte";
  import Spinner from "$lib/ui/Spinner.svelte";
  import { toast } from "$lib/ui/toast.svelte";
  import { refreshVault, vault, watchVault } from "../vault/vault.svelte";
  import Lightbox from "./Lightbox.svelte";
  import { fileStamp, previewUrl } from "./preview";

  let { owner }: { owner: EntityRef } = $props();

  let files = $state<Attachment[] | null>(null);
  let view = $state<"list" | "gallery">("list");
  let encrypt = $state(false);
  let busy = $state(false);
  let dragging = $state(false);
  /** The image open in the lightbox. */
  let viewing = $state<string | null>(null);

  // Secrets' files are always encrypted by the backend.
  let isSecret = $derived(owner.type === "secret");
  let unlocked = $derived(vault.status === "unlocked");
  let images = $derived((files ?? []).filter((f) => f.hasThumbnail));
  /** Images the lightbox may show: encrypted ones only while unlocked. */
  let viewable = $derived(images.filter((f) => !f.encrypted || unlocked));
  const showError = (err: unknown) => {
    toast.show({ kind: "error", message: errorMessage(err) });
  };

  async function reload() {
    files = await call(commands.listAttachments(owner));
  }

  $effect(() => {
    void [owner.type, owner.id];
    untrack(() => reload().catch(showError));
  });

  $effect(() => {
    refreshVault().catch(showError);
    return watchVault();
  });

  // An encrypted image must not stay on screen once the vault locks.
  $effect(() => {
    if (viewing !== null && !viewable.some((f) => f.id === viewing)) viewing = null;
  });

  async function attachAll(attach: (encrypt: boolean) => Promise<unknown>[]) {
    busy = true;
    try {
      const results = await Promise.allSettled(attach(encrypt && unlocked));
      for (const r of results) if (r.status === "rejected") showError(r.reason);
      await reload();
    } catch (err) {
      showError(err);
    } finally {
      busy = false;
    }
  }

  const attachPaths = (paths: string[]) =>
    attachAll((enc) => paths.map((path) => call(commands.attachFile(owner, path, enc))));

  async function pick() {
    const picked = await open({ multiple: true, directory: false }).catch(() => null);
    if (picked?.length) await attachPaths(picked);
  }

  // Tauri delivers dropped files with their paths as window events; the
  // panel shown on screen takes them.
  $effect(() => {
    let stop: (() => void) | undefined;
    let destroyed = false;
    getCurrentWebview()
      .onDragDropEvent(({ payload }) => {
        if (payload.type === "enter" || payload.type === "over") dragging = true;
        else if (payload.type === "leave") dragging = false;
        else {
          dragging = false;
          void attachPaths(payload.paths);
        }
      })
      .then((unlisten) => (destroyed ? unlisten() : (stop = unlisten)))
      .catch(() => {});
    return () => {
      destroyed = true;
      stop?.();
    };
  });

  function onpaste(event: ClipboardEvent) {
    const data = event.clipboardData;
    if (![...(data?.files ?? [])].some((f) => f.type.startsWith("image/"))) return;
    event.preventDefault();
    // The backend reads the image from the clipboard: no bytes over IPC.
    const fileName = `${m.files_pasted_name({ date: fileStamp(new Date()) })}.png`;
    void attachAll((enc) => [call(commands.attachClipboardImage(owner, fileName, enc))]);
  }

  const act = (action: Promise<unknown>) => action.catch(showError);

  async function saveAs(file: Attachment) {
    const path = await save({ defaultPath: file.fileName }).catch(() => null);
    if (path) await act(call(commands.saveAttachmentAs(file.id, path)));
  }

  async function remove(file: Attachment) {
    const ok = await confirm({
      title: m.files_delete_title(),
      message: m.files_delete_message({ name: file.fileName }),
      confirmLabel: m.ui_delete(),
      danger: true,
    });
    if (!ok) return;
    await act(call(commands.deleteAttachment(file.id)));
    await reload().catch(showError);
  }

  const hidden = (file: Attachment) => file.encrypted && !unlocked;
</script>

<svelte:window {onpaste} />

<div class="flex flex-col gap-4">
  <div
    data-testid="drop-zone"
    data-dragging={dragging}
    class="flex flex-wrap items-center gap-4 rounded-[var(--radius)] border border-dashed border-border
      p-4 text-sm text-muted data-[dragging=true]:border-accent data-[dragging=true]:bg-surface"
  >
    <Button onclick={pick} loading={busy}>{m.files_add()}</Button>
    <span>{m.ui_drop_files()}</span>
    <span class="ml-auto flex flex-col gap-1">
      {#if isSecret}
        <span class="flex items-center gap-1">
          <Lock class="size-4" aria-hidden="true" />
          {m.files_secret_encrypted()}
        </span>
      {:else}
        <Checkbox label={m.files_encrypt()} bind:checked={encrypt} disabled={!unlocked} />
        {#if !unlocked}
          <span class="text-xs">{m.files_encrypt_locked()}</span>
        {/if}
      {/if}
    </span>
  </div>

  <div role="group" aria-label={m.files_view()} class="flex gap-1">
    <Button
      variant={view === "list" ? "secondary" : "ghost"}
      size="sm"
      aria-pressed={view === "list"}
      onclick={() => (view = "list")}
    >
      <List class="size-4" aria-hidden="true" />
      {m.files_view_list()}
    </Button>
    <Button
      variant={view === "gallery" ? "secondary" : "ghost"}
      size="sm"
      aria-pressed={view === "gallery"}
      onclick={() => (view = "gallery")}
    >
      <LayoutGrid class="size-4" aria-hidden="true" />
      {m.files_view_gallery()}
    </Button>
  </div>

  {#if files === null}
    <div class="flex justify-center p-4"><Spinner /></div>
  {:else if view === "list"}
    {#if files.length === 0}
      <p class="text-sm text-muted">{m.files_empty()}</p>
    {:else}
      <ul
        aria-label={m.files_list()}
        class="flex flex-col divide-y divide-border rounded-[var(--radius)] border border-border"
      >
        {#each files as file (file.id)}
          <li class="flex items-center gap-3 px-3 py-2">
            {#if file.encrypted}
              <Lock class="size-4 shrink-0 text-muted" aria-label={m.files_encrypted()} />
            {:else}
              <FileIcon class="size-4 shrink-0 text-muted" aria-hidden="true" />
            {/if}
            <span class="min-w-0 flex-1 truncate text-sm text-fg">{file.fileName}</span>
            <span class="shrink-0 text-xs text-muted tabular-nums"
              >{formatFileSize(file.size, i18n.locale)}</span
            >
            <span class="hidden shrink-0 text-xs text-muted sm:inline"
              >{formatDate(file.createdAt, i18n.locale)}</span
            >
            <span class="flex shrink-0 gap-1">
              <Button
                variant="ghost"
                size="sm"
                aria-label={m.files_open({ name: file.fileName })}
                onclick={() => act(call(commands.openAttachment(file.id)))}
              >
                <ExternalLink class="size-4" aria-hidden="true" />
              </Button>
              <Button
                variant="ghost"
                size="sm"
                aria-label={m.files_save_as({ name: file.fileName })}
                onclick={() => saveAs(file)}
              >
                <Download class="size-4" aria-hidden="true" />
              </Button>
              {#if !file.encrypted}
                <Button
                  variant="ghost"
                  size="sm"
                  aria-label={m.files_reveal({ name: file.fileName })}
                  onclick={() => act(call(commands.revealAttachment(file.id)))}
                >
                  <FolderOpen class="size-4" aria-hidden="true" />
                </Button>
              {/if}
              <Button
                variant="ghost"
                size="sm"
                aria-label={m.files_delete({ name: file.fileName })}
                onclick={() => remove(file)}
              >
                <Trash2 class="size-4" aria-hidden="true" />
              </Button>
            </span>
          </li>
        {/each}
      </ul>
    {/if}
  {:else if images.length === 0}
    <p class="text-sm text-muted">{m.files_no_images()}</p>
  {:else}
    <div class="grid grid-cols-[repeat(auto-fill,minmax(8rem,1fr))] gap-3">
      {#each images as image (image.id)}
        <button
          type="button"
          aria-label={m.files_preview({ name: image.fileName })}
          disabled={hidden(image)}
          onclick={() => (viewing = image.id)}
          class="flex aspect-square items-center justify-center overflow-hidden rounded-[var(--radius)]
            border border-border bg-surface hover:border-accent disabled:cursor-not-allowed"
        >
          {#if hidden(image)}
            <Lock class="size-6 text-muted" aria-hidden="true" />
          {:else}
            <img
              src={previewUrl("thumb", image.id)}
              alt={image.fileName}
              loading="lazy"
              class="size-full object-cover"
            />
          {/if}
        </button>
      {/each}
    </div>
  {/if}
</div>

{#if viewing !== null}
  <Lightbox
    images={viewable}
    currentId={viewing}
    onnavigate={(id) => (viewing = id)}
    onclose={() => (viewing = null)}
  />
{/if}
