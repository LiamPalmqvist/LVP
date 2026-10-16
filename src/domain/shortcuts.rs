//! Bindable actions and their default keys, following VLC's hotkey set with
//! the design's overrides (R records, Ctrl+\ hides the UI).

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

macro_rules! actions {
    ($( $cat:literal => { $( $var:ident = $label:literal [$($key:expr),*] ),* $(,)? } )*) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        pub enum Action { $($($var),*),* }

        impl Action {
            pub const ALL: &'static [Action] = &[$($(Action::$var),*),*];

            pub fn label(self) -> &'static str {
                match self { $($(Action::$var => $label),*),* }
            }

            pub fn category(self) -> &'static str {
                match self { $($(Action::$var => $cat),*),* }
            }

            pub fn default_keys(self) -> Vec<KeyChord> {
                match self { $($(Action::$var => vec![$(KeyChord::parse($key).expect($key)),*]),*),* }
            }
        }

        pub const CATEGORIES: &[&str] = &[$($cat),*];
    };
}

actions! {
    "Basic" => {
        ToggleFullscreen = "Toggle fullscreen" ["F"],
        LeaveFullscreen = "Exit fullscreen / close dialog" ["Esc"],
        ShowPosition = "Show playback position and time" ["T"],
        ShowControls = "Show interface controls in fullscreen" ["I"],
        HideUi = "Hide interface" ["Ctrl+\\"],
        ToggleSidebar = "Show / hide the library sidebar" ["Ctrl+B"],
        ToggleQueue = "Show / hide the queue" ["Ctrl+L"],
        ToggleMiniPlayer = "Toggle the mini player" ["Ctrl+M"],
        OpenFile = "Open file" ["Ctrl+O"],
        OpenFolder = "Open folder" ["Ctrl+F"],
        OpenDisc = "Open disc" ["Ctrl+D"],
        OpenNetwork = "Open network stream" ["Ctrl+N"],
        OpenCapture = "Open capture device" ["Ctrl+C"],
        ConvertSave = "Convert / save" ["Ctrl+R"],
        Preferences = "Preferences" ["Ctrl+,", "Ctrl+P"],
        MediaInfo = "Media information" ["Ctrl+I"],
        EffectsFilters = "Effects and filters" ["Ctrl+E"],
        KeyboardShortcuts = "Keyboard shortcuts" ["F1"],
        Quit = "Quit" ["Ctrl+Q"],
    }
    "Playback" => {
        PlayPause = "Play / pause" ["Space"],
        Stop = "Stop playback" ["S"],
        Next = "Next track" ["N"],
        Previous = "Previous track" ["P"],
        NextProgram = "Next program / Service ID" ["X"],
        PreviousProgram = "Previous program / Service ID" ["Shift+X"],
        Faster = "Faster" ["+", "]"],
        Slower = "Slower" ["-", "["],
        NormalSpeed = "Normal speed" ["="],
        FasterFine = "Faster (fine)" ["Shift+]"],
        SlowerFine = "Slower (fine)" ["Shift+["],
        NextFrame = "Next frame" ["E"],
        JumpForwardVeryShort = "Very short forward jump" ["Shift+Right"],
        JumpBackwardVeryShort = "Very short backward jump" ["Shift+Left"],
        JumpForwardShort = "Short forward jump" ["Alt+Right"],
        JumpBackwardShort = "Short backward jump" ["Alt+Left"],
        JumpForwardMedium = "Medium forward jump" ["Ctrl+Right"],
        JumpBackwardMedium = "Medium backward jump" ["Ctrl+Left"],
        JumpForwardLong = "Long forward jump" ["Ctrl+Alt+Right"],
        JumpBackwardLong = "Long backward jump" ["Ctrl+Alt+Left"],
        JumpToTime = "Jump to time" ["Ctrl+T"],
        Record = "Record" ["R"],
        AbLoop = "Loop between point A and B" ["Shift+L"],
        CycleRepeat = "Repeat: off / all / one" ["L"],
        ToggleShuffle = "Shuffle" ["Alt+R"],
        AddBookmark = "Add bookmark" ["Ctrl+K"],
    }
    "Navigation" => {
        DiscMenu = "Go to the disc menu" ["Shift+M"],
        PreviousTitle = "Previous title" ["Shift+O"],
        NextTitle = "Next title" ["Shift+B"],
        PreviousChapter = "Previous chapter" ["Shift+P"],
        NextChapter = "Next chapter" ["Shift+N"],
        NavActivate = "Activate" ["Enter"],
        NavUp = "Navigate up" ["Up"],
        NavDown = "Navigate down" ["Down"],
        NavLeft = "Navigate left" ["Left"],
        NavRight = "Navigate right" ["Right"],
    }
    "Audio" => {
        VolumeUp = "Volume up" ["Ctrl+Up"],
        VolumeDown = "Volume down" ["Ctrl+Down"],
        Mute = "Mute" ["M"],
        CycleAudioTrack = "Cycle audio track" ["B"],
        CycleAudioDevice = "Cycle audio device" ["Shift+A"],
        AudioDelayUp = "Audio delay up" ["K"],
        AudioDelayDown = "Audio delay down" ["J"],
        CycleStereoMode = "Cycle stereo audio mode" ["Shift+E"],
        ToggleEqualizer = "Toggle the equalizer" ["Alt+E"],
    }
    "Subtitles" => {
        CycleSubtitle = "Cycle subtitle track" ["V"],
        CycleSubtitleReverse = "Cycle subtitle track in reverse order" ["Alt+V"],
        ToggleSubtitles = "Toggle subtitles" ["Shift+V"],
        SubtitleDelayUp = "Subtitle delay up" ["H"],
        SubtitleDelayDown = "Subtitle delay down" ["G"],
        SubtitleSyncAudio = "Mark the audio timestamp for sync" ["Shift+H"],
        SubtitleSyncSubtitle = "Mark the subtitle timestamp for sync" ["Shift+J"],
        SubtitleSyncApply = "Apply subtitle sync" ["Shift+K"],
        SubtitleSyncReset = "Reset subtitle sync" ["Ctrl+Shift+K"],
        SubtitlePositionUp = "Move subtitles up" ["Alt+Up"],
        SubtitlePositionDown = "Move subtitles down" ["Alt+Down"],
        SubtitleScaleUp = "Bigger subtitles" ["Ctrl+Shift+Up"],
        SubtitleScaleDown = "Smaller subtitles" ["Ctrl+Shift+Down"],
        AddSubtitleFile = "Add subtitle file" ["Ctrl+Shift+S"],
    }
    "Video" => {
        CycleAspectRatio = "Cycle aspect ratio" ["A"],
        CycleCrop = "Cycle crop" ["C"],
        ToggleAutoscale = "Toggle autoscaling" ["O"],
        ZoomIn = "Zoom in" ["Alt+O"],
        ZoomOut = "Zoom out" ["Alt+Shift+O"],
        CycleDeinterlace = "Toggle deinterlacing" ["D"],
        CycleDeinterlaceMode = "Cycle deinterlace modes" ["Shift+D"],
        CycleVideoTrack = "Cycle video track" ["Alt+Shift+V"],
        Snapshot = "Take a snapshot" ["Shift+S"],
        AlwaysOnTop = "Always on top" ["Ctrl+Shift+T"],
        Wallpaper = "Wallpaper mode" ["W"],
        PictureInPicture = "Picture in picture" ["Ctrl+Shift+P"],
        CropTopMore = "Crop one pixel from the top" ["Alt+I"],
        CropTopLess = "Uncrop one pixel from the top" ["Alt+Shift+I"],
        CropBottomMore = "Crop one pixel from the bottom" ["Alt+K"],
        CropBottomLess = "Uncrop one pixel from the bottom" ["Alt+Shift+K"],
        CropLeftMore = "Crop one pixel from the left" ["Alt+J"],
        CropLeftLess = "Uncrop one pixel from the left" ["Alt+Shift+J"],
        CropRightMore = "Crop one pixel from the right" ["Alt+L"],
        CropRightLess = "Uncrop one pixel from the right" ["Alt+Shift+L"],
    }
}

