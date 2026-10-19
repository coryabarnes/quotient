//! The desktop app: an editor with each result beside the line that produced it.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui::{
    self, Color32, FontId, Galley, Key, KeyboardShortcut, Modifiers, Rect, RichText, ScrollArea, Sense, TextBuffer,
    TextEdit, ThemePreference, Vec2, pos2, vec2,
};
use egui::text::{CCursor, CCursorRange, LayoutJob, TextFormat, TextWrapping};
use egui::text_edit::TextEditState;
use quotient::highlight::{self, Class, Completion};
use quotient::{Document, Kind, Settings, WELCOME, calculate, catalog, units};

const AUTOSAVE_AFTER: Duration = Duration::from_secs(2);
const STATUS_FOR: Duration = Duration::from_secs(4);
const FONT_SIZE: f32 = 15.0;
/// Space between the results column's edges and its text.
const RESULT_PADDING: f32 = 14.0;
const AUTHOR: &str = env!("CARGO_PKG_AUTHORS");
const COPYRIGHT_YEAR: &str = "2026";
const LICENSE: &str = include_str!("../LICENSE");
const ICON: &[u8] = include_bytes!("../assets/quotient.png");

fn editor_id() -> egui::Id {
    egui::Id::unique("quotient-editor")
}

fn font() -> FontId {
    FontId::monospace(FONT_SIZE)
}

pub fn run(path: Option<PathBuf>) -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Quotient")
            .with_app_id("quotient")
            .with_icon(
                eframe::icon_data::from_png_bytes(include_bytes!("../assets/quotient.png"))
                    .expect("bundled Quotient icon must be a valid PNG"),
            )
            .with_inner_size([1100.0, 760.0])
            .with_min_inner_size([520.0, 320.0])
            .with_drag_and_drop(true)
            .with_icon(eframe::icon_data::from_png_bytes(ICON).expect("the bundled icon is a valid PNG")),
        ..Default::default()
    };
    eframe::run_native("Quotient", options, Box::new(move |cc| Ok(Box::new(App::new(cc, path)))))
}

// ---------------------------------------------------------------- appearance

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Theme {
    System,
    Light,
    Dark,
}

struct Palette {
    background: Color32,
    results_background: Color32,
    divider: Color32,
    hover: Color32,
    text: Color32,
    number: Color32,
    function: Color32,
    unit: Color32,
    constant: Color32,
    keyword: Color32,
    operator: Color32,
    comment: Color32,
    string: Color32,
    result: Color32,
    assignment: Color32,
    error: Color32,
}

fn palette(dark: bool) -> Palette {
    let hex = |rgb: u32| Color32::from_rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8);
    if dark {
        Palette {
            background: hex(0x171a21),
            results_background: hex(0x1c2029),
            divider: hex(0x2d3340),
            hover: hex(0x2a303c),
            text: hex(0xe2e8f0),
            number: hex(0xf6ad55),
            function: hex(0x63b3ed),
            unit: hex(0x4fd1c5),
            constant: hex(0xf687b3),
            keyword: hex(0xb794f4),
            operator: hex(0xa0aec0),
            comment: hex(0x718096),
            string: hex(0x9ae6b4),
            result: hex(0x68d391),
            assignment: hex(0xb794f4),
            error: hex(0xfc8181),
        }
    } else {
        Palette {
            background: hex(0xffffff),
            results_background: hex(0xf5f7fa),
            divider: hex(0xe2e8f0),
            hover: hex(0xe6ebf2),
            text: hex(0x1a202c),
            number: hex(0xc05621),
            function: hex(0x2b6cb0),
            unit: hex(0x2c7a7b),
            constant: hex(0xb83280),
            keyword: hex(0x6b46c1),
            operator: hex(0x4a5568),
            comment: hex(0x8a94a6),
            string: hex(0x2f855a),
            result: hex(0x2f855a),
            assignment: hex(0x6b46c1),
            error: hex(0xc53030),
        }
    }
}

fn class_color(class: Class, p: &Palette) -> Color32 {
    match class {
        Class::Plain | Class::Variable => p.text,
        Class::Number => p.number,
        Class::Function => p.function,
        Class::Unit => p.unit,
        Class::Constant => p.constant,
        Class::Keyword => p.keyword,
        Class::Operator => p.operator,
        Class::Comment => p.comment,
        Class::Text => p.string,
    }
}

fn highlight_job(text: &str, variables: &[String], p: &Palette, wrap_width: f32) -> LayoutJob {
    let mut job = LayoutJob::default();
    for (range, class) in highlight::highlight(text, variables) {
        job.append(&text[range], 0.0, TextFormat::simple(font(), class_color(class, p)));
    }
    job.wrap.max_width = wrap_width;
    job
}

// ---------------------------------------------------------------- preferences

#[derive(Clone, Copy, PartialEq, Debug)]
struct Prefs {
    settings: Settings,
    theme: Theme,
}

impl Default for Prefs {
    fn default() -> Self {
        Self { settings: Settings::default(), theme: Theme::System }
    }
}

impl Prefs {
    fn encode(&self) -> String {
        let f = self.settings.format;
        format!(
            "digits={};thousands={};precision={};theme={}",
            f.digits.map_or("full".into(), |d| d.to_string()),
            f.thousands as u8,
            self.settings.precision.unwrap_or(0),
            match self.theme {
                Theme::System => "system",
                Theme::Light => "light",
                Theme::Dark => "dark",
            }
        )
    }

    fn decode(s: &str) -> Self {
        let mut p = Prefs::default();
        for pair in s.split(';') {
            let Some((k, v)) = pair.split_once('=') else { continue };
            match k {
                "digits" if v == "full" => p.settings.format.digits = None,
                "digits" => {
                    if let Ok(d) = v.parse::<usize>()
                        && (1..=17).contains(&d)
                    {
                        p.settings.format.digits = Some(d);
                    }
                }
                "thousands" => p.settings.format.thousands = v == "1",
                "precision" => p.settings.precision = v.parse::<usize>().ok().filter(|d| (2..=1000).contains(d)),
                "theme" => {
                    p.theme = match v {
                        "light" => Theme::Light,
                        "dark" => Theme::Dark,
                        _ => Theme::System,
                    }
                }
                _ => {}
            }
        }
        p
    }
}

