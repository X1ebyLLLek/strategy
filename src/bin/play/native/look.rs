//! How the window looks: the colours of the era (three skins: the staff's table, parchment,
//! rusted steel), the fonts (OFL, `assets/fonts`), the icons (the sprite `assets/ui/icons.svg`,
//! each cut out as a picture of its own) and the portraits (`assets/portraits`, built into the
//! game).

use std::collections::HashMap;
use std::sync::Arc;
use eframe::egui::{self, Color32, FontData, FontDefinitions, FontFamily};

/// The colours of a skin.
#[derive(Clone, Copy)]
pub struct Skin {
    pub desk: Color32,
    pub desk_ink: Color32,
    pub paper: Color32,
    pub paper2: Color32,
    pub edge: Color32,
    pub ink: Color32,
    pub soft: Color32,
    pub accent: Color32,
}

fn hex(s: &str) -> Color32 {
    let v = u32::from_str_radix(s.trim_start_matches('#'), 16).unwrap_or(0);
    Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

impl Skin {
    /// The colour of words on the accent: white or the desk's dark, whichever reads better.
    pub fn on_accent(&self) -> Color32 {
        let c = |x: Color32| super::uitest::contrast(x, self.accent);
        if c(Color32::WHITE) >= c(self.desk) {
            Color32::WHITE
        } else {
            self.desk
        }
    }

    pub fn of(theme: &str) -> Skin {
        match theme {
            "parchment" => Skin {
                desk: hex("#2d1f15"),
                desk_ink: hex("#f1e3c0"),
                paper: hex("#ecdcb2"),
                paper2: hex("#e3cf9c"),
                edge: hex("#a98a50"),
                ink: hex("#33210f"),
                soft: hex("#5e4628"),
                accent: hex("#861a1a"),
            },
            "rust" => Skin {
                desk: hex("#1b1a18"),
                desk_ink: hex("#ece0c6"),
                paper: hex("#34302a"),
                paper2: hex("#403a32"),
                edge: hex("#7d4a27"),
                ink: hex("#efe5cf"),
                soft: hex("#c7b796"),
                accent: hex("#e0a526"),
            },
            _ => Skin {
                desk: hex("#2b2e25"),
                desk_ink: hex("#ece4cc"),
                paper: hex("#ece4cd"),
                paper2: hex("#e2d8bc"),
                edge: hex("#b9a979"),
                ink: hex("#26241f"),
                soft: hex("#564f40"),
                accent: hex("#9b2a1e"),
            },
        }
    }
}

/// The fonts of the era (OFL): a serif for the text, letters for headings (the staff's narrow
/// grotesque; engraved small capitals for the age of muskets; a heavy block for the wastes), a
/// mono for numbers.
pub fn fonts(ctx: &egui::Context, theme: &str) {
    let mut f = FontDefinitions::default();
    let (body, bold, head): (&'static [u8], &'static [u8], &'static [u8]) = match theme {
        "parchment" => (
            include_bytes!("../../../../assets/fonts/old-standard-regular.ttf"),
            include_bytes!("../../../../assets/fonts/old-standard-bold.ttf"),
            include_bytes!("../../../../assets/fonts/cormorant-sc-semibold.ttf"),
        ),
        "rust" => (
            include_bytes!("../../../../assets/fonts/pt-serif-regular.ttf"),
            include_bytes!("../../../../assets/fonts/pt-serif-bold.ttf"),
            include_bytes!("../../../../assets/fonts/russo-one.ttf"),
        ),
        _ => (
            include_bytes!("../../../../assets/fonts/pt-serif-regular.ttf"),
            include_bytes!("../../../../assets/fonts/pt-serif-bold.ttf"),
            include_bytes!("../../../../assets/fonts/oswald.ttf"),
        ),
    };
    f.font_data.insert("serif".into(), FontData::from_static(body));
    f.font_data.insert("serif-bold".into(), FontData::from_static(bold));
    f.font_data.insert("head".into(), FontData::from_static(head));
    f.font_data.insert("mono".into(), FontData::from_static(include_bytes!("../../../../assets/fonts/pt-mono.ttf")));
    f.families.get_mut(&FontFamily::Proportional).unwrap().insert(0, "serif".into());
    f.families.get_mut(&FontFamily::Monospace).unwrap().insert(0, "mono".into());
    f.families.insert(FontFamily::Name("head".into()), vec!["head".into(), "serif".into()]);
    f.families.insert(FontFamily::Name("bold".into()), vec!["serif-bold".into(), "serif".into()]);
    ctx.set_fonts(f);
}

/// The style of the window in a skin: paper panels, ink text, the accent for what matters.
pub fn style(ctx: &egui::Context, s: &Skin) {
    let mut st = (*ctx.style()).clone();
    let v = &mut st.visuals;
    let dark = s.paper.r() < 128;
    *v = if dark { egui::Visuals::dark() } else { egui::Visuals::light() };
    v.panel_fill = s.paper;
    v.window_fill = s.paper;
    v.extreme_bg_color = s.paper2;
    v.faint_bg_color = s.paper2;
    // the ink on paper; on what is chosen or pressed (the accent) — whichever of ink and paper
    // reads better there
    let on_accent = if super::uitest::contrast(s.paper, s.accent) >= super::uitest::contrast(s.ink, s.accent) { s.paper } else { s.ink };
    v.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, s.ink);
    v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, s.ink);
    v.widgets.hovered.fg_stroke = egui::Stroke::new(1.5, s.ink);
    v.widgets.active.fg_stroke = egui::Stroke::new(1.5, on_accent);
    v.widgets.open.fg_stroke = egui::Stroke::new(1.0, s.ink);
    v.window_stroke = egui::Stroke::new(1.0, s.edge);
    v.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, s.edge);
    v.widgets.inactive.bg_fill = s.paper2;
    v.widgets.inactive.weak_bg_fill = s.paper2;
    v.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, s.edge);
    v.widgets.hovered.bg_fill = s.paper;
    v.widgets.hovered.weak_bg_fill = s.paper;
    v.widgets.hovered.bg_stroke = egui::Stroke::new(1.5, s.accent);
    v.widgets.active.bg_fill = s.accent;
    v.widgets.active.weak_bg_fill = s.accent;
    v.selection.bg_fill = s.accent;
    v.selection.stroke = egui::Stroke::new(1.0, on_accent);
    v.hyperlink_color = s.accent;
    // (a slider's rail filled up to its handle with the accent: seen on any paper)
    v.slider_trailing_fill = true;
    // (a hand over what can be pressed; the caret in ink)
    v.interact_cursor = Some(egui::CursorIcon::PointingHand);
    v.text_cursor.stroke = egui::Stroke::new(2.0, s.ink);
    v.window_shadow = egui::epaint::Shadow { offset: egui::vec2(0.0, 6.0), blur: 18.0, spread: 0.0, color: Color32::from_black_alpha(140) };
    v.popup_shadow = egui::epaint::Shadow { offset: egui::vec2(0.0, 3.0), blur: 10.0, spread: 0.0, color: Color32::from_black_alpha(120) };
    v.window_rounding = 3.0.into();
    v.menu_rounding = 3.0.into();
    v.widgets.inactive.rounding = 2.0.into();
    v.widgets.hovered.rounding = 2.0.into();
    v.widgets.active.rounding = 2.0.into();
    st.spacing.item_spacing = egui::vec2(8.0, 6.0);
    st.spacing.button_padding = egui::vec2(10.0, 4.0);
    st.text_styles.insert(egui::TextStyle::Body, egui::FontId::proportional(15.0));
    st.text_styles.insert(egui::TextStyle::Button, egui::FontId::new(15.0, FontFamily::Name("head".into())));
    st.text_styles.insert(egui::TextStyle::Heading, egui::FontId::new(19.0, FontFamily::Name("head".into())));
    st.text_styles.insert(egui::TextStyle::Small, egui::FontId::proportional(12.5));
    ctx.set_style(st);
}

