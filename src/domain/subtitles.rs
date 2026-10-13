//! Batch subtitle matching for "Add Subtitles to Playlist/Folder": which
//! subtitle files belong to which videos, under VLC's subtitle autodetection
//! levels, and what a copied subtitle should be called so VLC loads it.

use std::path::{Path, PathBuf};

use super::media::{MediaKind, kind_for_path, matching_subtitles};

/// VLC's "sub-autodetect-fuzzy" levels, as named in the design's
/// "sub filename" dropdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SubtitleMatch {
    Off,
    AnyTitle,
    ContainsTitle,
    #[default]
    ExtraCharacters,
    Exact,
}

impl SubtitleMatch {
    pub const ALL: [SubtitleMatch; 5] = [
        SubtitleMatch::Off,
        SubtitleMatch::AnyTitle,
        SubtitleMatch::ContainsTitle,
        SubtitleMatch::ExtraCharacters,
        SubtitleMatch::Exact,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SubtitleMatch::Off => "Off",
            SubtitleMatch::AnyTitle => "Any title",
            SubtitleMatch::ContainsTitle => "Contains title",
            SubtitleMatch::ExtraCharacters => "Title with extra characters",
            SubtitleMatch::Exact => "Exact title match",
        }
    }
}

fn stem(p: &Path) -> String {
    p.file_stem().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default()
}

/// Lower-cased letters and digits only, so "The_Wand.S01E01" and
/// "the wand s01e01" compare equal.
fn squash(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect()
}

/// The season and episode in a name: `S01E02`, `s1e2` or `1x02`.
pub fn episode_code(name: &str) -> Option<(u32, u32)> {
    let lower = name.to_lowercase();
    let b = lower.as_bytes();
    let digits = |from: usize| -> (u32, usize) {
        let mut end = from;
        while end < b.len() && end - from < 3 && b[end].is_ascii_digit() {
            end += 1;
        }
        (lower[from..end].parse().unwrap_or(0), end)
    };
    for i in 0..b.len() {
        // sNNeNN
        if b[i] == b's' && (i == 0 || !b[i - 1].is_ascii_alphabetic()) && i + 1 < b.len() && b[i + 1].is_ascii_digit() {
            let (s, j) = digits(i + 1);
            if j < b.len() && b[j] == b'e' && j + 1 < b.len() && b[j + 1].is_ascii_digit() {
                let (e, _) = digits(j + 1);
                return Some((s, e));
            }
        }
        // NxNN
        if b[i].is_ascii_digit() && (i == 0 || !b[i - 1].is_ascii_alphanumeric()) {
            let (s, j) = digits(i);
            if j < b.len() && b[j] == b'x' && j + 1 < b.len() && b[j + 1].is_ascii_digit() {
                let (e, k) = digits(j + 1);
                if k == b.len() || !b[k].is_ascii_digit() {
                    return Some((s, e));
                }
            }
        }
    }
    None
}

/// Subtitle files that may belong to `video` under `rule`, best first.
pub fn candidates(video: &Path, subs: &[PathBuf], rule: SubtitleMatch) -> Vec<PathBuf> {
    let subs: Vec<PathBuf> = subs.iter().filter(|s| kind_for_path(s) == MediaKind::Subtitle).cloned().collect();
    match rule {
        SubtitleMatch::Off => Vec::new(),
        SubtitleMatch::Exact => matching_subtitles(video, &subs, false),
        SubtitleMatch::ExtraCharacters => matching_subtitles(video, &subs, true),
        SubtitleMatch::ContainsTitle | SubtitleMatch::AnyTitle => {
            let mut out = matching_subtitles(video, &subs, true);
            let v = squash(&stem(video));
            let ep = episode_code(&stem(video));
            for s in &subs {
                if out.contains(s) {
                    continue;
                }
                let ss = squash(&stem(s));
                let same_episode = ep.is_some() && episode_code(&stem(s)) == ep;
                if (!v.is_empty() && (ss.contains(&v) || v.contains(&ss))) || same_episode {
                    out.push(s.clone());
                }
            }
            if rule == SubtitleMatch::AnyTitle {
                for s in &subs {
                    if !out.contains(s) {
                        out.push(s.clone());
                    }
                }
            }
            out
        }
    }
}

/// Matches every video; each gets its candidates and the index of the
/// default choice. A subtitle already chosen for an earlier video is not
/// picked again by default when another candidate is free.
pub fn pair_all(videos: &[PathBuf], subs: &[PathBuf], rule: SubtitleMatch) -> Vec<(Vec<PathBuf>, Option<usize>)> {
    let mut taken: Vec<PathBuf> = Vec::new();
    videos
        .iter()
        .map(|v| {
            let c = candidates(v, subs, rule);
            let pick = c.iter().position(|s| !taken.contains(s)).or(if c.is_empty() { None } else { Some(0) });
            if let Some(i) = pick {
                taken.push(c[i].clone());
            }
            (c, pick)
        })
        .collect()
}