/// Keys the app recognises, named the way the shortcut table prints them.
pub const KEY_NAMES: &[&str] = &[
    "A",
    "B",
    "C",
    "D",
    "E",
    "F",
    "G",
    "H",
    "I",
    "J",
    "K",
    "L",
    "M",
    "N",
    "O",
    "P",
    "Q",
    "R",
    "S",
    "T",
    "U",
    "V",
    "W",
    "X",
    "Y",
    "Z",
    "0",
    "1",
    "2",
    "3",
    "4",
    "5",
    "6",
    "7",
    "8",
    "9",
    "F1",
    "F2",
    "F3",
    "F4",
    "F5",
    "F6",
    "F7",
    "F8",
    "F9",
    "F10",
    "F11",
    "F12",
    "Space",
    "Enter",
    "Esc",
    "Tab",
    "Backspace",
    "Delete",
    "Insert",
    "Home",
    "End",
    "PageUp",
    "PageDown",
    "Up",
    "Down",
    "Left",
    "Right",
    "+",
    "-",
    "=",
    "[",
    "]",
    "\\",
    ";",
    "'",
    ",",
    ".",
    "/",
    "`",
];

/// A key with modifiers. `ctrl` means Command on macOS.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(into = "String", try_from = "String")]
pub struct KeyChord {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub key: String,
}

