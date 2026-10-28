<script lang="ts">
  // Dev-only gallery of UI kit components and states (FR-UI-7).
  // Not localized on purpose: it is never shipped.
  import { setTheme } from "$lib/app/theme.svelte";
  import Accordion from "$lib/ui/Accordion.svelte";
  import Button from "$lib/ui/Button.svelte";
  import Checkbox from "$lib/ui/Checkbox.svelte";
  import ColorPicker from "$lib/ui/ColorPicker.svelte";
  import Combobox from "$lib/ui/Combobox.svelte";
  import DatePicker from "$lib/ui/DatePicker.svelte";
  import DateRangePicker from "$lib/ui/DateRangePicker.svelte";
  import DateTimePicker from "$lib/ui/DateTimePicker.svelte";
  import Dialog from "$lib/ui/Dialog.svelte";
  import DropdownMenu from "$lib/ui/DropdownMenu.svelte";
  import FileDropZone from "$lib/ui/FileDropZone.svelte";
  import MoneyField from "$lib/ui/MoneyField.svelte";
  import NumberField from "$lib/ui/NumberField.svelte";
  import Popover from "$lib/ui/Popover.svelte";
  import Progress from "$lib/ui/Progress.svelte";
  import RadioGroup from "$lib/ui/RadioGroup.svelte";
  import Select from "$lib/ui/Select.svelte";
  import Slider from "$lib/ui/Slider.svelte";
  import Spinner from "$lib/ui/Spinner.svelte";
  import Switch from "$lib/ui/Switch.svelte";
  import Tabs from "$lib/ui/Tabs.svelte";
  import TagInput from "$lib/ui/TagInput.svelte";
  import TextArea from "$lib/ui/TextArea.svelte";
  import TextField from "$lib/ui/TextField.svelte";
  import TimePicker from "$lib/ui/TimePicker.svelte";
  import Tooltip from "$lib/ui/Tooltip.svelte";
  import { confirm, prompt } from "$lib/ui/dialogs.svelte";
  import { toast } from "$lib/ui/toast.svelte";

  let dark = $state(false);
  let dialogOpen = $state(false);
  let tab = $state("one");
  let status = $state<string | null>("active");
  let client = $state<string | null>(null);
  let tags = $state(["urgent", "frontend"]);
  let hours = $state<number | null>(1.5);
  let amount = $state<number | null>(250050);
  let volume = $state(40);
  let color = $state("#3b5bdb");
  let date = $state<string | null>(null);
  let time = $state<string | null>("09:30");
  let dateTime = $state<string | null>(null);
  let from = $state<string | null>(null);
  let to = $state<string | null>(null);
  let kind = $state("company");
  let checked = $state(true);
  let toggled = $state(false);

  const options = [
    { value: "lead", label: "Lead" },
    { value: "active", label: "Active" },
    { value: "paused", label: "Paused", disabled: true },
  ];
</script>

