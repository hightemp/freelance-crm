# User guide

freelance-crm keeps your clients, work, time and money in one desktop app.
Everything is stored on your computer; your data is never sent anywhere.

This guide is also in the app: **Help** in the sidebar or **F1**. Type in
its search box to find a topic.

## Getting started

1. Open **Settings** and choose the language, the default currency and the
   first day of the week.
2. Under **Profile**, enter your country, legal form and requisites: they
   appear on your invoices. Upload a logo, signature and stamp under
   **Documents** if you want them on invoices.
3. Add your first client from **Clients → New client**.

The sidebar leads to every section; the header has the search box and the
timer.

## Clients

A client is a company or a person. Its card has tabs:

- **Overview** — contact methods, the default currency and hourly rate.
- **Contacts** — people at a company, one of them primary.
- **Requisites** — legal and bank details, checked by the rules of the
  client's country (INN, KPP, IBAN, EIN and others).
- **Fields** — your own fields (text, link, e-mail, phone, date, number).
- **Notes** — Markdown notes; pinned ones stay on top.
- **Projects**, **Tasks**, **Events**, **Secrets**, **Files**, **Time**,
  **Finance** — everything linked to the client.
- **History** — calls, meetings, e-mails and messages. From an entry you can
  create a follow-up task or event.

Clients can be tagged, archived and restored. Deleting a client tells you
what goes with it; clients with invoices cannot be deleted.

## Projects and tasks

A project belongs to a client and is billed hourly or at a fixed price, in
its own currency and rate.

Tasks can belong to a project, a client or neither (the inbox). Each has a
status, priority, deadline (a date, or a date and time), an estimate,
checklist items, tags, notes and files. Show them as a list with filters or
as a **board**: drag cards between columns. The board's columns can be
renamed, added, reordered and given work-in-progress limits in the column
menu.

**Ctrl+N** (Cmd+N on macOS) from anywhere adds a task to the inbox.

## Calendar

Month, week, day and agenda views show events and, as layers you can switch
off, task deadlines, project deadlines and invoice due dates.

- Click a free slot or a day to create an event there. Events repeat daily,
  weekly, monthly or yearly, in a time zone
  of your choice. Editing a repeated event asks whether to change this
  occurrence, the following ones or all of them.
- **Reminders** come as system notifications, also when the window is
  closed (the app keeps running in the tray). Reminders missed while the app
  was not running are shown at the next start.
- **Import** and **Export** read and write `.ics` files, for exchanging
  events with other calendars.

## Time

Start the timer in the header (or the tray menu) on a project or a task, or
add entries by hand on the **Time** page. The journal shows entries by day;
the report groups them by day, project or task with the amounts at the
project's or client's rate. Settings can round entries up (to 5, 15, 30
minutes and so on) for reports and invoices. The report exports to CSV.

## Finance

The **Finance** page has three tabs.

- **Overview** — for a period and a currency: income, expenses, profit,
  unpaid and overdue amounts, income by month and by client, and open
  invoices. Currencies are never added together.
- **Invoices** — create an invoice by hand, or **From time**: pick a client
  and a period to bill the unbilled time as a draft. Invoices are numbered
  automatically by the template in Settings unless you type a number. Save
  or preview the **PDF** (Russian or international layout, in the invoice's
  language). Record **payments**, in full or in parts; the status follows
  (sent, partially paid, paid, overdue). A paid or cancelled invoice can no
  longer be changed. An invoice with a due date can have a reminder.
- **Expenses** — amounts by category, optionally for a project, with
  receipts attached.

**Taxes** are off by default. Turn them on in Settings and choose a regime
(NPD, USN "Income" or your own rules): the overview then shows the
estimated tax and the income after it. This is an estimate, not
accounting.

## Vault

The vault keeps passwords, API keys, SSH keys, server and bank card details
encrypted with your **master password**.

- When you create the vault you get a **recovery key**. Write it down: it
  is the only way back if you forget the master password.
- The vault locks after a period of inactivity (Settings → Security), from
  the lock button or from the tray menu. Locked, secrets show their titles
  only.
- Reveal or copy a value with the buttons next to it; the password generator
  makes passwords and passphrases; SSH keys can be generated, imported and
  exported.
- A secret with an expiry date (a domain, a certificate) shows it in the
  calendar.

## Search

**Ctrl+K** (Cmd+K), **Ctrl+F** or the **Search** button opens one box for
everything: type to find clients, contacts, projects, tasks, events, notes,
history, fields, file names, invoices and secret titles (never their
values). The same box runs commands: new task, client or project, start or
stop the timer, lock the vault, go to a page. Arrows move, Enter opens.

The **Home** screen shows overdue tasks and today's, the week's events,
unpaid invoices, the running timer and the clients you worked with lately.

## Backups

Settings → **Backups**:

- **Create backup** writes everything (data and files) into one encrypted
  `.fcrm-backup` file, with a password you choose. Keep the password: the
  backup cannot be opened without it.
- **Restore from backup** replaces all current data with the backup's. Your
  current data is backed up first, into the `backups` folder of the data
  folder, with the same password.
- **Automatic backups** run daily or weekly into a folder you choose and
  keep the newest few. They are protected either by the vault (opened with
  the master password, also on another computer) or by a password kept in
  the system keychain.
- **Export data** writes your data as JSON and CSV for other programs.
  Secrets are not exported.

## Settings

Language, theme (light, dark or as the system), interface scale
(**Ctrl+plus/minus/0** also work), currency, first day of the week, time
rounding, the invoice number template, the vault's auto-lock, taxes,
documents, backups, and System: whether closing the window keeps the app
running in the tray, and whether it starts with the system.

**Updates.** At startup the app checks GitHub for a new version (turn this
off in Settings → Updates, or check by hand there). When there is one, an
**Update to …** button appears at the top: it shows what changed and
**Install and restart** downloads the update, checks its signature, installs
it and restarts. Your data stays as it is and is migrated if needed.

## Where the data is

The database and files are in the app's data folder:

- Linux: `~/.local/share/com.hightemp.freelance-crm`
- Windows: `%APPDATA%\com.hightemp.freelance-crm`
- macOS: `~/Library/Application Support/com.hightemp.freelance-crm`

See [security.md](security.md) for how it is protected.
