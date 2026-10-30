<script lang="ts">
  import Trash from "@lucide/svelte/icons/trash-2";
  import Upload from "@lucide/svelte/icons/upload";
  import { open } from "@tauri-apps/plugin-dialog";
  import { commands, type ImageKind } from "$lib/api/bindings";
  import { call } from "$lib/api/client";
  import { appSettings, updateSettings } from "$lib/app/settings.svelte";
  import { errorMessage } from "$lib/i18n/errors";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import Switch from "$lib/ui/Switch.svelte";
  import { toast } from "$lib/ui/toast.svelte";

  const KINDS: { kind: ImageKind; label: () => string }[] = [
    { kind: "logo", label: m.profile_image_logo },
    { kind: "signature", label: m.profile_image_signature },
    { kind: "stamp", label: m.profile_image_stamp },
  ];

  /** Data URLs of the set images. */
  let images = $state<Partial<Record<ImageKind, string | null>>>({});

  const showError = (err: unknown) => toast.show({ kind: "error", message: errorMessage(err) });

  async function load(kind: ImageKind) {
    images[kind] = await call(commands.profileImage(kind));
  }

  $effect(() => {
    void Promise.all(KINDS.map(({ kind }) => load(kind))).catch(showError);
  });

  async function choose(kind: ImageKind) {
    const path = await open({
      multiple: false,
      directory: false,
      filters: [{ name: "PNG, JPEG", extensions: ["png", "jpg", "jpeg"] }],
    }).catch(() => null);
    if (!path) return;
    try {
      await call(commands.setProfileImage(kind, path));
      await load(kind);
    } catch (err) {
      showError(err);
    }
  }

  async function remove(kind: ImageKind) {
    try {
      await call(commands.deleteProfileImage(kind));
      images[kind] = null;
    } catch (err) {
      showError(err);
    }
  }
</script>

<div class="flex max-w-2xl flex-col gap-4">
  <Switch
    label={m.settings_show_signature()}
    bind:checked={
      () => appSettings.value.invoiceShowSignature,
      (on) => void updateSettings({ invoiceShowSignature: on })
    }
  />
  <p class="text-sm text-muted">{m.profile_image_hint()}</p>
  <div class="grid gap-4 sm:grid-cols-3">
    {#each KINDS as { kind, label } (kind)}
      <div
        role="group"
        aria-labelledby="image-{kind}"
        class="flex flex-col gap-2 rounded-md border border-border p-3"
      >
        <span id="image-{kind}" class="text-sm font-medium text-fg">{label()}</span>
        <div class="flex h-24 items-center justify-center rounded bg-surface">
          {#if images[kind]}
            <img src={images[kind]} alt={label()} class="max-h-24 max-w-full object-contain" />
          {:else}
            <span class="text-sm text-muted">{m.profile_image_none()}</span>
          {/if}
        </div>
        <div class="flex gap-2">
          <Button size="sm" onclick={() => choose(kind)}>
            <Upload class="size-4" aria-hidden="true" />
            <span class="sr-only">{m.profile_image_choose({ kind: label() })}</span>
          </Button>
          {#if images[kind]}
            <Button
              size="sm"
              variant="ghost"
              aria-label={m.profile_image_remove({ kind: label() })}
              onclick={() => remove(kind)}
            >
              <Trash class="size-4" aria-hidden="true" />
            </Button>
          {/if}
        </div>
      </div>
    {/each}
  </div>
</div>