<div data-testid="ui-kit-showcase" class="flex flex-col gap-10 p-6">
  <header class="flex items-center justify-between">
    <h1 class="text-xl font-semibold text-fg">UI kit</h1>
    <Switch
      label="Dark preview"
      bind:checked={() => dark, (v) => ((dark = v), setTheme(v ? "dark" : "light"))}
    />
  </header>

  <section class="flex flex-col gap-3">
    <h2 class="text-base font-semibold text-fg">Buttons</h2>
    <div class="flex flex-wrap gap-2">
      <Button variant="primary">Primary</Button>
      <Button>Secondary</Button>
      <Button variant="ghost">Ghost</Button>
      <Button variant="danger">Danger</Button>
      <Button size="sm">Small</Button>
      <Button disabled>Disabled</Button>
      <Button variant="primary" loading>Loading</Button>
    </div>
  </section>

  <section class="grid max-w-2xl grid-cols-2 gap-4">
    <h2 class="col-span-2 text-base font-semibold text-fg">Text fields</h2>
    <TextField label="Default" placeholder="Type here" />
    <TextField label="Required" required hint="With a hint" />
    <TextField label="With error" error="This field is required" />
    <TextField label="Disabled field" disabled value="Read me" />
    <NumberField label="Hours" bind:value={hours} min={0} step={0.5} />
    <MoneyField label="Rate" currency="RUB" bind:value={amount} />
    <div class="col-span-2"><TextArea label="Notes" /></div>
  </section>

  <section class="flex max-w-2xl flex-col gap-4">
    <h2 class="text-base font-semibold text-fg">Choices</h2>
    <div class="flex gap-6">
      <Checkbox label="Checked" bind:checked />
      <Checkbox label="Mixed" indeterminate />
      <Checkbox label="Disabled checkbox" disabled />
      <Switch label="Switch" bind:checked={toggled} />
    </div>
    <RadioGroup
      label="Client type"
      bind:value={kind}
      options={[
        { value: "person", label: "Person" },
        { value: "company", label: "Company" },
        { value: "gov", label: "Disabled option", disabled: true },
      ]}
    />
    <Slider label="Slider" bind:value={volume} step={5} />
  </section>

  <section class="grid max-w-2xl grid-cols-2 gap-4">
    <h2 class="col-span-2 text-base font-semibold text-fg">Pickers</h2>
    <Select label="Select" {options} bind:value={status} />
    <Combobox
      label="Combobox"
      options={[
        { value: "1", label: "ООО Ромашка" },
        { value: "2", label: "Acme Corp" },
      ]}
      onCreate={async (label) => ({ value: label, label })}
      bind:value={client}
    />
    <TagInput label="Tags" suggestions={["design", "backend", "urgent"]} bind:value={tags} />
    <ColorPicker label="Color" bind:value={color} />
    <div class="col-span-2">
      <FileDropZone label="Attach files" onPick={() => {}} onDrop={() => {}} />
    </div>
  </section>

  <section class="grid max-w-2xl grid-cols-2 gap-4">
    <h2 class="col-span-2 text-base font-semibold text-fg">Dates</h2>
    <DatePicker label="Date" bind:value={date} />
    <TimePicker label="Time" bind:value={time} />
    <div class="col-span-2">
      <DateTimePicker dateLabel="Start date" timeLabel="Start time" bind:value={dateTime} />
    </div>
    <div class="col-span-2">
      <DateRangePicker startLabel="From" endLabel="To" bind:start={from} bind:end={to} />
    </div>
  </section>

  <section class="flex max-w-2xl flex-col gap-4">
    <h2 class="text-base font-semibold text-fg">Overlays</h2>
    <div class="flex flex-wrap gap-2">
      <Button onclick={() => (dialogOpen = true)}>Dialog</Button>
      <Button
        onclick={() =>
          confirm({
            title: "Delete client?",
            message: "Cannot be undone.",
            danger: true,
            confirmLabel: "Delete",
          })}
      >
        Confirm
      </Button>
      <Button onclick={() => prompt({ title: "Rename", label: "Name", initial: "Todo" })}
        >Prompt</Button
      >
      <Tooltip text="Tooltip text">
        {#snippet children(trigger)}<Button {...trigger}>Tooltip</Button>{/snippet}
      </Tooltip>
      <Popover label="Popover">
        {#snippet trigger(props)}<Button {...props}>Popover</Button>{/snippet}
        <p class="text-sm">Popover content</p>
      </Popover>
      <DropdownMenu
        items={[
          { id: "edit", label: "Edit" },
          { id: "archive", label: "Archive", disabled: true },
          { separator: true },
          { id: "delete", label: "Delete", danger: true },
        ]}
        onSelect={(id) => toast.show({ message: `Selected ${id}` })}
      >
        {#snippet trigger(props)}<Button {...props}>Menu</Button>{/snippet}
      </DropdownMenu>
      <Button onclick={() => toast.show({ message: "Saved" })}>Toast</Button>
      <Button onclick={() => toast.show({ message: "Failed", kind: "error" })}>Error toast</Button>
    </div>
    <Dialog bind:open={dialogOpen} title="Dialog" description="A modal dialog.">
      <TextField label="Inside dialog" />
      {#snippet footer()}<Button variant="primary" onclick={() => (dialogOpen = false)}>Done</Button
        >{/snippet}
    </Dialog>
  </section>

  <section class="flex max-w-2xl flex-col gap-4">
    <h2 class="text-base font-semibold text-fg">Feedback</h2>
    <div class="flex items-center gap-4"><Spinner /><Progress label="Progress" value={60} /></div>
    <Progress label="Indeterminate" />
    <Tabs
      label="Tabs"
      bind:value={tab}
      tabs={[
        { id: "one", label: "One" },
        { id: "two", label: "Two" },
        { id: "three", label: "Disabled tab", disabled: true },
      ]}
    >
      {#snippet panel(id)}<p class="text-sm">Panel {id}</p>{/snippet}
    </Tabs>
    <Accordion
      items={[
        { id: "a", title: "Section A" },
        { id: "b", title: "Section B" },
      ]}
    >
      {#snippet content(id)}<p>Content {id}</p>{/snippet}
    </Accordion>
  </section>
</div>
