# Manual QA checklist: macOS

End-to-end tests run on macOS too (WebdriverIO with an embedded driver).
This list covers only what they cannot: signing, notarization and the
system's own dialogs and permissions. Go through it for each release
candidate, on Apple Silicon and on Intel, with a fresh user account where
noted.

Record the build (version, commit, architecture) and the macOS version.

## Installation and Gatekeeper

- [ ] The `.dmg` opens, shows the app and the Applications shortcut; dragging
      the app there copies it.
- [ ] `spctl -a -vv /Applications/freelance-crm.app` says `accepted` and
      `source=Notarized Developer ID` (signed builds only).
- [ ] `codesign --verify --deep --strict --verbose=2 /Applications/freelance-crm.app`
      reports no errors.
- [ ] The first launch from Finder opens without the "cannot be opened" or
      "damaged" warning (signed and notarized builds). An unsigned build
      shows the warning and opens with right click → Open.
- [ ] The app icon is sharp in the Dock, Finder, the app switcher and
      Launchpad, in light and dark appearance.

## Window, menu bar and tray

- [ ] The window opens at its last size and position; full screen and
      split view work.
- [ ] The menu bar icon (tray) appears; its menu opens, shows the running
      timer, stops it, locks the vault and quits.
- [ ] With "Keep running in the tray when closed" on, closing the window
      leaves the app running; clicking the menu bar icon brings it back.
      With it off, closing quits.
- [ ] Cmd+Q quits; launching the app again while it runs focuses the
      existing window instead of starting a second one.

## Keyboard

- [ ] Cmd+K and Cmd+F open the search, Cmd+N the quick task, Cmd+plus,
      Cmd+minus and Cmd+0 change the interface scale.
- [ ] Cmd+C, Cmd+V, Cmd+X, Cmd+A and Cmd+Z work in text fields, and the
      right-click menu in a text field offers cut, copy and paste.

## System dialogs and permissions

- [ ] Open dialogs (attach files, restore a backup, import `.ics`, choose
      the backup folder) and save dialogs (invoice PDF, backup, CSV, `.ics`)
      are native and remember the last folder.
- [ ] The first reminder asks for permission to send notifications;
      allowed, reminders appear in Notification Center, also with the
      window closed. Denied, the app keeps working.
- [ ] Saving the automatic backup password asks for access to the
      Keychain at most once; "Always Allow" sticks across restarts. The
      item is visible in Keychain Access as `freelance-crm`.
- [ ] "Start with the system" adds the app to System Settings → General →
      Login Items; after logging out and in, the app starts (in the tray
      when closing to it is on). Turning it off removes the item.
- [ ] Opening a file or a link from the app uses the default app or browser.
- [ ] Opening an encrypted file shows it, and the decrypted copy disappears
      after locking the vault.

## Data

- [ ] A fresh install creates
      `~/Library/Application Support/com.hightemp.freelance-crm`.
- [ ] An update (installing a newer build over the old one) keeps the data
      and migrates it; the vault unlocks with the same master password.
- [ ] The previous release offers this one (**Update to …** in the header);
      **Install and restart** replaces the app in Applications, restarts it
      and keeps the data.
- [ ] A backup made on macOS restores on Linux or Windows, and the other
      way round (vault-locked backups with the master password).

## Appearance

- [ ] Light, dark and "as the system" themes follow the system appearance
      change while the app runs.
- [ ] Text is crisp on Retina and non-Retina displays; Cyrillic text and
      invoice PDFs render correctly.
