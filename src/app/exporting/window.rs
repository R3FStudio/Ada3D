//! Export form layout and always-visible progress/footer actions.
use super::options::{icon_options, image_options};
use super::*;

pub(super) fn draw(ctx: &egui::Context, state: &mut State, doc: &Arc<Document>, camera: &Camera) {
    let colors = appearance::palette(ctx);
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(colors.panel).inner_margin(22))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(26.0), Sense::hover());
                icons::draw(ui.painter(), Icon::Export, rect, ui.visuals().text_color());
                ui.heading("Export artwork");
            });
            ui.label(
                egui::RichText::new("Choose a format, tune the output, then save a copy.")
                    .color(colors.muted),
            );
            ui.add_space(14.0);
            let footer_height = if state.job.is_some() {
                145.0
            } else if !state.outputs.is_empty() {
                130.0
            } else {
                65.0
            };
            egui::ScrollArea::vertical()
                .max_height((ui.available_height() - footer_height).max(180.0))
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.add_enabled_ui(state.job.is_none(), |ui| form(ui, state, doc));
                });
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(10.0);
            footer(ui, ctx, state, doc, camera);
        });
}

fn form(ui: &mut egui::Ui, state: &mut State, doc: &Document) {
    let old_format = state.options.format;
    egui::Grid::new("export-basics")
        .num_columns(2)
        .spacing([18.0, 12.0])
        .show(ui, |ui| {
            ui.label("Format");
            egui::ComboBox::from_id_salt("format")
                .selected_text(state.options.format.label())
                .width(250.0)
                .show_ui(ui, |ui| {
                    for format in [Format::Png, Format::Jpeg, Format::Ico, Format::Obj] {
                        ui.selectable_value(&mut state.options.format, format, format.label());
                    }
                });
            ui.end_row();
            if state.options.format != Format::Obj {
                ui.label("Artwork");
                egui::ComboBox::from_id_salt("source")
                    .selected_text(if state.options.source == Source::Canvas {
                        "2D canvas"
                    } else {
                        "Canvas + 3D scene"
                    })
                    .width(250.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut state.options.source, Source::Canvas, "2D canvas");
                        ui.selectable_value(
                            &mut state.options.source,
                            Source::Scene,
                            "Canvas + 3D scene",
                        );
                    });
                ui.end_row();
                ui.label("Resampling");
                egui::ComboBox::from_id_salt("sampling")
                    .selected_text(state.options.sampling.label())
                    .width(250.0)
                    .show_ui(ui, |ui| {
                        for sampling in [Sampling::Nearest, Sampling::Triangle, Sampling::Lanczos] {
                            ui.selectable_value(
                                &mut state.options.sampling,
                                sampling,
                                sampling.label(),
                            );
                        }
                    });
                ui.end_row();
            }
        });
    if old_format != state.options.format {
        state.path.set_extension(state.options.format.extension());
        state.path_text = state.path.to_string_lossy().into_owned();
        state.overwrite_pending = false;
        ui.ctx().request_repaint_of(egui::ViewportId::ROOT);
    }
    ui.add_space(14.0);
    ui.separator();
    ui.add_space(10.0);
    match state.options.format {
        Format::Ico => icon_options(ui, &mut state.options),
        Format::Obj => {
            ui.label(format!(
                "{} editable objects · world-space mesh and normals",
                doc.objects.len()
            ));
            ui.label("OBJ exports geometry. Keep the project file for editable objects and canvas colors.");
        }
        _ => image_options(ui, state, doc),
    }
    ui.add_space(14.0);
    ui.separator();
    ui.add_space(12.0);
    ui.label(egui::RichText::new("Destination").strong());
    ui.horizontal(|ui| {
        if ui
            .add_sized(
                [ui.available_width() - 82.0, 28.0],
                egui::TextEdit::singleline(&mut state.path_text),
            )
            .changed()
        {
            state.path = PathBuf::from(&state.path_text);
            state.overwrite_pending = false;
        }
        if ui.button("Browse…").clicked()
            && let Some(path) = rfd::FileDialog::new()
                .add_filter(
                    state.options.format.label(),
                    &[state.options.format.extension()],
                )
                .set_file_name(state.path.file_name().unwrap_or_default().to_string_lossy())
                .save_file()
        {
            state.path = path;
            state.path.set_extension(state.options.format.extension());
            state.path_text = state.path.to_string_lossy().into_owned();
            state.overwrite_pending = false;
        }
    });
    let muted = appearance::palette(ui.ctx()).muted;
    for path in export::output_paths(&state.path, &state.options)
        .iter()
        .skip(1)
    {
        ui.label(
            egui::RichText::new(format!("Also writes {}", path.display()))
                .small()
                .color(muted),
        );
    }
    ui.add_space(8.0);
    ui.label(
        egui::RichText::new("Exports use the artwork snapshot captured when this window opened.")
            .small()
            .color(muted),
    );
}

fn footer(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    state: &mut State,
    doc: &Arc<Document>,
    camera: &Camera,
) {
    if let Some(job) = &state.job {
        jobs::progress_ui(ui, &job.progress());
        if ui.button("Cancel export").clicked() {
            job.cancel();
        }
        return;
    }
    if !state.message.is_empty() {
        ui.label(&state.message);
    }
    for path in &state.outputs {
        ui.add(egui::Label::new(egui::RichText::new(path.to_string_lossy()).small()).truncate())
            .on_hover_text(path.display().to_string());
    }
    if state.overwrite_pending {
        ui.label(
            egui::RichText::new("Existing destination files will be replaced.")
                .color(Color32::from_rgb(235, 170, 77)),
        );
    }
    if icons::primary_action(
        ui,
        Icon::Export,
        if state.overwrite_pending {
            "Replace & export"
        } else {
            "Export"
        },
    )
    .clicked()
    {
        state.path.set_extension(state.options.format.extension());
        state.path_text = state.path.to_string_lossy().into_owned();
        if !state.overwrite_pending
            && export::output_paths(&state.path, &state.options)
                .iter()
                .any(|p| p.exists())
        {
            state.overwrite_pending = true;
            return;
        }
        state.overwrite_pending = false;
        state.message.clear();
        state.outputs.clear();
        if let Err(error) = state.options.validate() {
            state.message = error;
            return;
        }
        let path = state.path.clone();
        let options = state.options.clone();
        let doc = doc.clone();
        let camera = camera.clone();
        match jobs::Job::start(ctx, viewport(), move |report| {
            export::run(&path, &doc, &camera, &options, report)
        }) {
            Ok(job) => state.job = Some(job),
            Err(error) => state.message = error,
        }
    }
}
