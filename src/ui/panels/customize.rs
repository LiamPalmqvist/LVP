//! The Customize Player dialog (`2160:6137`, `6587:19923`): a palette of
//! toolbar items above a live, editable preview of the control bar drawn
//! with dashed outlines where the bar sits. Items are dragged from the
//! palette into the bar, around the bar, or out of it to remove them.

use egui::{Align2, Color32, CornerRadius, Pos2, Rect, Sense, Shape, Stroke, Ui, Vec2, pos2, vec2};

use crate::app::App;
use crate::domain::toolbar::{ToolbarItem, ToolbarLayout};
use crate::ui::icons::Icon;
use crate::ui::state::UiState;
use crate::ui::theme::{font, with_alpha};
use crate::ui::widgets::{self, pal};

const DIALOG_W: f32 = 1048.0;
const TILE_W: f32 = 86.0;
const TILE_GAP: f32 = 24.0;
const TILE_ROW: f32 = 120.0;
const COLUMNS: usize = 9;

/// Items of the default bar that can be removed and put back.
const FIXED: &[ToolbarItem] = &[
    ToolbarItem::Previous,
    ToolbarItem::PlayPause,
    ToolbarItem::Next,
    ToolbarItem::CurrentTime,
    ToolbarItem::SeekBar,
    ToolbarItem::Duration,
    ToolbarItem::Volume,
    ToolbarItem::AudioTracks,
    ToolbarItem::SubtitleTracks,
    ToolbarItem::QuickSettings,
    ToolbarItem::Queue,
];

#[derive(Clone, Copy, PartialEq)]
enum Source {
    Palette(ToolbarItem),
    Bar(usize, usize),
}

pub struct CustomizeUi {
    draft: ToolbarLayout,
    drag: Option<Source>,
}

/// Where a dragged item would land.
#[derive(Clone, Copy, PartialEq)]
enum Target {
    Bar(usize, usize),
    Remove,
}

pub fn icon_for(item: ToolbarItem) -> Option<Icon> {
    use ToolbarItem::*;
    Some(match item {
        Previous => Icon::Previous,
        PlayPause => Icon::Play,
        Next => Icon::Next,
        Volume => Icon::VolumeHigh,
        AudioTracks => Icon::Audio,
        SubtitleTracks => Icon::Subtitles,
        QuickSettings => Icon::Settings,
        Queue => Icon::Lists,
        Fullscreen => Icon::Fullscreen,
        Divider => Icon::Divider,
        PlaybackSpeed => Icon::Speed,
        Forward10 => Icon::Forward10Sec,
        Backward10 => Icon::Backward10Sec,
        Forward15 => Icon::Forward15Sec,
        Backward15 => Icon::Backward15Sec,
        Stop => Icon::Stop,
        Repeat => Icon::RepeatRight,
        Shuffle => Icon::ShuffleRight,
        Record => Icon::Record,
        Snapshot => Icon::Snapshot,
        PictureInPicture => Icon::Pip,
        Hdr => Icon::Hdr,
        Zoom => Icon::Zoom,
        RightSidebar => Icon::SidebarRight,
        LeftSidebar => Icon::SidebarLeft,
        More => Icon::More,
        QuitVlc => Icon::Quit,
        VlcSettings => Icon::Preferences,
        GoHome => Icon::Home,
        Thumbnail => Icon::Thumbnail,
        FileName | ArtistName | FolderName | PlaylistName | Properties => Icon::Info,
        Replay => Icon::Replay,
        Chapters => Icon::Chapters,
        Favorite => Icon::Favorite2tone,
        Bookmark => Icon::Bookmark,
        Bookmarks => Icon::Bookmarks,
        Playlists => Icon::Playlist,
        FrameStep => Icon::Next2tone,
        AbLoop => Icon::Refresh,
        CurrentTime | Duration | SeekBar | Spacer => return None,
    })
}