// ---------------------------------------------------------------- state

/// Everything the editor's layout depends on besides the text itself.
#[derive(Clone, PartialEq)]
struct LayoutKey {
    text: String,
    wrap_width: f32,
    dark: bool,
    variables: Vec<String>,
    /// Extra space below a line, to fit its result when that wraps: (line, height).
    gaps: Vec<(usize, f32)>,
}

/// Opens vertical space below the given lines by moving later rows down, so a
/// long result has room beside its line. Cursor placement and clicks use these
/// row positions too, so editing is unaffected.
fn add_gaps(galley: &Galley, gaps: &[(usize, f32)]) -> Galley {
    let mut galley = galley.clone();
    let mut line = 0;
    let mut shift = 0.0;
    let last = galley.rows.len().saturating_sub(1);
    for (i, row) in galley.rows.iter_mut().enumerate() {
        row.pos.y += shift;
        // A gap goes after the last row of its line.
        if row.ends_with_newline || i == last {
            shift += gaps.iter().filter(|(l, _)| *l == line).map(|(_, h)| h).sum::<f32>();
            line += 1;
        }
    }
    galley.rect.max.y += shift;
    galley.mesh_bounds.max.y += shift;
    galley
}

/// Lays out a result wrapped to the results column.
fn result_galley(ui: &egui::Ui, text: &str, color: Color32, width: f32) -> Arc<Galley> {
    let mut job = LayoutJob::single_section(text.to_string(), TextFormat::simple(font(), color));
    job.wrap = TextWrapping::wrap_at_width(width);
    ui.fonts_mut(|f| f.layout_job(job))
}

/// Something that would replace the current untitled text, waiting for confirmation.
#[derive(Clone, Debug)]
enum Action {
    New,
    Welcome,
    Open,
    OpenPath(PathBuf),
}

struct Status {
    text: String,
    error: bool,
    at: Instant,
}

/// The autocomplete popup. Byte offsets refer to the text as of the last frame.
struct Popup {
    items: Vec<Completion>,
    selected: usize,
    word_start: usize,
    cursor: usize,
}

