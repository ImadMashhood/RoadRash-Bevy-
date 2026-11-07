# Original Road Rash interface — November 6, 2026

The imported game now defaults to its original artwork, Badloc/Futura fonts,
menu geometry and motorcycle dashboard. The modern opaque menu cards and
rectangular health/boost panel are replaced in the standard presentation.

- [x] Load the original TrueType faces from `TEXT/BADLOC.DLL` and `TEXT/FUTB.DLL`
  as font data. No DLL is executed or redistributed.
- [x] Main menu: four 160×60 source slots along the bottom. Thrash, Big Game and
  Restroom work; the offline build shows Mano-à-Mano disabled.
- [x] Rider selection: eight original full-screen portraits and the source
  three-column selection grid.
- [x] Course selection: original course postcards and the five-course grid.
- [x] Restroom: original artwork, thirteen source menu entries, native captions,
  descriptions, positions, white text and yellow selection. Save controls are
  disabled without a career; career levels remain locked.
- [x] Street: original street image and the Panzer Klub / Olley's / Go Home strip.
  Racing remains reachable through the club; repair is available in the showroom.
- [x] Use original showroom/bike, credits, result, level-up and finale backgrounds.
  Additional settings, saves and controller screens retain their paginated
  controls with the original fonts and transparent menu styling.
- [x] RAT, SUPER and SPORT dashboards use the original indexed BOB artwork,
  palette, transparent mask and high-resolution EXE coordinate table.
- [x] Populate original health, condition and opponent meters, speedometer,
  tachometer, place, rider names, distance and colored opponent gap/arrow.
  Meter normalization uses base stamina. Gear ratios and sine samples come from
  the imported source tables; presentation does not modify simulation state.
- [x] Restore the large native countdown over a dimmed race view. Fresh imported
  profiles start with sound captions off; saved accessibility preferences take precedence.
- [x] Preserve high-contrast and larger-text alternatives, keyboard/controller
  navigation, pointer hitboxes and viewport/HiDPI conversion.
- [x] Keep repair, backup restore, credits and quit reachable after reorganizing
  the menu; regression-test their frontend transitions.
- [x] Preserve Bevy's shared white texture when allocating the owned dashboard
  surface; regression-test asset ownership so dimming and accessibility panels remain visible.
- [x] Verify eleven native captures, including menus, three dashboards, countdown
  dimming and high-contrast settings.
- [x] Pass workspace tests, strict Clippy and formatting.

[Machine-readable receipt](original-ui.json). Source screenshots, fonts and game
art remain private in `local/original-ui-2026-11-06/`; no commercial pixels or
fonts are added to Git. Native source data must be imported before launch.

```sh
CARGO_TARGET_DIR=../target cargo run --locked --release -p rr-game -- \
  --install-root local/game-install --catalog local/catalog --frontend on
```

This reproduces the source assets/layouts in Bevy; it is not a certification of
pixel-identical Windows/GDI font rendering. The extended settings have no exact
original counterpart. Original Windows-runtime comparison remains owner-waived.
This UI change does not close the outstanding police, interaction, campaign,
performance or release-acceptance gates.

Sources and implementation:

- [Extracted original menu records](../baseline/executable/menu-items.json):
  screens 9, 10/27, 14, 15 and 16.
- [Pinned C++ HUD layout](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/hud.cpp):
  high-resolution layout at `0x00474f50 + 2*0xdc`, indexed artwork, meters and needles.
- [PC screenshot gallery](https://www.oldgames.sk/en/gallery.php?image=15850):
  visual cross-check for native type and selection styling.
- [Native UI adapter](../../apps/game/src/frontend/original.rs),
  [menu layout](../../apps/game/src/frontend/original/menus.rs),
  [raster primitives](../../apps/game/src/frontend/original/raster.rs).
