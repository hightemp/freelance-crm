// Test data shared by secret component tests (mirrors crm_core templates).
import type { SecretTemplate, Secret_Serialize as Secret } from "$lib/api/bindings";

const field = (
  key: string,
  opts: { required?: boolean; secret?: boolean; multiline?: boolean } = {},
) => ({
  key,
  required: opts.required ?? false,
  secret: opts.secret ?? false,
  multiline: opts.multiline ?? false,
});

export const TEMPLATES: SecretTemplate[] = [
  {
    kind: "login",
    fields: [field("url"), field("username"), field("password", { required: true, secret: true })],
  },
  {
    kind: "ssh_key",
    fields: [
      field("private_key", { required: true, secret: true, multiline: true }),
      field("public_key", { multiline: true }),
      field("passphrase", { secret: true }),
      field("host"),
      field("user"),
      field("port"),
    ],
  },
  { kind: "env", fields: [field("content", { required: true, secret: true, multiline: true })] },
  { kind: "custom", fields: [] },
];

export function secret(id: string, title: string, patch: Partial<Secret> = {}): Secret {
  return {
    id,
    title,
    kind: "login",
    clientId: null,
    clientName: null,
    projectId: null,
    projectName: null,
    expiresOn: null,
    tagIds: [],
    createdAt: 0,
    updatedAt: 0,
    ...patch,
  };
}
