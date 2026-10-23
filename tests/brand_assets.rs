use std::{fs, path::Path};

fn check_mark(bytes: &[u8], size: u32) -> image::RgbaImage {
    let image = image::load_from_memory(bytes).unwrap().into_rgba8();
    assert_eq!(image.dimensions(), (size, size));
    for (x, y) in [(size / 8, size / 8), (size * 7 / 8, size * 7 / 8)] {
        assert_eq!(image.get_pixel(x, y)[3], 0, "background at {size}px");
    }
    assert!(image.get_pixel(size / 2, size / 2)[3] >= 240);
    assert!(image.pixels().any(|pixel| pixel[3] == 255));
    assert!(image.pixels().any(|pixel| pixel[3] > 0 && pixel[3] < 255));
    image
}

#[test]
fn platform_icons_share_the_transparent_mark() {
    let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    let png = fs::read(assets.join("brand/app-icon.png")).unwrap();
    let canonical = check_mark(&png, 256);
    let ico = fs::read(assets.join("brand/app.ico")).unwrap();
    assert_eq!(&ico[..6], &[0, 0, 1, 0, 7, 0]);
    for (index, size) in [16, 24, 32, 48, 64, 128, 256].into_iter().enumerate() {
        let entry = &ico[6 + index * 16..6 + (index + 1) * 16];
        let length = u32::from_le_bytes(entry[8..12].try_into().unwrap()) as usize;
        let offset = u32::from_le_bytes(entry[12..16].try_into().unwrap()) as usize;
        let frame = check_mark(&ico[offset..offset + length], size);
        if size == 256 {
            assert_eq!(frame, canonical);
        }
    }
    for size in [16, 32, 128, 256, 512] {
        for (suffix, scale) in [("", 1), ("@2x", 2)] {
            let path = assets.join(format!(
                "macos/Ada3D.iconset/icon_{size}x{size}{suffix}.png"
            ));
            let frame = check_mark(&fs::read(path).unwrap(), size * scale);
            if size * scale == 256 {
                assert_eq!(frame, canonical);
            }
        }
    }
    let svg = fs::read_to_string(assets.join("brand/mark.svg")).unwrap();
    assert!(!svg.contains("<rect"));
}
