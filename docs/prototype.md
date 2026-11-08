# Overlay behavior and tests

For the official daemon-supervised live installation, see
[install and rollback](install.md). The commands below remain standalone previews.

## Run

Requires Hyprland, Quickshell (tested with Omarchy's 0.3.2), and Voxtype 1.1.1
with its packaged QML modules and `voxtype-audio-bridge`.

```sh
bin/preview --demo  # Synthetic audio, follows focus, no recording or typing.
bin/preview         # Reads the running daemon's state and live audio levels.
```

Both commands run in the foreground. **Ctrl+C stops the preview.** It does not
install anything, change settings, restart services, or replace your current OSD.
In live mode the existing OSD will also appear; this is intentional until we
explicitly switch over. Do not launch both preview modes simultaneously.

Demo cycles through seven seconds of simulated recording and three seconds of
transcribing. Click another app to see the indicator follow it. The surface is
click-through, takes no keyboard focus, and reserves no desktop space.

## Behavior

- Uses the current focused Hyprland client, not the window at recording start.
- Shows a square-corner #0D0D0D indicator whose box is derived from a perfect grid,
  not an independently fixed width/height. At 125% it is 136 × 36 logical pixels;
  at 100% it is 141 × 36. Sizing prioritizes identical squares and gaps over the
  previous 133 × 30 target. Centered twelve logical pixels
  above the focused window's bottom. Border thickness matches Hyprland's current
  `general:border_size` in logical pixels, independent of UI size; rendered border
  thickness snaps to physical pixels too. No window outline, rounded dots,
  gradients, glow, shadows, blur, fonts, or external images.
- The 27 × 6 grid scales from the original 80 × 17 layout. Every dot has one
  integer physical-pixel side used for both width and height, integer physical
  positions, and IDENTICAL pitch on both axes. Dot side and gap are selected first;
  the box expands around them. Dots are never squeezed to accommodate the frame.
  A full black gap separates the grid from the border, too.
- The full grid is drawn in one `Canvas.Image` buffer with smoothing disabled,
  cancelling Canvas's DPR transform before painting integer rectangles. Merely
  setting QML Rectangle width equal to height was insufficient at fractional DPR:
  separate rasterized edges could gain a row/column and erase the gaps. At 125%
  the default uses consistent 4 × 4 physical-pixel dots with 2px separators on both
  axes. Outer dimensions are multiples of the display-scale numerator, so both
  logical Canvas dimensions and physical buffer dimensions are whole pixels.
- The production shell explicitly supplies `PanelWindow.devicePixelRatio`, NOT
  `Screen.devicePixelRatio`. On this Wayland session the screen reports 2 while
  the window actually renders at 1.25. Using the screen value caused the remaining
  irregularities even though offscreen physical-buffer tests had passed.
- Dots and border use Omarchy's current theme accent. Fixed inactive opacity is
  20%; active levels are 40/60/80/100%, from the horizontal/vertical minimum.
- Reference capacities are
  `[1,1,1,2,2,3,3,4,4,5,5,6,6,6,6,6,5,5,4,4,3,3,2,2,1,1,1]`.
  Centered stacks start at `round((6-count)/2)`. The reference has 92 active
  dots: 10 at 100%, 30 at 80%, 38 at 60%, and 14 at 40%.
- Speech changes stack counts at 80ms intervals (12.5 fps), with no interpolation.
  Mirrored, bounded noise keeps the hill symmetric and never exceeds capacity.
  Demo uses layered sine waves and deterministic noise; live uses existing audio.
- During transcription, a stepped pulse sweeps across the reference silhouette.
- Hides at idle, missing/hidden windows, and when Voxtype requests `--no-osd`.
- Seeds initial focus with a read-only `hyprctl` query, then follows native focus
  events (including losing focus). Never highlights Omarchy's screensaver.
- Does not read or store transcripts, record audio itself, or invoke output tools.
- Geometry uses logical pixels, avoiding double-scaling on fractional displays.
  Each monitor has its own clipped, transparent layer surface. A spanning window
  retains one indicator at its original bottom center, clipped at monitor seams.
- Window geometry refreshes every 80 ms only while active. Theme colors are
  watched live; a two-second refresh also handles replaced theme symlinks and
  updates compositor border thickness. Neither reads nor refreshes modify desktop
  configuration. Only the black fill and square corners remain fixed.
- Missing/stale audio frames decay to one active dot per column, not synthetic
  speech. Live mode never runs the simulated envelope.
- The visual reflects peak/RMS amplitude, **not** a raw microphone waveform.
- Windows smaller than the grid-fitted box plus vertical insets hide it rather than
  shrinking or overflowing it. Fullscreen windows use the same bottom inset.

## Reusable visual

`overlay/DotIndicator.qml` is pure QtQuick and can render independently. Defaults
simulate listening activity; set `simulate: false` and supply `level` (0–1) for
live input. `animate: false` displays the reference silhouette. `sizeMultiplier`
defaults to 1.56 as a target dot/gap scale, not an exact multiplier for the box.
`accent` and
`borderThickness` are supplied by the desktop adapter; `pixelScale` is an optional
integer inspection scale (default 1). `devicePixelRatio` defaults to 1 and MUST be
supplied by the embedding window for HiDPI. Do not substitute the screen's DPR.
Canvas drawing uses native pixels, not a fractionally stretched grid; the window
wrapper also snaps global placement, including the client offset.
For the original 2px dots / 1px gaps / green taper at 100% scaling, set
`sizeMultiplier: 1`, `borderThickness: 1`, and `accent: "#267814"`. Its new box is
84 × 21, allowing a black gap between the original 80 × 17 grid and the frame.
`daemonState: "idle"` holds one dot per column; `"transcribing"` runs the sweep.
The full desktop overlay still hides at idle.

## Tests

```sh
bin/test
```

Runs Qt Quick tests offscreen with fake data. Covers fractional scaling, negative
monitor origins, spanning/portrait/fullscreen geometry, invisible/invalid clients,
bounded sample history, audio clamping, exact reference opacity counts, symmetric
simulation bounds, timer lifecycle, live accent/border property propagation,
square geometry and spacing, and every rendered pixel at 1×/2×/3× for both the
original design and the current enlarged version in three example theme accents.
The runner also builds `tests/physical_pixels.cpp` (requires a C++ compiler,
pkg-config and Qt 6 Quick/QML/Gui development headers) and captures the full native
framebuffer rather than a logical-coordinate crop. It checks all 162 solid square
footprints and black separators at DPR 1, 1.25, 1.5, 1.75 and 2, across three window
offsets and three states. `bin/test-pixels` runs just these checks. The 125% test
also passed with the OpenGL/RHI backend. All fixtures contain only fake content.
For the final compositor stage, run `bin/test-compositor-pixels` inside Hyprland
(requires grim and ImageMagick). It briefly creates a tiny opaque synthetic grid
in a transparent, no-focus/click-through panel, captures ONLY that grid, checks
every physical pixel (dots, separators, padding and border), and stops only its
own probe. It leaves the user's preview/daemon alone. This catches screen/window
DPR mismatches that offscreen tests cannot reproduce. The real 125% compositor
check passes all 162 4 × 4 dots and equal 2px gaps. See the
[native synthetic capture](indicator-native.png).
Diagnostic captures go to `build/preview-*.png` and
`build/indicator-reference-*x.png` / `build/indicator-themed-*x.png` and
`build/physical-dpr-*.png` and `build/compositor-dots.png`; all contain only
synthetic visuals.

For a read-only integration check against the current session (no windows or
recordings), run `qs --path overlay/IntegrationCheck.qml`. It reports state,
initial focus availability, monitor count, accent, border thickness and rounding,
then exits.
Both demo and live frontends have also passed startup smoke checks; real focus
switching, drag/resize behavior, and multiple monitors still need hands-on testing.

## Limitations and next checks

- Preview mode is independent. Installed mode uses Voxtype's own OSD supervisor;
  it requires no separate overlay autostart/service.
- Geometry polling can trail compositor move/resize animations. Real multi-monitor
  hotplug, mixed-scale spanning, and rotated outputs still need desktop testing.
- Popups are not individually tracked. Secure/lock surfaces and exact text fields
  are not dictation targets this UI can identify.
- This is a focus indicator, not a delivery guarantee. Focus may change while
  Voxtype is typing, and this overlay cannot prove that insertion succeeded.
  Therefore no green "success" pulse is shown merely on returning to idle.
- The editor remains a separate planned Qt Quick/C++ app. Its eventual Save flow
  must preserve comments/unrelated settings, validate, back up, write atomically,
  and offer an explicit restart rather than silently restarting dictation.
