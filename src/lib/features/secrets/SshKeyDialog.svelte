<script lang="ts">
  import { commands, type SshKeyDto, type SshKeyType } from "$lib/api/bindings";
  import { call } from "$lib/api/client";
  import { errorMessage } from "$lib/i18n/errors";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import Dialog from "$lib/ui/Dialog.svelte";
  import RadioGroup from "$lib/ui/RadioGroup.svelte";
  import TextField from "$lib/ui/TextField.svelte";
  import { toast } from "$lib/ui/toast.svelte";

  let {
    ongenerated,
    oncancel,
  }: {
    ongenerated: (key: SshKeyDto, passphrase: string) => void;
    oncancel: () => void;
  } = $props();

  const formId = $props.id();
  let keyType = $state<SshKeyType>("ed25519");
  let comment = $state("");
  let passphrase = $state("");
  let busy = $state(false);

  const types = [
    { value: "ed25519", label: "Ed25519" },
    { value: "rsa4096", label: "RSA 4096" },
  ];

  async function generate(event: SubmitEvent) {
    event.preventDefault();
    busy = true;
    try {
      const key = await call(
        commands.generateSshKey(keyType, comment.trim(), passphrase === "" ? null : passphrase),
      );
      ongenerated(key, passphrase);
    } catch (err) {
      toast.show({ kind: "error", message: errorMessage(err) });
    } finally {
      busy = false;
    }
  }
</script>

<Dialog open title={m.ssh_generate_title()} onclose={oncancel}>
  <form id={formId} class="flex w-96 max-w-full flex-col gap-4" novalidate onsubmit={generate}>
    <RadioGroup
      label={m.ssh_key_type()}
      options={types}
      bind:value={() => keyType, (v) => (keyType = v as SshKeyType)}
    />
    <TextField label={m.ssh_comment()} bind:value={comment} />
    <TextField
      label={m.secret_field_passphrase()}
      type="password"
      autocomplete="new-password"
      bind:value={passphrase}
    />
  </form>
  {#snippet footer()}
    <Button onclick={oncancel}>{m.ui_cancel()}</Button>
    <Button type="submit" form={formId} variant="primary" loading={busy}>
      {busy ? m.ssh_generating() : m.ssh_generate()}
    </Button>
  {/snippet}
</Dialog>
