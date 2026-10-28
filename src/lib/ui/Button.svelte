<script lang="ts">
  import type { Snippet } from "svelte";
  import type { HTMLButtonAttributes } from "svelte/elements";
  import Spinner from "./Spinner.svelte";

  type Variant = "primary" | "secondary" | "ghost" | "danger";

  interface Props extends HTMLButtonAttributes {
    variant?: Variant;
    size?: "sm" | "md";
    loading?: boolean;
    children: Snippet;
  }

  let {
    variant = "secondary",
    size = "md",
    loading = false,
    disabled = false,
    type = "button",
    children,
    ...rest
  }: Props = $props();

  const variants: Record<Variant, string> = {
    primary: "bg-accent text-accent-fg border-transparent hover:bg-accent-hover",
    secondary: "bg-surface text-fg border-border hover:bg-surface-hover",
    ghost: "bg-transparent text-fg border-transparent hover:bg-surface-hover",
    danger: "bg-danger text-accent-fg border-transparent hover:opacity-90",
  };
</script>

<button
  {...rest}
  {type}
  disabled={disabled || loading}
  aria-busy={loading || undefined}
  data-variant={variant}
  class={[
    "inline-flex items-center justify-center gap-2 rounded-[var(--radius)] border font-medium",
    "transition-colors select-none disabled:cursor-not-allowed disabled:opacity-50",
    size === "sm" ? "h-7 px-2.5 text-xs" : "h-8 px-3 text-sm",
    variants[variant],
    rest.class,
  ]}
>
  {#if loading}<Spinner size={14} />{/if}
  {@render children()}
</button>
