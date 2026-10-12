//! Reading and writing playlist files: M3U/M3U8, PLS and XSPF.

use quick_xml::Reader;
use quick_xml::events::Event;

use super::media::{MediaItem, MediaSource, path_to_file_url};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaylistFormat {
    M3u,
    Pls,
    Xspf,
}

impl PlaylistFormat {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_ascii_lowercase().as_str() {
            "m3u" | "m3u8" => Some(Self::M3u),
            "pls" => Some(Self::Pls),
            "xspf" => Some(Self::Xspf),
            _ => None,
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Self::M3u => "m3u8",
            Self::Pls => "pls",
            Self::Xspf => "xspf",
        }
    }
}

fn resolve(entry: &str, base: Option<&std::path::Path>) -> MediaSource {
    let e = entry.trim();
    if e.contains("://") {
        return MediaSource::from_user_input(e);
    }
    let p = std::path::Path::new(e);
    match base {
        Some(b) if p.is_relative() => MediaSource::File(b.join(p)),
        _ => MediaSource::File(p.to_path_buf()),
    }
}

/// Parses playlist text. Relative paths resolve against `base` (the
/// playlist's folder).
pub fn parse(text: &str, format: PlaylistFormat, base: Option<&std::path::Path>) -> Vec<MediaItem> {
    match format {
        PlaylistFormat::M3u => parse_m3u(text, base),
        PlaylistFormat::Pls => parse_pls(text, base),
        PlaylistFormat::Xspf => parse_xspf(text, base),
    }
}

