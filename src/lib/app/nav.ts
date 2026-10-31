import type { Component } from "svelte";
import Calendar from "@lucide/svelte/icons/calendar";
import CircleHelp from "@lucide/svelte/icons/circle-help";
import FolderKanban from "@lucide/svelte/icons/folder-kanban";
import House from "@lucide/svelte/icons/house";
import KeyRound from "@lucide/svelte/icons/key-round";
import ListChecks from "@lucide/svelte/icons/list-checks";
import Settings from "@lucide/svelte/icons/settings";
import Timer from "@lucide/svelte/icons/timer";
import Users from "@lucide/svelte/icons/users";
import Wallet from "@lucide/svelte/icons/wallet";
import type { RouteId } from "$app/types";
import { m } from "$lib/paraglide/messages";

/** Routes without parameters; only these can be navigation targets. */
type StaticRouteId = Exclude<RouteId, `${string}[${string}`>;

export interface NavItem {
  href: StaticRouteId;
  label: () => string;
  icon: Component;
}

export const NAV_ITEMS: NavItem[] = [
  { href: "/", label: m.nav_home, icon: House },
  { href: "/clients", label: m.nav_clients, icon: Users },
  { href: "/projects", label: m.nav_projects, icon: FolderKanban },
  { href: "/tasks", label: m.nav_tasks, icon: ListChecks },
  { href: "/calendar", label: m.nav_calendar, icon: Calendar },
  { href: "/time", label: m.nav_time, icon: Timer },
  { href: "/finance", label: m.nav_finance, icon: Wallet },
  { href: "/vault", label: m.nav_vault, icon: KeyRound },
  { href: "/settings", label: m.nav_settings, icon: Settings },
  { href: "/help", label: m.nav_help, icon: CircleHelp },
];

export function isActive(href: string, path: string): boolean {
  return href === "/" ? path === "/" : path === href || path.startsWith(`${href}/`);
}
