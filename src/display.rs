//! Display — viewport sizing and final-frame upscale.
//!
//! We have two independent knobs:
//!
//! 1. **Viewport size** (`VIEW_W`, `VIEW_H`) — the virtual resolution we
//!    render *into*. Larger viewport = more world visible at once at the
//!    same sprite size. Default is the original J2ME 240×320.
//!
//! 2. **Render scale** — how much we upscale the final framebuffer to
//!    fill the window. Default 2×, so a 240×320 game opens in a 480×640
//!    window on a modern monitor.
//!
//! Conceptually:
//!
//!     world  ──draw──▶  offscreen render target (VIEW_W × VIEW_H)
//!                                 │
//!                                 ▼  nearest-neighbor upscale
//!                       window framebuffer (VIEW_W·S × VIEW_H·S)
//!
//! The upscale is nearest-neighbor so pixel art stays crisp. The original
//! game's UI assumed a 240×320 canvas with a 20px softkey strip at the
//! bottom; we preserve that *inside* the viewport so HUD layout code doesn't
//! need to know what the user picked for view size.

use macroquad::prelude::*;

/// One global instance, set up once in `main`. Holds the offscreen render
/// target and the chosen view/scale parameters so other code doesn't need
/// to think about them.
pub struct Display {
    /// The viewport (virtual resolution) we draw the world into.
    pub view_w: f32,
    pub view_h: f32,
    /// Upscale factor applied to the offscreen target when blitting to the
    /// real window. Window size = (view_w * scale, view_h * scale).
    pub scale: f32,
    /// Height of the UI softkey strip at the bottom of the viewport.
    /// Always 20px regardless of view size so the HUD stays readable.
    pub hud_h: f32,
    /// Loaded vector font used for rendering text.
    pub font: Font,
    /// The offscreen surface we draw into. Kept separately from the camera
    /// because we need to sample its texture during the present step.
    target: RenderTarget,
    /// Camera that writes draws into `target`.
    target_cam: Camera2D,
}

/// Hard limits so the user can't ask for a window that won't fit on a
/// reasonable monitor. Raise these if you want.
pub const MIN_VIEW: f32 = 160.0;
pub const MAX_VIEW: f32 = 1920.0;
pub const MIN_SCALE: f32 = 1.0;
pub const MAX_SCALE: f32 = 8.0;

impl Display {
    /// Create the offscreen render target and the camera that draws into it.
    pub fn new(view_w: f32, view_h: f32, scale: f32, font: Font) -> Self {
        let view_w = view_w.clamp(MIN_VIEW, MAX_VIEW);
        let view_h = view_h.clamp(MIN_VIEW, MAX_VIEW);
        let scale = scale.clamp(MIN_SCALE, MAX_SCALE);

        // Render target is created at high-resolution window size to support
        // high-resolution vector font rasterization.
        let target = render_target((view_w * scale) as u32, (view_h * scale) as u32);
        target.texture.set_filter(FilterMode::Linear);

        // `from_display_rect` already sets a negative Y zoom so (0,0) is
        // top-left and Y grows downward, matching the default screen-space
        // drawing conventions used elsewhere in the code.
        let mut target_cam = Camera2D::from_display_rect(Rect::new(0.0, 0.0, view_w, view_h));
        target_cam.render_target = Some(target.clone());

        Self {
            view_w,
            view_h,
            scale,
            // The original game's 20px softkey bar. We could scale this with
            // view size, but keeping it fixed means it's always one line of
            // 14pt text — readable on any view.
            hud_h: 20.0,
            font,
            target,
            target_cam,
        }
    }

    /// Window dimensions in real screen pixels.
    pub fn window_size(&self) -> (i32, i32) {
        ((self.view_w * self.scale) as i32, (self.view_h * self.scale) as i32)
    }

    /// Height of the play area (everything above the HUD strip).
    pub fn play_h(&self) -> f32 {
        self.view_h - self.hud_h
    }

    /// Center of the play area — where the camera follows the player.
    pub fn play_center(&self) -> Vec2 {
        vec2(self.view_w * 0.5, self.play_h() * 0.5)
    }

    /// Begin drawing the world. All `draw_*` calls between this and
    /// `end_and_present` go into the offscreen target at viewport
    /// coordinates, not the window.
    pub fn begin_world_frame(&self) {
        set_camera(&self.target_cam);
    }

    /// Stop drawing the world and upscale the offscreen target to fill the
    /// real window. Call this once per frame after all world/HUD drawing
    /// is done.
    pub fn end_and_present(&self) {
        // Switch back to the default screen camera.
        set_default_camera();

        // Clear the window border (visible if the user resized the window
        // away from the exact (view*scale) ratio).
        clear_background(BLACK);

        // Compute a centered, aspect-preserving destination rect. If the
        // window happens to be a non-integer multiple of the view, we
        // center the upscaled image and let black bars fill the gap.
        let win_w = screen_width();
        let win_h = screen_height();
        let target_aspect = self.view_w / self.view_h;
        let win_aspect = win_w / win_h;

        let (dest_w, dest_h) = if win_aspect > target_aspect {
            // Window wider than target ratio — pillarbox.
            (win_h * target_aspect, win_h)
        } else {
            // Window taller than target ratio — letterbox.
            (win_w, win_w / target_aspect)
        };
        let dest_x = (win_w - dest_w) * 0.5;
        let dest_y = (win_h - dest_h) * 0.5;

        draw_texture_ex(
            &self.target.texture,
            dest_x,
            dest_y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(dest_w, dest_h)),
                // Render-target textures use OpenGL's bottom-left origin;
                // flipping Y here puts the image right-side-up when sampled
                // with screen-space drawing.
                flip_y: true,
                ..Default::default()
            },
        );
    }

    /// Draw crisp, high-resolution vector text inside the camera's logical viewport.
    pub fn draw_text(&self, text: &str, x: f32, y: f32, font_size: f32, color: Color) {
        let size = (font_size * self.scale).round() as u16;
        let scale = 1.0 / self.scale;
        draw_text_ex(
            text,
            x,
            y,
            TextParams {
                font_size: size,
                font_scale: scale,
                font: Some(&self.font),
                color,
                ..Default::default()
            },
        );
    }

    /// Measure text using our high-resolution vector font and return logical-space dimensions.
    pub fn measure_text(&self, text: &str, font_size: f32) -> TextDimensions {
        let size = (font_size * self.scale).round() as u16;
        let scale = 1.0 / self.scale;
        macroquad::text::measure_text(text, Some(&self.font), size, scale)
    }
}
