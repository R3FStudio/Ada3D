# Tool behavior

This reference describes what each tool does. For step-by-step instructions, see the [user guide](user-guide.md).

| Tool | Interaction | Result |
|---|---|---|
| Brush / Pencil / Marker | Drag on the canvas | A continuous stroke using the selected color, width and opacity |
| Eraser | Drag | Remove pixel opacity |
| Bucket | Click | Fill a four-connected region with matching RGBA pixels |
| Picker | Click a canvas pixel | Copy its RGBA value without editing artwork |
| Color-dialog sampler | Click the sampler icon, then a canvas pixel | Close the dialog, sample, restore the previous tool and reopen the dialog |
| Line / Rectangle / Ellipse | Drag, release | Preview a shape and commit it as one undo step |
| Select | Drag a marquee | Select pixels in 2D or a visible object in 3D |
| Transform handles | Drag inside, on any of four corners/four edge midpoints, or on the rotation handle | Move, resize or rotate; pixel resizing keeps the opposite anchor fixed |

## Color and transparency

Brush and Bucket have Color and Transparency targets. Transparency removes alpha without adding workspace pixels to the document. PNG export preserves the resulting alpha. Workspace and checkerboard displays are optional views behind transparent pixels.

The color window opens in the center and fits its controls. Escape during sampling returns to the dialog with the previous color. Sampling an object in 3D reads its unlit surface color. Empty areas use the rendered background; picking waits for a current preview.

## Documents and history

A completed stroke, fill, shape or transform gesture produces one undo transaction. Display changes do not enter history. New, Open and Close ask before replacing unsaved work. Save preserves editable pixels and scene parameters. PNG exports an image; OBJ exports geometry and normals.

Save and Open use .ada3dproject exclusively. Project I/O rejects other file extensions before reading or writing artwork.

Welcome lists the twelve most recently saved or opened projects. A project opens through a background worker that counts bytes read and decoded before validating the document. Failed and cancelled loads retain the current artwork.

Canvas dimensions are limited to 2048 pixels per side. Resizing keeps the top-left overlap and adds transparent pixels. Imports support PNG and JPEG within the same bounds.

## Navigation

Scroll zooms; middle-drag pans; right-drag orbits in 3D. Canvas right-click menus expose selection, color picking, undo/redo, Fit and export. While a popup menu is open, editor controls, canvas gestures and editor shortcuts are blocked. Clicking outside or pressing Escape dismisses the popup without acting on the editor underneath; choosing a menu item runs that action. Selection outlines and all eight handles remain visible. A separate action bar beside the selection offers quarter turns and Done while allowing handle interaction. Settings categories sit on the left, and the back arrow returns to editing. Help opens in its own window.

Project, export and context-menu actions have icons and labels. Import and export are available directly in the title bar. At larger interface scales, title-bar actions use icons with tooltips and the zoom slider moves to the status bar.

Export opens a native window with destination, format and encoding options. PNG, JPEG, multi-size ICO and OBJ are supported. Progress counts completed render work, icon frames and bytes written. The native editor caption controls remain platform-managed. See [export options](exporting.md).

## Updates

Checks run once at startup or from Settings → About. Stable releases are compared by semantic version and matched to the current OS and architecture. Downloads report received bytes. Installation follows checksum, archive-path and executable validation, then waits for the app to exit. Manual restart requests use the normal unsaved-work confirmation.

## 3D surface editing

The 3D sidebar opens Shapes by default, with eight thumbnail buttons. Click adds a centered primitive; dragging into the scene places it on the camera-facing plane through the origin. Each placement is undoable.

Paint uses the visible mesh’s perspective-correct surface coordinates. Brush, Pencil, Marker, Eraser, Bucket, transparency and surface sampling operate on per-object paint. Orbiting does not alter paint coordinates. A brush gesture commits one history transaction and does not change canvas pixels. Bucket affects the clicked object’s entire surface.

Surface maps are 256 × 256 pixels with an 8 MiB project paint budget. Their memory counts toward history limits. Projects without paint maps retain their original colors. Project saves and scene image exports retain paint; OBJ exports geometry only.

### 2D shape palette

The 2D Shapes tab offers twelve labeled thumbnails in three columns. Clicking selects the drawing tool without changing artwork; dragging a thumbnail onto the canvas places one shape at the drop position. Canvas drags preview a shape from the original pixels and commit one undo step. Shapes support filled or outlined rendering, color, alpha and stroke width. The Brushes tab retains its painting tools.
