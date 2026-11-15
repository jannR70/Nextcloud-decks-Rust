#![windows_subsystem = "windows"]

use eframe::egui::{
    self, pos2, vec2, Align, Button, Color32, CursorIcon, FontFamily, FontId, Frame,
    Id, Layout, Margin, Order, Rect, RichText, Rounding, Sense, Stroke, TextEdit, TextStyle,
};
use reqwest::blocking::Client;
use reqwest::Method;
use serde::{de::DeserializeOwned, Deserialize, Deserializer, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

// ---------------------------------------------------------------- Farben & Maße

const BG: Color32 = Color32::from_rgb(0x0f, 0x11, 0x17);
const PANEL: Color32 = Color32::from_rgb(0x15, 0x18, 0x21);
const COLUMN: Color32 = Color32::from_rgb(0x1a, 0x1e, 0x2a);
const CARD: Color32 = Color32::from_rgb(0x25, 0x2a, 0x3a);
const CARD_HOVER: Color32 = Color32::from_rgb(0x2d, 0x33, 0x47);
const BORDER: Color32 = Color32::from_rgb(0x32, 0x38, 0x4b);
const ACCENT: Color32 = Color32::from_rgb(0x6d, 0x7c, 0xff);
const ACCENT_DIM: Color32 = Color32::from_rgb(0x3b, 0x43, 0x8f);
const TEXT: Color32 = Color32::from_rgb(0xe6, 0xe8, 0xf0);
const MUTED: Color32 = Color32::from_rgb(0x8b, 0x92, 0xa8);
const GREEN: Color32 = Color32::from_rgb(0x4c, 0xc3, 0x8a);
const ORANGE: Color32 = Color32::from_rgb(0xf0, 0xa2, 0x3c);
const DANGER: Color32 = Color32::from_rgb(0xd9, 0x4f, 0x5f);

const COL_INNER: f32 = 264.0; // Breite einer Spalte (ohne Rand)
const CARD_INNER: f32 = 240.0; // Breite einer Karte (ohne Rand)

fn setup_fonts(ctx: &egui::Context) {
    // Segoe UI (Windows) als Schrift, falls vorhanden
    let mut fonts = egui::FontDefinitions::default();
    if let Ok(bytes) = std::fs::read(r"C:\Windows\Fonts\segoeui.ttf") {
        fonts
            .font_data
            .insert("segoe".to_owned(), egui::FontData::from_owned(bytes).into());
        if let Some(list) = fonts.families.get_mut(&FontFamily::Proportional) {
            list.insert(0, "segoe".to_owned());
        }
    }
    // Fett-Familie (Segoe UI Bold), sonst normale Schrift
    let mut bold: Vec<String> = vec![];
    if let Ok(bytes) = std::fs::read(r"C:\Windows\Fonts\segoeuib.ttf") {
        fonts
            .font_data
            .insert("segoe-bold".to_owned(), egui::FontData::from_owned(bytes).into());
        bold.push("segoe-bold".to_owned());
    }
    if let Some(list) = fonts.families.get(&FontFamily::Proportional) {
        bold.extend(list.iter().cloned());
    }
    fonts.families.insert(FontFamily::Name("bold".into()), bold);
    ctx.set_fonts(fonts);
}

/// Setzt das dunkle Design. Wird erneut aufgerufen, falls Windows auf "hell"
/// steht und egui deshalb das helle Standard-Theme aktiviert.
fn apply_theme(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.panel_fill = BG;
    v.window_fill = PANEL;
    v.extreme_bg_color = Color32::from_rgb(0x10, 0x12, 0x19);
    v.faint_bg_color = COLUMN;
    v.window_rounding = Rounding::same(14.0);
    v.menu_rounding = Rounding::same(10.0);
    v.window_stroke = Stroke::new(1.0_f32, BORDER);
    v.selection.bg_fill = ACCENT_DIM;
    v.selection.stroke = Stroke::new(1.0_f32, ACCENT);

    let r = Rounding::same(8.0);
    v.widgets.noninteractive.rounding = r;
    v.widgets.inactive.rounding = r;
    v.widgets.hovered.rounding = r;
    v.widgets.active.rounding = r;
    v.widgets.open.rounding = r;

    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT);
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    v.widgets.inactive.bg_fill = CARD;
    v.widgets.inactive.weak_bg_fill = CARD;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT);
    v.widgets.hovered.bg_fill = CARD_HOVER;
    v.widgets.hovered.weak_bg_fill = CARD_HOVER;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, ACCENT);
    v.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    v.widgets.active.bg_fill = ACCENT_DIM;
    v.widgets.active.weak_bg_fill = ACCENT_DIM;
    v.widgets.active.bg_stroke = Stroke::new(1.0_f32, ACCENT);
    v.widgets.active.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    ctx.set_visuals(v);

    ctx.style_mut(|s| {
        s.spacing.item_spacing = vec2(8.0, 8.0);
        s.spacing.button_padding = vec2(12.0, 6.0);
        s.spacing.interact_size.y = 30.0;
        s.spacing.scroll = egui::style::ScrollStyle::floating();
        s.text_styles
            .insert(TextStyle::Heading, FontId::new(22.0, FontFamily::Proportional));
        s.text_styles
            .insert(TextStyle::Body, FontId::new(14.5, FontFamily::Proportional));
        s.text_styles
            .insert(TextStyle::Button, FontId::new(14.5, FontFamily::Proportional));
        s.text_styles
            .insert(TextStyle::Small, FontId::new(12.0, FontFamily::Proportional));
    });
}


// ---------------------------------------------------------------- Markdown

use egui::text::{CCursor, CCursorRange, LayoutJob, TextFormat};

fn bold_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("bold".into()))
}

fn char_to_byte(s: &str, c: usize) -> usize {
    s.char_indices().nth(c).map(|(i, _)| i).unwrap_or(s.len())
}

/// Umschließt die Auswahl (oder einen Platzhalter) mit Markdown-Zeichen.
fn md_wrap(ctx: &egui::Context, id: Id, text: &mut String, before: &str, after: &str, placeholder: &str) {
    let mut state = TextEdit::load_state(ctx, id).unwrap_or_default();
    let total = text.chars().count();
    let (a, b) = state
        .cursor
        .char_range()
        .map(|r| {
            let (x, y) = (r.primary.index.min(total), r.secondary.index.min(total));
            (x.min(y), x.max(y))
        })
        .unwrap_or((total, total));
    let (ba, bb) = (char_to_byte(text, a), char_to_byte(text, b));
    let selected = if a == b { placeholder.to_string() } else { text[ba..bb].to_string() };
    let ins = format!("{before}{selected}{after}");
    text.replace_range(ba..bb, &ins);
    let start = a + before.chars().count();
    let end = start + selected.chars().count();
    state
        .cursor
        .set_char_range(Some(CCursorRange::two(CCursor::new(start), CCursor::new(end))));
    state.store(ctx, id);
    ctx.memory_mut(|m| m.request_focus(id));
}