impl KeyChord {
    pub fn parse(s: &str) -> Option<Self> {
        let mut chord = KeyChord { ctrl: false, alt: false, shift: false, key: String::new() };
        let s = s.trim();
        // "+" alone (or "Ctrl++") names the plus key.
        let (mods, key) = match s.rsplit_once('+') {
            Some((m, "")) => (m.strip_suffix('+').unwrap_or(m), "+"),
            Some((m, k)) => (m, k),
            None => ("", s),
        };
        for m in mods.split('+').map(str::trim).filter(|m| !m.is_empty()) {
            match m.to_ascii_lowercase().as_str() {
                "ctrl" | "cmd" | "command" => chord.ctrl = true,
                "alt" | "option" | "opt" => chord.alt = true,
                "shift" => chord.shift = true,
                _ => return None,
            }
        }
        let key = key.trim();
        let canon = KEY_NAMES.iter().find(|k| k.eq_ignore_ascii_case(key))?;
        chord.key = (*canon).to_string();
        Some(chord)
    }
}

impl fmt::Display for KeyChord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ctrl = if cfg!(target_os = "macos") { "Cmd" } else { "Ctrl" };
        let alt = if cfg!(target_os = "macos") { "Option" } else { "Alt" };
        let mut parts = Vec::new();
        if self.ctrl {
            parts.push(ctrl);
        }
        if self.alt {
            parts.push(alt);
        }
        if self.shift {
            parts.push("Shift");
        }
        parts.push(&self.key);
        write!(f, "{}", parts.join(" + "))
    }
}

impl From<KeyChord> for String {
    fn from(k: KeyChord) -> String {
        let mut s = String::new();
        if k.ctrl {
            s.push_str("Ctrl+");
        }
        if k.alt {
            s.push_str("Alt+");
        }
        if k.shift {
            s.push_str("Shift+");
        }
        s.push_str(&k.key);
        s
    }
}

impl TryFrom<String> for KeyChord {
    type Error = String;
    fn try_from(s: String) -> Result<Self, String> {
        KeyChord::parse(&s).ok_or_else(|| format!("unknown key chord {s:?}"))
    }
}

/// User key bindings. Only overrides are stored; everything else uses the defaults.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Shortcuts {
    pub overrides: BTreeMap<Action, Vec<KeyChord>>,
    pub global: BTreeMap<Action, Vec<KeyChord>>,
}

impl Shortcuts {
    pub fn keys(&self, action: Action) -> Vec<KeyChord> {
        self.overrides.get(&action).cloned().unwrap_or_else(|| action.default_keys())
    }

    pub fn set(&mut self, action: Action, keys: Vec<KeyChord>) {
        if keys == action.default_keys() {
            self.overrides.remove(&action);
        } else {
            self.overrides.insert(action, keys);
        }
    }

    /// The action bound to a chord, if any.
    pub fn action_for(&self, chord: &KeyChord) -> Option<Action> {
        Action::ALL.iter().copied().find(|a| self.keys(*a).iter().any(|k| k == chord))
    }

    /// Other actions already using a chord, for conflict warnings.
    pub fn conflicts(&self, action: Action, chord: &KeyChord) -> Vec<Action> {
        Action::ALL.iter().copied().filter(|a| *a != action && self.keys(*a).contains(chord)).collect()
    }

    pub fn reset(&mut self) {
        self.overrides.clear();
        self.global.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_parse_and_are_unique() {
        let mut seen = std::collections::HashMap::new();
        for a in Action::ALL {
            for k in a.default_keys() {
                if let Some(prev) = seen.insert(k.clone(), *a) {
                    panic!("{k} bound to both {prev:?} and {a:?}");
                }
            }
        }
    }

    #[test]
    fn parses_modifiers_and_plus() {
        let k = KeyChord::parse("Ctrl+Alt+Right").unwrap();
        assert!(k.ctrl && k.alt && !k.shift);
        assert_eq!(k.key, "Right");
        assert_eq!(KeyChord::parse("+").unwrap().key, "+");
        assert_eq!(KeyChord::parse("Ctrl++").unwrap().key, "+");
        assert!(KeyChord::parse("Hyper+X").is_none());
    }

    #[test]
    fn overrides_and_lookup() {
        let mut s = Shortcuts::default();
        let space = KeyChord::parse("Space").unwrap();
        assert_eq!(s.action_for(&space), Some(Action::PlayPause));
        s.set(Action::Stop, vec![space.clone()]);
        assert_eq!(s.conflicts(Action::Stop, &space), vec![Action::PlayPause]);
        s.set(Action::Stop, Action::Stop.default_keys());
        assert!(s.overrides.is_empty());
    }

    #[test]
    fn serde_uses_strings() {
        let mut s = Shortcuts::default();
        s.set(Action::Quit, vec![KeyChord::parse("Ctrl+Shift+Q").unwrap()]);
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("Ctrl+Shift+Q"));
        let back: Shortcuts = serde_json::from_str(&json).unwrap();
        assert_eq!(back, s);
    }
}