/// Items offered in the palette: everything not already in the bar.
fn palette(draft: &ToolbarLayout) -> Vec<ToolbarItem> {
    let mut items: Vec<ToolbarItem> = ToolbarItem::PALETTE.to_vec();
    items.extend(FIXED.iter().copied());
    items.into_iter().filter(|i| i.repeatable() || !draft.contains(*i)).collect()
}

fn dashed_rect(ui: &Ui, rect: Rect, color: Color32) {
    let s = Stroke::new(1.0, color);
    let c = [rect.left_top(), rect.right_top(), rect.right_bottom(), rect.left_bottom(), rect.left_top()];
    for w in c.windows(2) {
        ui.painter().extend(Shape::dashed_line(&[w[0], w[1]], s, 3.0, 3.0));
    }
}

/// The glyph of an item: its icon, a time, a name or a seek track.
fn paint_item(ui: &Ui, item: ToolbarItem, rect: Rect, color: Color32) {
    let p = pal(ui.ctx());
    match item {
        ToolbarItem::Divider => {
            ui.painter().rect_filled(Rect::from_center_size(rect.center(), vec2(1.0, 20.0)), 0, p.divider_bright);
        }
        ToolbarItem::CurrentTime | ToolbarItem::Duration => {
            ui.painter().text(rect.center(), Align2::CENTER_CENTER, "00:00:00", font::regular(14.0), color);
        }
        ToolbarItem::SeekBar => {
            let track = Rect::from_center_size(rect.center(), vec2(rect.width() - 16.0, 8.0));
            ui.painter().rect_filled(track, CornerRadius::same(4), p.bg_b2);
            let fill = Rect::from_min_size(track.min, vec2(track.width() * 0.08, track.height()));
            ui.painter().rect_filled(fill, CornerRadius::same(4), p.bg_accent);
        }
        ToolbarItem::Spacer => {}
        ToolbarItem::PlayPause => {
            let pr = Rect::from_center_size(rect.center(), vec2(54.0, 28.0));
            ui.painter().rect_filled(pr, 6, p.accent);
            let fg = if p.dark { p.fg_b1 } else { Color32::WHITE };
            widgets::paint_icon(ui, Rect::from_center_size(pr.center(), Vec2::splat(24.0)), Icon::Play, fg);
        }
        ToolbarItem::FileName | ToolbarItem::ArtistName | ToolbarItem::FolderName | ToolbarItem::PlaylistName => {
            widgets::text_clipped(
                ui,
                rect.center(),
                Align2::CENTER_CENTER,
                item.label(),
                font::regular(14.0),
                color,
                rect.width() - 8.0,
            );
        }
        _ => {
            if let Some(i) = icon_for(item) {
                widgets::paint_icon(ui, Rect::from_center_size(rect.center(), Vec2::splat(24.0)), i, color);
            }
        }
    }
}

fn item_width(item: ToolbarItem) -> Option<f32> {
    use ToolbarItem::*;
    Some(match item {
        SeekBar | Spacer => return None,
        PlayPause => 58.0,
        CurrentTime | Duration => 76.0,
        Volume => 120.0,
        Divider => 14.0,
        PlaybackSpeed => 48.0,
        Thumbnail => 50.0,
        FileName | ArtistName | FolderName | PlaylistName => 110.0,
        _ => 30.0,
    })
}

/// Height of the bar for a layout (mirrors the control bar).
fn bar_height(l: &ToolbarLayout) -> f32 {
    let rows = l.rows.len().max(1) as f32;
    if rows > 1.0 { 40.0 + (rows - 1.0) * 50.0 } else { 54.0 }
}

