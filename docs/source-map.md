# Finding your way around the source

Use this map to find the code for a feature. Document algorithms are separate from the interface.

| Directory | Responsibility |
|---|---|
| src/app | Editor state and user workflows |
| src/ui | Shared styling, icons, Settings and illustrated Help |
| src/core | Pixels, documents, history, scenes and file formats |
| src/updater | Release checks, verified downloads and installation |
| src/platform | Operating-system window integration |
| tests | Cross-module workflow tests |
| assets | App artwork, icons and guide illustrations |
| tools | Icon regeneration and desktop installation/bundling |

## Editor workflows

| File | What to change here |
|---|---|
| app/mod.rs | State, construction and frame orchestration |
| app/canvas.rs | Canvas input and painting gestures |
| app/shape_tools.rs | 2D thumbnails and drag placement |
| core/raster/shapes.rs | Polygon outlines and signed edge distance |
| app/scene_tools.rs | 3D shape palette, drag placement and surface painting |
| core/scene/paint.rs | Object-space paint coordinates and surface storage |
| core/scene/shapes.rs | Additional primitive meshes |
| app/selection.rs | Marquee and transform handles |
| app/selection/actions.rs | Floating selection actions and quarter-turn commands |
| app/dialogs.rs | Color dialog, sampler lifecycle and confirmations |
| app/toolbar.rs | Title-bar commands and tool strip |
| app/inspector.rs | Brush, palette, canvas and object controls |
| app/document.rs | Save/open/import/export, history and shortcuts |
| app/startup.rs | Splash, welcome and window appearance |
| app/menus.rs | Project and export menu actions |
| app/projects.rs | Recent projects, background loading and loading progress |
| app/jobs.rs | Bounded file workers, cancellation and progress display |
| app/exporting/mod.rs | Native export window state and worker lifecycle |
| app/exporting/window.rs | Export form, destination and progress footer |
| app/exporting/options.rs | Image encoding and icon-size controls |
| core/export.rs | PNG/JPEG/ICO export and embedding files |
| core/progress.rs | Measured work counters shared by file workflows |
| platform/linux_backdrop.rs | Wayland and X11 blur negotiation |
| platform/macos_backdrop.rs | Native vibrancy and window appearance |
| ui/help/topics.rs | Help instructions and screenshot annotations |
| updater/network.rs | GitHub metadata and download progress |
| updater/archive.rs | Package validation and extraction |
| updater/install.rs | Platform restart helpers |
| updater/packages.rs | Installed package detection and native installer selection |
| tools/installers.py | Native package builders and wizard artwork |
| tools/windows-setup.iss | Windows setup wizard |
| ui/startup.rs | Logo splash, progress presentation and startup fades |
| app/preferences.rs | Preference changes and persistence |
| app/render.rs | Preview-worker requests and texture updates |
| app/capture.rs | Developer screenshot and smoke fixtures |
| app/tests.rs | Editor workflow regressions |

## Drawing and scenes

core/raster.rs stores straight-alpha RGBA pixels and implements strokes, fill and shapes. core/selection.rs samples original selected pixels when transforming them. core/history.rs groups gestures into undo transactions.

core/scene/mod.rs defines scene objects, transforms and cameras. geometry.rs generates meshes; renderer.rs handles projection, clipping, shading and picking; export.rs writes OBJ files. Scene tests sit beside those modules and in tests/scene_workflows.rs.

## Making a change

Settings pages live in ui/settings.rs. Their navigation, preview cards and switches are in ui/settings/controls.rs. Icon-only toolbar buttons attach their descriptive tooltip at the call site; the shared icon control omits empty hover labels.

Start in the file for the workflow you are changing. Keep display state out of saved artwork, validate file and geometry inputs, and preserve one undo transaction per gesture. Run formatting, Clippy and the relevant workflow tests before building a release. New behavior should have a regression test when it affects artwork or files.

[Architecture](architecture.md) Â· [Build instructions](platforms.md) Â· [Tool behavior](functional-spec.md)
