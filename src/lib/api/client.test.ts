import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, describe, expect, it } from "vitest";
import { commands } from "./bindings";
import { AppError, call } from "./client";

afterEach(() => clearMocks());

describe("call", () => {
  it("returns data on success", async () => {
    mockIPC((cmd) => {
      if (cmd === "app_info") return { version: "0.1.0", schemaVersion: 1 };
    });
    await expect(call(commands.appInfo())).resolves.toEqual({
      version: "0.1.0",
      schemaVersion: 1,
    });
  });

  it("throws AppError with code and params for a CoreError", async () => {
    mockIPC(() => {
      throw { code: "client.not_found", params: { id: "42" } };
    });
    const err = await call(commands.appInfo()).catch((e: unknown) => e);
    expect(err).toBeInstanceOf(AppError);
    expect(err).toMatchObject({ code: "client.not_found", params: { id: "42" } });
  });

  it("maps unexpected rejections to internal.unknown", async () => {
    mockIPC(() => {
      throw "plugin exploded";
    });
    const err = await call(commands.appInfo()).catch((e: unknown) => e);
    expect(err).toBeInstanceOf(AppError);
    expect(err).toMatchObject({ code: "internal.unknown", params: {} });
  });
});