/// Lays out one row; returns each item's rect.
fn layout_row(rect: Rect, row: &[ToolbarItem]) -> Vec<Rect> {
    let gap = 10.0;
    let inner = rect.shrink2(vec2(20.0, 0.0));
    let fixed: f32 = row.iter().filter_map(|i| item_width(*i)).sum::<f32>() + gap * row.len().saturating_sub(1) as f32;
    let seek_n = row.iter().filter(|i| **i == ToolbarItem::SeekBar).count() as f32;
    let spacer_n = row.iter().filter(|i| **i == ToolbarItem::Spacer).count() as f32;
    let free = (inner.width() - fixed).max(0.0);
    let (seek_w, spacer_w) =
        if seek_n > 0.0 { (free / seek_n, 24.0) } else { (0.0, (free / spacer_n.max(1.0)).max(24.0)) };
    let mut x = inner.min.x;
    row.iter()
        .map(|i| {
            let w = match i {
                ToolbarItem::SeekBar => seek_w.max(60.0),
                ToolbarItem::Spacer => spacer_w,
                _ => item_width(*i).unwrap_or(30.0),
            };
            let r = Rect::from_min_size(pos2(x, rect.center().y - 16.0), vec2(w, 32.0));
            x += w + gap;
            r
        })
        .collect()
}

