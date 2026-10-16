//! Network pages: what a discovery service found, and containers (shares,
//! UPnP folders) browsed on a background thread so the UI never blocks.

use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender, channel};

use crate::app::App;
use crate::app::ports::BrowseItem;
use crate::domain::library::PlayRecord;
use crate::domain::media::{MediaItem, MediaSource};
use crate::ui::icons::Icon;
use crate::ui::state::UiState;

use super::Page;
use super::model::{self, PageKind, PageModel, Shape};

enum Browse {
    Loading,
    Done(Vec<BrowseItem>),
    Failed(String),
}

type Reply = (String, Result<Vec<BrowseItem>, String>);

#[derive(Default)]
pub struct NetState {
    folders: HashMap<String, Browse>,
    chan: Option<(Sender<Reply>, Receiver<Reply>)>,
}

/// Collects finished browses; true when something arrived (the page must
/// be rebuilt).
pub fn changed(net: &mut NetState) -> bool {
    let Some((_, rx)) = &net.chan else { return false };
    let mut any = false;
    while let Ok((mrl, r)) = rx.try_recv() {
        net.folders.insert(
            mrl,
            match r {
                Ok(v) => Browse::Done(v),
                Err(e) => Browse::Failed(e),
            },
        );
        any = true;
    }
    any
}

/// Forgets browse results so containers are listed again.
pub fn refresh(net: &mut NetState) {
    net.folders.retain(|_, b| matches!(b, Browse::Loading));
}

fn crumbs(last: &str) -> Vec<(String, Option<Page>)> {
    vec![model::crumb_of("Network", Some(Page::Network)), model::crumb_of(last, None)]
}

/// Turns browse results into a page: containers first, then media.
fn items_model(kind: PageKind, title: &str, mut items: Vec<BrowseItem>, q: &str) -> PageModel {
    items.sort_by(|a, b| {
        b.is_container.cmp(&a.is_container).then_with(|| crate::domain::library::natural_cmp(&a.name, &b.name))
    });
    let mut m = model::model_of(kind, crumbs(title));
    m.sortable = false;
    m.empty = ("Nothing found yet", "Devices and shares appear here as they are discovered.");
    let mut sec = model::media_section_of(Vec::new(), Shape::Square, Some(title.to_string()));
    for it in items.into_iter().filter(|i| model::keep_q(&i.name, q)) {
        if it.is_container {
            let page = Page::NetFolder { mrl: it.source.mrl(), name: it.name.clone() };
            sec.tiles.push(model::nav_tile_of(it.name, Icon::Folder, page));
        } else {
            let mut item = MediaItem::new(it.source);
            item.meta.title = Some(it.name);
            let i = sec.playable.len();
            let mut t = model::media_tile(&item, PlayRecord::default(), i);
            if t.icon == Icon::Video && item.source.kind() == crate::domain::media::MediaKind::Stream {
                t.icon = Icon::Streaming;
            }
            sec.tiles.push(t);
            sec.playable.push(item);
        }
    }
    m.sections.push(sec);
    m
}

/// A discovery service (UPnP, SMB, SAP, Bonjour, podcasts...).
pub fn service(app: &mut App, _st: &mut UiState, name: &str, q: &str) -> PageModel {
    let title = app.engine.services().into_iter().find(|s| s.name == name).map(|s| s.long_name).unwrap_or(name.into());
    let items = app.engine.service_items(name);
    let mut m = items_model(PageKind::Service, &title, items, q);
    // Discovery keeps finding things; keep the spinner while it is empty.
    m.loading = m.count() == 0;
    m
}

/// A container on the network, listed on a worker thread.
pub fn folder(app: &mut App, st: &mut UiState, mrl: &str, name: &str, q: &str) -> PageModel {
    let net = &mut st.library.net;
    match net.folders.get(mrl) {
        None => {
            let (tx, _) = net.chan.get_or_insert_with(channel);
            let tx = tx.clone();
            let engine = app.engine.clone();
            let src = MediaSource::from_user_input(mrl);
            let key = mrl.to_string();
            std::thread::spawn(move || {
                let r = engine.browse(&src).map_err(|e| format!("{e:#}"));
                let _ = tx.send((key, r));
            });
            net.folders.insert(mrl.to_string(), Browse::Loading);
            loading(name)
        }
        Some(Browse::Loading) => loading(name),
        Some(Browse::Done(items)) => {
            let mut m = items_model(PageKind::NetFolder, name, items.clone(), q);
            m.empty = ("This folder is empty", "There is nothing here VLC can play.");
            m
        }
        Some(Browse::Failed(e)) => {
            let mut m = model::model_of(PageKind::NetFolder, crumbs(name));
            m.error = Some(format!("Could not open {name}: {e}"));
            m.sections.push(model::media_section_of(Vec::new(), Shape::Square, None));
            m
        }
    }
}

fn loading(name: &str) -> PageModel {
    let mut m = model::model_of(PageKind::NetFolder, crumbs(name));
    m.loading = true;
    m.sortable = false;
    m.sections.push(model::media_section_of(Vec::new(), Shape::Square, None));
    m
}