/// Setzt ein Präfix (z. B. "- ", "## ") an den Anfang der aktuellen Zeile.
fn md_prefix(ctx: &egui::Context, id: Id, text: &mut String, prefix: &str) {
    let mut state = TextEdit::load_state(ctx, id).unwrap_or_default();
    let total = text.chars().count();
    let a = state
        .cursor
        .char_range()
        .map(|r| r.primary.index.min(r.secondary.index).min(total))
        .unwrap_or(total);
    let ba = char_to_byte(text, a);
    let line_start = text[..ba].rfind('\n').map(|i| i + 1).unwrap_or(0);
    text.insert_str(line_start, prefix);
    let pos = a + prefix.chars().count();
    state
        .cursor
        .set_char_range(Some(CCursorRange::one(CCursor::new(pos))));
    state.store(ctx, id);
    ctx.memory_mut(|m| m.request_focus(id));
}

fn md_inline(src: &str, size: f32, color: Color32, width: f32, force_bold: bool) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.wrap.max_width = width;
    let (mut bold, mut italic, mut code) = (force_bold, false, false);
    let mut buf = String::new();
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0;

    fn flush(job: &mut LayoutJob, buf: &mut String, bold: bool, italic: bool, code: bool, size: f32, color: Color32, link: bool) {
        if buf.is_empty() {
            return;
        }
        let mut f = TextFormat {
            font_id: if code {
                FontId::new(size - 1.0, FontFamily::Monospace)
            } else if bold {
                bold_font(size)
            } else {
                FontId::new(size, FontFamily::Proportional)
            },
            color: if link { ACCENT } else if code { Color32::from_rgb(0xff, 0xb8, 0x6c) } else { color },
            italics: italic,
            ..Default::default()
        };
        if code {
            f.background = Color32::from_rgb(0x10, 0x12, 0x19);
        }
        if link {
            f.underline = Stroke::new(1.0_f32, ACCENT);
        }
        job.append(buf, 0.0, f);
        buf.clear();
    }

    while i < chars.len() {
        let c = chars[i];
        if c == '`' {
            flush(&mut job, &mut buf, bold, italic, code, size, color, false);
            code = !code;
            i += 1;
        } else if !code && c == '*' && chars.get(i + 1) == Some(&'*') {
            flush(&mut job, &mut buf, bold, italic, code, size, color, false);
            bold = !bold || force_bold;
            i += 2;
        } else if !code && (c == '*' || c == '_') && !(c == '_' && i > 0 && chars[i - 1].is_alphanumeric()) {
            flush(&mut job, &mut buf, bold, italic, code, size, color, false);
            italic = !italic;
            i += 1;
        } else if !code && c == '[' {
            // [Text](url)
            let rest: String = chars[i..].iter().collect();
            if let (Some(close), true) = (rest.find("]("), true) {
                if let Some(end) = rest[close..].find(')') {
                    flush(&mut job, &mut buf, bold, italic, code, size, color, false);
                    let label = &rest[1..close];
                    let mut l = label.to_string();
                    flush(&mut job, &mut l, bold, italic, code, size, color, true);
                    i += rest[..close + end + 1].chars().count();
                    continue;
                }
            }
            buf.push(c);
            i += 1;
        } else {
            buf.push(c);
            i += 1;
        }
    }
    flush(&mut job, &mut buf, bold, italic, code, size, color, false);
    job
}

/// Zeigt Markdown an. Gibt die Zeilennummer zurück, wenn eine Checkbox angeklickt wurde.
fn render_md(ui: &mut egui::Ui, src: &str) -> Option<usize> {
    let mut toggled = None;
    let mut in_code = false;
    let w = ui.available_width() - 4.0;
    for (n, line) in src.lines().enumerate() {
        let t = line.trim_end();
        if t.trim_start().starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if in_code {
            ui.label(
                RichText::new(t)
                    .font(FontId::new(13.0, FontFamily::Monospace))
                    .color(Color32::from_rgb(0xff, 0xb8, 0x6c))
                    .background_color(Color32::from_rgb(0x10, 0x12, 0x19)),
            );
            continue;
        }
        let trimmed = t.trim_start();
        let indent = ((t.len() - trimmed.len()) as f32 / 2.0).floor() * 14.0;

        if trimmed.is_empty() {
            ui.add_space(4.0);
        } else if let Some(h) = trimmed.strip_prefix("### ") {
            ui.label(md_inline(h, 16.0, TEXT, w, true));
        } else if let Some(h) = trimmed.strip_prefix("## ") {
            ui.add_space(2.0);
            ui.label(md_inline(h, 19.0, TEXT, w, true));
        } else if let Some(h) = trimmed.strip_prefix("# ") {
            ui.add_space(4.0);
            ui.label(md_inline(h, 23.0, TEXT, w, true));
        } else if trimmed == "---" || trimmed == "***" {
            ui.separator();
        } else if let Some(q) = trimmed.strip_prefix("> ") {
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(vec2(3.0, 18.0), Sense::hover());
                ui.painter().rect_filled(r, 1.5_f32, ACCENT_DIM);
                ui.label(md_inline(q, 14.5, MUTED, w - 14.0, false));
            });
        } else if trimmed.starts_with("- [ ] ")
            || trimmed.starts_with("- [x] ")
            || trimmed.starts_with("- [X] ")
        {
            let checked = !trimmed.starts_with("- [ ] ");
            let rest = &trimmed[6..];
            ui.horizontal(|ui| {
                ui.add_space(indent);
                let mut c = checked;
                if ui.checkbox(&mut c, "").clicked() {
                    toggled = Some(n);
                }
                let col = if checked { MUTED } else { TEXT };
                ui.label(md_inline(rest, 14.5, col, w - 40.0 - indent, false));
            });
        } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            ui.horizontal_top(|ui| {
                ui.add_space(indent + 4.0);
                ui.label(RichText::new("•").color(ACCENT));
                ui.label(md_inline(&trimmed[2..], 14.5, TEXT, w - 28.0 - indent, false));
            });
        } else if let Some(pos) = trimmed.find(". ").filter(|&p| p > 0 && p < 4 && trimmed[..p].chars().all(|c| c.is_ascii_digit())) {
            ui.horizontal_top(|ui| {
                ui.add_space(indent + 4.0);
                ui.label(RichText::new(format!("{}.", &trimmed[..pos])).color(ACCENT));
                ui.label(md_inline(&trimmed[pos + 2..], 14.5, TEXT, w - 34.0 - indent, false));
            });
        } else {
            ui.label(md_inline(trimmed, 14.5, TEXT, w, false));
        }
    }
    toggled
}

fn toggle_checkbox_line(src: &str, line: usize) -> String {
    let mut out: Vec<String> = src.lines().map(|l| l.to_string()).collect();
    if let Some(l) = out.get_mut(line) {
        if l.contains("- [ ] ") {
            *l = l.replacen("- [ ] ", "- [x] ", 1);
        } else if l.contains("- [x] ") {
            *l = l.replacen("- [x] ", "- [ ] ", 1);
        } else if l.contains("- [X] ") {
            *l = l.replacen("- [X] ", "- [ ] ", 1);
        }
    }
    let mut r = out.join("\n");
    if src.ends_with('\n') {
        r.push('\n');
    }
    r
}


