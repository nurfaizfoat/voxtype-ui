# Use as your live Voxtype OSD

Tested with Voxtype 1.1.1, Quickshell 0.3.2 and Omarchy/Hyprland. Requires the
packaged `/usr/share/voxtype/quickshell/voxtype-shared/` modules and
`voxtype-audio-bridge` on PATH. This replaces only the recording indicator, not
the speech engine, model, language selection, output method or keyboard bindings.

Voxtype's own OSD supervisor starts and stops the UI. No additional autostart or
overlay service is needed. An installed copy runs independently of this checkout.

## Install

Run these commands from the repository root, **while Voxtype is idle**. Stop any
`bin/preview` first. This guide uses the default user config/data locations; adapt
them if you use custom XDG paths or a differently named daemon service.

1. Back up your configuration. If updating an existing installation, also back up
   its overlay directory and any existing `50-voxtype-ui.conf` before replacing
   them. Do not overwrite an unrelated service override.

   ```sh
   backup="$HOME/.local/state/voxtype-ui/backups/$(date +%Y%m%d-%H%M%S)"
   mkdir -p "$backup"
   chmod 700 "$backup"
   cp -p "$HOME/.config/voxtype/config.toml" "$backup/config.toml"
   ```

2. Install a stable copy of the QML and the service environment override:

   ```sh
   mkdir -p "$HOME/.local/share/voxtype-ui"
   cp -a overlay "$HOME/.local/share/voxtype-ui/"
   mkdir -p "$HOME/.config/systemd/user/voxtype.service.d"
   cp integration/50-voxtype-ui.conf "$HOME/.config/systemd/user/voxtype.service.d/"
   ```

3. Select the official Quickshell frontend. The Voxtype CLI preserves comments
   and unrelated config. The resolved `osd.enabled` must also be `true` (default).

   ```sh
   voxtype config set osd.frontend quickshell
   systemctl --user daemon-reload
   systemctl --user restart voxtype.service
   ```

   **This restarts the daemon once**, replacing its GTK4 OSD child with the
   official Quickshell launcher pointing at this indicator. The existing daemon
   unit and model/output settings are not replaced. The service starts the UI
   again on future sessions without requiring a terminal.

4. Check startup, then use your usual dictation shortcut:

   ```sh
   systemctl --user status voxtype.service
   journalctl --user -u voxtype.service -n 30 --no-pager
   voxtype status
   ```

   The indicator is hidden at idle. Recording uses real daemon audio; transcription
   shows the stepped sweep. `--no-osd` recordings remain suppressed. Test real
   speech in a disposable text field; the install procedure does not record or
   inject text automatically.

## Update

Stop recording first. Back up the installed overlay, then copy the new `overlay/`
files into `~/.local/share/voxtype-ui/overlay/`. Quickshell normally watches QML/JS
files and reloads them without restarting Voxtype. Verify the targeted instance:

```sh
qs log --path "$HOME/.local/share/voxtype-ui/overlay/shell.qml" --tail 12
```

Look for `Reloading configuration...` followed by `Configuration Loaded`, without
new QML errors. The listening-sensitivity update was applied this way with the
same daemon and Quickshell PIDs. If file watching is disabled or the reload does
not succeed, explicitly confirm a service restart while idle rather than assuming
the new UI is active. Keep the service override pointing at the installed
directory, not a development checkout or worktree.

## Restore the stock GTK4 OSD

For a previously default GTK4 setup:

```sh
voxtype config set osd.frontend gtk4
rm -- "$HOME/.config/systemd/user/voxtype.service.d/50-voxtype-ui.conf"
systemctl --user daemon-reload
systemctl --user restart voxtype.service
```

Remove only this project's override, not the whole service override directory.
If you had an earlier custom OSD/override, restore its saved settings instead.
The original config backup is a recovery reference; do not blindly copy it over
later unrelated edits. The installed QML can remain on disk for future reuse.