struct App {
    text: String,
    doc: Document,
    prefs: Prefs,
    path: Option<PathBuf>,
    dirty: bool,
    last_edit: Instant,
    status: Option<Status>,
    show_help: bool,
    show_about: bool,
    help_query: String,
    confirm: Option<Action>,
    popup: Option<Popup>,
    /// Escape hides the popup for this word until it changes.
    dismissed: Option<(usize, String)>,
    /// A completion clicked in the popup, applied on the next frame.
    clicked: Option<usize>,
    /// The highlighted layout of the last text, reused while nothing changes.
    layout_cache: Option<(LayoutKey, Arc<Galley>)>,
    title: String,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>, path: Option<PathBuf>) -> Self {
        let storage = cc.storage;
        let get = |k: &str| storage.and_then(|s| s.get_string(k));
        let prefs = get("prefs").map(|s| Prefs::decode(&s)).unwrap_or_default();
        let text = get("scratch").unwrap_or_else(|| WELCOME.to_string());
        let mut app = App {
            doc: calculate(&text, &prefs.settings),
            text,
            prefs,
            path: None,
            dirty: false,
            last_edit: Instant::now(),
            status: None,
            show_help: false,
            show_about: false,
            help_query: String::new(),
            confirm: None,
            popup: None,
            dismissed: None,
            clicked: None,
            layout_cache: None,
            title: String::new(),
        };
        app.apply_theme(&cc.egui_ctx);
        let last = get("path").filter(|p| !p.is_empty()).map(PathBuf::from);
        if let Some(p) = path {
            app.open_path(&p);
        } else if let Some(p) = last.filter(|p| p.is_file()) {
            app.open_path(&p);
            app.status = None;
        }
        app
    }

    fn apply_theme(&self, ctx: &egui::Context) {
        ctx.set_theme(match self.prefs.theme {
            Theme::System => ThemePreference::System,
            Theme::Light => ThemePreference::Light,
            Theme::Dark => ThemePreference::Dark,
        });
    }

    fn recalculate(&mut self) {
        self.doc = calculate(&self.text, &self.prefs.settings);
    }

    fn edited(&mut self) {
        self.recalculate();
        self.dirty = true;
        self.last_edit = Instant::now();
    }

    fn say(&mut self, text: impl Into<String>, error: bool) {
        self.status = Some(Status { text: text.into(), error, at: Instant::now() });
    }

    // ------------------------------------------------------------ files

    /// Untitled text that would be lost by replacing it.
    fn has_unsaved_scratch(&self) -> bool {
        self.path.is_none() && !self.text.trim().is_empty() && self.text != WELCOME
    }

    fn request(&mut self, action: Action) {
        // Saved documents are written before switching; untitled work needs a yes.
        if self.path.is_some() && self.dirty {
            self.save(false);
        }
        if self.has_unsaved_scratch() {
            self.confirm = Some(action);
        } else {
            self.perform(action);
        }
    }

    fn perform(&mut self, action: Action) {
        match action {
            Action::New | Action::Welcome => {
                self.text = if matches!(action, Action::Welcome) { WELCOME.to_string() } else { String::new() };
                self.path = None;
                self.dirty = false;
                self.recalculate();
            }
            Action::Open => {
                if let Some(p) = file_dialog("Open").pick_file() {
                    self.open_path(&p);
                }
            }
            Action::OpenPath(p) => self.open_path(&p),
        }
    }

    fn open_path(&mut self, path: &Path) {
        match std::fs::read_to_string(path) {
            Ok(content) => {
                self.text = content.replace("\r\n", "\n");
                self.path = Some(path.to_path_buf());
                self.dirty = false;
                self.recalculate();
                self.say(format!("Opened {}", path.display()), false);
            }
            Err(e) => self.say(format!("Could not open {}: {e}", path.display()), true),
        }
    }

    fn save(&mut self, force_dialog: bool) -> bool {
        let path = match (&self.path, force_dialog) {
            (Some(p), false) => p.clone(),
            _ => {
                let name = self.path.as_ref().and_then(|p| p.file_name()).map_or("calculation.txt".into(), |n| n.to_string_lossy().into_owned());
                match file_dialog("Save").set_file_name(name).save_file() {
                    Some(p) => p,
                    None => return false,
                }
            }
        };
        match std::fs::write(&path, &self.text) {
            Ok(()) => {
                self.say(format!("Saved {}", path.display()), false);
                self.path = Some(path);
                self.dirty = false;
                true
            }
            Err(e) => {
                self.say(format!("Could not save: {e}"), true);
                false
            }
        }
    }

    fn autosave(&mut self) {
        if let (true, Some(path)) = (self.dirty, &self.path) {
            match std::fs::write(path, &self.text) {
                Ok(()) => self.dirty = false,
                Err(e) => self.say(format!("Auto-save failed: {e}"), true),
            }
        }
    }

    /// Every line with its result, as plain text.
    fn results_text(&self) -> String {
        self.doc.outputs.iter().map(|o| o.text.as_str()).collect::<Vec<_>>().join("\n")
    }

    fn export_results(&mut self) {
        let name = self.path.as_ref().and_then(|p| p.file_stem()).map_or("results".into(), |n| format!("{} results", n.to_string_lossy()));
        if let Some(p) = rfd::FileDialog::new().set_title("Export Results").add_filter("Text", &["txt"]).set_file_name(format!("{name}.txt")).save_file() {
            match std::fs::write(&p, self.results_text() + "\n") {
                Ok(()) => self.say(format!("Exported results to {}", p.display()), false),
                Err(e) => self.say(format!("Could not export: {e}"), true),
            }
        }
    }

    // ------------------------------------------------------------ input

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let pressed = |m: Modifiers, k: Key| ctx.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(m, k)));
        // Check the Shift variants first: Ctrl+S would also match Ctrl+Shift+S.
        if pressed(Modifiers::COMMAND | Modifiers::SHIFT, Key::S) {
            self.save(true);
        } else if pressed(Modifiers::COMMAND, Key::S) {
            self.save(false);
        }
        if pressed(Modifiers::COMMAND | Modifiers::SHIFT, Key::C) {
            ctx.copy_text(self.results_text());
            self.say("Copied all results", false);
        }
        if pressed(Modifiers::COMMAND, Key::N) {
            self.request(Action::New);
        }
        if pressed(Modifiers::COMMAND, Key::O) {
            self.request(Action::Open);
        }
        if pressed(Modifiers::NONE, Key::F1) || pressed(Modifiers::COMMAND, Key::Questionmark) || pressed(Modifiers::COMMAND, Key::Slash) {
            self.show_help = !self.show_help;
        }
        let dropped: Option<PathBuf> = ctx.input(|i| i.raw.dropped_files.first().map(|f| f.path().to_path_buf()));
        if let Some(p) = dropped {
            self.request(Action::OpenPath(p));
        }
    }

    /// Keys for the autocomplete popup, handled before the editor sees them.
    fn handle_popup_keys(&mut self, ctx: &egui::Context) {
        let Some(popup) = &mut self.popup else {
            self.clicked = None;
            return;
        };
        let n = popup.items.len();
        let key = |k: Key| ctx.input_mut(|i| i.consume_key(Modifiers::NONE, k));
        if key(Key::ArrowDown) {
            popup.selected = (popup.selected + 1) % n;
        }
        if key(Key::ArrowUp) {
            popup.selected = (popup.selected + n - 1) % n;
        }
        let accept = if key(Key::Tab) { Some(popup.selected) } else { self.clicked.take() };
        if key(Key::Escape) {
            self.dismissed = Some((popup.word_start, self.text[popup.word_start..popup.cursor].to_string()));
            self.popup = None;
            return;
        }
        if let Some(i) = accept {
            let popup = self.popup.take().expect("popup");
            self.accept_completion(ctx, &popup.items[i], popup.word_start, popup.cursor);
        }
    }

    fn accept_completion(&mut self, ctx: &egui::Context, item: &Completion, start: usize, cursor: usize) {
        if cursor > self.text.len() || !self.text.is_char_boundary(start) || !self.text.is_char_boundary(cursor) {
            return;
        }
        let mut insert = item.insert.clone();
        if insert.ends_with('(') && self.text[cursor..].starts_with('(') {
            insert.pop();
        }
        self.text.replace_range(start..cursor, &insert);
        let new_cursor = self.text[..start + insert.len()].chars().count();
        let mut state = TextEditState::load(ctx, editor_id()).unwrap_or_default();
        state.cursor.set_char_range(Some(CCursorRange::one(CCursor::new(new_cursor))));
        state.store(ctx, editor_id());
        ctx.memory_mut(|m| m.request_focus(editor_id()));
        self.edited();
    }

    // ------------------------------------------------------------ layout

    fn menu_bar(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.add(egui::Button::new("New").shortcut_text("Ctrl+N")).clicked() {
                    self.request(Action::New);
                }
                if ui.add(egui::Button::new("Open…").shortcut_text("Ctrl+O")).clicked() {
                    self.request(Action::Open);
                }
                ui.separator();
                if ui.add(egui::Button::new("Save").shortcut_text("Ctrl+S")).clicked() {
                    self.save(false);
                }
                if ui.add(egui::Button::new("Save As…").shortcut_text("Ctrl+Shift+S")).clicked() {
                    self.save(true);
                }
                if ui.button("Export Results…").clicked() {
                    self.export_results();
                }
                ui.separator();
                if ui.button("Quit").clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            ui.menu_button("Edit", |ui| {
                if ui.add(egui::Button::new("Copy All Results").shortcut_text("Ctrl+Shift+C")).clicked() {
                    ctx.copy_text(self.results_text());
                    self.say("Copied all results", false);
                }
                if ui.button("Show Welcome Example").clicked() {
                    self.request(Action::Welcome);
                }
            });
            ui.menu_button("View", |ui| self.view_menu(ui));
            ui.menu_button("Help", |ui| {
                if ui.add(egui::Button::new("Help & Functions").shortcut_text("F1")).clicked() {
                    self.show_help = true;
                }
                ui.separator();
                if ui.button(format!("About Quotient {}", env!("CARGO_PKG_VERSION"))).clicked() {
                    self.show_about = true;
                }
            });
        });
    }

    fn view_menu(&mut self, ui: &mut egui::Ui) {
        let before = self.prefs;
        ui.label(RichText::new("Theme").strong());
        ui.radio_value(&mut self.prefs.theme, Theme::System, "Follow system");
        ui.radio_value(&mut self.prefs.theme, Theme::Light, "Light");
        ui.radio_value(&mut self.prefs.theme, Theme::Dark, "Dark");
        ui.separator();
        ui.label(RichText::new("Digits shown").strong());
        let digits = &mut self.prefs.settings.format.digits;
        ui.radio_value(digits, Some(6), "6");
        ui.radio_value(digits, Some(10), "10");
        ui.radio_value(digits, Some(14), "14 (default)");
        ui.radio_value(digits, None, "All (exact floating point)");
        ui.checkbox(&mut self.prefs.settings.format.thousands, "Thousands separators");
        ui.separator();
        ui.label(RichText::new("High precision").strong());
        let precision = &mut self.prefs.settings.precision;
        ui.radio_value(precision, None, "Off (fast, 15–17 digits)");
        ui.radio_value(precision, Some(30), "30 digits");
        ui.radio_value(precision, Some(50), "50 digits");
        ui.radio_value(precision, Some(100), "100 digits");
        ui.separator();
        ui.label(RichText::new("Zoom: Ctrl + / Ctrl − / Ctrl 0").weak());
        if self.prefs != before {
            self.apply_theme(ui.ctx());
            self.recalculate();
        }
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let name = match &self.path {
                Some(p) => {
                    let n = p.file_name().unwrap_or_default().to_string_lossy().into_owned();
                    if self.dirty { format!("{n} — edited") } else { n }
                }
                None => "Untitled (kept between sessions)".into(),
            };
            ui.label(RichText::new(name).weak());
            if let Some(s) = &self.status {
                if s.at.elapsed() < STATUS_FOR {
                    ui.separator();
                    let color = if s.error { palette(ui.visuals().dark_mode).error } else { ui.visuals().text_color() };
                    ui.label(RichText::new(&s.text).color(color));
                    ui.ctx().request_repaint_after(STATUS_FOR);
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let s = self.prefs.settings;
                let mut mode = match s.format.digits {
                    Some(d) => format!("{d} digits"),
                    None => "all digits".into(),
                };
                if let Some(p) = s.precision {
                    mode = format!("High precision · {p} digits");
                }
                ui.label(RichText::new(mode).weak());
                let errors = self.doc.outputs.iter().filter(|o| o.kind == Kind::Error).count();
                if errors > 0 {
                    ui.separator();
                    let p = palette(ui.visuals().dark_mode);
                    ui.label(RichText::new(format!("{errors} error{}", if errors == 1 { "" } else { "s" })).color(p.error));
                }
            });
        });
    }

    fn editor(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let dark = ui.visuals().dark_mode;
        let p = palette(dark);
        let total = ui.available_width();
        let gap = 12.0;
        let editor_width = (total * 0.6).clamp(200.0_f32.min(total), (total - 140.0).max(120.0));
        let results_width = (total - editor_width - gap).max(60.0);

        // Results wider than their column wrap; the editor opens a gap below
        // their line so the whole result fits without covering the next one.
        let result_width = results_width - RESULT_PADDING;
        let row_height = ui.fonts_mut(|f| f.row_height(&font()));
        let gaps: Vec<(usize, f32)> = self
            .doc
            .outputs
            .iter()
            .filter(|o| o.kind != Kind::Info)
            .filter_map(|o| {
                let height = result_galley(ui, o.value.as_deref()?, Color32::WHITE, result_width).size().y;
                (height > row_height + 0.5).then_some((o.end_line, height - row_height))
            })
            .collect();

        ScrollArea::vertical().id_salt("document").auto_shrink([false, false]).show(ui, |ui| {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                let variables = self.doc.variables.clone();
                let cache = &mut self.layout_cache;
                let mut layouter = |ui: &egui::Ui, buf: &dyn TextBuffer, wrap_width: f32| -> Arc<Galley> {
                    let key = LayoutKey { text: buf.as_str().to_string(), wrap_width, dark, variables: variables.clone(), gaps: gaps.clone() };
                    if let Some((k, g)) = cache.as_ref()
                        && *k == key
                    {
                        return g.clone();
                    }
                    let job = highlight_job(&key.text, &variables, &p, wrap_width);
                    let mut galley = ui.fonts_mut(|f| f.layout_job(job));
                    if !key.gaps.is_empty() {
                        galley = Arc::new(add_gaps(&galley, &key.gaps));
                    }
                    *cache = Some((key, galley.clone()));
                    galley
                };
                let output = TextEdit::multiline(&mut self.text)
                    .id(editor_id())
                    .font(font())
                    .frame(egui::Frame::NONE)
                    .desired_width(editor_width)
                    .desired_rows(32)
                    .lock_focus(true)
                    .hint_text(RichText::new("Type a calculation, like 5 km to mi").font(font()))
                    .layouter(&mut layouter)
                    .show(ui);
                if output.response.response.changed() {
                    self.edited();
                    // The gaps depend on the new results; lay out again next frame.
                    ctx.request_repaint();
                }
                ui.add_space(gap);
                let height = output.response.response.rect.height().max(ui.clip_rect().bottom() - output.response.response.rect.top());
                let (column, _) = ui.allocate_exact_size(vec2(results_width, height), Sense::hover());
                ui.painter().rect_filled(column, 0.0, p.results_background);
                ui.painter().vline(column.left(), column.y_range(), egui::Stroke::new(1.0, p.divider));
                // The line being typed on: its errors are shown muted until you move on.
                let editing_line = output
                    .cursor_range
                    .filter(|_| output.response.response.has_focus())
                    .map(|r| self.text.chars().take(r.primary.index.0).filter(|c| *c == '\n').count());
                self.paint_results(ui, &output.galley, output.galley_pos, column, &p, editing_line);
                self.update_popup(&ctx, &output);
            });
        });
    }

    /// Draws each result at the height of its last line.
    fn paint_results(&mut self, ui: &mut egui::Ui, galley: &Galley, origin: egui::Pos2, column: Rect, p: &Palette, editing_line: Option<usize>) {
        // The first row of each logical line (a long line can wrap onto several rows).
        let mut line_rows: Vec<Rect> = Vec::new();
        let mut starts_line = true;
        for row in &galley.rows {
            if starts_line {
                line_rows.push(row.rect().translate(origin.to_vec2()));
            }
            starts_line = row.ends_with_newline;
        }
        let outputs: Vec<_> = self.doc.outputs.iter().filter(|o| o.kind != Kind::Info && o.value.is_some()).cloned().collect();
        for out in outputs {
            let Some(row) = line_rows.get(out.end_line) else { continue };
            let value = out.value.clone().unwrap_or_default();
            let editing = editing_line.is_some_and(|l| (out.start_line..=out.end_line).contains(&l));
            let color = match out.kind {
                Kind::Error if editing => p.comment,
                Kind::Error => p.error,
                Kind::Assignment => p.assignment,
                _ => p.result,
            };
            // The whole result, wrapped to the column; the editor left room for it.
            let text = result_galley(ui, &value, color, column.width() - RESULT_PADDING);
            let rect = Rect::from_min_size(pos2(column.left() + 10.0, row.top()), vec2(column.width() - RESULT_PADDING, text.size().y));
            let response = ui.interact(rect, ui.scope_id().with(("result", out.start_line)), Sense::click());
            if response.hovered() {
                ui.painter().rect_filled(rect.expand2(Vec2::new(4.0, 0.0)), 3.0, p.hover);
            }
            ui.painter().galley(rect.left_top(), text, color);
            if out.kind == Kind::Error {
                continue;
            }
            let response = response.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Click to copy");
            if response.clicked() {
                ui.ctx().copy_text(value.clone());
                let short: String = value.lines().next().unwrap_or("").chars().take(40).collect();
                self.say(format!("Copied {short}"), false);
            }
        }
    }

    /// Recomputes autocomplete suggestions for the word before the cursor and shows them.
    fn update_popup(&mut self, ctx: &egui::Context, output: &egui::text_edit::TextEditOutput) {
        let previous = self.popup.take();
        let focused = output.response.response.has_focus();
        let Some(range) = output.cursor_range.filter(|r| r.primary.index == r.secondary.index && focused) else {
            return;
        };
        let cursor_char = range.primary.index.0;
        let cursor = self.text.char_indices().nth(cursor_char).map_or(self.text.len(), |(b, _)| b);
        if self.text[cursor..].chars().next().is_some_and(|c| c.is_alphanumeric() || c == '_') {
            return;
        }
        let (start, word) = highlight::word_before(&self.text, cursor);
        if word.chars().count() < 2 || self.dismissed.as_ref().is_some_and(|(s, w)| *s == start && w == word) {
            return;
        }
        let items = highlight::complete(word, &self.doc.variables, 8);
        if items.is_empty() {
            return;
        }
        // Keep the arrow-key selection while the same suggestions are showing.
        let selected = previous
            .filter(|p| p.word_start == start && p.items == items)
            .map_or(0, |p| p.selected.min(items.len() - 1));
        let at = output.galley_pos + output.galley.pos_from_cursor(CCursor::new(cursor_char)).left_bottom().to_vec2();
        let mut clicked = None;
        egui::Area::new(egui::Id::unique("quotient-completions")).order(egui::Order::Foreground).fixed_pos(at + vec2(0.0, 2.0)).show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_max_width(460.0);
                for (i, item) in items.iter().enumerate() {
                    let selected = i == selected;
                    let label = RichText::new(&item.label).font(font()).strong();
                    let r = ui.add(egui::Button::selectable(selected, label).right_text(RichText::new(short(&item.detail, 52)).weak()));
                    if r.clicked() {
                        clicked = Some(i);
                    }
                }
                ui.label(RichText::new("Tab to insert · Esc to dismiss").weak().small());
            });
        });
        if clicked.is_some() {
            self.clicked = clicked;
            ctx.request_repaint();
        }
        self.popup = Some(Popup { items, selected, word_start: start, cursor });
    }

    fn help_window(&mut self, ctx: &egui::Context) {
        let mut open = self.show_help;
        egui::Window::new("Quotient Help").open(&mut open).default_size([620.0, 560.0]).vscroll(true).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Search:");
                ui.add(TextEdit::singleline(&mut self.help_query).hint_text("function, unit or topic").desired_width(260.0));
            });
            ui.separator();
            let q = self.help_query.trim().to_lowercase();
            if q.is_empty() {
                guide(ui);
            }
            for category in catalog::categories() {
                let entries: Vec<_> = catalog::ENTRIES
                    .iter()
                    .filter(|e| e.category == category)
                    .filter(|e| q.is_empty() || e.name.to_lowercase().contains(&q) || e.description.to_lowercase().contains(&q))
                    .collect();
                if entries.is_empty() {
                    continue;
                }
                egui::CollapsingHeader::new(format!("{category} ({})", entries.len())).default_open(!q.is_empty()).show(ui, |ui| {
                    for e in entries {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new(e.signature).monospace().strong());
                            ui.label(e.description);
                        });
                    }
                });
            }
            let unit_matches: Vec<&units::UnitDef> = units::UNITS
                .iter()
                .chain(units::TEMPERATURES)
                .filter(|u| q.is_empty() || u.symbol.to_lowercase().contains(&q) || u.long.iter().any(|l| l.contains(&q)))
                .collect();
            if !unit_matches.is_empty() {
                egui::CollapsingHeader::new(format!("Units ({})", unit_matches.len())).default_open(!q.is_empty()).show(ui, |ui| {
                    egui::Grid::new("units").striped(true).show(ui, |ui| {
                        for u in unit_matches {
                            let mut names: Vec<&str> = u.aliases.iter().chain(u.long).copied().collect();
                            names.dedup();
                            ui.label(RichText::new(u.symbol).monospace().strong());
                            ui.label(names.join(", "));
                            ui.label(RichText::new(units::describe_dim(&u.dim)).weak());
                            ui.label(RichText::new(u.prefix_note()).weak());
                            ui.end_row();
                        }
                    });
                });
            }
        });
        self.show_help = open;
    }

    fn about_window(&mut self, ctx: &egui::Context) {
        let modal = egui::Modal::new(egui::Id::unique("quotient-about")).show(ctx, |ui| {
            ui.set_width(440.0);
            ui.vertical_centered(|ui| {
                ui.heading(RichText::new("Quotient").size(26.0).strong());
                ui.label(RichText::new(format!("Version {}", env!("CARGO_PKG_VERSION"))).weak());
                ui.add_space(6.0);
                ui.label("Math in plain text.");
                ui.add_space(10.0);
                ui.label(RichText::new(format!("Copyright © {COPYRIGHT_YEAR} {AUTHOR}")).strong());
                ui.label("Released under the MIT License.");
                ui.add_space(6.0);
                ui.label(RichText::new("Based on calctxt, © Jason Lotito, also under the MIT License.").weak());
            });
            ui.add_space(8.0);
            egui::CollapsingHeader::new("License").show(ui, |ui| {
                ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                    ui.label(RichText::new(LICENSE).monospace().size(11.5));
                });
            });
            ui.add_space(8.0);
            ui.vertical_centered(|ui| ui.button("Close").clicked()).inner
        });
        if modal.inner || modal.should_close() {
            self.show_about = false;
        }
    }

    fn confirm_modal(&mut self, ctx: &egui::Context) {
        let Some(action) = self.confirm.clone() else { return };
        let mut choice = None;
        egui::Modal::new(egui::Id::unique("quotient-confirm")).show(ctx, |ui| {
            ui.set_max_width(380.0);
            ui.heading("Replace this calculation?");
            ui.label("Your untitled calculation has not been saved to a file.");
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Save…").clicked() {
                    choice = Some(1);
                }
                if ui.button("Discard").clicked() {
                    choice = Some(2);
                }
                if ui.button("Cancel").clicked() {
                    choice = Some(0);
                }
            });
        });
        match choice {
            Some(1) => {
                self.confirm = None;
                if self.save(true) {
                    self.perform(action);
                }
            }
            Some(2) => {
                self.confirm = None;
                self.perform(action);
            }
            Some(_) => self.confirm = None,
            None => {}
        }
    }

    fn update_title(&mut self, ctx: &egui::Context) {
        let title = match &self.path {
            Some(p) => format!("{}{} — Quotient", p.file_name().unwrap_or_default().to_string_lossy(), if self.dirty { " •" } else { "" }),
            None => "Quotient".into(),
        };
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
    }
}

