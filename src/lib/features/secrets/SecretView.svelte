<script lang="ts">
  import Copy from "@lucide/svelte/icons/copy";
  import Download from "@lucide/svelte/icons/download";
  import Eye from "@lucide/svelte/icons/eye";
  import EyeOff from "@lucide/svelte/icons/eye-off";
  import Pencil from "@lucide/svelte/icons/pencil";
  import Trash2 from "@lucide/svelte/icons/trash-2";
  import { save } from "@tauri-apps/plugin-dialog";
  import { goto } from "$app/navigation";
  import { resolve } from "$app/paths";
  import {
    commands,
    type FieldRef,
    type SecretOutline,
    type SecretTemplate,
    type Secret_Serialize as Secret,
  } from "$lib/api/bindings";
  import { call } from "$lib/api/client";
  import { errorMessage } from "$lib/i18n/errors";
  import { formatIsoDate } from "$lib/i18n/format";
  import { i18n } from "$lib/i18n/locale.svelte";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import { confirm } from "$lib/ui/dialogs.svelte";
  import Spinner from "$lib/ui/Spinner.svelte";
  import { toast } from "$lib/ui/toast.svelte";
  import AttachmentPanel from "../files/AttachmentPanel.svelte";
  import { vault, watchVault } from "../vault/vault.svelte";
  import { fieldLabel, kindLabel } from "./labels";

  let { id }: { id: string } = $props();

  interface Row {
    ref: FieldRef;
    label: string;
    secret: boolean;
    multiline: boolean;
  }

  let secret = $state<Secret | null>(null);
  let rows = $state<Row[]>([]);
  /** Decrypted values by row label; dropped as soon as the vault locks. */
  let revealed = $state<Record<string, string>>({});
  let loadError = $state<string | null>(null);

  const showError = (err: unknown) => toast.show({ kind: "error", message: errorMessage(err) });

  function rowsOf(template: SecretTemplate | undefined, outline: SecretOutline): Row[] {
    const fields = (template?.fields ?? [])
      .filter((f) => outline.fields.includes(f.key))
      .map((f) => ({
        ref: { type: "field", key: f.key } as FieldRef,
        label: fieldLabel(f.key),
        secret: f.secret,
        multiline: f.multiline,
      }));
    const custom = outline.custom.map((c, index) => ({
      ref: { type: "custom", index } as FieldRef,
      label: c.key,
      secret: c.secret,
      multiline: false,
    }));
    return [...fields, ...custom];
  }

  async function reveal(row: Row) {
    try {
      revealed[row.label] = await call(commands.revealSecretField(id, row.ref));
    } catch (err) {
      showError(err);
    }
  }

  $effect(() => {
    void id;
    Promise.all([
      call(commands.getSecret(id)),
      commands.listSecretTemplates(),
      call(commands.secretOutline(id)),
    ]).then(
      ([s, templates, outline]) => {
        secret = s;
        rows = rowsOf(
          templates.find((t) => t.kind === s.kind),
          outline,
        );
        // Plain values are shown right away; secret ones only on demand.
        for (const row of rows) if (!row.secret) void reveal(row);
      },
      (err) => (loadError = errorMessage(err)),
    );
  });

  $effect(() => watchVault());

  // Forget every decrypted value as soon as the vault is no longer open.
  $effect(() => {
    if (vault.status !== "unlocked") revealed = {};
  });

  async function copy(row: Row) {
    try {
      await call(commands.copySecretField(id, row.ref));
      toast.show({ message: m.secret_copied({ name: row.label }) });
    } catch (err) {
      showError(err);
    }
  }

  async function exportKey() {
    const path = await save({ defaultPath: "id_ed25519" }).catch(() => null);
    if (!path) return;
    try {
      await call(commands.exportSshKey(id, path));
      toast.show({ message: m.ssh_exported_toast() });
    } catch (err) {
      showError(err);
    }
  }

  async function remove() {
    if (!secret) return;
    const ok = await confirm({
      title: m.secret_delete_title({ name: secret.title }),
      confirmLabel: m.ui_delete(),
      danger: true,
    });
    if (!ok) return;
    try {
      await call(commands.deleteSecret(id));
      toast.show({ message: m.secret_deleted_toast() });
      await goto(resolve("/vault"));
    } catch (err) {
      showError(err);
    }
  }