/// Subtitle files not chosen for any video.
pub fn unmatched(subs: &[PathBuf], chosen: &[Option<PathBuf>]) -> Vec<PathBuf> {
    subs.iter().filter(|s| !chosen.iter().any(|c| c.as_ref() == Some(*s))).cloned().collect()
}

/// `en`, `eng`, `pt-br`, `zh-Hant`: two or three letters, optionally a
/// dash and a region or script.
fn is_language_tag(t: &str) -> bool {
    let (lang, rest) = t.split_once('-').map(|(a, b)| (a, Some(b))).unwrap_or((t, None));
    let letters =
        |s: &str, lo: usize, hi: usize| (lo..=hi).contains(&s.len()) && s.chars().all(|c| c.is_ascii_alphabetic());
    letters(lang, 2, 3) && rest.is_none_or(|r| letters(r, 2, 4))
}

/// Where to copy `sub` so VLC autoloads it with `video`: next to the video,
/// named after it, keeping a trailing language tag (`.en`, `.eng`, `.pt-br`).
pub fn copy_target(video: &Path, sub: &Path) -> PathBuf {
    let ext = sub.extension().map(|e| e.to_string_lossy().into_owned()).unwrap_or_else(|| "srt".into());
    let sub_stem = sub.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let lang = sub_stem.rsplit_once('.').map(|(_, tag)| tag).filter(|t| is_language_tag(t));
    let video_stem = video.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let name = match lang {
        Some(l) => format!("{video_stem}.{l}.{ext}"),
        None => format!("{video_stem}.{ext}"),
    };
    video.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pb(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    #[test]
    fn episode_codes() {
        assert_eq!(episode_code("The_Wand_S01E02"), Some((1, 2)));
        assert_eq!(episode_code("show.s2e10.720p"), Some((2, 10)));
        assert_eq!(episode_code("Show - 3x07"), Some((3, 7)));
        assert_eq!(episode_code("1080p.x264"), None);
        assert_eq!(episode_code("Frieren_01"), None);
    }

    #[test]
    fn levels_widen_the_match() {
        let subs = vec![
            pb("/s/Show.S01E01.srt"),
            pb("/s/Show.S01E01.en.srt"),
            pb("/s/[Group] Show - 1x01 [1080p].ass"),
            pb("/s/Other.srt"),
            pb("/s/readme.nfo"),
        ];
        let v = Path::new("/v/Show.S01E01.mkv");
        assert_eq!(candidates(v, &subs, SubtitleMatch::Off), Vec::<PathBuf>::new());
        assert_eq!(candidates(v, &subs, SubtitleMatch::Exact), vec![pb("/s/Show.S01E01.srt")]);
        assert_eq!(candidates(v, &subs, SubtitleMatch::ExtraCharacters).len(), 2);
        let contains = candidates(v, &subs, SubtitleMatch::ContainsTitle);
        assert_eq!(contains.len(), 3);
        assert_eq!(contains[2], pb("/s/[Group] Show - 1x01 [1080p].ass"));
        assert_eq!(candidates(v, &subs, SubtitleMatch::AnyTitle).len(), 4);
    }

    #[test]
    fn pairing_prefers_free_subtitles() {
        let videos = vec![pb("/v/A.mkv"), pb("/v/A2.mkv")];
        let subs = vec![pb("/s/A.srt"), pb("/s/A2.srt")];
        let pairs = pair_all(&videos, &subs, SubtitleMatch::ExtraCharacters);
        assert_eq!(pairs[0].0[pairs[0].1.unwrap()], pb("/s/A.srt"));
        assert_eq!(pairs[1].0[pairs[1].1.unwrap()], pb("/s/A2.srt"));
        let chosen: Vec<Option<PathBuf>> = vec![Some(pb("/s/A.srt")), None];
        assert_eq!(unmatched(&subs, &chosen), vec![pb("/s/A2.srt")]);
    }

    #[test]
    fn copy_names_follow_the_video() {
        assert_eq!(
            copy_target(Path::new("/v/The_Wand_S01E01.mp4"), Path::new("/s/the.wand.s01e01.HDTV.srt")),
            pb("/v/The_Wand_S01E01.srt")
        );
        assert_eq!(copy_target(Path::new("/v/Ep 1.mkv"), Path::new("/s/ep1.en.srt")), pb("/v/Ep 1.en.srt"));
        assert_eq!(copy_target(Path::new("/v/Ep 1.mkv"), Path::new("/s/ep1.pt-br.ass")), pb("/v/Ep 1.pt-br.ass"));
    }
}