fn parse_m3u(text: &str, base: Option<&std::path::Path>) -> Vec<MediaItem> {
    let mut out = Vec::new();
    let mut pending: Option<(Option<i64>, String)> = None;
    for line in text.lines().map(str::trim) {
        if line.is_empty() {
            continue;
        }
        if let Some(info) = line.strip_prefix("#EXTINF:") {
            let (dur, title) = info.split_once(',').unwrap_or((info, ""));
            let secs: Option<f64> = dur.split_whitespace().next().and_then(|d| d.parse().ok());
            pending = Some((secs.filter(|s| *s > 0.0).map(|s| (s * 1000.0) as i64), title.trim().to_string()));
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        let mut item = MediaItem::new(resolve(line, base));
        if let Some((dur, title)) = pending.take() {
            item.meta.duration_ms = dur;
            if !title.is_empty() {
                item.meta.title = Some(title);
            }
        }
        out.push(item);
    }
    out
}

fn parse_pls(text: &str, base: Option<&std::path::Path>) -> Vec<MediaItem> {
    use std::collections::BTreeMap;
    let mut files: BTreeMap<u32, MediaItem> = BTreeMap::new();
    let mut titles: BTreeMap<u32, String> = BTreeMap::new();
    let mut lengths: BTreeMap<u32, i64> = BTreeMap::new();
    for line in text.lines().map(str::trim) {
        let Some((k, v)) = line.split_once('=') else { continue };
        let lk = k.to_ascii_lowercase();
        let num = |prefix: &str| lk.strip_prefix(prefix).and_then(|n| n.parse::<u32>().ok());
        if let Some(n) = num("file") {
            files.insert(n, MediaItem::new(resolve(v, base)));
        } else if let Some(n) = num("title") {
            titles.insert(n, v.trim().to_string());
        } else if let Some(n) = num("length")
            && let Ok(s) = v.trim().parse::<i64>()
            && s > 0
        {
            lengths.insert(n, s * 1000);
        }
    }
    files
        .into_iter()
        .map(|(n, mut item)| {
            item.meta.title = titles.remove(&n).filter(|t| !t.is_empty());
            item.meta.duration_ms = lengths.remove(&n);
            item
        })
        .collect()
}

fn parse_xspf(text: &str, base: Option<&std::path::Path>) -> Vec<MediaItem> {
    // No trimming per text piece: spaces next to entities matter.
    let mut reader = Reader::from_str(text);
    let mut out = Vec::new();
    let mut cur: Option<MediaItem> = None;
    let mut options: Vec<String> = Vec::new();
    // Text arrives in pieces around entity references, so collect it until
    // the element closes.
    let mut buf = String::new();
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                if e.local_name().as_ref() == b"track" {
                    cur = Some(MediaItem::new(MediaSource::Location(String::new())));
                    options.clear();
                }
                buf.clear();
            }
            Ok(Event::Text(t)) => buf.push_str(&t.decode().map(|c| c.into_owned()).unwrap_or_default()),
            Ok(Event::GeneralRef(r)) => {
                let name = String::from_utf8_lossy(r.as_ref()).into_owned();
                let ch = match name.as_str() {
                    "amp" => Some('&'),
                    "lt" => Some('<'),
                    "gt" => Some('>'),
                    "quot" => Some('"'),
                    "apos" => Some('\''),
                    n if n.starts_with("#x") => u32::from_str_radix(&n[2..], 16).ok().and_then(char::from_u32),
                    n if n.starts_with('#') => n[1..].parse().ok().and_then(char::from_u32),
                    _ => None,
                };
                if let Some(c) = ch {
                    buf.push(c);
                }
            }
            Ok(Event::End(e)) => {
                let name = String::from_utf8_lossy(e.local_name().as_ref()).into_owned();
                let v = std::mem::take(&mut buf).trim().to_string();
                if name == "track" {
                    if let Some(mut item) = cur.take()
                        && item.source != MediaSource::Location(String::new())
                    {
                        item.options = std::mem::take(&mut options);
                        out.push(item);
                    }
                } else if let Some(item) = cur.as_mut() {
                    match name.as_str() {
                        "location" => item.source = resolve(&v, base),
                        "title" => item.meta.title = Some(v),
                        "creator" => item.meta.artist = Some(v),
                        "album" => item.meta.album = Some(v),
                        "duration" => item.meta.duration_ms = v.trim().parse().ok(),
                        "image" => item.meta.artwork_url = Some(v),
                        "option" => options.push(format!(":{v}")),
                        _ => {}
                    }
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    out
}

fn location(item: &MediaItem) -> String {
    match &item.source {
        MediaSource::File(p) => path_to_file_url(p),
        MediaSource::Location(l) => l.clone(),
    }
}

/// Serialises a playlist. M3U writes plain paths for local files so other
/// players can read it; XSPF uses URIs as the format requires.
pub fn write(name: &str, items: &[MediaItem], format: PlaylistFormat) -> String {
    match format {
        PlaylistFormat::M3u => {
            let mut s = String::from("#EXTM3U\n");
            s.push_str(&format!("#PLAYLIST:{name}\n"));
            for i in items {
                let secs = i.meta.duration_ms.map(|d| d / 1000).unwrap_or(-1);
                s.push_str(&format!("#EXTINF:{secs},{}\n", i.title()));
                match &i.source {
                    MediaSource::File(p) => s.push_str(&p.to_string_lossy()),
                    MediaSource::Location(l) => s.push_str(l),
                }
                s.push('\n');
            }
            s
        }
        PlaylistFormat::Pls => {
            let mut s = String::from("[playlist]\n");
            for (n, i) in items.iter().enumerate() {
                let n = n + 1;
                let file = match &i.source {
                    MediaSource::File(p) => p.to_string_lossy().into_owned(),
                    MediaSource::Location(l) => l.clone(),
                };
                s.push_str(&format!("File{n}={file}\nTitle{n}={}\n", i.title()));
                s.push_str(&format!("Length{n}={}\n", i.meta.duration_ms.map(|d| d / 1000).unwrap_or(-1)));
            }
            s.push_str(&format!("NumberOfEntries={}\nVersion=2\n", items.len()));
            s
        }
        PlaylistFormat::Xspf => {
            let esc = |v: &str| quick_xml::escape::escape(v).into_owned();
            let mut s = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
            s.push_str("<playlist xmlns=\"http://xspf.org/ns/0/\" xmlns:vlc=\"http://www.videolan.org/vlc/playlist/ns/0/\" version=\"1\">\n");
            s.push_str(&format!("\t<title>{}</title>\n\t<trackList>\n", esc(name)));
            for (n, i) in items.iter().enumerate() {
                s.push_str("\t\t<track>\n");
                s.push_str(&format!("\t\t\t<location>{}</location>\n", esc(&location(i))));
                s.push_str(&format!("\t\t\t<title>{}</title>\n", esc(&i.title())));
                if let Some(a) = &i.meta.artist {
                    s.push_str(&format!("\t\t\t<creator>{}</creator>\n", esc(a)));
                }
                if let Some(a) = &i.meta.album {
                    s.push_str(&format!("\t\t\t<album>{}</album>\n", esc(a)));
                }
                if let Some(d) = i.meta.duration_ms {
                    s.push_str(&format!("\t\t\t<duration>{d}</duration>\n"));
                }
                s.push_str("\t\t\t<extension application=\"http://www.videolan.org/vlc/playlist/0\">\n");
                s.push_str(&format!("\t\t\t\t<vlc:id>{n}</vlc:id>\n"));
                for o in &i.options {
                    s.push_str(&format!("\t\t\t\t<vlc:option>{}</vlc:option>\n", esc(o.trim_start_matches(':'))));
                }
                s.push_str("\t\t\t</extension>\n\t\t</track>\n");
            }
            s.push_str("\t</trackList>\n</playlist>\n");
            s
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn items() -> Vec<MediaItem> {
        let mut a = MediaItem::file("/music/a & b.mp3");
        a.meta.title = Some("A & B".into());
        a.meta.duration_ms = Some(61_000);
        a.meta.artist = Some("Someone".into());
        let b = MediaItem::new(MediaSource::Location("https://radio.example/stream".into()));
        vec![a, b]
    }

    #[test]
    fn round_trip_all_formats() {
        for f in [PlaylistFormat::M3u, PlaylistFormat::Pls, PlaylistFormat::Xspf] {
            let text = write("Mix", &items(), f);
            let back = parse(&text, f, None);
            assert_eq!(back.len(), 2, "{f:?}");
            assert_eq!(back[0].source, MediaSource::File("/music/a & b.mp3".into()), "{f:?}");
            assert_eq!(back[0].meta.title.as_deref(), Some("A & B"), "{f:?}");
            assert_eq!(back[0].meta.duration_ms, Some(61_000), "{f:?}");
            assert_eq!(back[1].source.mrl(), "https://radio.example/stream", "{f:?}");
        }
    }

    #[test]
    fn m3u_relative_paths() {
        let v = parse("#EXTM3U\nsub/song.mp3\n", PlaylistFormat::M3u, Some(Path::new("/lists")));
        assert_eq!(v[0].source, MediaSource::File("/lists/sub/song.mp3".into()));
    }
}
