<p align="center"><img src="assets/brand/mark.svg" width="80" alt="Ada3D" /></p>
<h1 align="center">Ada3D</h1>
<p align="center">Draw in 2D. Create in 3D.<br/>By R3F Studio · GPLv3</p>

![Ada3D workspace](docs/images/workspace.png)

## Download

Download the package for Windows, macOS or Linux from [**Releases**](https://github.com/R3FStudio/Ada3D/releases).

- **Windows:** extract the ZIP and run **Ada3D.exe**.
- **macOS:** extract the ZIP and move **Ada3D.app** to Applications.
- **Linux:** extract the ZIP and run **Ada3D**. See [system requirements](docs/platforms.md).

## Create

Use brushes, shapes and a canvas color sampler. Select pixels to move, resize or rotate them. Paint transparency with a brush or bucket, and export images as PNG, JPEG or multi-size ICO. Build scenes with eight 3D shapes, paint their surfaces, and export scene images or OBJ geometry.

Save editable work as **.ada3dproject**. Recent projects appear in Welcome.

| Selection tools | Settings |
|---|---|
| ![Eight-handle selection](docs/images/selection.png) | ![Settings](docs/images/settings.png) |

[3D shapes and painting](docs/3d-editing.md) · [User guide](docs/user-guide.md) · [Exporting](docs/exporting.md) · [Building and packaging](docs/platforms.md)

## Contribute

Start with the [source map](docs/source-map.md). Build with Rust 1.88 or newer:

```sh
cargo build --release --locked
```

See [Contributing](CONTRIBUTING.md) for development checks.

## License

Copyright © 2026 **R3F Studio**. Ada3D is licensed under the [GNU GPLv3](LICENSE). [Third-party notices](THIRD_PARTY_NOTICES.md) cover dependencies and fonts.
