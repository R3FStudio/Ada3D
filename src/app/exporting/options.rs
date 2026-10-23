use super::*;

pub(super) fn image_options(ui: &mut egui::Ui, state: &mut State, doc: &Document) {
    ui.label(egui::RichText::new("Image options").strong());
    ui.horizontal(|ui| {
        let width = ui
            .add(
                egui::DragValue::new(&mut state.options.width)
                    .prefix("W ")
                    .range(1..=2048),
            )
            .changed();
        let height = ui
            .add(
                egui::DragValue::new(&mut state.options.height)
                    .prefix("H ")
                    .range(1..=2048),
            )
            .changed();
        ui.checkbox(&mut state.aspect_lock, "Keep aspect ratio");
        if state.aspect_lock {
            let ratio = doc.canvas.width as f64 / doc.canvas.height as f64;
            if width {
                state.options.height = (state.options.width as f64 / ratio)
                    .round()
                    .clamp(1.0, 2048.0) as u32;
                state.options.width = (state.options.height as f64 * ratio)
                    .round()
                    .clamp(1.0, 2048.0) as u32;
            } else if height {
                state.options.width = (state.options.height as f64 * ratio)
                    .round()
                    .clamp(1.0, 2048.0) as u32;
                state.options.height = (state.options.width as f64 / ratio)
                    .round()
                    .clamp(1.0, 2048.0) as u32;
            }
        }
    });
    if state.options.format == Format::Png {
        ui.checkbox(&mut state.options.preserve_alpha, "Keep transparent pixels");
        ui.horizontal(|ui| {
            ui.label("Compression");
            egui::ComboBox::from_id_salt("png-compression")
                .selected_text(
                    ["Fast", "Balanced", "Smallest file"][state.options.png_compression as usize],
                )
                .show_ui(ui, |ui| {
                    for (n, label) in ["Fast", "Balanced", "Smallest file"].iter().enumerate() {
                        ui.selectable_value(&mut state.options.png_compression, n as u8, *label);
                    }
                });
        });
    } else {
        ui.add(egui::Slider::new(&mut state.options.jpeg_quality, 1..=100).text("JPEG quality"));
        ui.label("JPEG replaces transparency with the chosen background.");
    }
    if state.options.format == Format::Jpeg || !state.options.preserve_alpha {
        ui.horizontal(|ui| {
            ui.label("Background");
            ui.color_edit_button_srgb(&mut state.options.background);
        });
    }
}

pub(super) fn icon_options(ui: &mut egui::Ui, options: &mut Options) {
    ui.label(egui::RichText::new("Icon sizes").strong());
    ui.horizontal_wrapped(|ui| {
        for size in [16, 24, 32, 48, 64, 128, 256] {
            let mut selected = options.icon_sizes.contains(&size);
            if ui.checkbox(&mut selected, format!("{size}")).changed() {
                if selected {
                    options.icon_sizes.push(size);
                } else {
                    options.icon_sizes.retain(|n| *n != size);
                }
            }
        }
    });
    if ui.small_button("Auto · all standard sizes").clicked() {
        options.icon_sizes = vec![16, 24, 32, 48, 64, 128, 256];
    }
    ui.label(
        egui::RichText::new("One ICO contains all selected sizes as embedded PNG images.").small(),
    );
    ui.horizontal(|ui| {
        ui.label("Square fit");
        egui::ComboBox::from_id_salt("icon-fit")
            .selected_text(match options.icon_fit {
                IconFit::Contain => "Fit with padding",
                IconFit::Cover => "Crop to fill",
                IconFit::Stretch => "Stretch",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut options.icon_fit, IconFit::Contain, "Fit with padding");
                ui.selectable_value(&mut options.icon_fit, IconFit::Cover, "Crop to fill");
                ui.selectable_value(&mut options.icon_fit, IconFit::Stretch, "Stretch");
            });
    });
    ui.checkbox(&mut options.preserve_alpha, "Keep transparent pixels");
    if !options.preserve_alpha {
        ui.horizontal(|ui| {
            ui.label("Background");
            ui.color_edit_button_srgb(&mut options.background);
        });
    }
    ui.add_space(10.0);
    egui::CollapsingHeader::new("Embed this icon in an application").show(ui,|ui| {
        ui.checkbox(&mut options.resource_file,"Create Windows .rc resource file");
        if options.resource_file {ui.horizontal(|ui|{ui.label("Resource ID");ui.add(egui::DragValue::new(&mut options.resource_id).range(1..=65535));});}
        ui.checkbox(&mut options.rust_file,"Create Rust include_bytes! snippet");
        ui.label(egui::RichText::new("Companion files reference the exported ICO. They can be included in your application's build.").small());
    });
}