pub fn show(ui: &mut Ui, bar: Rect, app: &mut App, st: &mut UiState) {
    let ctx = ui.ctx().clone();
    let p = pal(&ctx);
    st.panel = None;
    let cu = st
        .panels
        .customize
        .get_or_insert_with(|| CustomizeUi { draft: app.settings.interface.toolbar.clone(), drag: None });
    let screen = ctx.content_rect();

    // Dim everything and swallow clicks below the dialog and the bar.
    egui::Area::new(egui::Id::new("customize-backdrop")).order(egui::Order::Middle).fixed_pos(screen.min).show(
        &ctx,
        |ui| {
            let (r, _) = ui.allocate_exact_size(screen.size(), Sense::click_and_drag());
            ui.painter().rect_filled(r, 0, with_alpha(Color32::BLACK, 0.45));
        },
    );

    // The bar preview, where the real bar is.
    let bh = bar_height(&cu.draft);
    let bar_rect = Rect::from_min_max(pos2(bar.min.x, bar.max.y - bh), bar.max);
    let mut item_rects: Vec<Vec<(Rect, ToolbarItem)>> = Vec::new();
    let mut row_rects: Vec<Rect> = Vec::new();
    let mut remove_at = None;
    egui::Area::new(egui::Id::new("customize-bar")).order(egui::Order::Foreground).fixed_pos(bar_rect.min).show(
        &ctx,
        |ui| {
            let (r, _) = ui.allocate_exact_size(bar_rect.size(), Sense::hover());
            ui.painter().rect_filled(r, 0, p.bg_a4);
            let n = cu.draft.rows.len().max(1);
            let mut y = r.min.y;
            for (ri, row) in cu.draft.rows.iter().enumerate() {
                let h = if n == 1 {
                    54.0
                } else if ri == 0 {
                    40.0
                } else {
                    50.0
                };
                let rr = Rect::from_min_size(pos2(r.min.x, y), vec2(r.width(), h));
                row_rects.push(rr);
                let rects = layout_row(rr, row);
                let mut items = Vec::new();
                for (ii, (ir, item)) in rects.iter().zip(row.iter()).enumerate() {
                    let resp = ui.interact(*ir, ui.id().with(("bar-item", ri, ii)), Sense::click_and_drag());
                    let dragging = cu.drag == Some(Source::Bar(ri, ii));
                    if resp.hovered() && !dragging {
                        ui.painter().rect_filled(*ir, 4, p.hover);
                    }
                    let col = if dragging { p.fg_a4 } else { p.fg_a2 };
                    dashed_rect(ui, *ir, if dragging { p.fg_a4 } else { p.fg_a3 });
                    paint_item(ui, *item, *ir, col);
                    if resp.drag_started() {
                        cu.drag = Some(Source::Bar(ri, ii));
                    }
                    let resp = resp.on_hover_text(format!("{}: drag to move, right-click to remove", item.label()));
                    if resp.secondary_clicked() {
                        remove_at = Some((ri, ii));
                    }
                    items.push((*ir, *item));
                }
                item_rects.push(items);
                y += h;
            }
        },
    );
    if let Some((r, i)) = remove_at {
        cu.draft.remove_at(r, i);
    }

    // The dialog.
    let items = palette(&cu.draft);
    let tile_rows = items.len().div_ceil(COLUMNS).max(1);
    let body_h = 34.0 * 2.0 + (tile_rows as f32 - 1.0) * TILE_ROW + 82.0;
    let size = vec2(DIALOG_W, 94.0 + body_h + 72.0);
    let space = Rect::from_min_max(screen.min, pos2(screen.max.x, bar_rect.min.y));
    let pos = pos2(space.center().x - size.x / 2.0, (space.center().y - size.y / 2.0).max(space.min.y + 8.0));
    let mut save = false;
    let mut cancel = false;
    let mut add_click = None;
    let dialog = egui::Area::new(egui::Id::new("customize-dialog")).order(egui::Order::Foreground).fixed_pos(pos).show(
        &ctx,
        |ui| {
            egui::Frame::new()
                .fill(p.bg_b1_t)
                .corner_radius(14)
                .stroke(Stroke::new(1.0, p.divider_dim))
                .shadow(egui::epaint::Shadow {
                    offset: [0, 8],
                    blur: 30,
                    spread: 0,
                    color: Color32::from_black_alpha(118),
                })
                .show(ui, |ui| {
                    ui.set_min_size(size);
                    ui.set_max_size(size);
                    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                    let (head, _) = ui.allocate_exact_size(vec2(size.x, 94.0), Sense::hover());
                    ui.painter().rect_filled(head, CornerRadius { nw: 14, ne: 14, sw: 0, se: 0 }, p.bg_b2);
                    ui.painter().text(
                        pos2(head.min.x + 19.0, head.min.y + 24.0),
                        Align2::LEFT_CENTER,
                        "Customize Player",
                        font::regular(14.0),
                        p.fg_a2,
                    );
                    ui.painter().text(
                        pos2(head.min.x + 19.0, head.min.y + 68.0),
                        Align2::LEFT_CENTER,
                        "Drag items around and out of the box below to re-order them. Drag items into the box to \
                         remove them from the player.",
                        font::regular(14.0),
                        p.fg_a3,
                    );
                    let close = Rect::from_min_size(pos2(head.max.x - 41.0, head.min.y + 8.0), vec2(32.0, 32.0));
                    let cr = ui.interact(close, ui.id().with("close"), Sense::click());
                    if cr.hovered() {
                        ui.painter().rect_filled(close, 4, p.hover);
                    }
                    widgets::paint_icon(
                        ui,
                        Rect::from_center_size(close.center(), Vec2::splat(24.0)),
                        Icon::Close,
                        p.fg_a2,
                    );
                    if cr.on_hover_text("Cancel").clicked() {
                        cancel = true;
                    }
                    // Palette tiles.
                    let (body, _) = ui.allocate_exact_size(vec2(size.x, body_h), Sense::hover());
                    let x0 = body.min.x + 34.0;
                    for (k, item) in items.iter().enumerate() {
                        let (c, r) = (k % COLUMNS, k / COLUMNS);
                        let tile = Rect::from_min_size(
                            pos2(x0 + c as f32 * (TILE_W + TILE_GAP), body.min.y + 34.0 + r as f32 * TILE_ROW),
                            vec2(TILE_W, 100.0),
                        );
                        let resp = ui.interact(tile, ui.id().with(("tile", k)), Sense::click_and_drag());
                        let boxr = Rect::from_center_size(pos2(tile.center().x, tile.min.y + 16.0), vec2(30.0, 32.0));
                        if resp.hovered() {
                            ui.painter().rect_filled(boxr.expand(4.0), 6, p.hover);
                        }
                        dashed_rect(ui, boxr, p.fg_a3);
                        let glyph = match item {
                            ToolbarItem::CurrentTime | ToolbarItem::Duration => Some(Icon::Recent),
                            ToolbarItem::SeekBar => Some(Icon::Divider),
                            _ => icon_for(*item),
                        };
                        if let Some(g) = glyph {
                            widgets::paint_icon(
                                ui,
                                Rect::from_center_size(boxr.center(), Vec2::splat(24.0)),
                                g,
                                p.fg_a2,
                            );
                        }
                        let galley = ui.painter().layout(
                            item.label().to_string(),
                            font::regular(14.0),
                            if resp.hovered() { p.fg1 } else { p.fg_a2 },
                            TILE_W,
                        );
                        let gp = pos2(tile.center().x - galley.size().x / 2.0, tile.min.y + 48.0);
                        ui.painter().galley(gp, galley, p.fg_a2);
                        if resp.drag_started() {
                            cu.drag = Some(Source::Palette(*item));
                        }
                        if resp.on_hover_text("Drag into the bar, or click to add at the end").clicked() {
                            add_click = Some(*item);
                        }
                    }
                    // Footer.
                    let (foot, _) = ui.allocate_exact_size(vec2(size.x, 72.0), Sense::hover());
                    let mut fui = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(foot.shrink(20.0))
                            .layout(egui::Layout::left_to_right(egui::Align::Center)),
                    );
                    fui.spacing_mut().item_spacing.x = 12.0;
                    if widgets::button_with(&mut fui, None, "Cancel", widgets::ButtonKind::Secondary, vec2(100.0, 32.0))
                        .clicked()
                    {
                        cancel = true;
                    }
                    if widgets::button_with(&mut fui, None, "Reset", widgets::ButtonKind::Ghost, vec2(100.0, 32.0))
                        .on_hover_text("Restore the default bar")
                        .clicked()
                    {
                        cu.draft = ToolbarLayout::default();
                    }
                    let mut two = cu.draft.rows.len() > 1;
                    fui.label(egui::RichText::new("Two rows").font(font::regular(14.0)).color(p.fg_a3));
                    if widgets::toggle(&mut fui, &mut two).changed() {
                        cu.draft = if two {
                            ToolbarLayout::two_row()
                        } else {
                            ToolbarLayout { rows: vec![cu.draft.rows.concat()] }
                        };
                    }
                    fui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if save_button(ui).clicked() {
                            save = true;
                        }
                    });
                });
        },
    );
    if let Some(item) = add_click {
        let last = cu.draft.rows.len().saturating_sub(1);
        let n = cu.draft.rows.get(last).map(|r| r.len()).unwrap_or(0);
        cu.draft.insert(item, last, n);
    }

    // Dragging: a ghost under the pointer and a marker where it will land.
    if let Some(src) = cu.drag {
        let pointer = ctx.pointer_interact_pos();
        let target = pointer.map(|pt| drop_target(pt, &row_rects, &item_rects)).unwrap_or(Target::Remove);
        let item = match src {
            Source::Palette(i) => Some(i),
            Source::Bar(r, i) => cu.draft.rows.get(r).and_then(|row| row.get(i)).copied(),
        };
        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("customize-drag")));
        if let (Some(pt), Some(item)) = (pointer, item) {
            let gr = Rect::from_center_size(pt, vec2(item_width(item).unwrap_or(60.0).max(30.0), 32.0));
            painter.rect_filled(gr, 4, with_alpha(p.bg_a1, 0.9));
            painter.rect_stroke(gr, 4, Stroke::new(1.0, p.accent), egui::StrokeKind::Inside);
            if let Some(tex) =
                icon_for(item).and_then(|i| crate::ui::icons::texture(&ctx, i, 24.0, widgets::colors(&p, p.fg1)))
            {
                let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
                painter.image(tex.id(), Rect::from_center_size(gr.center(), Vec2::splat(24.0)), uv, Color32::WHITE);
            } else {
                painter.text(gr.center(), Align2::CENTER_CENTER, item.label(), font::regular(12.0), p.fg1);
            }
        }
        if let Target::Bar(r, i) = target {
            let x = marker_x(r, i, &row_rects, &item_rects);
            let rr = row_rects[r];
            painter.line_segment([pos2(x, rr.min.y + 8.0), pos2(x, rr.max.y - 8.0)], Stroke::new(2.0, p.accent));
        }
        if !ctx.input(|i| i.pointer.any_down()) {
            cu.drag = None;
            if let Some(item) = item {
                apply_drop(&mut cu.draft, src, item, target);
            }
        }
    }
    let _ = dialog;

    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        cancel = true;
    }
    if save {
        let draft = cu.draft.clone();
        app.update_settings(|s| s.interface.toolbar = draft);
        st.panels.customize = None;
        st.panels.customize_open = false;
    } else if cancel {
        st.panels.customize = None;
        st.panels.customize_open = false;
    }
}