// ---------------------------------------------------------------- Datum & Checklisten

use chrono::{DateTime, Local, NaiveDate, NaiveTime, SecondsFormat, TimeZone, Utc};

fn parse_due_local(s: &str) -> Option<DateTime<Local>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.with_timezone(&Local))
}

fn due_to_fields(due: &Option<String>) -> (String, String) {
    match due.as_deref().and_then(parse_due_local) {
        Some(d) => (d.format("%d.%m.%Y").to_string(), d.format("%H:%M").to_string()),
        None => (String::new(), String::new()),
    }
}

/// Leeres Datum = keine Fälligkeit. Gibt den Zeitpunkt als UTC-String für Nextcloud zurück.
fn fields_to_due(date: &str, time: &str) -> Result<Option<String>, String> {
    let date = date.trim();
    if date.is_empty() {
        return Ok(None);
    }
    let d = NaiveDate::parse_from_str(date, "%d.%m.%Y")
        .map_err(|_| "Datum bitte als TT.MM.JJJJ eingeben (z. B. 24.12.2026)".to_string())?;
    let time = time.trim();
    let t = if time.is_empty() {
        NaiveTime::from_hms_opt(12, 0, 0).unwrap()
    } else {
        NaiveTime::parse_from_str(time, "%H:%M")
            .map_err(|_| "Uhrzeit bitte als HH:MM eingeben (z. B. 14:30)".to_string())?
    };
    let local = Local
        .from_local_datetime(&d.and_time(t))
        .earliest()
        .ok_or_else(|| "Dieser Zeitpunkt existiert nicht (Zeitumstellung)".to_string())?;
    Ok(Some(
        local
            .with_timezone(&Utc)
            .to_rfc3339_opts(SecondsFormat::Secs, true),
    ))
}

/// (erledigt, gesamt) der Markdown-Checkboxen
fn checklist_stats(desc: &str) -> (usize, usize) {
    let mut done = 0;
    let mut total = 0;
    for l in desc.lines() {
        let t = l.trim_start();
        if t.starts_with("- [ ] ") || t.starts_with("* [ ] ") {
            total += 1;
        } else if t.starts_with("- [x] ")
            || t.starts_with("- [X] ")
            || t.starts_with("* [x] ")
            || t.starts_with("* [X] ")
        {
            total += 1;
            done += 1;
        }
    }
    (done, total)
}

// ---------------------------------------------------------------- Datenmodell

fn null_default<'de, D, T>(d: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(d)?.unwrap_or_default())
}

#[derive(Clone, Deserialize)]
struct Board {
    id: u64,
    title: String,
    #[serde(default, deserialize_with = "null_default")]
    archived: bool,
    #[serde(default, rename = "deletedAt", deserialize_with = "null_default")]
    deleted_at: i64,
}

#[derive(Clone, Deserialize)]
struct Card {
    id: u64,
    title: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default, deserialize_with = "null_default")]
    order: i64,
    #[serde(default, deserialize_with = "null_default")]
    archived: bool,
    #[serde(default)]
    duedate: Option<String>,
}

#[derive(Clone, Deserialize)]
struct Stack {
    id: u64,
    title: String,
    #[serde(default, deserialize_with = "null_default")]
    order: i64,
    #[serde(default)]
    cards: Option<Vec<Card>>,
}

// ---------------------------------------------------------------- API

#[derive(Clone)]
struct Api {
    client: Client,
    base: String,
    user: String,
    token: String,
}

impl Api {
    fn new(url: &str, user: &str, token: &str) -> Self {
        let url = url.trim().trim_end_matches('/');
        let url = if url.starts_with("http://") || url.starts_with("https://") {
            url.to_string()
        } else {
            format!("https://{url}")
        };
        let client = Client::builder()
            .timeout(Duration::from_secs(20))
            .build()
            .expect("HTTP-Client konnte nicht erstellt werden");
        Self {
            client,
            base: format!("{url}/index.php/apps/deck/api/v1.0"),
            user: user.trim().to_string(),
            token: token.trim().to_string(),
        }
    }

    fn call(&self, method: Method, path: &str, body: Option<Value>) -> Result<Value, String> {
        let mut rb = self
            .client
            .request(method, format!("{}{}", self.base, path))
            .basic_auth(&self.user, Some(&self.token))
            .header("OCS-APIRequest", "true")
            .header("Accept", "application/json");
        if let Some(b) = body {
            rb = rb.json(&b);
        }
        let resp = rb.send().map_err(|e| format!("Verbindungsfehler: {e}"))?;
        let status = resp.status();
        let text = resp.text().unwrap_or_default();
        if status.as_u16() == 401 {
            return Err("Anmeldung fehlgeschlagen (401). Benutzername und App-Passwort prüfen.".into());
        }
        if !status.is_success() {
            return Err(format!("HTTP {status}: {text}"));
        }
        if text.trim().is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_str(&text).map_err(|e| format!("Antwort nicht lesbar: {e}"))
    }

    fn typed<T: DeserializeOwned>(&self, method: Method, path: &str) -> Result<T, String> {
        let v = self.call(method, path, None)?;
        serde_json::from_value(v).map_err(|e| format!("Antwort nicht lesbar: {e}"))
    }

    fn boards(&self) -> Result<Vec<Board>, String> {
        let all: Vec<Board> = self.typed(Method::GET, "/boards")?;
        Ok(all
            .into_iter()
            .filter(|b| !b.archived && b.deleted_at == 0)
            .collect())
    }

    fn stacks(&self, board: u64) -> Result<Vec<Stack>, String> {
        self.typed(Method::GET, &format!("/boards/{board}/stacks"))
    }

    fn add_board(&self, title: &str) -> Result<(), String> {
        self.call(
            Method::POST,
            "/boards",
            Some(json!({ "title": title, "color": "6d7cff" })),
        )?;
        Ok(())
    }

    fn add_stack(&self, board: u64, title: &str, order: i64) -> Result<(), String> {
        self.call(
            Method::POST,
            &format!("/boards/{board}/stacks"),
            Some(json!({ "title": title, "order": order })),
        )?;
        Ok(())
    }

    fn delete_stack(&self, board: u64, stack: u64) -> Result<(), String> {
        self.call(Method::DELETE, &format!("/boards/{board}/stacks/{stack}"), None)?;
        Ok(())
    }

    fn add_card(&self, board: u64, stack: u64, title: &str) -> Result<(), String> {
        self.call(
            Method::POST,
            &format!("/boards/{board}/stacks/{stack}/cards"),
            Some(json!({ "title": title, "type": "plain", "order": 0 })),
        )?;
        Ok(())
    }

    fn update_card(
        &self,
        board: u64,
        stack: u64,
        card: u64,
        title: &str,
        desc: &str,
        order: i64,
        duedate: Option<String>,
    ) -> Result<(), String> {
        self.call(
            Method::PUT,
            &format!("/boards/{board}/stacks/{stack}/cards/{card}"),
            Some(json!({
                "title": title,
                "description": desc,
                "type": "plain",
                "owner": self.user,
                "order": order,
                "duedate": duedate
            })),
        )?;
        Ok(())
    }

