# 3D shapes and painting

## Add shapes

Choose **3D** or **3D shapes** to open the shape palette on the right. Click a thumbnail to add it at the center of the scene. Drag a thumbnail onto the workspace to place it there instead.

![Shape palette](images/scene.png)

The palette contains Cube, Sphere, Cylinder, Cone, Pyramid, Ring, Capsule and Prism. Expand **Shape color** to choose a color for new shapes.

## Arrange your scene

Choose **Select** and click an object. Drag inside its selection to move it, use the corner or edge handles to resize it, or drag the circular handle to rotate. **Selected object** contains position, rotation, scale and delete controls. **Objects** lists the shapes in the scene.

Right-drag to orbit the camera. Scroll to zoom, middle-drag to pan, or choose **View → Reset view** to return to the starting viewpoint.

## Paint surfaces

Choose **Paint** above the shape palette, or click **Brushes** while in 3D. This keeps you in the scene and opens the painting controls.

![3D surface painting](images/scene-paint.png)

| Tool | Use |
|---|---|
| Brush, Pencil, Marker | Drag across a surface. Set thickness, opacity and color in the right panel. |
| Bucket | Color the whole object you click. |
| Picker | Sample a surface color without its lighting shade. |
| Eraser | Remove opacity from the area you brush. |
| Transparency | Use a brush or Bucket to remove opacity; Removal controls the amount. |

Painting affects visible surfaces. Orbit to paint the back or sides. Paint stays attached when the object moves, scales or rotates. A stroke is one undo step. The 2D canvas is separate.

## Save and export

**Save** preserves editable objects and surface paint in a **.ada3dproject** file. Older projects open with their original object colors.

For an image, choose **Export → Artwork: 3D scene**. PNG and ICO preserve transparency. OBJ contains the mesh geometry; save a project alongside it to retain editable surface paint.
