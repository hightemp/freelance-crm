// Screenshots for the README (`pnpm screenshots`): fills a throwaway
// database with demo data through the app's own commands, then saves the
// main screens to docs/screenshots/<lang>-<screen>.png.
import { $, browser, expect } from "@wdio/globals";
import { join, resolve } from "node:path";
import { button, heading, tab } from "../helpers";

const lang = process.env.SCREENSHOTS_LANG === "ru" ? "ru" : "en";
const out = resolve(import.meta.dirname, "..", "..", "docs", "screenshots");

const t = {
  en: {
    clients: [
      ["Northwind Studio", "Northwind Studio LLC"],
      ["Acme Robotics", "Acme Robotics Inc."],
      ["Blue Harbor Cafe", null],
    ],
    person: ["Fischer", "Lena"],
    projects: ["Website redesign", "Mobile app MVP", "Brand identity", "Online store"],
    tasks: [
      ["Homepage layout", "in_progress", "high", 0],
      ["Product page mockups", "todo", "normal", 2],
      ["Fix checkout bug", "todo", "urgent", -1],
      ["Logo variations", "review", "normal", 1],
      ["API for orders", "in_progress", "normal", 4],
      ["Brand guidelines PDF", "todo", "low", 6],
      ["Set up analytics", "done", "normal", -3],
      ["Onboarding screens", "todo", "high", 3],
    ],
    events: [
      ["Design review with Northwind", 0, "11:00", "12:30"],
      ["Call with Acme", 1, "15:00", "16:00"],
      ["Workshop: brand values", 2, "10:00", "12:30"],
      ["Lunch with Lena", 2, "13:30", "14:30"],
      ["Sprint demo", 3, "16:00", "17:00"],
      ["Store launch", 4, "11:00", "12:00"],
      ["Invoice review", 5, "10:00", "11:00"],
      ["Call with Blue Harbor", 6, "17:00", "18:00"],
      ["Kick-off: store v2", 8, "11:00", "12:00"],
      ["Retrospective", 9, "15:00", "16:00"],
    ],
    weekly: "Weekly planning",
    work: ["Layout", "Mockups", "Review fixes", "API work", "Call notes"],
    item: "Website redesign, design and front-end",
    expense: ["Software", "Figma subscription"],
    home: "Home",
    board: "Board",
    week: "Week",
  },
  ru: {
    clients: [
      ["Студия Север", "ООО «Студия Север»"],
      ["Ромашка", "ООО «Ромашка»"],
      ["Кофейня «Гавань»", null],
    ],
    person: ["Петрова", "Анна"],
    projects: [
      "Редизайн сайта",
      "MVP мобильного приложения",
      "Фирменный стиль",
      "Интернет-магазин",
    ],
    tasks: [
      ["Вёрстка главной", "in_progress", "high", 0],
      ["Макеты карточки товара", "todo", "normal", 2],
      ["Ошибка в корзине", "todo", "urgent", -1],
      ["Варианты логотипа", "review", "normal", 1],
      ["API заказов", "in_progress", "normal", 4],
      ["Брендбук в PDF", "todo", "low", 6],
      ["Подключить аналитику", "done", "normal", -3],
      ["Экраны онбординга", "todo", "high", 3],
    ],
    events: [
      ["Обсуждение макетов со «Студией Север»", 0, "11:00", "12:30"],
      ["Созвон с «Ромашкой»", 1, "15:00", "16:00"],
      ["Воркшоп: ценности бренда", 2, "10:00", "12:30"],
      ["Обед с Анной", 2, "13:30", "14:30"],
      ["Демо спринта", 3, "16:00", "17:00"],
      ["Запуск магазина", 4, "11:00", "12:00"],
      ["Сверка счетов", 5, "10:00", "11:00"],
      ["Созвон с «Гаванью»", 6, "17:00", "18:00"],
      ["Старт второй версии магазина", 8, "11:00", "12:00"],
      ["Ретроспектива", 9, "15:00", "16:00"],
    ],
    weekly: "Планирование недели",
    work: ["Вёрстка", "Макеты", "Правки после ревью", "API", "Созвон"],
    item: "Редизайн сайта: дизайн и вёрстка",
    expense: ["Программы", "Подписка Figma"],
    home: "Главная",
    board: "Доска",
    week: "Неделя",
  },
}[lang];

const currency = lang === "ru" ? "RUB" : "USD";
const rate = lang === "ru" ? 300_000 : 6_000; // per hour, minor units

async function invoke<T = Record<string, unknown>>(cmd: string, args: object = {}): Promise<T> {
  const result = (await browser.execute(
    (cmd, args) =>
      (
        window as unknown as {
          __TAURI__: { core: { invoke: (c: string, a: object) => Promise<unknown> } };
        }
      ).__TAURI__.core
        .invoke(cmd, args)
        .then((ok) => ({ ok }))
        .catch((error: unknown) => ({ error: JSON.stringify(error) })),
    cmd,
    args,
  )) as { ok: T } | { error: string };
  if ("error" in result) throw new Error(`${cmd}: ${result.error}`);
  return result.ok;
}

const pad = (n: number) => String(n).padStart(2, "0");
function day(offset: number): string {
  const d = new Date();
  d.setDate(d.getDate() + offset);
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}
function at(offset: number, time: string): number {
  return new Date(`${day(offset)}T${time}`).getTime();
}
const tz = Intl.DateTimeFormat().resolvedOptions().timeZone;

