//! Shortcut actions. Player actions run here; window and dialog actions come
//! back as a `UiCommand` for the UI to carry out.

use super::App;
use crate::domain::effects::StereoMode;
use crate::domain::shortcuts::Action;
use crate::domain::time::format_hms;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiCommand {
    ToggleFullscreen,
    LeaveFullscreen,
    ShowControls,
    HideUi,
    ToggleSidebar,
    ToggleQueue,
    ToggleMiniPlayer,
    OpenFile,
    OpenFolder,
    OpenDisc,
    OpenNetwork,
    OpenCapture,
    ConvertSave,
    Preferences,
    MediaInfo,
    EffectsFilters,
    KeyboardShortcuts,
    JumpToTime,
    AlwaysOnTop,
    Wallpaper,
    PictureInPicture,
    AddSubtitleFile,
    Quit,
}

impl App {
    pub fn perform(&mut self, action: Action) -> Option<UiCommand> {
        use Action::*;
        let c = &self.settings.controls;
        let (vs, s, m, l) = (
            c.skip_very_short_s as i64 * 1000,
            c.skip_short_s as i64 * 1000,
            c.skip_medium_s as i64 * 1000,
            c.skip_long_s as i64 * 1000,
        );
        match action {
            ToggleFullscreen => return Some(UiCommand::ToggleFullscreen),
            LeaveFullscreen => return Some(UiCommand::LeaveFullscreen),
            ShowPosition => {
                let st = &self.status;
                let text = format!("{} / {}", format_hms(st.time_ms), format_hms(st.length_ms));
                self.show_osd(text);
            }
            ShowControls => return Some(UiCommand::ShowControls),
            HideUi => return Some(UiCommand::HideUi),
            ToggleSidebar => return Some(UiCommand::ToggleSidebar),
            ToggleQueue => return Some(UiCommand::ToggleQueue),
            ToggleMiniPlayer => return Some(UiCommand::ToggleMiniPlayer),
            OpenFile => return Some(UiCommand::OpenFile),
            OpenFolder => return Some(UiCommand::OpenFolder),
            OpenDisc => return Some(UiCommand::OpenDisc),
            OpenNetwork => return Some(UiCommand::OpenNetwork),
            OpenCapture => return Some(UiCommand::OpenCapture),
            ConvertSave => return Some(UiCommand::ConvertSave),
            Preferences => return Some(UiCommand::Preferences),
            MediaInfo => return Some(UiCommand::MediaInfo),
            EffectsFilters => return Some(UiCommand::EffectsFilters),
            KeyboardShortcuts => return Some(UiCommand::KeyboardShortcuts),
            Quit => return Some(UiCommand::Quit),

            PlayPause => self.toggle_pause(),
            Stop => self.stop(),
            Next => self.next(),
            Previous => self.previous(),
            NextProgram | PreviousProgram => self.show_osd("Program selection is not available for this stream"),
            Faster => self.rate_step(true, false),
            Slower => self.rate_step(false, false),
            NormalSpeed => self.set_rate(1.0),
            FasterFine => self.rate_step(true, true),
            SlowerFine => self.rate_step(false, true),
            NextFrame => self.frame_step(),
            JumpForwardVeryShort => self.jump(vs),
            JumpBackwardVeryShort => self.jump(-vs),
            JumpForwardShort => self.jump(s),
            JumpBackwardShort => self.jump(-s),
            JumpForwardMedium => self.jump(m),
            JumpBackwardMedium => self.jump(-m),
            JumpForwardLong => self.jump(l),
            JumpBackwardLong => self.jump(-l),
            JumpToTime => return Some(UiCommand::JumpToTime),
            Record => self.toggle_record(),
            AbLoop => self.cycle_ab_loop(),
            CycleRepeat => {
                let r = self.queue.repeat.cycle();
                self.set_repeat(r);
            }
            ToggleShuffle => {
                let on = !self.queue.shuffle;
                self.set_shuffle(on);
            }
            AddBookmark => self.add_bookmark(None),

            DiscMenu => self.disc_menu(),
            PreviousTitle => self.title_step(false),
            NextTitle => self.title_step(true),
            PreviousChapter => self.chapter_step(false),
            NextChapter => self.chapter_step(true),
            NavActivate => self.engine.send(super::EngineCommand::Navigate(super::Nav::Activate)),
            NavUp => self.engine.send(super::EngineCommand::Navigate(super::Nav::Up)),
            NavDown => self.engine.send(super::EngineCommand::Navigate(super::Nav::Down)),
            NavLeft => self.engine.send(super::EngineCommand::Navigate(super::Nav::Left)),
            NavRight => self.engine.send(super::EngineCommand::Navigate(super::Nav::Right)),

            VolumeUp => self.change_volume(self.settings.audio.volume_step.max(1)),
            VolumeDown => self.change_volume(-self.settings.audio.volume_step.max(1)),
            Mute => self.toggle_mute(),
            CycleAudioTrack => self.cycle_audio_track(),
            CycleAudioDevice => self.cycle_audio_device(),
            AudioDelayUp => {
                let d = self.status.audio_delay_us + 50_000;
                self.set_audio_delay(d);
            }
            AudioDelayDown => {
                let d = self.status.audio_delay_us - 50_000;
                self.set_audio_delay(d);
            }
            CycleStereoMode => {
                let all = StereoMode::ALL;
                let i = all.iter().position(|m| *m == self.stereo_mode).unwrap_or(0);
                self.set_stereo_mode(all[(i + 1) % all.len()]);
            }
            ToggleEqualizer => self.toggle_equalizer(),

            CycleSubtitle => self.cycle_subtitle_track(false),
            CycleSubtitleReverse => self.cycle_subtitle_track(true),
            ToggleSubtitles => self.toggle_subtitles(),
            SubtitleDelayUp => {
                let d = self.status.subtitle_delay_us + 50_000;
                self.set_subtitle_delay(d);
            }
            SubtitleDelayDown => {
                let d = self.status.subtitle_delay_us - 50_000;
                self.set_subtitle_delay(d);
            }
            SubtitleSyncAudio => self.subtitle_sync_mark_audio(),
            SubtitleSyncSubtitle => self.subtitle_sync_mark_subtitle(),
            SubtitleSyncApply => self.subtitle_sync_apply(),
            SubtitleSyncReset => self.subtitle_sync_reset(),
            SubtitlePositionUp | SubtitlePositionDown | SubtitleScaleUp | SubtitleScaleDown => {
                let st = &mut self.settings.subtitles;
                use crate::domain::settings::{FontSize, SubPosition};
                match action {
                    SubtitlePositionUp => {
                        st.position = match st.position {
                            SubPosition::Bottom => SubPosition::Center,
                            _ => SubPosition::Top,
                        }
                    }
                    SubtitlePositionDown => {
                        st.position = match st.position {
                            SubPosition::Top => SubPosition::Center,
                            _ => SubPosition::Bottom,
                        }
                    }
                    _ => {
                        let all = FontSize::ALL;
                        let i = all.iter().position(|f| *f == st.font_size).unwrap_or(0);
                        let j =
                            if action == SubtitleScaleUp { (i + 1).min(all.len() - 1) } else { i.saturating_sub(1) };
                        st.font_size = all[j];
                    }
                }
                let s = self.settings.clone();
                self.update_settings(|x| *x = s);
                self.show_osd("Subtitle style applies to the next video");
            }
            AddSubtitleFile => return Some(UiCommand::AddSubtitleFile),

            CycleAspectRatio => self.cycle_aspect(),
            CycleCrop => self.cycle_crop(),
            ToggleAutoscale => {
                self.geometry.zoom = if self.geometry.zoom == 0.0 { 1.0 } else { 0.0 };
                self.show_osd(if self.geometry.zoom == 0.0 { "Scale to window" } else { "Original size" });
            }
            ZoomIn => self.zoom_step(true),
            ZoomOut => self.zoom_step(false),
            CycleDeinterlace => {
                use crate::domain::effects::DeinterlaceMode as D;
                let next = match self.settings.video.deinterlace {
                    D::Off => D::On,
                    D::On => D::Automatic,
                    D::Automatic => D::Off,
                };
                let method = self.settings.video.deinterlace_method.clone();
                self.set_deinterlace(next, method);
                self.show_osd(format!("Deinterlace: {}", next.label()));
            }
            CycleDeinterlaceMode => {
                use crate::domain::effects::DEINTERLACE_METHODS as M;
                let i = M.iter().position(|(k, _)| *k == self.settings.video.deinterlace_method).unwrap_or(0);
                let (k, label) = M[(i + 1) % M.len()];
                let mode = self.settings.video.deinterlace;
                self.set_deinterlace(mode, k.to_string());
                self.show_osd(format!("Deinterlace mode: {label}"));
            }
            CycleVideoTrack => self.cycle_video_track(),
            Snapshot => {
                self.snapshot();
            }
            AlwaysOnTop => return Some(UiCommand::AlwaysOnTop),
            Wallpaper => return Some(UiCommand::Wallpaper),
            PictureInPicture => return Some(UiCommand::PictureInPicture),
            CropTopMore => self.crop_pixels('t', true),
            CropTopLess => self.crop_pixels('t', false),
            CropBottomMore => self.crop_pixels('b', true),
            CropBottomLess => self.crop_pixels('b', false),
            CropLeftMore => self.crop_pixels('l', true),
            CropLeftLess => self.crop_pixels('l', false),
            CropRightMore => self.crop_pixels('r', true),
            CropRightLess => self.crop_pixels('r', false),
        }
        None
    }
}
