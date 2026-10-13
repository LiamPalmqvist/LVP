//! The "System" group: Network, Performance and Recording (Extensions has
//! its own file).

use super::Cx;
use super::kit::{Btn, Form, Pick};
use crate::domain::settings::*;

pub fn network(f: &mut Form, s: &mut Settings, _cx: &mut Cx) {
    let n = &mut s.network;
    f.section("Buffering");
    if f.choice(
        "Buffering",
        Some("How much media is loaded ahead. More buffering survives slow connections but starts later."),
        &mut n.buffering,
        BufferingPreset::ALL,
        BufferingPreset::label,
    ) {
        let [file, disc, net, live] = n.buffering.caching_ms();
        n.file_caching_ms = file;
        n.disc_caching_ms = disc;
        n.network_caching_ms = net;
        n.live_caching_ms = live;
    }
    f.int("Local files", None, &mut n.file_caching_ms, 0..=60_000, "ms");
    f.int("Discs", None, &mut n.disc_caching_ms, 0..=60_000, "ms");
    f.int("Network streams", None, &mut n.network_caching_ms, 0..=60_000, "ms");
    f.int("Live capture", None, &mut n.live_caching_ms, 0..=60_000, "ms");

    f.section("Proxy");
    f.text(
        "HTTP proxy",
        Some("A proxy server for web streams, as http://host:port. Leave empty to use the system proxy."),
        &mut n.http_proxy,
        "System proxy",
    );
    f.password(
        "HTTP proxy password",
        Some("The password for the proxy, if it needs one."),
        &mut n.http_proxy_password,
        "",
    );

    f.section("Security");
    f.toggle(
        "Verify certificates against the system trust store",
        Some("Refuse secure (https) streams whose certificate your system does not trust."),
        &mut n.verify_certs,
    );
    f.choice(
        "Cipher preference",
        Some("Trade-off between the strongest encryption and the fastest for secure streams."),
        &mut n.cipher,
        CipherPreference::ALL,
        CipherPreference::label,
    );

    f.advanced(|f| {
        f.text(
            "User agent",
            Some("How the app identifies itself to web servers. Leave empty for VLC's default."),
            &mut n.user_agent,
            "VLC/3.0",
        );
    });
}

const DECODERS: &[&str] = &[
    "Automatic",
    "avcodec",
    "videotoolbox",
    "dav1d",
    "libmpeg2",
    "faad",
    "flac",
    "vorbis",
    "opus",
    "a52",
    "dca",
    "mpg123",
    "subsdec",
];
const ENCODERS: &[&str] =
    &["Automatic", "x264", "x265", "avcodec", "vpx", "theora", "vorbis", "opus", "flac", "lame", "twolame", "araw"];

pub fn performance(f: &mut Form, s: &mut Settings, cx: &mut Cx) {
    let pf = &mut s.performance;
    f.section("Decoding");
    f.choice(
        "Hardware-accelerated decoding",
        Some("Let the graphics chip decode video. Uses less power; turn off if the picture shows glitches."),
        &mut pf.hw_decoding,
        HwDecoding::ALL,
        HwDecoding::label,
    );
    f.choice(
        "Video post-processing",
        Some("Smooths blocky artefacts in low quality video at the cost of processor time."),
        &mut pf.post_processing,
        PostProcessing::ALL,
        PostProcessing::label,
    );
    f.choice(
        "Skip the H.264 loop filter",
        Some("Skipping the deblocking filter speeds up decoding on slow machines but lowers quality."),
        &mut pf.skip_loop_filter,
        SkipLoopFilter::ALL,
        SkipLoopFilter::label,
    );
    f.int(
        "Decoding threads",
        Some("How many processor threads decode video. 0 picks automatically."),
        &mut pf.decoding_threads,
        0..=64,
        "",
    );

    f.section("Smooth Playback");
    f.toggle("Drop frames when the system can't keep up", None, &mut pf.drop_late_frames);
    f.toggle("Give VLC real-time priority", None, &mut pf.realtime_priority);
    f.int(
        "Process priority",
        Some("Higher values give playback more of the processor than other apps. 0 is normal."),
        &mut pf.process_priority,
        -20..=20,
        "",
    );
    if f.action_wide("Rescan plugins", None, "Rescan", Btn::Normal) {
        cx.toast(
            "VLC's plugin cache lives inside VLC.app and rebuilds itself when plugins change, so no rescan is needed.",
        );
    }

    f.advanced(|f| {
        f.section("Developer");
        f.pick(
            "Encoder",
            Some("Force a particular encoder module when converting. Automatic lets VLC choose."),
            &mut pf.encoder,
            ENCODERS,
        );
        f.pick(
            "Decoder",
            Some("Force a particular decoder module. Automatic lets VLC choose."),
            &mut pf.decoder,
            DECODERS,
        );
        f.choice(
            "Clock synchronisation",
            Some("Whether live streams follow the sender's clock. Default lets VLC decide per stream."),
            &mut pf.clock_sync,
            ClockSync::ALL,
            ClockSync::label,
        );
        f.int(
            "Clock jitter",
            Some("How much timing wobble is tolerated before the clock is corrected."),
            &mut pf.clock_jitter_ms,
            0..=60_000,
            "ms",
        );
        f.int(
            "Clock reference averaging",
            Some("How many clock samples are averaged. Higher is steadier but slower to react."),
            &mut pf.clock_averaging,
            1..=1000,
            "",
        );
    });
}

pub fn recording(f: &mut Form, s: &mut Settings, _cx: &mut Cx) {
    let r = &mut s.recording;
    f.section("Recording");
    f.path("Save recordings to", Some("The folder recordings are saved in."), &mut r.record_dir, Pick::Folder);
    f.toggle("Record without re-encoding when possible", None, &mut r.record_no_reencode);

    f.section("Casting");
    f.toggle("Discover cast devices", None, &mut r.discover_cast);
    f.choice(
        "Cast quality",
        Some("Picture quality sent to Chromecast devices. Lower values convert faster on slow machines."),
        &mut r.cast_quality,
        CastQuality::ALL,
        CastQuality::label,
    );

    f.section("Streaming");
    f.toggle("Play locally while streaming", None, &mut r.stream_play_locally);
    f.toggle("Include audio", None, &mut r.stream_audio);
    f.toggle("Include video", None, &mut r.stream_video);
    f.toggle("Include subtitles", None, &mut r.stream_subtitles);
    f.choice(
        "Encoding preset",
        Some("Faster presets convert quicker; slower ones give smaller files at the same quality."),
        &mut r.preset,
        EncodingPreset::ALL,
        EncodingPreset::label,
    );
    f.choice(
        "Tuning",
        Some("Optimise encoding for a kind of content, such as film or animation."),
        &mut r.tune,
        EncodingTune::ALL,
        EncodingTune::label,
    );
}