async function seed() {
  const settings = await invoke("get_settings");
  await invoke("save_settings", {
    settings: { ...settings, locale: lang, theme: "light", defaultCurrency: currency },
  });

  const clients = [];
  for (const [name, legal] of t.clients) {
    clients.push(
      await invoke<{ id: string }>("create_client", {
        input: {
          kind: "company",
          companyName: name,
          legalName: legal,
          status: "active",
          defaultCurrency: currency,
          defaultRateMinor: rate,
        },
      }),
    );
  }
  await invoke("create_client", {
    input: {
      kind: "person",
      lastName: t.person[0],
      firstName: t.person[1],
      status: "lead",
      defaultCurrency: currency,
    },
  });

  const owners = [0, 1, 0, 2];
  const projects = [];
  for (const [i, name] of t.projects.entries()) {
    projects.push(
      await invoke<{ id: string }>("create_project", {
        input: {
          clientId: clients[owners[i]].id,
          name,
          descriptionMd: "",
          status: "active",
          billingType: i === 1 ? "fixed" : "hourly",
          currency,
          rateMinor: rate,
          budgetMinor: i === 1 ? rate * 120 : null,
          startDate: day(-60),
          deadline: day(20 + i * 10),
        },
      }),
    );
  }

  const columns = await invoke<{ id: string; nameKey: string | null }[]>("list_board_columns");
  const status = (key: string) => columns.find((c) => c.nameKey === key)?.id ?? null;
  const tasks = [];
  for (const [i, [title, key, priority, due]] of t.tasks.entries()) {
    tasks.push(
      await invoke<{ id: string }>("create_task", {
        input: {
          title,
          descriptionMd: "",
          statusId: status(key as string),
          priority,
          projectId: projects[i % projects.length].id,
          dueDate: day(due as number),
          estimateMinutes: 120 + i * 30,
        },
      }),
    );
  }

  // Events fill this week, Monday to Sunday, and a few go into the next one.
  const monday = (new Date().getDay() + 6) % 7;
  for (const [title, offset, start, end] of t.events) {
    await invoke("create_event", {
      input: {
        title,
        descriptionMd: "",
        allDay: false,
        start: `${day((offset as number) - monday)}T${start}`,
        end: `${day((offset as number) - monday)}T${end}`,
        tzid: tz,
        clientId: clients[0].id,
      },
    });
  }
  await invoke("create_event", {
    input: {
      title: t.weekly,
      descriptionMd: "",
      allDay: false,
      start: `${day(-monday)}T09:00`,
      end: `${day(-monday)}T10:00`,
      tzid: tz,
      rrule: "FREQ=WEEKLY",
    },
  });

  // Two weeks of tracked time, weekdays only.
  const slots = [
    ["09:15", "11:45"],
    ["10:00", "12:20"],
    ["13:10", "15:40"],
    ["14:00", "17:05"],
    ["16:00", "18:30"],
  ];
  for (let d = -13; d <= 0; d++) {
    const weekday = new Date(at(d, "12:00")).getDay();
    if (weekday === 0 || weekday === 6) continue;
    for (const i of [0, 1]) {
      const [from, to] = slots[(Math.abs(d) * 2 + i * 3) % slots.length];
      await invoke("create_time_entry", {
        input: {
          target: { taskId: tasks[(Math.abs(d) + i * 3) % tasks.length].id },
          description: t.work[(Math.abs(d) + i) % t.work.length],
          startedAt: at(d, from),
          endedAt: at(d, to),
          billable: true,
        },
      });
    }
  }

  // Invoices and payments over half a year, for the dashboard.
  for (let month = 9; month >= 0; month--) {
    const issued = day(-month * 30 - 10);
    const hours = 20 + ((month * 7) % 15);
    const invoice = await invoke<{ id: string; number: string }>("create_invoice", {
      input: {
        number: `INV-2026-${String(10 - month).padStart(3, "0")}`,
        clientId: clients[owners[month % projects.length]].id,
        projectId: projects[month % projects.length].id,
        currency,
        issueDate: issued,
        dueDate: day(-month * 30 + 4),
        status: "sent",
        items: [
          {
            description: t.item,
            quantityMilli: hours * 1000,
            unit: lang === "ru" ? "ч" : "h",
            price: { amountMinor: rate, currency },
          },
        ],
        discount: { type: "none" },
        taxBp: 0,
        notesMd: "",
        language: lang,
      },
    });
    const total = hours * rate;
    if (month > 0) {
      await invoke("create_payment", {
        input: {
          invoiceId: invoice.id,
          amount: { amountMinor: month === 1 ? total / 2 : total, currency },
          paidOn: day(-month * 30),
          method: "bank",
          note: "",
        },
      });
    }
    await invoke("create_expense", {
      input: {
        category: t.expense[0],
        amount: { amountMinor: rate * (3 + (month % 4)), currency },
        spentOn: day(-month * 30 - 3),
        description: t.expense[1],
      },
    });
  }
}

async function shot(name: string) {
  // No focus ring from the last click; charts and transitions settle.
  await browser.execute(() => (document.activeElement as HTMLElement | null)?.blur());
  await browser.pause(800);
  await browser.saveScreenshot(join(out, `${lang}-${name}.png`));
}

describe("README screenshots", () => {
  it(`captures the main screens (${lang})`, async () => {
    await browser.setWindowSize(1440, 900).catch(() => {});
    await seed();
    await browser.execute(() => location.reload());
    await expect(heading(t.home)).toBeDisplayed();
    await shot("home");

    await $('a[href="/tasks"]').click();
    await tab(t.board).click();
    await shot("board");

    await $('a[href="/calendar"]').click();
    await button(t.week).click();
    await shot("calendar");

    await $('a[href="/time"]').click();
    await shot("time");

    await $('a[href="/finance"]').click();
    await shot("finance");
  });
});
