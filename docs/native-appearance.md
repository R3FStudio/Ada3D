# Window appearance

Ada3D follows your device theme by default. Choose **Light**, **Dark** or **System** in **Settings → Appearance**.

- **Windows:** Mica is selected by default and adds a subtle wallpaper tint; Acrylic blurs desktop colors. The title bar shares the selected material and Windows draws its own controls.
- **macOS:** frosted glass uses native vibrancy and follows the selected light or dark theme. Window controls keep their standard placement.
- **Linux:** Acrylic is selected automatically. Ada3D requests blur through the Wayland background-effect protocol, with KDE blur support for older compositors and X11. Availability depends on your compositor and its effects settings.

If blur is unavailable, Ada3D uses a matching solid background. There is no solid-background setting. Desktop accessibility settings, remote sessions and graphics support may affect the result.