fn short(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let cut: String = s.chars().take(max - 1).collect();
    format!("{cut}…")
}

fn file_dialog(title: &str) -> rfd::FileDialog {
    rfd::FileDialog::new()
        .set_title(title)
        .add_filter("Text", &["txt", "calctxt", "qt"])
        .add_filter("All files", &["*"])
}

fn guide(ui: &mut egui::Ui) {
    let section = |ui: &mut egui::Ui, title: &str, lines: &[(&str, &str)]| {
        egui::CollapsingHeader::new(title).default_open(title == "Basics").show(ui, |ui| {
            egui::Grid::new(title).num_columns(2).spacing([16.0, 4.0]).show(ui, |ui| {
                for (code, what) in lines {
                    ui.label(RichText::new(*code).monospace());
                    ui.label(*what);
                    ui.end_row();
                }
            });
        });
    };
    section(ui, "Basics", &[
        ("2 + 3 * 4", "Each line is calculated as you type"),
        ("x = 5", "Variables; use them on later lines"),
        ("f(x) = x^2 + 1", "Define your own functions"),
        ("ans, line3", "The previous result, or the result on line 3"),
        ("// note", "Comments, on their own line or after a calculation"),
        ("sum(\n  1, 2,\n  3\n)", "Open brackets continue onto the next line"),
        ("2pi, 3(x + 1)", "Implicit multiplication"),
        ("5!, 2^10, 7 mod 3", "Factorial, powers, remainder"),
        ("[1, 2; 3, 4], 1:10", "Matrices and ranges"),
        ("x > 0 ? 1 : -1", "Conditions, with and, or, not"),
        ("0xff, 0b101, 1_000", "Hex, binary and grouped numbers"),
    ]);
    section(ui, "Percentages", &[
        ("20% of 150", "30"),
        ("80 + 15%", "92 (adds 15% of 80)"),
        ("25% off 80", "60"),
        ("5% on 200", "210"),
    ]);
    section(ui, "Units", &[
        ("5 km to mi", "Convert with \"to\" or \"in\""),
        ("60 mph * 2.5 h to km", "Units combine through arithmetic"),
        ("72 degF to degC", "Temperatures (also °C, °F, K)"),
        ("sin(30 deg)", "Angles work with trigonometry"),
        ("1.5 GiB to MB", "Data sizes with SI and binary prefixes"),
        ("splitUnit(1.75 h, [h, min])", "1 h, 45 min"),
    ]);
    section(ui, "Numbers & formats", &[
        ("sqrt(-4), (1 + 2i) * i", "Complex numbers"),
        ("255 to hex / bin / oct", "Other bases"),
        ("0.75 to fraction", "3/4"),
        ("0.256 to percent", "25.6%"),
        ("1234567 to sci", "Scientific notation"),
        ("View ▸ High precision", "Exact decimals and up to 100 digits"),
    ]);
    section(ui, "Keyboard", &[
        ("Tab", "Accept the autocomplete suggestion"),
        ("Ctrl+N / Ctrl+O", "New / Open"),
        ("Ctrl+S / Ctrl+Shift+S", "Save / Save As (saved files also auto-save)"),
        ("Ctrl+Shift+C", "Copy every line with its result"),
        ("Click a result", "Copy it"),
        ("F1", "This help"),
    ]);
    ui.add_space(6.0);
    ui.label(RichText::new("Functions").strong());
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let p = palette(ui.visuals().dark_mode);
        self.handle_popup_keys(&ctx);
        self.handle_shortcuts(&ctx);
        if self.popup.is_none() && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            self.show_help = false;
        }

        egui::Panel::top("menu").show(ui, |ui| self.menu_bar(ui));
        egui::Panel::bottom("status").show(ui, |ui| self.status_bar(ui));
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(p.background).inner_margin(egui::Margin { left: 16, right: 0, top: 12, bottom: 0 }))
            .show(ui, |ui| self.editor(ui));

        if self.show_help {
            self.help_window(&ctx);
        }
        if self.show_about {
            self.about_window(&ctx);
        }
        self.confirm_modal(&ctx);

        if ctx.input(|i| i.viewport().close_requested()) {
            self.autosave();
        } else if self.dirty && self.path.is_some() {
            let waited = self.last_edit.elapsed();
            if waited >= AUTOSAVE_AFTER {
                self.autosave();
            } else {
                ctx.request_repaint_after(AUTOSAVE_AFTER - waited);
            }
        }
        self.update_title(&ctx);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        storage.set_string("prefs", self.prefs.encode());
        storage.set_string("path", self.path.as_ref().map(|p| p.display().to_string()).unwrap_or_default());
        if self.path.is_none() {
            storage.set_string("scratch", self.text.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::accesskit::Role;
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;
    use quotient::FormatOptions;

    fn harness() -> Harness<'static, App> {
        Harness::builder().with_size([1000.0, 700.0]).build_eframe(|cc| App::new(cc, None))
    }

    /// Starts with an empty document and the editor focused.
    fn empty_harness() -> Harness<'static, App> {
        let mut h = harness();
        h.state_mut().text.clear();
        h.state_mut().recalculate();
        h.run();
        h.get_by_role(Role::MultilineTextInput).focus();
        h.run();
        h
    }

    fn type_text(h: &mut Harness<'_, App>, text: &str) {
        h.get_by_role(Role::MultilineTextInput).type_text(text);
        h.run();
    }

    fn values(h: &Harness<'_, App>) -> Vec<String> {
        h.state().doc.outputs.iter().filter_map(|o| o.value.clone()).collect()
    }

    #[test]
    fn prefs_round_trip() {
        let p = Prefs {
            settings: Settings { format: FormatOptions { digits: None, thousands: true }, precision: Some(50) },
            theme: Theme::Dark,
        };
        assert_eq!(Prefs::decode(&p.encode()), p);
        assert_eq!(Prefs::decode(""), Prefs::default());
        assert_eq!(Prefs::decode("digits=999;precision=-3;theme=purple"), Prefs::default());
    }

    #[test]
    fn starts_with_the_welcome_example_and_no_errors() {
        let h = harness();
        assert_eq!(h.state().text, WELCOME);
        assert!(h.state().doc.outputs.iter().all(|o| o.kind != Kind::Error));
    }

    #[test]
    fn typing_recalculates() {
        let mut h = empty_harness();
        type_text(&mut h, "x = 2 + 3");
        h.key_press(Key::Enter);
        h.run();
        type_text(&mut h, "x * 10 km to m");
        assert_eq!(values(&h), ["5", "50000 m"]);
        assert!(h.state().dirty);
    }

    #[test]
    fn tab_accepts_a_suggestion() {
        let mut h = empty_harness();
        type_text(&mut h, "sq");
        let popup = h.state().popup.as_ref().expect("suggestions for \"sq\"");
        assert_eq!(popup.items[0].label, "sqrt");
        h.key_press(Key::Tab);
        h.run();
        assert_eq!(h.state().text, "sqrt(");
        type_text(&mut h, "16)");
        assert_eq!(values(&h), ["4"]);
    }

    #[test]
    fn arrows_choose_and_escape_dismisses() {
        let mut h = empty_harness();
        type_text(&mut h, "si");
        let second = h.state().popup.as_ref().expect("suggestions").items[1].label.clone();
        h.key_press(Key::ArrowDown);
        h.run();
        assert_eq!(h.state().popup.as_ref().expect("still open").selected, 1);
        h.key_press(Key::Tab);
        h.run();
        assert_eq!(h.state().text, format!("{second}("));

        let mut h = empty_harness();
        type_text(&mut h, "si");
        h.key_press(Key::Escape);
        h.run();
        assert!(h.state().popup.is_none());
        assert_eq!(h.state().text, "si");
        // Tab no longer inserts anything from the dismissed popup.
        h.key_press(Key::Tab);
        h.run();
        assert!(!h.state().text.starts_with("sin"));
    }

    #[test]
    fn variables_are_suggested() {
        let mut h = empty_harness();
        type_text(&mut h, "budget = 100");
        h.key_press(Key::Enter);
        h.run();
        type_text(&mut h, "bu");
        assert_eq!(h.state().popup.as_ref().expect("suggestions").items[0].label, "budget");
    }

    #[test]
    fn help_opens_searches_and_closes() {
        let mut h = harness();
        h.key_press(Key::F1);
        h.run();
        assert!(h.state().show_help);
        assert!(h.query_by_label("Quotient Help").is_some());
        h.state_mut().help_query = "erf".into();
        h.run();
        assert!(h.query_by_label_contains("Error function").is_some());
        h.key_press(Key::Escape);
        h.run();
        assert!(!h.state().show_help);
    }

    #[test]
    fn view_menu_changes_settings() {
        let mut h = harness();
        h.state_mut().text = "1234567.891".into();
        h.state_mut().recalculate();
        h.get_by_label("View").click();
        h.run();
        h.get_by_label("Thousands separators").click();
        h.run();
        assert!(h.state().prefs.settings.format.thousands);
        assert_eq!(values(&h), ["1,234,567.891"]);
        // Choosing an item closes the menu, as usual.
        h.get_by_label("View").click();
        h.run();
        h.get_by_label("50 digits").click();
        h.run();
        assert_eq!(h.state().prefs.settings.precision, Some(50));
    }

    #[test]
    fn new_asks_before_discarding_untitled_work() {
        let mut h = harness();
        h.state_mut().text = "my important sums".into();
        h.run();
        h.key_press_modifiers(Modifiers::COMMAND, Key::N);
        h.run();
        assert!(h.state().confirm.is_some());
        h.get_by_label("Cancel").click();
        h.run();
        assert_eq!(h.state().text, "my important sums");
        h.key_press_modifiers(Modifiers::COMMAND, Key::N);
        h.run();
        h.get_by_label("Discard").click();
        h.run();
        assert!(h.state().text.is_empty());
    }

    #[test]
    fn saved_files_autosave() {
        let dir = std::env::temp_dir().join(format!("quotient-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("sums.txt");
        std::fs::write(&file, "1 + 1").unwrap();
        let mut h = harness();
        h.state_mut().open_path(&file);
        h.run();
        assert_eq!(values(&h), ["2"]);
        h.get_by_role(Role::MultilineTextInput).focus();
        h.run();
        type_text(&mut h, " + 1");
        assert!(h.state().dirty);
        h.state_mut().last_edit = Instant::now() - AUTOSAVE_AFTER;
        h.run();
        assert!(!h.state().dirty);
        let saved = std::fs::read_to_string(&file).unwrap();
        assert!(saved.contains("+ 1"), "{saved}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn about_shows_author_copyright_and_license() {
        let mut h = harness();
        h.get_by_label("Help").click();
        h.run();
        h.get_by_label_contains("About Quotient").click();
        h.run();
        assert!(h.state().show_about);
        assert!(h.query_by_label(&format!("Copyright © {COPYRIGHT_YEAR} {AUTHOR}")).is_some());
        assert!(h.query_by_label("Released under the MIT License.").is_some());
        assert!(LICENSE.contains(&format!("Copyright (c) {COPYRIGHT_YEAR} {AUTHOR}")), "LICENSE and Cargo.toml authors disagree");
        h.get_by_label("Close").click();
        h.run();
        assert!(!h.state().show_about);
    }

    #[test]
    fn long_results_wrap_and_push_the_next_line_down() {
        let mut h = harness();
        h.state_mut().text = "1:60\n1 + 1".into();
        h.state_mut().recalculate();
        h.run();
        let (key, galley) = h.state().layout_cache.clone().expect("laid out");
        assert_eq!(key.gaps.len(), 1, "the long list needs extra room");
        assert_eq!(key.gaps[0].0, 0, "below line 1");
        // The second line starts below the first by the normal row height plus the gap.
        let rows = &galley.rows;
        let spacing = rows[1].pos.y - rows[0].pos.y;
        assert!(spacing > rows[0].rect().height() + key.gaps[0].1 - 1.0, "spacing {spacing}");
        // Short results need no gaps.
        h.state_mut().text = "1 + 1\n2 + 2".into();
        h.state_mut().recalculate();
        h.run();
        assert!(h.state().layout_cache.as_ref().expect("laid out").0.gaps.is_empty());
    }

    #[test]
    fn multi_line_results_are_shown_in_full() {
        let mut h = harness();
        h.state_mut().text = "trend(1, 2, 4)\nx = 1".into();
        h.state_mut().recalculate();
        h.run();
        let (key, _) = h.state().layout_cache.clone().expect("laid out");
        // trend() has 6 lines of output, so its line gets room for 5 more rows.
        let row = h.ctx.fonts_mut(|f| f.row_height(&font()));
        assert_eq!(key.gaps.len(), 1);
        let rows_of_gap = key.gaps[0].1 / row;
        assert!((4.5..5.5).contains(&rows_of_gap), "gap of {rows_of_gap} rows");
    }

    #[test]
    fn renders() {
        let mut h = harness();
        h.run();
        let image = h.render().expect("render");
        if let Ok(dir) = std::env::var("QUOTIENT_SCREENSHOTS") {
            image.save(format!("{dir}/welcome.png")).unwrap();
        }
        h.key_press(Key::F1);
        h.run();
        let image = h.render().expect("render");
        if let Ok(dir) = std::env::var("QUOTIENT_SCREENSHOTS") {
            image.save(format!("{dir}/help.png")).unwrap();
        }

        let mut h = harness();
        h.state_mut().show_about = true;
        h.run();
        let image = h.render().expect("render");
        if let Ok(dir) = std::env::var("QUOTIENT_SCREENSHOTS") {
            image.save(format!("{dir}/about.png")).unwrap();
        }

        let mut h = empty_harness();
        h.state_mut().prefs.theme = Theme::Light;
        h.state().apply_theme(&h.ctx);
        type_text(&mut h, "price = 1299\ntax = 8% of price\nprice + tax to fraction\n3 cups to mL\n1:40\ntrend(3, 5, 4)\nsqrt(-9)\nbogus + 1\nmea");
        let image = h.render().expect("render");
        if let Ok(dir) = std::env::var("QUOTIENT_SCREENSHOTS") {
            image.save(format!("{dir}/light-popup.png")).unwrap();
        }
    }
}