    fn move_card(
        &self,
        board: u64,
        _stack: u64,
        card: u64,
        target: u64,
        order: i64,
    ) -> Result<(), String> {
        self.call(
            Method::PUT,
            // Nextcloud nimmt die Ziel-Liste aus der URL, nicht aus dem Body
            &format!("/boards/{board}/stacks/{target}/cards/{card}/reorder"),
            Some(json!({ "order": order, "stackId": target })),
        )?;
        Ok(())
    }

    fn delete_card(&self, board: u64, stack: u64, card: u64) -> Result<(), String> {
        self.call(
            Method::DELETE,
            &format!("/boards/{board}/stacks/{stack}/cards/{card}"),
            None,
        )?;
        Ok(())
    }
}

// ---------------------------------------------------------------- Konfiguration

#[derive(Serialize, Deserialize, Default)]
struct Config {
    url: String,
    user: String,
    token: String,
}

fn config_path() -> Option<PathBuf> {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("DeckGui").join("config.json"))
}

fn load_config() -> Option<Config> {
    let text = std::fs::read_to_string(config_path()?).ok()?;
    serde_json::from_str(&text).ok()
}

fn save_config(cfg: &Config) {
    if let Some(p) = config_path() {
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(text) = serde_json::to_string_pretty(cfg) {
            let _ = std::fs::write(p, text);
        }
    }
}

fn delete_config() {
    if let Some(p) = config_path() {
        let _ = std::fs::remove_file(p);
    }
}

// ---------------------------------------------------------------- App

enum Msg {
    LoggedIn(Result<(Api, Vec<Board>), String>),
    Boards(Result<Vec<Board>, String>),
    Stacks(u64, Result<Vec<Stack>, String>),
    Done(Result<(), String>),
}

enum Action {
    SelectBoard(u64),
    Refresh,
    Logout,
    AddBoard(String),
    AddStack(String),
    AskDeleteStack(u64, String),
    AddCard(u64, String),
    EditCard(u64, Card),
    MoveCard { from: u64, card: u64, to: u64, index: usize },
}

enum Danger {
    Card(u64, u64),
    Stack(u64),
}

/// Wird beim Ziehen einer Karte mitgeführt.
#[derive(Clone, Default)]
struct DragPayload {
    stack: u64,
    card: u64,
    title: String,
    desc: String,
    offset: egui::Vec2,
}

/// Eigener Drag-Zustand (unabhängig vom egui-Payload, der beim Loslassen
/// sofort gelöscht wird).
struct DragAndDrop;

impl DragAndDrop {
    fn key() -> Id {
        Id::new("deck_drag_payload")
    }

    fn set_payload(ctx: &egui::Context, p: DragPayload) {
        ctx.data_mut(|d| d.insert_temp(Self::key(), p));
    }

    fn payload<T: Clone + Send + Sync + 'static>(ctx: &egui::Context) -> Option<T> {
        ctx.data(|d| d.get_temp::<T>(Self::key()))
    }

    fn clear(ctx: &egui::Context) {
        ctx.data_mut(|d| d.remove_temp::<DragPayload>(Self::key()));
    }
}

struct EditCard {
    stack: u64,
    id: u64,
    order: i64,
    title: String,
    desc: String,
    due_date: String,
    due_time: String,
    error: String,
}

#[derive(PartialEq)]
enum Screen {
    Login,
    Main,
}

struct App {
    screen: Screen,
    url: String,
    user: String,
    token: String,
    remember: bool,
    api: Option<Api>,
    boards: Vec<Board>,
    selected: Option<u64>,
    stacks: Vec<Stack>,
    status: String,
    busy: usize,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
    editing: Option<EditCard>,
    confirm: Option<(String, Danger)>,
    new_cards: HashMap<u64, String>,
    new_stack: String,
    new_board: String,
}

