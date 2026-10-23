pub(super) struct Topic {
    pub title: &'static str,
    pub image: usize,
    pub steps: &'static [&'static str],
    pub points: &'static [[f32; 2]],
}
pub(super) const NAMES: [&str; 9] = [
    "Drawing",
    "2D shapes",
    "Transparency",
    "Selection",
    "3D shapes",
    "3D painting",
    "Save and export",
    "Settings",
    "Shortcuts",
];
pub(super) const TOPICS: [Topic; 8] = [
    Topic {
        title: "Draw and choose colors",
        image: 0,
        steps: &[
            "Choose Brush, Pencil or Marker in the right panel.",
            "Adjust thickness and opacity, then drag on the canvas.",
            "Click Choose color, or use Picker to sample a canvas pixel. The sampler in the color dialog returns to that dialog with the chosen color.",
        ],
        points: &[[0.88, 0.25], [0.88, 0.48], [0.88, 0.66]],
    },
    Topic {
        title: "Create with 2D shapes",
        image: 7,
        steps: &[
            "Open 2D shapes to choose from twelve labeled shape thumbnails.",
            "Choose a thumbnail, then drag across the canvas to set its size. Or drag the thumbnail onto the canvas to place a shape.",
            "Choose a color, stroke radius and Filled shape before drawing. Undo removes a placement; Select lets you move, resize or rotate its pixels.",
        ],
        points: &[[0.13, 0.07], [0.88, 0.39], [0.88, 0.62]],
    },
    Topic {
        title: "Paint transparency",
        image: 1,
        steps: &[
            "Switch Paint from Color to Transparency.",
            "Drag a brush stroke or choose Bucket to remove opacity from a connected area.",
            "Use Export to save a PNG or ICO with alpha. The workspace shown through transparent pixels is not part of the exported image.",
        ],
        points: &[[0.89, 0.37], [0.35, 0.50], [0.70, 0.025]],
    },
    Topic {
        title: "Select, resize and rotate",
        image: 2,
        steps: &[
            "Click the Select arrow, then drag a box around pixels in 2D or an object in 3D.",
            "Drag inside to move. Four corner handles resize both dimensions; four edge handles resize one dimension. Pixel resizing keeps the opposite side fixed.",
            "Drag the circular arrow for free rotation, or use Rotate left/right on the action bar. Done finishes the selection. Each transform is one undo step.",
        ],
        points: &[[0.40, 0.07], [0.30, 0.56], [0.28, 0.77]],
    },
    Topic {
        title: "Build a 3D scene",
        image: 3,
        steps: &[
            "Choose 3D to open the shape palette. Click a thumbnail to add it, or drag it into the scene.",
            "Choose Select and click an object. Drag its handles to move, resize or rotate it; Selected object has numeric controls.",
            "Right-drag to orbit; scroll to zoom. Shapes includes cubes, spheres, cylinders, cones, pyramids, rings, capsules and prisms.",
        ],
        points: &[[0.88, 0.35], [0.88, 0.60], [0.43, 0.50]],
    },
    Topic {
        title: "Paint 3D objects",
        image: 6,
        steps: &[
            "In 3D, choose Paint in the side panel, or click Brushes. Pick Brush, Pencil or Marker, then choose a color, thickness and opacity.",
            "Drag across an object to paint its visible surface. Right-drag to orbit and reach another side. Picker samples its original surface color; Bucket colors the whole object.",
            "Eraser and Transparency remove surface opacity. Undo reverses a stroke. Save a project to keep editable surface paint, or export the 3D scene as PNG. OBJ exports geometry only.",
        ],
        points: &[[0.79, 0.26], [0.30, 0.51], [0.87, 0.43]],
    },
    Topic {
        title: "Save and export",
        image: 4,
        steps: &[
            "Save keeps your canvas and objects in an editable .ada3dproject. Welcome lists recent projects.",
            "Click Export for a separate window. Choose a format, source, dimensions and resampling. PNG and ICO support transparency; JPEG uses a solid background.",
            "Choose the destination path and click Export. ICO can include several sizes and optional embedding helpers. Cancel stops an unfinished export.",
        ],
        points: &[[0.27, 0.14], [0.48, 0.42], [0.75, 0.92]],
    },
    Topic {
        title: "Customize Ada3D",
        image: 5,
        steps: &[
            "Choose a category on the left. Appearance controls theme, window material and accent color.",
            "Workspace controls transparency display, the grid, cursor, Welcome and reduced motion.",
            "About includes the version, R3F Studio, license and software updates. Check for updates here or when the app starts.",
        ],
        points: &[[0.10, 0.247], [0.10, 0.32], [0.10, 0.388]],
    },
];