/// The icons of the sprite (`assets/ui/icons.svg`, made by `tools/make_icons.py`), each an SVG of its own (white, to be tinted), by name
/// (`econ`, `army`…).
pub fn icons() -> HashMap<String, Arc<[u8]>> {
    let mut out = HashMap::new();
    let svg = String::from_utf8_lossy(include_bytes!("../../../../assets/ui/icons.svg")).into_owned();
    let mut rest = svg.as_str();
    while let Some(i) = rest.find("<symbol id=\"i-") {
        rest = &rest[i + 14..];
        let Some(q) = rest.find('"') else { break };
        let name = rest[..q].to_string();
        let Some(vb0) = rest.find("viewBox=\"") else { break };
        let vb = &rest[vb0 + 9..];
        let Some(vb1) = vb.find('"') else { break };
        let view = vb[..vb1].to_string();
        let Some(open) = rest.find('>') else { break };
        let Some(close) = rest.find("</symbol>") else { break };
        let body = &rest[open + 1..close];
        let doc = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{view}\" width=\"64\" height=\"64\" fill=\"#ffffff\">{body}</svg>");
        out.insert(name, Arc::from(doc.into_bytes().into_boxed_slice()));
        rest = &rest[close..];
    }
    out
}

/// An icon as a picture, tinted.
pub fn icon(icons: &HashMap<String, Arc<[u8]>>, name: &str, size: f32, tint: Color32) -> egui::Image<'static> {
    let bytes = icons.get(name).cloned().unwrap_or_else(|| Arc::from(Vec::new().into_boxed_slice()));
    egui::Image::from_bytes(format!("bytes://icon/{name}.svg"), egui::load::Bytes::Shared(bytes)).fit_to_exact_size(egui::vec2(size, size)).tint(tint)
}

/// A portrait (one of `assets/portraits`, as the game gave it to the person), framed.
pub fn portrait(face: &str, w: f32) -> Option<egui::Image<'static>> {
    if face.is_empty() {
        return None;
    }
    let bytes = crate::portraits::picture(face)?;
    Some(egui::Image::from_bytes(format!("bytes://portrait/{face}.jpg"), egui::load::Bytes::Static(bytes)).fit_to_exact_size(egui::vec2(w, w * 1.25)).rounding(2.0))
}