fn first_line(s: &str) -> String {
    let line = s.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
    let line = line
        .trim_start()
        .trim_start_matches(|c: char| c == '#' || c == '>' || c == '-' || c == '*' || c == ' ')
        .trim_start_matches("[ ] ")
        .trim_start_matches("[x] ")
        .trim_start_matches("[X] ");
    let line = &line.replace("**", "").replace('`', "");
    let line = line.as_str();
    let short: String = line.chars().take(90).collect();
    if line.chars().count() > 90 {
        format!("{short}…")
    } else {
        short
    }
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_fonts(&cc.egui_ctx);
        apply_theme(&cc.egui_ctx);
        let (tx, rx) = channel();
        let mut app = Self {
            screen: Screen::Login,
            url: String::new(),
            user: String::new(),
            token: String::new(),
            remember: true,
            api: None,
            boards: vec![],
            selected: None,
            stacks: vec![],
            status: String::new(),
            busy: 0,
            tx,
            rx,
            editing: None,
            confirm: None,
            new_cards: HashMap::new(),
            new_stack: String::new(),
            new_board: String::new(),
        };
        if let Some(cfg) = load_config() {
            app.url = cfg.url;
            app.user = cfg.user;
            app.token = cfg.token;
            if !app.url.is_empty() && !app.token.is_empty() {
                app.login(&cc.egui_ctx);
            }
        }
        app
    }

    fn spawn<F>(&mut self, ctx: &egui::Context, f: F)
    where
        F: FnOnce() -> Msg + Send + 'static,
    {
        self.busy += 1;
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(f());
            ctx.request_repaint();
        });
    }

    fn login(&mut self, ctx: &egui::Context) {
        let api = Api::new(&self.url, &self.user, &self.token);
        self.status = "Anmeldung läuft …".into();
        self.spawn(ctx, move || match api.boards() {
            Ok(b) => Msg::LoggedIn(Ok((api, b))),
            Err(e) => Msg::LoggedIn(Err(e)),
        });
    }

    fn refresh_stacks(&mut self, ctx: &egui::Context) {
        if let (Some(api), Some(b)) = (self.api.clone(), self.selected) {
            self.spawn(ctx, move || Msg::Stacks(b, api.stacks(b)));
        }
    }

    fn set_boards(&mut self, boards: Vec<Board>) {
        self.boards = boards;
        let still_there = self
            .selected
            .map(|id| self.boards.iter().any(|b| b.id == id))
            .unwrap_or(false);
        if !still_there {
            self.selected = self.boards.first().map(|b| b.id);
            self.stacks.clear();
        }
    }

    fn handle(&mut self, ctx: &egui::Context, m: Msg) {
        match m {
            Msg::LoggedIn(Ok((api, boards))) => {
                if self.remember {
                    save_config(&Config {
                        url: self.url.clone(),
                        user: self.user.clone(),
                        token: self.token.clone(),
                    });
                }
                self.api = Some(api);
                self.set_boards(boards);
                self.screen = Screen::Main;
                self.status.clear();
                self.refresh_stacks(ctx);
            }
            Msg::LoggedIn(Err(e)) => {
                self.status = e;
                self.screen = Screen::Login;
            }
            Msg::Boards(Ok(b)) => {
                self.set_boards(b);
                self.refresh_stacks(ctx);
            }
            Msg::Boards(Err(e)) => self.status = e,
            Msg::Stacks(b, Ok(mut s)) => {
                if self.selected == Some(b) {
                    s.sort_by_key(|x| x.order);
                    for st in s.iter_mut() {
                        if let Some(c) = st.cards.as_mut() {
                            c.sort_by_key(|c| c.order);
                        }
                    }
                    self.stacks = s;
                }
            }
            Msg::Stacks(_, Err(e)) => self.status = e,
            Msg::Done(Ok(())) => {
                self.status.clear();
                self.refresh_stacks(ctx);
            }
            Msg::Done(Err(e)) => {
                self.status = e;
                self.refresh_stacks(ctx);
            }
        }
    }

    fn apply(&mut self, ctx: &egui::Context, a: Action) {
        match a {
            Action::SelectBoard(id) => {
                self.selected = Some(id);
                self.stacks.clear();
                self.refresh_stacks(ctx);
                return;
            }
            Action::Refresh => {
                if let Some(api) = self.api.clone() {
                    self.spawn(ctx, move || Msg::Boards(api.boards()));
                }
                return;
            }
            Action::Logout => {
                delete_config();
                self.api = None;
                self.boards.clear();
                self.stacks.clear();
                self.selected = None;
                self.token.clear();
                self.screen = Screen::Login;
                return;
            }
            Action::AddBoard(title) => {
                if let Some(api) = self.api.clone() {
                    self.spawn(ctx, move || match api.add_board(&title) {
                        Ok(()) => Msg::Boards(api.boards()),
                        Err(e) => Msg::Boards(Err(e)),
                    });
                }
                return;
            }
            Action::AskDeleteStack(id, title) => {
                self.confirm = Some((
                    format!("Liste „{title}“ samt aller Karten löschen?"),
                    Danger::Stack(id),
                ));
                return;
            }
            Action::EditCard(stack, c) => {
                let (due_date, due_time) = due_to_fields(&c.duedate);
                self.editing = Some(EditCard {
                    stack,
                    id: c.id,
                    order: c.order,
                    title: c.title,
                    desc: c.description.unwrap_or_default(),
                    due_date,
                    due_time,
                    error: String::new(),
                });
                return;
            }
            _ => {}
        }

        let (api, b) = match (self.api.clone(), self.selected) {
            (Some(a), Some(b)) => (a, b),
            _ => return,
        };
        match a {
            Action::AddStack(title) => {
                let order = self.stacks.len() as i64;
                self.spawn(ctx, move || Msg::Done(api.add_stack(b, &title, order)));
            }
            Action::AddCard(stack, title) => {
                self.spawn(ctx, move || Msg::Done(api.add_card(b, stack, &title)));
            }
            Action::MoveCard { from, card, to, index } => {
                // Sofort lokal verschieben, damit nichts springt
                let mut moved: Option<Card> = None;
                if let Some(s) = self.stacks.iter_mut().find(|s| s.id == from) {
                    if let Some(list) = s.cards.as_mut() {
                        if let Some(pos) = list.iter().position(|x| x.id == card) {
                            moved = Some(list.remove(pos));
                        }
                    }
                }
                if let Some(c) = moved {
                    if let Some(s) = self.stacks.iter_mut().find(|s| s.id == to) {
                        let list = s.cards.get_or_insert_with(Vec::new);
                        let i = index.min(list.len());
                        list.insert(i, c);
                    }
                }
                self.spawn(ctx, move || {
                    Msg::Done(api.move_card(b, from, card, to, index as i64))
                });
            }
            _ => {}
        }
    }

    // ------------------------------------------------------------ Oberflächen

    fn login_ui(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(Frame::none().fill(BG))
            .show(ctx, |ui| {
                let top = (ui.available_height() * 0.12).max(20.0);
                ui.vertical_centered(|ui| {
                    ui.add_space(top);
                    Frame::none()
                        .fill(PANEL)
                        .rounding(16.0_f32)
                        .stroke(Stroke::new(1.0_f32, BORDER))
                        .inner_margin(Margin::same(28.0))
                        .show(ui, |ui| {
                            ui.set_width(320.0);
                            ui.with_layout(Layout::top_down(Align::Min), |ui| {
                                ui.label(
                                    RichText::new("Nextcloud Deck").size(24.0).strong().color(TEXT),
                                );
                                ui.label(RichText::new("Melde dich mit deinem Konto an").color(MUTED));
                                ui.add_space(12.0);

                                ui.label(RichText::new("Server-URL").small().color(MUTED));
                                let r1 = ui.add(
                                    TextEdit::singleline(&mut self.url)
                                        .hint_text("https://cloud.example.de")
                                        .desired_width(f32::INFINITY),
                                );
                                ui.label(RichText::new("Benutzername").small().color(MUTED));
                                let r2 = ui.add(
                                    TextEdit::singleline(&mut self.user).desired_width(f32::INFINITY),
                                );
                                ui.label(RichText::new("App-Passwort").small().color(MUTED));
                                let r3 = ui.add(
                                    TextEdit::singleline(&mut self.token)
                                        .password(true)
                                        .desired_width(f32::INFINITY),
                                );
                                let enter = (r1.lost_focus() || r2.lost_focus() || r3.lost_focus())
                                    && ui.input(|i| i.key_pressed(egui::Key::Enter));

                                ui.checkbox(&mut self.remember, "Angemeldet bleiben");
                                ui.add_space(4.0);
                                let btn = Button::new(
                                    RichText::new("Anmelden").strong().color(Color32::WHITE),
                                )
                                .fill(ACCENT)
                                .min_size(vec2(ui.available_width(), 38.0));
                                if (ui.add_enabled(self.busy == 0, btn).clicked() || enter)
                                    && self.busy == 0
                                {
                                    self.login(ctx);
                                }
                                if !self.status.is_empty() {
                                    let color = if self.busy > 0 { MUTED } else { DANGER };
                                    ui.label(RichText::new(&self.status).color(color));
                                }
                            });
                        });
                });
            });
    }

    fn main_ui(&mut self, ctx: &egui::Context) {
        let mut actions: Vec<Action> = vec![];
        let payload = DragAndDrop::payload::<DragPayload>(ctx);
        if payload.is_some() {
            ctx.set_cursor_icon(CursorIcon::Grabbing);
        }

        egui::TopBottomPanel::top("top")
            .frame(Frame::none().fill(PANEL).inner_margin(Margin::symmetric(16.0, 10.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Nextcloud Deck").size(17.0).strong().color(ACCENT));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("Abmelden").clicked() {
                            actions.push(Action::Logout);
                        }
                        if ui.button("Aktualisieren").clicked() {
                            actions.push(Action::Refresh);
                        }
                        ui.label(RichText::new(&self.user).color(MUTED));
                    });
                });
            });

        if self.busy > 0 || !self.status.is_empty() {
            egui::TopBottomPanel::bottom("status")
                .frame(Frame::none().fill(PANEL).inner_margin(Margin::symmetric(16.0, 6.0)))
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        if self.busy > 0 {
                            ui.spinner();
                        }
                        if !self.status.is_empty() {
                            ui.label(RichText::new(&self.status).color(DANGER));
                        }
                    });
                });
        }

        egui::SidePanel::left("boards")
            .exact_width(220.0)
            .frame(Frame::none().fill(PANEL).inner_margin(Margin::same(12.0)))
            .show(ctx, |ui| {
                ui.label(RichText::new("BOARDS").small().strong().color(MUTED));
                ui.add_space(2.0);
                for b in &self.boards {
                    let sel = self.selected == Some(b.id);
                    let btn = Button::new(
                        RichText::new(&b.title).color(if sel { Color32::WHITE } else { TEXT }),
                    )
                    .fill(if sel { ACCENT } else { Color32::TRANSPARENT })
                    .stroke(Stroke::NONE)
                    .min_size(vec2(ui.available_width(), 34.0));
                    if ui.add(btn).clicked() && !sel {
                        actions.push(Action::SelectBoard(b.id));
                    }
                }
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(4.0);
                let r = ui.add(
                    TextEdit::singleline(&mut self.new_board)
                        .hint_text("+ Neues Board")
                        .desired_width(f32::INFINITY),
                );
                let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if enter && !self.new_board.trim().is_empty() {
                    actions.push(Action::AddBoard(
                        std::mem::take(&mut self.new_board).trim().to_string(),
                    ));
                }
            });

        egui::CentralPanel::default()
            .frame(Frame::none().fill(BG).inner_margin(Margin::same(16.0)))
            .show(ctx, |ui| {
                if self.selected.is_none() {
                    ui.label(RichText::new("Kein Board vorhanden. Lege links ein neues Board an.").color(MUTED));
                    return;
                }
                let stacks = &self.stacks;
                let new_cards = &mut self.new_cards;
                let new_stack = &mut self.new_stack;
                let height = ui.available_height();
                let released = payload.is_some() && !ctx.input(|i| i.pointer.any_down());

                let mut card_rects: Vec<(u64, u64, Rect)> = vec![];
                let mut col_rects: Vec<(u64, Rect, f32)> = vec![];

                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.horizontal_top(|ui| {
                            ui.spacing_mut().item_spacing.x = 14.0;
                            for stack in stacks.iter() {
                                let cards: Vec<&Card> = stack
                                    .cards
                                    .as_deref()
                                    .unwrap_or(&[])
                                    .iter()
                                    .filter(|c| !c.archived)
                                    .collect();

                                let col = Frame::none()
                                    .fill(COLUMN)
                                    .rounding(12.0_f32)
                                    .stroke(Stroke::new(1.0_f32, BORDER))
                                    .inner_margin(Margin::same(12.0))
                                    .show(ui, |ui| {
                                        ui.set_width(COL_INNER);
                                        ui.set_min_height((height - 24.0 - 20.0).max(160.0));
                                        ui.vertical(|ui| {

                                        ui.horizontal(|ui| {
                                            ui.label(
                                                RichText::new(&stack.title)
                                                    .size(15.5)
                                                    .strong()
                                                    .color(TEXT),
                                            );
                                            ui.label(
                                                RichText::new(cards.len().to_string())
                                                    .small()
                                                    .color(MUTED),
                                            );
                                            ui.with_layout(
                                                Layout::right_to_left(Align::Center),
                                                |ui| {
                                                    let del = Button::new(
                                                        RichText::new("×").size(18.0).color(MUTED),
                                                    )
                                                    .frame(false);
                                                    if ui
                                                        .add(del)
                                                        .on_hover_text("Liste löschen")
                                                        .clicked()
                                                    {
                                                        actions.push(Action::AskDeleteStack(
                                                            stack.id,
                                                            stack.title.clone(),
                                                        ));
                                                    }
                                                },
                                            );
                                        });
                                        ui.add_space(2.0);
                                        let list_top = ui.cursor().top();

                                        for card in &cards {
                                            let dragging_this = payload
                                                .as_ref()
                                                .map(|p| p.card == card.id)
                                                .unwrap_or(false);
                                            let fill = if dragging_this {
                                                Color32::from_rgba_unmultiplied(0x25, 0x2a, 0x3a, 90)
                                            } else {
                                                CARD
                                            };
                                            let fr = Frame::none()
                                                .fill(fill)
                                                .rounding(8.0_f32)
                                                .stroke(Stroke::new(1.0_f32, BORDER))
                                                .inner_margin(Margin::symmetric(12.0, 10.0))
                                                .show(ui, |ui| {
                                                    ui.set_width(CARD_INNER);
                                                    ui.label(
                                                        RichText::new(&card.title)
                                                            .strong()
                                                            .color(TEXT),
                                                    );
                                                    let desc = card.description.as_deref().unwrap_or("");
                                                    let (done, total) = checklist_stats(desc);
                                                    if total > 0 {
                                                        // Checkliste ersetzt die Beschreibung in der Vorschau
                                                        let all = done == total;
                                                        let col = if all { GREEN } else { MUTED };
                                                        ui.label(
                                                            RichText::new(format!("Checkliste {done}/{total}"))
                                                                .small()
                                                                .color(col),
                                                        );
                                                        ui.add(
                                                            egui::ProgressBar::new(done as f32 / total as f32)
                                                                .desired_width(CARD_INNER)
                                                                .desired_height(5.0)
                                                                .fill(if all { GREEN } else { ACCENT })
                                                                .rounding(3.0_f32),
                                                        );
                                                    } else if !desc.trim().is_empty() {
                                                        ui.label(
                                                            RichText::new(first_line(desc))
                                                                .small()
                                                                .color(MUTED),
                                                        );
                                                    }
                                                    if let Some(dl) =
                                                        card.duedate.as_deref().and_then(parse_due_local)
                                                    {
                                                        let diff = dl.signed_duration_since(Local::now());
                                                        let col = if diff.num_seconds() < 0 {
                                                            DANGER
                                                        } else if diff.num_hours() < 24 {
                                                            ORANGE
                                                        } else {
                                                            MUTED
                                                        };
                                                        ui.label(
                                                            RichText::new(format!(
                                                                "Fällig {}",
                                                                dl.format("%d.%m.%Y %H:%M")
                                                            ))
                                                            .small()
                                                            .color(col),
                                                        );
                                                    }
                                                });
                                            let rect = fr.response.rect;
                                            let resp = ui.interact(
                                                rect,
                                                Id::new(("card", card.id)),
                                                Sense::click_and_drag(),
                                            );
                                            if resp.hovered() && payload.is_none() {
                                                ctx.set_cursor_icon(CursorIcon::PointingHand);
                                                ui.painter().rect_stroke(
                                                    rect,
                                                    8.0_f32,
                                                    Stroke::new(1.5_f32, ACCENT),
                                                );
                                            }
                                            if resp.clicked() {
                                                actions.push(Action::EditCard(
                                                    stack.id,
                                                    (*card).clone(),
                                                ));
                                            }
                                            if resp.drag_started() {
                                                let off = resp
                                                    .interact_pointer_pos()
                                                    .map(|p| p - rect.min)
                                                    .unwrap_or(vec2(20.0, 14.0));
                                                DragAndDrop::set_payload(
                                                    ctx,
                                                    DragPayload {
                                                        stack: stack.id,
                                                        card: card.id,
                                                        title: card.title.clone(),
                                                        desc: card
                                                            .description
                                                            .clone()
                                                            .unwrap_or_default(),
                                                        offset: off,
                                                    },
                                                );
                                            }
                                            card_rects.push((stack.id, card.id, rect));
                                        }

                                        ui.add_space(2.0);
                                        let entry = new_cards.entry(stack.id).or_default();
                                        let r = ui.add(
                                            TextEdit::singleline(entry)
                                                .hint_text("+ Karte hinzufügen")
                                                .desired_width(f32::INFINITY),
                                        );
                                        let enter = r.lost_focus()
                                            && ui.input(|i| i.key_pressed(egui::Key::Enter));
                                        if enter && !entry.trim().is_empty() {
                                            actions.push(Action::AddCard(
                                                stack.id,
                                                entry.trim().to_string(),
                                            ));
                                            entry.clear();
                                            r.request_focus();
                                        }
                                        list_top
                                        })
                                    });
                                col_rects.push((stack.id, col.response.rect, col.inner.inner));
                            }

                            // Spalte zum Anlegen einer neuen Liste
                            Frame::none()
                                .fill(PANEL)
                                .rounding(12.0_f32)
                                .stroke(Stroke::new(1.0_f32, BORDER))
                                .inner_margin(Margin::same(12.0))
                                .show(ui, |ui| {
                                    ui.set_width(COL_INNER);
                                    let r = ui.add(
                                        TextEdit::singleline(new_stack)
                                            .hint_text("+ Neue Liste")
                                            .desired_width(f32::INFINITY),
                                    );
                                    let enter =
                                        r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                                    if enter && !new_stack.trim().is_empty() {
                                        actions.push(Action::AddStack(new_stack.trim().to_string()));
                                        new_stack.clear();
                                    }
                                });
                        });

                        // Drag & Drop: Zielspalte, Einfügeposition, Markierung, Ablegen
                        if let Some(p) = &payload {
                            if let Some(pos) = ctx.pointer_latest_pos() {
                                let target = col_rects
                                    .iter()
                                    .find(|c| pos.x >= c.1.left() - 7.0 && pos.x <= c.1.right() + 7.0);
                                if let Some(&(sid, crect, list_top)) = target {
                                    let others: Vec<&(u64, u64, Rect)> = card_rects
                                        .iter()
                                        .filter(|c| c.0 == sid && c.1 != p.card)
                                        .collect();
                                    let idx =
                                        others.iter().filter(|c| c.2.center().y < pos.y).count();
                                    let line_y = if idx < others.len() {
                                        others[idx].2.top() - 4.0
                                    } else if let Some(l) = others.last() {
                                        l.2.bottom() + 4.0
                                    } else {
                                        list_top + 2.0
                                    };
                                    ui.painter().rect_stroke(
                                        crect,
                                        12.0_f32,
                                        Stroke::new(1.5_f32, ACCENT_DIM),
                                    );
                                    ui.painter().rect_filled(
                                        Rect::from_min_max(
                                            pos2(crect.left() + 12.0, line_y - 1.5),
                                            pos2(crect.right() - 12.0, line_y + 1.5),
                                        ),
                                        2.0_f32,
                                        ACCENT,
                                    );
                                    if released {
                                        let cur_idx = if p.stack == sid {
                                            stacks
                                                .iter()
                                                .find(|s| s.id == sid)
                                                .and_then(|s| s.cards.as_ref())
                                                .and_then(|c| {
                                                    c.iter()
                                                        .filter(|x| !x.archived)
                                                        .position(|x| x.id == p.card)
                                                })
                                        } else {
                                            None
                                        };
                                        if cur_idx != Some(idx) {
                                            actions.push(Action::MoveCard {
                                                from: p.stack,
                                                card: p.card,
                                                to: sid,
                                                index: idx,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    });
            });

        if payload.is_some() && !ctx.input(|i| i.pointer.any_down()) {
            DragAndDrop::clear(ctx);
        }

        // Karte unter dem Mauszeiger beim Ziehen
        if let Some(p) = &payload {
            if ctx.input(|i| i.pointer.any_down()) {
                if let Some(pos) = ctx.pointer_latest_pos() {
                    egui::Area::new(Id::new("drag_ghost"))
                        .order(Order::Tooltip)
                        .fixed_pos(pos - p.offset)
                        .interactable(false)
                        .show(ctx, |ui| {
                            Frame::none()
                                .fill(CARD_HOVER)
                                .rounding(8.0_f32)
                                .stroke(Stroke::new(1.5_f32, ACCENT))
                                .shadow(ctx.style().visuals.popup_shadow)
                                .inner_margin(Margin::symmetric(12.0, 10.0))
                                .show(ui, |ui| {
                                    ui.set_width(CARD_INNER);
                                    ui.label(RichText::new(&p.title).strong().color(TEXT));
                                    if !p.desc.trim().is_empty() {
                                        ui.label(
                                            RichText::new(first_line(&p.desc)).small().color(MUTED),
                                        );
                                    }
                                });
                        });
                }
            }
        }

        // Karte bearbeiten
        let mut save = false;
        let mut cancel = false;
        let mut delete = false;
        if let Some(e) = self.editing.as_mut() {
            let desc_id = Id::new("card_desc_editor");
            egui::Window::new("Karte bearbeiten")
                .collapsible(false)
                .resizable(true)
                .default_width(980.0)
                .min_width(640.0)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(RichText::new("Titel").small().color(MUTED));
                    ui.add(TextEdit::singleline(&mut e.title).desired_width(f32::INFINITY));
                    ui.add_space(4.0);

                    ui.label(RichText::new("Fälligkeit").small().color(MUTED));
                    ui.horizontal_wrapped(|ui| {
                        ui.add(
                            TextEdit::singleline(&mut e.due_date)
                                .hint_text("TT.MM.JJJJ")
                                .desired_width(110.0),
                        );
                        ui.add(
                            TextEdit::singleline(&mut e.due_time)
                                .hint_text("12:00")
                                .desired_width(64.0),
                        );
                        let today = Local::now().date_naive();
                        if ui.button("Heute").clicked() {
                            e.due_date = today.format("%d.%m.%Y").to_string();
                        }
                        if ui.button("Morgen").clicked() {
                            e.due_date = (today + chrono::Days::new(1)).format("%d.%m.%Y").to_string();
                        }
                        if ui.button("In 1 Woche").clicked() {
                            e.due_date = (today + chrono::Days::new(7)).format("%d.%m.%Y").to_string();
                        }
                        if ui.button("Entfernen").clicked() {
                            e.due_date.clear();
                            e.due_time.clear();
                        }
                    });
                    if !e.error.is_empty() {
                        ui.label(RichText::new(&e.error).color(DANGER));
                    }
                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Beschreibung (Markdown)").small().color(MUTED));
                        let (done, total) = checklist_stats(&e.desc);
                        if total > 0 {
                            ui.label(
                                RichText::new(format!("· Checkliste {done}/{total}"))
                                    .small()
                                    .color(if done == total { GREEN } else { MUTED }),
                            );
                        }
                    });

                    // Werkzeugleiste
                    ui.horizontal_wrapped(|ui| {
                        let mut b = |ui: &mut egui::Ui, label: RichText, tip: &str| -> bool {
                            ui.add(Button::new(label).min_size(vec2(32.0, 28.0)))
                                .on_hover_text(tip)
                                .clicked()
                        };
                        if b(ui, RichText::new("B").font(bold_font(15.0)), "Fett") {
                            md_wrap(ctx, desc_id, &mut e.desc, "**", "**", "fett");
                        }
                        if b(ui, RichText::new("I").italics(), "Kursiv") {
                            md_wrap(ctx, desc_id, &mut e.desc, "*", "*", "kursiv");
                        }
                        if b(ui, RichText::new("</>").monospace(), "Code") {
                            md_wrap(ctx, desc_id, &mut e.desc, "`", "`", "code");
                        }
                        ui.separator();
                        if b(ui, RichText::new("H1").font(bold_font(14.0)), "Überschrift 1") {
                            md_prefix(ctx, desc_id, &mut e.desc, "# ");
                        }
                        if b(ui, RichText::new("H2").font(bold_font(14.0)), "Überschrift 2") {
                            md_prefix(ctx, desc_id, &mut e.desc, "## ");
                        }
                        if b(ui, RichText::new("H3").font(bold_font(14.0)), "Überschrift 3") {
                            md_prefix(ctx, desc_id, &mut e.desc, "### ");
                        }
                        ui.separator();
                        if b(ui, RichText::new("•"), "Aufzählung") {
                            md_prefix(ctx, desc_id, &mut e.desc, "- ");
                        }
                        if b(ui, RichText::new("1."), "Nummerierte Liste") {
                            md_prefix(ctx, desc_id, &mut e.desc, "1. ");
                        }
                        if b(ui, RichText::new("[ ]"), "Checkliste") {
                            md_prefix(ctx, desc_id, &mut e.desc, "- [ ] ");
                        }
                        if b(ui, RichText::new("\""), "Zitat") {
                            md_prefix(ctx, desc_id, &mut e.desc, "> ");
                        }
                        if b(ui, RichText::new("Link"), "Link einfügen") {
                            md_wrap(ctx, desc_id, &mut e.desc, "[", "](https://)", "Text");
                        }
                    });
                    ui.add_space(2.0);

                    // Editor links, Vorschau rechts
                    let mut toggled = None;
                    ui.columns(2, |cols| {
                        egui::ScrollArea::vertical()
                            .id_salt("md_src")
                            .max_height(360.0)
                            .min_scrolled_height(360.0)
                            .show(&mut cols[0], |ui| {
                                ui.add(
                                    TextEdit::multiline(&mut e.desc)
                                        .id(desc_id)
                                        .font(FontId::new(14.0, FontFamily::Monospace))
                                        .desired_rows(17)
                                        .desired_width(f32::INFINITY)
                                        .hint_text("Beschreibung in Markdown …"),
                                );
                            });
                        Frame::none()
                            .fill(COLUMN)
                            .rounding(8.0_f32)
                            .stroke(Stroke::new(1.0_f32, BORDER))
                            .inner_margin(Margin::same(10.0))
                            .show(&mut cols[1], |ui| {
                                egui::ScrollArea::vertical()
                                    .id_salt("md_preview")
                                    .max_height(340.0)
                                    .min_scrolled_height(340.0)
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| {
                                        if e.desc.trim().is_empty() {
                                            ui.label(RichText::new("Vorschau").color(MUTED));
                                        } else {
                                            toggled = render_md(ui, &e.desc);
                                        }
                                    });
                            });
                    });
                    if let Some(line) = toggled {
                        e.desc = toggle_checkbox_line(&e.desc, line);
                    }

                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        let ok = Button::new(RichText::new("Speichern").strong().color(Color32::WHITE))
                            .fill(ACCENT);
                        if ui.add(ok).clicked() {
                            save = true;
                        }
                        if ui.button("Abbrechen").clicked() {
                            cancel = true;
                        }
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let del = Button::new(RichText::new("Löschen").color(Color32::WHITE))
                                .fill(DANGER);
                            if ui.add(del).clicked() {
                                delete = true;
                            }
                        });
                    });
                });
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                cancel = true;
            }
        }
        if save {
            let parsed = self
                .editing
                .as_ref()
                .map(|e| fields_to_due(&e.due_date, &e.due_time));
            match parsed {
                Some(Err(msg)) => {
                    if let Some(e) = self.editing.as_mut() {
                        e.error = msg;
                    }
                }
                Some(Ok(due)) => {
                    if let (Some(e), Some(api), Some(b)) =
                        (self.editing.take(), self.api.clone(), self.selected)
                    {
                        self.spawn(ctx, move || {
                            Msg::Done(api.update_card(
                                b,
                                e.stack,
                                e.id,
                                e.title.trim(),
                                &e.desc,
                                e.order,
                                due,
                            ))
                        });
                    }
                }
                None => {}
            }
        } else if delete {
            if let Some(e) = self.editing.take() {
                self.confirm = Some((
                    format!("Karte „{}“ löschen?", e.title),
                    Danger::Card(e.stack, e.id),
                ));
            }
        } else if cancel {
            self.editing = None;
        }

        // Löschen bestätigen
        let mut yes = false;
        let mut no = false;
        if let Some((text, _)) = self.confirm.as_ref() {
            egui::Window::new("Bestätigen")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(text);
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        let del = Button::new(RichText::new("Löschen").color(Color32::WHITE))
                            .fill(DANGER);
                        if ui.add(del).clicked() {
                            yes = true;
                        }
                        if ui.button("Abbrechen").clicked() {
                            no = true;
                        }
                    });
                });
        }
        if yes {
            if let (Some((_, danger)), Some(api), Some(b)) =
                (self.confirm.take(), self.api.clone(), self.selected)
            {
                self.spawn(ctx, move || match danger {
                    Danger::Card(s, c) => Msg::Done(api.delete_card(b, s, c)),
                    Danger::Stack(s) => Msg::Done(api.delete_stack(b, s)),
                });
            }
        } else if no {
            self.confirm = None;
        }

        for a in actions {
            self.apply(ctx, a);
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.style().visuals.panel_fill != BG {
            apply_theme(ctx);
        }
        let msgs: Vec<Msg> = self.rx.try_iter().collect();
        for m in msgs {
            self.busy = self.busy.saturating_sub(1);
            self.handle(ctx, m);
        }
        match self.screen {
            Screen::Login => self.login_ui(ctx),
            Screen::Main => self.main_ui(ctx),
        }
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 740.0])
            .with_min_inner_size([760.0, 480.0])
            .with_title("Nextcloud Deck"),
        ..Default::default()
    };
    eframe::run_native(
        "Nextcloud Deck",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}
