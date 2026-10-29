import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { emit } from "@tauri-apps/api/event";
import { clearMocks, mockConvertFileSrc, mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import type { Attachment, EntityRef, VaultStatus } from "$lib/api/bindings";
import DialogHost from "$lib/ui/DialogHost.svelte";
import { resetDialogs } from "$lib/ui/dialogs.svelte";
import { expectNoA11yViolations } from "../../../test/a11y";
import { resetVault } from "../vault/vault.svelte";
import AttachmentPanel from "./AttachmentPanel.svelte";

const OWNER: EntityRef = { type: "client", id: "c1" };
let files: Attachment[] = [];
let status: VaultStatus = "unlocked";
let calls: { cmd: string; args: Record<string, unknown> }[] = [];
let picked: string[] | null = null;
let n = 0;

function file(name: string, mime: string, extra: Partial<Attachment> = {}): Attachment {
  n += 1;
  return {
    id: `a${n}`,
    blobId: `b${n}`,
    fileName: name,
    mime,
    size: 2048,
    encrypted: false,
    hasThumbnail: mime.startsWith("image/"),
    createdAt: 1_700_000_000_000 + n,
    ...extra,
  };
}

beforeEach(() => {
  n = 0;
  status = "unlocked";
  calls = [];
  picked = null;
  files = [
    file("Договор.pdf", "application/pdf"),
    file("photo.png", "image/png"),
    file("passport.jpg", "image/jpeg", { encrypted: true }),
  ];
  resetVault();
  mockWindows("main");
  mockConvertFileSrc("linux");
  mockIPC(
    (cmd, args) => {
      const a = (args ?? {}) as Record<string, unknown>;
      calls.push({ cmd, args: a });
      switch (cmd) {
        case "vault_status":
          return status;
        case "list_attachments":
          expect(a.owner).toEqual(OWNER);
          return [...files].reverse();
        case "attach_file": {
          const name = String(a.path).split("/").pop()!;
          const added = file(name, "application/octet-stream", { encrypted: a.encrypt as boolean });
          files.push(added);
          return added;
        }
        case "attach_clipboard_image": {
          const added = file(String(a.fileName), "image/png");
          files.push(added);
          return added;
        }
        case "delete_attachment":
          files = files.filter((f) => f.id !== a.id);
          return null;
        case "plugin:dialog|open":
          return picked;
        case "plugin:dialog|save":
          return "/home/me/copy.pdf";
        default:
          return null;
      }
    },
    { shouldMockEvents: true },
  );
});

afterEach(() => {
  cleanup();
  clearMocks();
  resetDialogs();
});

const named = (cmd: string) => calls.filter((c) => c.cmd === cmd).map((c) => c.args);

describe("AttachmentPanel", () => {
  it("lists files newest first with their size", async () => {
    render(AttachmentPanel, { owner: OWNER });
    const rows = await screen.findAllByRole("listitem");
    expect(rows.map((r) => within(r).getByText(/\.(pdf|png|jpg)$/).textContent)).toEqual([
      "passport.jpg",
      "photo.png",
      "Договор.pdf",
    ]);
    expect(within(rows[2]).getByText("2 kB")).toBeInTheDocument();
    expect(within(rows[0]).getByLabelText("Encrypted")).toBeInTheDocument();
  });

  it("attaches files picked in the system dialog", async () => {
    picked = ["/home/me/a.pdf", "/home/me/b.txt"];
    render(AttachmentPanel, { owner: OWNER });
    await userEvent.click(await screen.findByRole("button", { name: "Add files" }));
    await waitFor(() => expect(named("attach_file")).toHaveLength(2));
    expect(named("attach_file")[0]).toEqual({
      owner: OWNER,
      path: "/home/me/a.pdf",
      encrypt: false,
    });
    expect(await screen.findByText("b.txt")).toBeInTheDocument();
  });

  it("attaches each dropped file", async () => {
    render(AttachmentPanel, { owner: OWNER });
    await screen.findByText("photo.png");
    await emit("tauri://drag-enter", { paths: ["/x/c.zip"], position: { x: 1, y: 1 } });
    expect(screen.getByTestId("drop-zone")).toHaveAttribute("data-dragging", "true");
    await emit("tauri://drag-drop", {
      paths: ["/x/c.zip", "/x/d.docx"],
      position: { x: 1, y: 1 },
    });
    await waitFor(() =>
      expect(named("attach_file").map((a) => a.path)).toEqual(["/x/c.zip", "/x/d.docx"]),
    );
    expect(screen.getByTestId("drop-zone")).toHaveAttribute("data-dragging", "false");
  });

  it("encrypts on request, only with an unlocked vault", async () => {
    status = "locked";
    const { unmount } = render(AttachmentPanel, { owner: OWNER });
    const locked = await screen.findByRole("checkbox", { name: "Encrypt" });
    expect(locked).toBeDisabled();
    expect(screen.getByText("Unlock the vault to encrypt files")).toBeInTheDocument();
    unmount();

    status = "unlocked";
    resetVault();
    picked = ["/home/me/scan.pdf"];
    render(AttachmentPanel, { owner: OWNER });
    const box = await screen.findByRole("checkbox", { name: "Encrypt" });
    await waitFor(() => expect(box).toBeEnabled());
    await userEvent.click(box);
    await userEvent.click(screen.getByRole("button", { name: "Add files" }));
    await waitFor(() => expect(named("attach_file")[0]?.encrypt).toBe(true));
  });

  it("always encrypts files of secrets", async () => {
    render(AttachmentPanel, { owner: { type: "secret", id: "s1" } });
    expect(await screen.findByText("Files of secrets are always encrypted")).toBeInTheDocument();
    expect(screen.queryByRole("checkbox", { name: "Encrypt" })).not.toBeInTheDocument();
  });

  it("shows only images in the gallery and pages through them", async () => {
    render(AttachmentPanel, { owner: OWNER });
    await screen.findByText("photo.png");
    await userEvent.click(screen.getByRole("button", { name: "Gallery" }));
    const thumbs = screen.getAllByRole("button", { name: /^View / });
    expect(thumbs.map((t) => t.getAttribute("aria-label"))).toEqual([
      "View passport.jpg",
      "View photo.png",
    ]);
    expect(screen.queryByText("Договор.pdf")).not.toBeInTheDocument();

    await userEvent.click(thumbs[0]);
    const lightbox = await screen.findByRole("dialog", { name: "passport.jpg" });
    expect(within(lightbox).getByRole("img", { name: "passport.jpg" })).toHaveAttribute(
      "src",
      expect.stringContaining("full"),
    );
    await userEvent.keyboard("{ArrowRight}");
    expect(await screen.findByRole("dialog", { name: "photo.png" })).toBeInTheDocument();
    // Wraps around.
    await userEvent.keyboard("{ArrowRight}");
    expect(await screen.findByRole("dialog", { name: "passport.jpg" })).toBeInTheDocument();
    await userEvent.keyboard("{ArrowLeft}");
    expect(await screen.findByRole("dialog", { name: "photo.png" })).toBeInTheDocument();
    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
  });

  it("hides encrypted previews while the vault is locked", async () => {
    status = "locked";
    render(AttachmentPanel, { owner: OWNER });
    await screen.findByText("photo.png");
    await userEvent.click(screen.getByRole("button", { name: "Gallery" }));
    expect(screen.getAllByRole("img").map((i) => i.getAttribute("alt"))).toEqual(["photo.png"]);
    expect(screen.getByRole("button", { name: "View passport.jpg" })).toBeDisabled();
  });

  it("views plain images while the vault is locked", async () => {
    status = "locked";
    render(AttachmentPanel, { owner: OWNER });
    await screen.findByText("photo.png");
    await userEvent.click(screen.getByRole("button", { name: "Gallery" }));
    await userEvent.click(screen.getByRole("button", { name: "View photo.png" }));
    expect(await screen.findByRole("dialog", { name: "photo.png" })).toBeInTheDocument();
    // Only one viewable image: no paging.
    expect(screen.queryByRole("button", { name: "Next image" })).not.toBeInTheDocument();
  });

  it("closes an encrypted image when the vault locks", async () => {
    render(AttachmentPanel, { owner: OWNER });
    await screen.findByText("photo.png");
    await userEvent.click(screen.getByRole("button", { name: "Gallery" }));
    await userEvent.click(screen.getByRole("button", { name: "View passport.jpg" }));
    await screen.findByRole("dialog", { name: "passport.jpg" });
    status = "locked";
    await emit("vault://locked");
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
  });

  it("opens, saves, reveals and deletes files", async () => {
    render(DialogHost);
    render(AttachmentPanel, { owner: OWNER });
    await userEvent.click(await screen.findByRole("button", { name: "Open Договор.pdf" }));
    expect(named("open_attachment")).toEqual([{ id: "a1" }]);

    await userEvent.click(screen.getByRole("button", { name: "Save Договор.pdf as…" }));
    await waitFor(() =>
      expect(named("save_attachment_as")).toEqual([{ id: "a1", path: "/home/me/copy.pdf" }]),
    );
    await userEvent.click(screen.getByRole("button", { name: "Show Договор.pdf in folder" }));
    expect(named("reveal_attachment")).toEqual([{ id: "a1" }]);
    // Decrypted copies are never shown in a folder.
    expect(
      screen.queryByRole("button", { name: "Show passport.jpg in folder" }),
    ).not.toBeInTheDocument();

    await userEvent.click(screen.getByRole("button", { name: "Delete Договор.pdf" }));
    const dialog = await screen.findByRole("alertdialog");
    await userEvent.click(within(dialog).getByRole("button", { name: "Delete" }));
    await waitFor(() => expect(screen.queryByText("Договор.pdf")).not.toBeInTheDocument());
  });

  it("attaches a pasted image", async () => {
    render(AttachmentPanel, { owner: OWNER });
    await screen.findByText("photo.png");
    const image = new File(["x"], "image.png", { type: "image/png" });
    await fireEvent.paste(window, {
      clipboardData: { types: ["Files"], files: [image], getData: () => "" },
    });
    await waitFor(() => expect(named("attach_clipboard_image")).toHaveLength(1));
    expect(named("attach_clipboard_image")[0]).toEqual({
      owner: OWNER,
      fileName: expect.stringMatching(/^Image \d{4}-\d{2}-\d{2} \d{2}-\d{2}-\d{2}\.png$/),
      encrypt: false,
    });
  });

  it("ignores pasted text", async () => {
    render(AttachmentPanel, { owner: OWNER });
    await screen.findByText("photo.png");
    await fireEvent.paste(window, {
      clipboardData: { types: ["text/plain"], files: [], getData: () => "hello" },
    });
    expect(named("attach_clipboard_image")).toHaveLength(0);
  });

  it("has no a11y violations", async () => {
    const { container } = render(AttachmentPanel, { owner: OWNER });
    await screen.findByText("photo.png");
    await expectNoA11yViolations(container);
  });
});