fn save_button(ui: &mut Ui) -> egui::Response {
    let p = pal(ui.ctx());
    let (rect, resp) = ui.allocate_exact_size(vec2(100.0, 32.0), Sense::click());
    let bg = if resp.hovered() { with_alpha(p.accent, 0.3) } else { p.bg_accent };
    ui.painter().rect_filled(rect, 6, bg);
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, "Save", font::semibold(14.0), p.accent);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn drop_target(pt: Pos2, rows: &[Rect], items: &[Vec<(Rect, ToolbarItem)>]) -> Target {
    for (r, rr) in rows.iter().enumerate() {
        if rr.contains(pt) {
            let i = items[r].iter().filter(|(ir, _)| ir.center().x < pt.x).count();
            return Target::Bar(r, i);
        }
    }
    Target::Remove
}

fn marker_x(r: usize, i: usize, rows: &[Rect], items: &[Vec<(Rect, ToolbarItem)>]) -> f32 {
    let row = &items[r];
    if row.is_empty() {
        rows[r].min.x + 20.0
    } else if i == 0 {
        row[0].0.min.x - 5.0
    } else {
        row[(i - 1).min(row.len() - 1)].0.max.x + 5.0
    }
}

fn apply_drop(draft: &mut ToolbarLayout, src: Source, item: ToolbarItem, target: Target) {
    match (src, target) {
        (Source::Palette(_), Target::Bar(r, i)) => draft.insert(item, r, i),
        (Source::Palette(_), Target::Remove) => {}
        (Source::Bar(r0, i0), Target::Bar(r, i)) => {
            if r0 == r && (i == i0 || i == i0 + 1) {
                return;
            }
            let rows_before = draft.rows.len();
            draft.remove_at(r0, i0);
            let mut row = r;
            let mut idx = i;
            if draft.rows.len() < rows_before && row > r0 {
                row -= 1;
            } else if row == r0 && i > i0 {
                idx -= 1;
            }
            draft.insert(item, row, idx);
        }
        (Source::Bar(r0, i0), Target::Remove) => {
            draft.remove_at(r0, i0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moving_within_a_row_keeps_everything_else() {
        let mut l = ToolbarLayout::default();
        let first = l.rows[0][0];
        apply_drop(&mut l, Source::Bar(0, 0), first, Target::Bar(0, 3));
        assert_eq!(l.rows[0][2], first);
        assert_eq!(l.rows[0].len(), ToolbarLayout::default().rows[0].len());
    }

    #[test]
    fn dragging_out_removes_and_palette_offers_it_again() {
        let mut l = ToolbarLayout::default();
        assert!(!palette(&l).contains(&ToolbarItem::Volume));
        let i = l.rows[0].iter().position(|x| *x == ToolbarItem::Volume).unwrap();
        apply_drop(&mut l, Source::Bar(0, i), ToolbarItem::Volume, Target::Remove);
        assert!(!l.contains(ToolbarItem::Volume));
        assert!(palette(&l).contains(&ToolbarItem::Volume));
        apply_drop(&mut l, Source::Palette(ToolbarItem::Stop), ToolbarItem::Stop, Target::Bar(0, 1));
        assert_eq!(l.rows[0][1], ToolbarItem::Stop);
    }

    #[test]
    fn moving_to_another_row() {
        let mut l = ToolbarLayout::two_row();
        let item = l.rows[1][0];
        apply_drop(&mut l, Source::Bar(1, 0), item, Target::Bar(0, 0));
        assert_eq!(l.rows[0][0], item);
        assert!(!l.rows[1].contains(&item));
    }
}
