//! Uploads decoded frames into a wgpu texture egui can draw, and works out
//! where the picture goes (aspect ratio, crop, zoom).

use egui::{Rect, TextureId, Vec2, pos2, vec2};
use egui_wgpu::wgpu;

use crate::app::VideoGeometry;
use crate::app::video::VideoOutput;
use crate::domain::effects::CropPixels;

pub struct VideoTexture {
    tex: Option<(wgpu::Texture, TextureId, u32, u32)>,
    serial: u64,
}

impl Default for VideoTexture {
    fn default() -> Self {
        Self::new()
    }
}

impl VideoTexture {
    pub fn new() -> Self {
        Self { tex: None, serial: u64::MAX }
    }

    /// Uploads the newest frame if it changed. Returns the texture and its
    /// pixel size when a picture is available.
    pub fn update(&mut self, rs: &egui_wgpu::RenderState, video: &VideoOutput) -> Option<(TextureId, [u32; 2])> {
        let serial = video.serial();
        if serial != self.serial {
            let uploaded = video.with_front(|serial, px, w, h, pitch| {
                let need_new = !matches!(&self.tex, Some((_, _, tw, th)) if *tw == w && *th == h);
                if need_new {
                    let texture = rs.device.create_texture(&wgpu::TextureDescriptor {
                        label: Some("video"),
                        size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                        view_formats: &[],
                    });
                    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                    let mut renderer = rs.renderer.write();
                    let id = match &self.tex {
                        Some((_, id, _, _)) => {
                            renderer.update_egui_texture_from_wgpu_texture(
                                &rs.device,
                                &view,
                                wgpu::FilterMode::Linear,
                                *id,
                            );
                            *id
                        }
                        None => renderer.register_native_texture(&rs.device, &view, wgpu::FilterMode::Linear),
                    };
                    self.tex = Some((texture, id, w, h));
                }
                let (texture, ..) = self.tex.as_ref().expect("texture");
                rs.queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    &px[..pitch as usize * h as usize],
                    wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(pitch), rows_per_image: Some(h) },
                    wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                );
                serial
            });
            match uploaded {
                Some(s) => self.serial = s,
                None => {
                    self.serial = serial;
                    return None;
                }
            }
        }
        video.size()?;
        self.tex.as_ref().map(|(_, id, w, h)| (*id, [*w, *h]))
    }
}

fn parse_ratio(s: &str) -> Option<f32> {
    let (a, b) = s.split_once(':')?;
    let (a, b): (f32, f32) = (a.trim().parse().ok()?, b.trim().parse().ok()?);
    (a > 0.0 && b > 0.0).then_some(a / b)
}

/// Where to draw a `w`x`h` picture inside `area`, and which part of the
/// texture to show.
pub fn fit(w: u32, h: u32, sar: (u32, u32), area: Rect, geo: &VideoGeometry, crop: &CropPixels) -> (Rect, Rect) {
    let (w, h) = (w.max(1) as f32, h.max(1) as f32);
    // Pixel crop from the effects panel, in texture space.
    let mut uv = Rect::from_min_max(
        pos2(crop.left as f32 / w, crop.top as f32 / h),
        pos2(1.0 - crop.right as f32 / w, 1.0 - crop.bottom as f32 / h),
    );
    if uv.width() <= 0.01 || uv.height() <= 0.01 {
        uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    }
    let sar = if sar.0 > 0 && sar.1 > 0 { sar.0 as f32 / sar.1 as f32 } else { 1.0 };
    let src_w = w * uv.width();
    let src_h = h * uv.height();
    // Display aspect of the visible source, or the forced one.
    let mut dar = geo.aspect.as_deref().and_then(parse_ratio).unwrap_or(src_w * sar / src_h);
    if let Some(target) = geo.crop.as_deref().and_then(parse_ratio) {
        // Crop to the target ratio, centred, in display space.
        if target > dar {
            let keep = dar / target;
            let dy = uv.height() * (1.0 - keep) / 2.0;
            uv = Rect::from_min_max(pos2(uv.min.x, uv.min.y + dy), pos2(uv.max.x, uv.max.y - dy));
        } else {
            let keep = target / dar;
            let dx = uv.width() * (1.0 - keep) / 2.0;
            uv = Rect::from_min_max(pos2(uv.min.x + dx, uv.min.y), pos2(uv.max.x - dx, uv.max.y));
        }
        dar = target;
    }
    let size = if geo.zoom > 0.0 {
        let hh = h * uv.height() * geo.zoom;
        vec2(hh * dar, hh)
    } else {
        let a = area.size();
        if a.x / a.y > dar { vec2(a.y * dar, a.y) } else { vec2(a.x, a.x / dar) }
    };
    let mut dest = Rect::from_center_size(area.center(), size);
    if geo.flip_h {
        std::mem::swap(&mut uv.min.x, &mut uv.max.x);
    }
    if geo.flip_v {
        std::mem::swap(&mut uv.min.y, &mut uv.max.y);
    }
    dest = egui::emath::GuiRounding::round_to_pixels(dest, 1.0);
    (dest, uv)
}

pub fn size_hint(w: u32, h: u32) -> Vec2 {
    vec2(w as f32, h as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area() -> Rect {
        Rect::from_min_size(pos2(0.0, 0.0), vec2(1600.0, 900.0))
    }

    #[test]
    fn letterboxes_wide_and_pillarboxes_narrow() {
        let (d, _) = fit(1920, 800, (1, 1), area(), &VideoGeometry::default(), &CropPixels::default());
        assert_eq!(d.width(), 1600.0);
        assert!((d.height() - 666.67).abs() < 1.0);
        let (d, _) = fit(640, 480, (1, 1), area(), &VideoGeometry::default(), &CropPixels::default());
        assert_eq!(d.height(), 900.0);
        assert_eq!(d.width(), 1200.0);
    }

    #[test]
    fn honours_sample_aspect_and_forced_aspect() {
        // Anamorphic DVD: 720x480 with 32:27 SAR is 16:9.
        let (d, _) = fit(720, 480, (32, 27), area(), &VideoGeometry::default(), &CropPixels::default());
        assert!((d.width() / d.height() - 16.0 / 9.0).abs() < 0.01);
        let geo = VideoGeometry { aspect: Some("4:3".into()), ..Default::default() };
        let (d, _) = fit(1920, 1080, (1, 1), area(), &geo, &CropPixels::default());
        assert!((d.width() / d.height() - 4.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn crop_ratio_trims_texture() {
        let geo = VideoGeometry { crop: Some("2.35:1".into()), ..Default::default() };
        let (d, uv) = fit(1920, 1080, (1, 1), area(), &geo, &CropPixels::default());
        assert!((d.width() / d.height() - 2.35).abs() < 0.02);
        assert!(uv.min.y > 0.1 && uv.max.y < 0.9 && uv.min.x == 0.0);
    }

    #[test]
    fn pixel_crop_and_zoom() {
        let crop = CropPixels { top: 108, bottom: 108, ..Default::default() };
        let geo = VideoGeometry { zoom: 0.5, ..Default::default() };
        let (d, uv) = fit(1920, 1080, (1, 1), area(), &geo, &crop);
        assert!((uv.min.y - 0.1).abs() < 1e-4);
        assert_eq!(d.height(), (1080.0 * 0.8 * 0.5f32).round());
    }
}