</script>

{#if loadError}
  <p class="text-sm text-danger" role="alert">{loadError}</p>
{:else if !secret}
  <div class="flex justify-center p-8"><Spinner /></div>
{:else}
  <div class="flex max-w-3xl flex-col gap-6">
    <header class="flex items-start gap-3">
      <div class="min-w-0 flex-1">
        <h1 class="text-xl font-semibold text-fg">{secret.title}</h1>
        <p class="flex flex-wrap gap-2 text-sm text-muted">
          <span>{kindLabel(secret.kind)}</span>
          {#if secret.clientId}
            <span aria-hidden="true">·</span>
            <a
              href={resolve("/clients/[id]", { id: secret.clientId })}
              class="hover:underline focus-visible:outline-2 focus-visible:outline-focus"
            >
              {secret.clientName}
            </a>
          {/if}
          {#if secret.projectId}
            <span aria-hidden="true">·</span>
            <a
              href={resolve("/projects/[id]", { id: secret.projectId })}
              class="hover:underline focus-visible:outline-2 focus-visible:outline-focus"
            >
              {secret.projectName}
            </a>
          {/if}
          {#if secret.expiresOn}
            <span aria-hidden="true">·</span>
            <span>{m.secret_expires({ date: formatIsoDate(secret.expiresOn, i18n.locale) })}</span>
          {/if}
        </p>
      </div>
      {#if secret.kind === "ssh_key"}
        <Button onclick={exportKey}>
          <Download class="size-4" aria-hidden="true" />
          {m.ssh_export()}
        </Button>
      {/if}
      <Button onclick={() => goto(resolve("/vault/[id]/edit", { id }))}>
        <Pencil class="size-4" aria-hidden="true" />
        {m.ui_edit()}
      </Button>
      <Button variant="ghost" aria-label={m.secret_delete()} onclick={remove}>
        <Trash2 class="size-4" aria-hidden="true" />
      </Button>
    </header>

    <dl class="flex flex-col divide-y divide-border rounded-[var(--radius)] border border-border">
      {#each rows as row (row.label)}
        {@const value = revealed[row.label]}
        <div class="grid grid-cols-[12rem_1fr] items-start gap-3 px-3 py-2 text-sm">
          <dt class="text-muted">{row.label}</dt>
          <dd class="flex min-w-0 items-start gap-3">
            <span class="min-w-0 flex-1 font-mono break-all whitespace-pre-wrap text-fg">
              {#if value !== undefined}{value}{:else if row.secret}<span
                  class="font-sans text-muted">{m.secret_hidden()}</span
                >{/if}
            </span>
            <span class="flex shrink-0 gap-1">
              {#if row.secret}
                {#if value === undefined}
                  <Button
                    variant="ghost"
                    size="sm"
                    aria-label={m.secret_show({ name: row.label })}
                    onclick={() => reveal(row)}
                  >
                    <Eye class="size-4" aria-hidden="true" />
                  </Button>
                {:else}
                  <Button
                    variant="ghost"
                    size="sm"
                    aria-label={m.secret_hide({ name: row.label })}
                    onclick={() => delete revealed[row.label]}
                  >
                    <EyeOff class="size-4" aria-hidden="true" />
                  </Button>
                {/if}
              {/if}
              <Button
                variant="ghost"
                size="sm"
                aria-label={m.secret_copy({ name: row.label })}
                onclick={() => copy(row)}
              >
                <Copy class="size-4" aria-hidden="true" />
              </Button>
            </span>
          </dd>
        </div>
      {/each}
    </dl>

    <section class="flex flex-col gap-2">
      <h2 class="text-base font-semibold text-fg">{m.client_tab_files()}</h2>
      <AttachmentPanel owner={{ type: "secret", id: secret.id }} />
    </section>
  </div>
{/if}
