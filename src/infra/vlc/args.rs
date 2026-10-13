//! libvlc instance arguments derived from the user's settings.

use crate::domain::effects::DeinterlaceMode;
use crate::domain::settings::*;

fn rgb(c: u32) -> String {
    format!("{}", c & 0xFFFFFF)
}

fn expand(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/")
        && let Some(home) = std::env::var_os("HOME")
    {
        return std::path::Path::new(&home).join(rest).to_string_lossy().into_owned();
    }
    path.to_string()
}

/// Arguments for the playback instance. Interface-only settings are left out.
pub fn playback(s: &Settings) -> Vec<String> {
    let mut a: Vec<String> = vec![
        "--no-video-title-show".into(),
        "--no-snapshot-preview".into(),
        "--no-osd".into(),
        "--no-stats".into(),
        "--intf=dummy".into(),
        "--no-media-library".into(),
    ];
    if s.general.collect_stats {
        a.retain(|x| x != "--no-stats");
        a.push("--stats".into());
    }
    // Logging.
    a.push(format!(
        "--verbose={}",
        match s.general.log_level {
            LogLevel::ErrorsOnly => 0,
            LogLevel::Warnings => 1,
            LogLevel::Info | LogLevel::Debug => 2,
        }
    ));
    if s.general.syslog {
        a.push("--syslog".into());
        a.push(format!("--syslog-ident={}", s.general.syslog_ident));
        a.push(format!("--syslog-facility={}", s.general.syslog_facility.label().replace(' ', "").to_lowercase()));
        if s.general.syslog_debug {
            a.push("--syslog-debug".into());
        }
    }

    // Playback.
    let p = &s.playback;
    if (p.default_speed - 1.0).abs() > 0.001 {
        a.push(format!("--rate={}", p.default_speed));
    }
    a.push(format!("--preferred-resolution={}", p.preferred_quality.vlc_value()));
    if p.seek_precisely {
        a.push("--no-input-fast-seek".into());
    } else {
        a.push("--input-fast-seek".into());
    }
    a.push(format!(
        "--avi-index={}",
        match p.repair_avi {
            AskAlways::Ask => 0,
            AskAlways::Always => 1,
            AskAlways::Never => 2,
        }
    ));
    if !p.load_matching_mkv {
        a.push("--no-mkv-preload-local-dir".into());
    }
    if !p.timeshift_dir.is_empty() {
        a.push(format!("--input-timeshift-path={}", expand(&p.timeshift_dir)));
    }
    a.push(format!(
        "--input-timeshift-granularity={}",
        match p.timeshift_size {
            BufferSize::Small => 10_485_760,
            BufferSize::Medium => 52_428_800,
            BufferSize::Large => 209_715_200,
        }
    ));
    if !p.optical_drive.is_empty() {
        a.push(format!("--dvd={}", p.optical_drive));
    }
    if p.default_audio_track > 1 {
        a.push(format!("--audio-track={}", p.default_audio_track - 1));
    }
    if p.default_subtitle_track > 1 {
        a.push(format!("--sub-track={}", p.default_subtitle_track - 1));
    }

    // Video.
    let v = &s.video;
    if !v.enabled {
        a.push("--no-video".into());
    }
    a.push(format!(
        "--deinterlace={}",
        match v.deinterlace {
            DeinterlaceMode::Off => 0,
            DeinterlaceMode::On => 1,
            DeinterlaceMode::Automatic => -1,
        }
    ));
    a.push(format!("--deinterlace-mode={}", v.deinterlace_method));
    if !v.custom_aspect_ratios.trim().is_empty() {
        a.push(format!("--custom-aspect-ratios={}", v.custom_aspect_ratios.trim()));
    }
    if !v.custom_crop_ratios.trim().is_empty() {
        a.push(format!("--custom-crop-ratios={}", v.custom_crop_ratios.trim()));
    }

    // Audio.
    let au = &s.audio;
    if au.passthrough {
        a.push("--spdif".into());
    }
    a.push(format!(
        "--force-dolby-surround={}",
        match au.dolby_surround {
            DolbyDetect::Automatic => 0,
            DolbyDetect::On => 1,
            DolbyDetect::Off => 2,
        }
    ));
    if au.preferred_language != "System language" && !au.preferred_language.is_empty() {
        a.push(format!("--audio-language={}", au.preferred_language));
    }
    if !au.effects.time_stretch {
        a.push("--no-audio-time-stretch".into());
    }
    if !au.enabled {
        a.push("--no-audio".into());
    }
    if au.speaker_setup != SpeakerSetup::Automatic {
        a.push(format!("--stereo-mode={}", au.speaker_setup.vlc_value()));
    }
    if au.replay_gain != ReplayGain::Off {
        a.push(format!("--audio-replay-gain-mode={}", au.replay_gain.vlc_value()));
        a.push(format!("--audio-replay-gain-preamp={}", au.replay_gain_preamp));
    }
    if let Some((module, effect)) = au.visualizer.vlc_module() {
        a.push(format!("--audio-visual={module}"));
        if let Some(e) = effect {
            a.push(format!("--effect-list={e}"));
        }
    }

    // Subtitles.
    let st = &s.subtitles;
    if !st.enabled {
        a.push("--no-spu".into());
    }
    if st.preferred_language != "System language" && !st.preferred_language.is_empty() {
        a.push(format!("--sub-language={}", st.preferred_language));
    }
    if st.encoding != "Automatic" {
        a.push(format!("--subsdec-encoding={}", st.encoding));
    }
    a.push(if st.autoload { "--sub-autodetect-file".into() } else { "--no-sub-autodetect-file".into() });
    a.push(format!(
        "--sub-autodetect-fuzzy={}",
        match st.matching {
            SubMatching::Exact => 1,
            SubMatching::ExtraChars => 2,
            SubMatching::ContainsTitle => 3,
            SubMatching::AllInFolder => 4,
        }
    ));
    if !st.subtitle_dirs.trim().is_empty() {
        a.push(format!("--sub-autodetect-path={}", st.subtitle_dirs.trim()));
    }
    if st.font != "System, sans-serif" && !st.font.is_empty() {
        a.push(format!("--freetype-font={}", st.font));
    }
    if st.monospace_font != "System monospace" && !st.monospace_font.is_empty() {
        a.push(format!("--freetype-monofont={}", st.monospace_font));
    }
    match st.size_units {
        SizeUnits::Relative => a.push(format!("--freetype-rel-fontsize={}", st.font_size.rel_value())),
        SizeUnits::Pixels if st.font_size_px > 0 => a.push(format!("--freetype-fontsize={}", st.font_size_px)),
        SizeUnits::Pixels => {}
    }
    if st.bold {
        a.push("--freetype-bold".into());
    }
    a.push(format!("--freetype-color={}", rgb(st.text_color)));
    a.push(format!("--freetype-opacity={}", (st.text_opacity.clamp(0.0, 1.0) * 255.0) as i32));
    a.push(format!("--freetype-outline-thickness={}", st.outline.vlc_value()));
    a.push(format!("--freetype-outline-color={}", rgb(st.outline_color)));
    a.push(format!("--freetype-shadow-opacity={}", if st.shadow { 128 } else { 0 }));
    a.push(format!("--freetype-shadow-color={}", rgb(st.shadow_color)));
    a.push(format!(
        "--freetype-shadow-distance={}",
        match st.shadow_distance {
            ShadowDistance::Small => 0.06,
            ShadowDistance::Medium => 0.1,
            ShadowDistance::Large => 0.16,
        }
    ));
    a.push(format!("--freetype-shadow-angle={}", st.shadow_angle));
    match st.background {
        SubBackground::None => a.push("--freetype-background-opacity=0".into()),
        SubBackground::Box | SubBackground::Line => {
            a.push(format!("--freetype-background-opacity={}", (st.background_opacity.clamp(0.0, 1.0) * 255.0) as i32));
            a.push(format!("--freetype-background-color={}", rgb(st.background_color)));
        }
    }
    a.push(format!(
        "--sub-text-scale={}",
        match st.font_size {
            FontSize::Smaller => 75,
            FontSize::Small => 90,
            FontSize::Large => 125,
            FontSize::Larger => 150,
            _ => 100,
        }
    ));
    match st.position {
        SubPosition::Bottom => {}
        SubPosition::Center => a.push("--sub-margin=-200".into()),
        SubPosition::Top => a.push("--sub-margin=-400".into()),
    }
    if st.ignore_ass_styling {
        a.push("--no-freetype-yuvp".into());
    }

    // Library / opening.
    if s.library.show_hidden {
        a.push("--show-hiddenfiles".into());
    }
    a.push(match s.library.folder_open {
        FolderOpen::IncludeSubfolders => "--recursive=expand".into(),
        FolderOpen::TopLevel => "--recursive=collapse".into(),
    });
    if s.library.artwork == ArtworkFetch::Internet || s.privacy.fetch_online_meta {
        a.push("--metadata-network-access".into());
    } else {
        a.push("--no-metadata-network-access".into());
    }

    // Network.
    let n = &s.network;
    a.push(format!("--file-caching={}", n.file_caching_ms));
    a.push(format!("--disc-caching={}", n.disc_caching_ms));
    a.push(format!("--network-caching={}", n.network_caching_ms));
    a.push(format!("--live-caching={}", n.live_caching_ms));
    if !n.http_proxy.trim().is_empty() && n.http_proxy != "System proxy" {
        a.push(format!("--http-proxy={}", n.http_proxy.trim()));
        if !n.http_proxy_password.is_empty() {
            a.push(format!("--http-proxy-pwd={}", n.http_proxy_password));
        }
    }
    if !n.verify_certs {
        a.push("--no-gnutls-system-trust".into());
    }
    a.push(format!(
        "--gnutls-priorities={}",
        match n.cipher {
            CipherPreference::Balanced => "NORMAL",
            CipherPreference::Security => "SECURE256",
            CipherPreference::Speed => "PERFORMANCE",
        }
    ));

    // Performance.
    let pf = &s.performance;
    a.push(format!(
        "--avcodec-hw={}",
        match pf.hw_decoding {
            HwDecoding::Automatic => "any",
            HwDecoding::VideoToolbox => "videotoolbox",
            HwDecoding::Disabled => "none",
        }
    ));
    if pf.post_processing != PostProcessing::Off {
        a.push("--video-filter=postproc".into());
        a.push(format!("--postproc-q={}", pf.post_processing.quality()));
    }
    a.push(format!(
        "--avcodec-skiploopfilter={}",
        match pf.skip_loop_filter {
            SkipLoopFilter::Never => 0,
            SkipLoopFilter::NonRef => 1,
            SkipLoopFilter::Bidir => 2,
            SkipLoopFilter::NonKey => 3,
            SkipLoopFilter::All => 4,
        }
    ));
    a.push(format!("--avcodec-threads={}", pf.decoding_threads));
    a.push(if pf.drop_late_frames { "--drop-late-frames".into() } else { "--no-drop-late-frames".into() });
    if pf.decoder != "Automatic" && !pf.decoder.is_empty() {
        a.push(format!("--codec={}", pf.decoder));
    }
    if pf.encoder != "Automatic" && !pf.encoder.is_empty() {
        a.push(format!("--encoder={}", pf.encoder));
    }
    match pf.clock_sync {
        ClockSync::Default => {}
        ClockSync::Disabled => a.push("--clock-synchro=0".into()),
        ClockSync::Enabled => a.push("--clock-synchro=1".into()),
    }
    a.push(format!("--clock-jitter={}", pf.clock_jitter_ms));

    // Recording and streaming.
    let r = &s.recording;
    a.push(format!("--input-record-path={}", expand(&r.record_dir)));
    a.push(format!("--sout-x264-preset={}", r.preset.label().to_lowercase()));
    if r.tune != EncodingTune::None {
        a.push(format!(
            "--sout-x264-tune={}",
            match r.tune {
                EncodingTune::StillImage => "stillimage".to_string(),
                EncodingTune::FastDecode => "fastdecode".to_string(),
                EncodingTune::ZeroLatency => "zerolatency".to_string(),
                t => t.label().to_lowercase(),
            }
        ));
    }

    // Free-form extras last so they win.
    a.extend(s.general.extra_args.split_whitespace().map(str::to_string));
    a
}

/// Arguments for the helper instance used for parsing, thumbnails,
/// discovery and conversion.
pub fn helper(s: &Settings) -> Vec<String> {
    let mut a = vec![
        "--intf=dummy".to_string(),
        "--no-video-title-show".into(),
        "--no-osd".into(),
        "--no-snapshot-preview".into(),
        "--no-media-library".into(),
        "--verbose=0".into(),
        "--avcodec-hw=none".into(),
    ];
    if s.library.artwork == ArtworkFetch::Internet || s.privacy.fetch_online_meta {
        a.push("--metadata-network-access".into());
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sane() {
        let a = playback(&Settings::default());
        assert!(a.contains(&"--avcodec-hw=any".to_string()));
        assert!(a.contains(&"--network-caching=1000".to_string()));
        assert!(a.iter().all(|x| x.starts_with("--")));
    }

    #[test]
    fn extra_args_are_appended() {
        let mut s = Settings::default();
        s.general.extra_args = "--no-audio  --foo=1".into();
        let a = playback(&s);
        assert_eq!(&a[a.len() - 2..], &["--no-audio".to_string(), "--foo=1".to_string()]);
    }
}
