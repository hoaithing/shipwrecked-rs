//! Day/night cycle.
//!
//! A simple time-of-day clock that runs `cycle_seconds` seconds per
//! in-game day. We overlay a tint on the world after rendering to suggest
//! the current sun position:
//!
//!   - 0.00 .. 0.20  early morning (warm orange tint)
//!   - 0.20 .. 0.70  full daylight (no tint)
//!   - 0.70 .. 0.85  sunset (deepening orange/red)
//!   - 0.85 .. 1.00  night (dark blue, partly transparent so you can still see)
//!
//! The default cycle (240s = 4 minutes per full day) is fast enough to be
//! visible during a casual play session.

use macroquad::prelude::*;

pub struct DayNight {
    /// Time progressed through a single day cycle, in seconds.
    pub time_s: f32,
    /// Total length of one in-game day in seconds.
    pub cycle_seconds: f32,
    /// Number of days survived.
    pub day_count: u32,
}

impl DayNight {
    pub fn new() -> Self {
        Self {
            time_s: 0.30 * 240.0, // start in mid-morning
            cycle_seconds: 240.0,
            day_count: 1,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.time_s += dt;
        if self.time_s >= self.cycle_seconds {
            self.time_s -= self.cycle_seconds;
            self.day_count += 1;
        }
    }

    /// 0.0 .. 1.0 progression through the day. 0=midnight, 0.5=noon.
    pub fn phase(&self) -> f32 {
        self.time_s / self.cycle_seconds
    }

    /// Human-readable label for the HUD.
    pub fn label(&self) -> &'static str {
        let p = self.phase();
        match p {
            x if x < 0.20 => "Dawn",
            x if x < 0.70 => "Day",
            x if x < 0.85 => "Dusk",
            _             => "Night",
        }
    }

    /// Tint color to overlay on the world after rendering. Premultiplied
    /// alpha so blending is `screen = screen * (1 - a) + tint * a` — i.e.
    /// a single `draw_rectangle` over the full play area.
    pub fn tint(&self) -> Color {
        let p = self.phase();
        // Helper to interpolate colors.
        let lerp = |a: (f32, f32, f32, f32), b: (f32, f32, f32, f32), t: f32| {
            Color::new(
                a.0 + (b.0 - a.0) * t,
                a.1 + (b.1 - a.1) * t,
                a.2 + (b.2 - a.2) * t,
                a.3 + (b.3 - a.3) * t,
            )
        };

        // Color stops (R, G, B, A). Alpha is the overlay strength.
        let dawn      = (1.0, 0.55, 0.30, 0.30);
        let day       = (1.0, 1.00, 1.00, 0.00);
        let dusk      = (1.0, 0.45, 0.20, 0.35);
        let night     = (0.10, 0.15, 0.45, 0.60);
        let predawn   = (0.10, 0.15, 0.45, 0.50);

        // Five segments stitched with linear interp:
        //   0.00 → 0.10  predawn → dawn       (night fades to orange)
        //   0.10 → 0.20  dawn → day            (orange fades out)
        //   0.20 → 0.70  full day              (no tint)
        //   0.70 → 0.80  day → dusk            (orange comes in)
        //   0.80 → 0.90  dusk → night          (orange darkens to blue)
        //   0.90 → 1.00  night → predawn       (stays dark)
        match p {
            x if x < 0.10 => lerp(predawn, dawn, x / 0.10),
            x if x < 0.20 => lerp(dawn, day, (x - 0.10) / 0.10),
            x if x < 0.70 => Color::new(day.0, day.1, day.2, day.3),
            x if x < 0.80 => lerp(day, dusk, (x - 0.70) / 0.10),
            x if x < 0.90 => lerp(dusk, night, (x - 0.80) / 0.10),
            x             => lerp(night, predawn, (x - 0.90) / 0.10),
        }
    }
}
