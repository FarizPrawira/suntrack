use eframe::egui;

// A procedurally drawn gold sun (disc + 8 rays) on a transparent background,
// used as the window/taskbar icon — no image file or decoder dependency.
pub fn sun_icon() -> egui::IconData {
    let size: usize = 128;
    let mut rgba = vec![0u8; size * size * 4];
    let center = size as f32 / 2.0;
    let scale = size as f32 / 64.0;
    let disc_r = 15.0 * scale;
    let ray_inner = 20.0 * scale;
    let ray_outer = 29.0 * scale;
    const GOLD: [u8; 3] = [0xF2, 0xC1, 0x4E];
    let quarter = std::f32::consts::FRAC_PI_4; // 45° between the 8 rays

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 + 0.5 - center;
            let dy = y as f32 + 0.5 - center;
            let dist = (dx * dx + dy * dy).sqrt();
            let lit = if dist <= disc_r {
                true
            } else if (ray_inner..=ray_outer).contains(&dist) {
                let angle = dy.atan2(dx);
                let nearest_ray = (angle / quarter).round() * quarter;
                (angle - nearest_ray).abs() < 0.20
            } else {
                false
            };
            if lit {
                let i = (y * size + x) * 4;
                rgba[i] = GOLD[0];
                rgba[i + 1] = GOLD[1];
                rgba[i + 2] = GOLD[2];
                rgba[i + 3] = 0xFF;
            }
        }
    }

    egui::IconData {
        rgba,
        width: size as u32,
        height: size as u32,
    }
}
