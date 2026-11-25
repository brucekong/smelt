//! NOMI 扩展创意配饰矢量渲染模块（37 款扩展配饰 + 后景层 + 屏内自适应海盗眼罩）。
//!
//! 全部采用统一 3D/2D 切平面投影基底 [`HeadCanvas`]，在 Smelt 侧边栏 36pt 头像与 44pt 行高下
//! 保持清晰的矢量轮廓与柔和微动效，同时与头部 3D 转头严格贴合不脱离。

use std::f32::consts::PI;

use gpui::{PathBuilder, Pixels, Point, Rgba, Window, point, px, rgba};

use super::genome::{NomiExpression, NomiHeadwear};

/// 3D / 2D 统一局部坐标投影画布（等价于 Canvas 2D 的 `transform(ux, uy, vx, vy, cx, cy)`）
#[derive(Clone, Copy)]
pub(crate) struct HeadCanvas {
    pub center: Point<Pixels>,
    pub r: f32,
    pub head_r: f32,
    pub ux: f32,
    pub uy: f32,
    pub vx: f32,
    pub vy: f32,
}

impl HeadCanvas {
    #[inline]
    pub fn pt(&self, lx: f32, ly: f32) -> Point<Pixels> {
        point(
            self.center.x + px(lx * self.ux + ly * self.vx),
            self.center.y + px(lx * self.uy + ly * self.vy),
        )
    }

    /// 创建局部平移 + 旋转子坐标系（等价于 `ctx.translate(tx, ty); ctx.rotate(angle)`）
    #[inline]
    pub fn sub(&self, tx: f32, ty: f32, angle: f32) -> Self {
        let (s, c) = angle.sin_cos();
        Self {
            center: self.pt(tx, ty),
            r: self.r,
            head_r: self.head_r,
            ux: c * self.ux + s * self.vx,
            uy: c * self.uy + s * self.vy,
            vx: -s * self.ux + c * self.vx,
            vy: -s * self.uy + c * self.vy,
        }
    }

    #[inline]
    pub fn fill(&self) -> LocalFill<'_> {
        LocalFill {
            pb: PathBuilder::fill(),
            c: self,
        }
    }

    #[inline]
    pub fn stroke(&self, width: f32) -> LocalStroke<'_> {
        LocalStroke {
            pb: PathBuilder::stroke(px(width.max(0.45))),
            c: self,
        }
    }

    pub fn fill_ellipse(
        &self,
        cx: f32,
        cy: f32,
        rx: f32,
        ry: f32,
        rot: f32,
        color: Rgba,
        window: &mut Window,
    ) {
        let sub = if rot.abs() > 1e-4 {
            self.sub(cx, cy, rot)
        } else {
            Self {
                center: self.pt(cx, cy),
                ..*self
            }
        };
        let steps = 28;
        let mut p = sub.fill();
        for i in 0..=steps {
            let theta = (i as f32 / steps as f32) * 2.0 * PI;
            let x = rx * theta.cos();
            let y = ry * theta.sin();
            if i == 0 {
                p.move_to(x, y);
            } else {
                p.line_to(x, y);
            }
        }
        p.close();
        p.paint(color, window);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn stroke_ellipse(
        &self,
        cx: f32,
        cy: f32,
        rx: f32,
        ry: f32,
        rot: f32,
        width: f32,
        color: Rgba,
        window: &mut Window,
    ) {
        let sub = if rot.abs() > 1e-4 {
            self.sub(cx, cy, rot)
        } else {
            Self {
                center: self.pt(cx, cy),
                ..*self
            }
        };
        let steps = 28;
        let mut p = sub.stroke(width);
        for i in 0..=steps {
            let theta = (i as f32 / steps as f32) * 2.0 * PI;
            let x = rx * theta.cos();
            let y = ry * theta.sin();
            if i == 0 {
                p.move_to(x, y);
            } else {
                p.line_to(x, y);
            }
        }
        p.close();
        p.paint(color, window);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn stroke_arc(
        &self,
        cx: f32,
        cy: f32,
        rx: f32,
        ry: f32,
        start_a: f32,
        end_a: f32,
        width: f32,
        color: Rgba,
        window: &mut Window,
    ) {
        let steps = 28;
        let mut p = self.stroke(width);
        for i in 0..=steps {
            let theta = start_a + (end_a - start_a) * (i as f32 / steps as f32);
            let x = cx + rx * theta.cos();
            let y = cy + ry * theta.sin();
            if i == 0 {
                p.move_to(x, y);
            } else {
                p.line_to(x, y);
            }
        }
        p.paint(color, window);
    }

    /// 水平圆角胶囊填充（用于无人机机身、面罩与发光双眼）
    pub fn fill_pill(
        &self,
        cx: f32,
        cy: f32,
        w: f32,
        h: f32,
        color: Rgba,
        window: &mut Window,
    ) {
        let rad = (w.min(h) * 0.5).max(0.01);
        let hx = (w * 0.5 - rad).max(0.0);
        let hy = (h * 0.5 - rad).max(0.0);
        let mut p = self.fill();
        let corners = [
            (cx + hx, cy - hy, -PI * 0.5, 0.0),
            (cx + hx, cy + hy, 0.0, PI * 0.5),
            (cx - hx, cy + hy, PI * 0.5, PI),
            (cx - hx, cy - hy, PI, 1.5 * PI),
        ];
        let mut first = true;
        for (ox, oy, a0, a1) in corners {
            for i in 0..=5 {
                let a = a0 + (a1 - a0) * (i as f32 / 5.0);
                let x = ox + a.cos() * rad;
                let y = oy + a.sin() * rad;
                if first {
                    p.move_to(x, y);
                    first = false;
                } else {
                    p.line_to(x, y);
                }
            }
        }
        p.close();
        p.paint(color, window);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn stroke_pill(
        &self,
        cx: f32,
        cy: f32,
        w: f32,
        h: f32,
        width: f32,
        color: Rgba,
        window: &mut Window,
    ) {
        let rad = (w.min(h) * 0.5).max(0.01);
        let hx = (w * 0.5 - rad).max(0.0);
        let hy = (h * 0.5 - rad).max(0.0);
        let mut p = self.stroke(width);
        let corners = [
            (cx + hx, cy - hy, -PI * 0.5, 0.0),
            (cx + hx, cy + hy, 0.0, PI * 0.5),
            (cx - hx, cy + hy, PI * 0.5, PI),
            (cx - hx, cy - hy, PI, 1.5 * PI),
        ];
        let mut first = true;
        for (ox, oy, a0, a1) in corners {
            for i in 0..=5 {
                let a = a0 + (a1 - a0) * (i as f32 / 5.0);
                let x = ox + a.cos() * rad;
                let y = oy + a.sin() * rad;
                if first {
                    p.move_to(x, y);
                    first = false;
                } else {
                    p.line_to(x, y);
                }
            }
        }
        p.close();
        p.paint(color, window);
    }

    #[inline]
    pub fn fill_circle(&self, cx: f32, cy: f32, rad: f32, color: Rgba, window: &mut Window) {
        self.fill_ellipse(cx, cy, rad, rad, 0.0, color, window);
    }

    #[inline]
    pub fn stroke_circle(
        &self,
        cx: f32,
        cy: f32,
        rad: f32,
        width: f32,
        color: Rgba,
        window: &mut Window,
    ) {
        self.stroke_ellipse(cx, cy, rad, rad, 0.0, width, color, window);
    }

    pub fn stroke_line(
        &self,
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
        width: f32,
        color: Rgba,
        window: &mut Window,
    ) {
        let mut p = self.stroke(width);
        p.move_to(x0, y0);
        p.line_to(x1, y1);
        p.paint(color, window);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn stroke_quad(
        &self,
        x0: f32,
        y0: f32,
        cx: f32,
        cy: f32,
        x1: f32,
        y1: f32,
        width: f32,
        color: Rgba,
        window: &mut Window,
    ) {
        let mut p = self.stroke(width);
        p.move_to(x0, y0);
        p.quad_to(cx, cy, x1, y1);
        p.paint(color, window);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn stroke_cubic(
        &self,
        x0: f32,
        y0: f32,
        c1x: f32,
        c1y: f32,
        c2x: f32,
        c2y: f32,
        x1: f32,
        y1: f32,
        width: f32,
        color: Rgba,
        window: &mut Window,
    ) {
        let mut p = self.stroke(width);
        p.move_to(x0, y0);
        p.cubic_to(c1x, c1y, c2x, c2y, x1, y1);
        p.paint(color, window);
    }

    /// 四角星芒
    pub fn fill_star4(&self, cx: f32, cy: f32, size: f32, color: Rgba, window: &mut Window) {
        let inner = size * 0.28;
        let mut p = self.fill();
        for i in 0..8 {
            let a = (i as f32) * PI * 0.25;
            let rad = if i % 2 == 0 { size } else { inner };
            let x = cx + a.cos() * rad;
            let y = cy + a.sin() * rad;
            if i == 0 {
                p.move_to(x, y);
            } else {
                p.line_to(x, y);
            }
        }
        p.close();
        p.paint(color, window);
    }

    /// 五角星
    pub fn fill_star5(
        &self,
        cx: f32,
        cy: f32,
        outer_r: f32,
        inner_r: f32,
        rot: f32,
        color: Rgba,
        window: &mut Window,
    ) {
        let mut p = self.fill();
        for i in 0..10 {
            let a = rot - PI * 0.5 + (i as f32) * PI / 5.0;
            let rad = if i % 2 == 0 { outer_r } else { inner_r };
            let x = cx + a.cos() * rad;
            let y = cy + a.sin() * rad;
            if i == 0 {
                p.move_to(x, y);
            } else {
                p.line_to(x, y);
            }
        }
        p.close();
        p.paint(color, window);
    }
}

pub(crate) struct LocalFill<'a> {
    pb: PathBuilder,
    c: &'a HeadCanvas,
}

impl<'a> LocalFill<'a> {
    #[inline]
    pub fn move_to(&mut self, x: f32, y: f32) {
        self.pb.move_to(self.c.pt(x, y));
    }

    #[inline]
    pub fn line_to(&mut self, x: f32, y: f32) {
        self.pb.line_to(self.c.pt(x, y));
    }

    /// 二次贝塞尔曲线（采用直觉顺序：先控制点，后终点）
    #[inline]
    pub fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        self.pb.curve_to(self.c.pt(x, y), self.c.pt(cx, cy));
    }

    /// 三次贝塞尔曲线（采用直觉顺序：先 c1、c2，后终点）
    #[inline]
    pub fn cubic_to(&mut self, c1x: f32, c1y: f32, c2x: f32, c2y: f32, x: f32, y: f32) {
        self.pb
            .cubic_bezier_to(self.c.pt(x, y), self.c.pt(c1x, c1y), self.c.pt(c2x, c2y));
    }

    #[inline]
    pub fn close(&mut self) {
        self.pb.close();
    }

    #[inline]
    pub fn paint(self, color: Rgba, window: &mut Window) {
        if let Ok(path) = self.pb.build() {
            window.paint_path(path, color);
        }
    }
}

pub(crate) struct LocalStroke<'a> {
    pb: PathBuilder,
    c: &'a HeadCanvas,
}

impl<'a> LocalStroke<'a> {
    #[inline]
    pub fn move_to(&mut self, x: f32, y: f32) {
        self.pb.move_to(self.c.pt(x, y));
    }

    #[inline]
    pub fn line_to(&mut self, x: f32, y: f32) {
        self.pb.line_to(self.c.pt(x, y));
    }

    #[inline]
    pub fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        self.pb.curve_to(self.c.pt(x, y), self.c.pt(cx, cy));
    }

    #[inline]
    pub fn cubic_to(&mut self, c1x: f32, c1y: f32, c2x: f32, c2y: f32, x: f32, y: f32) {
        self.pb
            .cubic_bezier_to(self.c.pt(x, y), self.c.pt(c1x, c1y), self.c.pt(c2x, c2y));
    }

    #[inline]
    pub fn close(&mut self) {
        self.pb.close();
    }

    #[inline]
    pub fn paint(self, color: Rgba, window: &mut Window) {
        if let Ok(path) = self.pb.build() {
            window.paint_path(path, color);
        }
    }
}

#[inline]
fn rgba_alpha(rgb: u32, alpha: f32) -> Rgba {
    let a = (alpha.clamp(0.0, 1.0) * 255.0).round() as u32;
    rgba((rgb << 8) | a)
}

#[inline]
fn eval_cubic(
    t: f32,
    p0: (f32, f32),
    p1: (f32, f32),
    p2: (f32, f32),
    p3: (f32, f32),
) -> (f32, f32, f32) {
    let mt = 1.0 - t;
    let x = mt * mt * mt * p0.0
        + 3.0 * mt * mt * t * p1.0
        + 3.0 * mt * t * t * p2.0
        + t * t * t * p3.0;
    let y = mt * mt * mt * p0.1
        + 3.0 * mt * mt * t * p1.1
        + 3.0 * mt * t * t * p2.1
        + t * t * t * p3.1;
    let dx =
        3.0 * mt * mt * (p1.0 - p0.0) + 6.0 * mt * t * (p2.0 - p1.0) + 3.0 * t * t * (p3.0 - p2.0);
    let dy =
        3.0 * mt * mt * (p1.1 - p0.1) + 6.0 * mt * t * (p2.1 - p1.1) + 3.0 * t * t * (p3.1 - p2.1);
    (x, y, dy.atan2(dx))
}

#[inline]
fn ray_circle_end(ox: f32, oy: f32, tx: f32, ty: f32, radius: f32) -> (f32, f32) {
    let dx = tx - ox;
    let dy = ty - oy;
    let a = dx * dx + dy * dy;
    if a <= 1e-6 {
        return (ox, oy);
    }
    let b = 2.0 * (ox * dx + oy * dy);
    let c = ox * ox + oy * oy - radius * radius;
    let disc = (b * b - 4.0 * a * c).max(0.0).sqrt();
    let t = ((-b + disc) / (2.0 * a)).max(0.0);
    (ox + t * dx, oy + t * dy)
}

#[inline]
fn color_with_alpha(color: Rgba, alpha: f32) -> Rgba {
    Rgba {
        a: (color.a * alpha).clamp(0.0, 1.0),
        ..color
    }
}

// ============================================================================
// 一、后景配饰层（位于球体后壳与金属外圈之后：虎尾、混天绫、恶魔翼尾、天使羽翼、行星星轨背弧、伴飞无人机后景）
// ============================================================================

pub(crate) fn draw_headwear_back(
    headwear: NomiHeadwear,
    c: &HeadCanvas,
    time_sec: f32,
    glow_color: Rgba,
    pitch: f32,
    window: &mut Window,
) {
    match headwear {
        NomiHeadwear::Tiger => draw_tiger_tail_back(c, time_sec, window),
        NomiHeadwear::Nezha => draw_nezha_sash_back(c, time_sec, window),
        NomiHeadwear::CyberDevil => draw_devil_back(c, time_sec, window),
        NomiHeadwear::AngelHalo => draw_angel_wings_back(c, time_sec, window),
        NomiHeadwear::SeekerWings => draw_seeker_wings_back(c, time_sec, window),
        NomiHeadwear::DriftSamurai => draw_drift_swords_back(c, time_sec, window),
        NomiHeadwear::GrimlockHelm => draw_grimlock_wings_back(c, time_sec, window),
        NomiHeadwear::ArceeHelm => draw_arcee_pods_back(c, time_sec, window),
        NomiHeadwear::HotrodHelm => draw_hotrod_spoiler_back(c, time_sec, window),
        NomiHeadwear::DevastatorHelm => draw_devastator_shoulders_back(c, time_sec, window),
        NomiHeadwear::WheeljackHelm => draw_wheeljack_wings_back(c, time_sec, window),
        NomiHeadwear::OrbitRing => draw_orbit_ring_half(c, time_sec, glow_color, pitch, true, window),
        NomiHeadwear::CybertronRing => draw_cybertron_ring_half(c, time_sec, glow_color, pitch, true, window),
        NomiHeadwear::MiniDrone => draw_mini_drone(c, time_sec, glow_color, true, window),
        NomiHeadwear::LaserbeakDrone => draw_laserbeak_drone(c, time_sec, true, window),
        _ => {}
    }
}

fn draw_tiger_tail_back(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let wag1 = (time_sec * 3.4).sin() * r * 0.075;
    let wag2 = (time_sec * 3.4 - 0.85).sin() * r * 0.125;

    let p0 = (r * 0.72, r * 0.44);
    let p1 = (r * 1.15 + wag1 * 0.45, r * 0.52 + wag1 * 0.25);
    let p2 = (r * 1.34 + wag1, r * 0.16 - wag1 * 0.35);
    let p3 = (r * 1.18 + wag2, -r * 0.05 - wag2 * 0.45);
    let tail_w = (r * 0.145).max(1.9);

    // 外轮廓 + 暖橘主体
    c.stroke_cubic(p0.0, p0.1, p1.0, p1.1, p2.0, p2.1, p3.0, p3.1, tail_w + (r * 0.055).max(0.9), rgba(0x7C2D12FF), window);
    c.stroke_cubic(p0.0, p0.1, p1.0, p1.1, p2.0, p2.1, p3.0, p3.1, tail_w, rgba(0xFB923CFF), window);
    c.stroke_cubic(
        p0.0,
        p0.1 - tail_w * 0.18,
        p1.0,
        p1.1 - tail_w * 0.18,
        p2.0 - tail_w * 0.14,
        p2.1 - tail_w * 0.10,
        p3.0 - tail_w * 0.10,
        p3.1 - tail_w * 0.10,
        (tail_w * 0.24).max(0.5),
        rgba(0xFFF7ED42),
        window,
    );

    // 沿曲线切线绘出虎尾深棕斑纹
    for t in [0.36_f32, 0.56, 0.74, 0.89] {
        let (x, y, ang) = eval_cubic(t, p0, p1, p2, p3);
        let seg = c.sub(x, y, ang);
        seg.fill_ellipse(0.0, 0.0, tail_w * 0.20, tail_w * 0.48, 0.0, rgba(0x3B1106FF), window);
    }

    // 软萌深棕尾尖
    let (tx, ty, tang) = eval_cubic(0.99, p0, p1, p2, p3);
    let tip = c.sub(tx, ty, tang);
    tip.fill_ellipse(0.0, 0.0, tail_w * 0.54, tail_w * 0.48, 0.0, rgba(0x3B1106FF), window);
}

fn draw_nezha_sash_back(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let w1 = (time_sec * 3.6).sin() * r * 0.085;
    let w2 = (time_sec * 3.6 - 0.9).sin() * r * 0.115;
    let w3 = (time_sec * 3.6 - 0.5).cos() * r * 0.095;
    let main_w = (r * 0.15).max(2.0);

    let draw_ribbon = |pts: [(f32, f32); 4], window: &mut Window| {
        let [p0, p1, p2, p3] = pts;
        // 细金镶边
        c.stroke_cubic(p0.0, p0.1, p1.0, p1.1, p2.0, p2.1, p3.0, p3.1, main_w + (r * 0.048).max(0.8), rgba(0xFCD34DFF), window);
        // 炽红主带身
        c.stroke_cubic(p0.0, p0.1, p1.0, p1.1, p2.0, p2.1, p3.0, p3.1, main_w, rgba(0xEF4444FF), window);
        // 柔光丝绸高光
        c.stroke_cubic(p0.0, p0.1, p1.0, p1.1, p2.0, p2.1, p3.0, p3.1, (main_w * 0.25).max(0.5), rgba(0xFEE2E24D), window);
    };

    // 左侧翻飞混天绫（略微收敛适配侧边栏）
    draw_ribbon(
        [
            (-r * 0.72, r * 0.30),
            (-r * 1.14, r * 0.52 + w1 * 0.5),
            (-r * 1.38 + w1 * 0.3, -r * 0.12 + w2 * 0.6),
            (-r * 1.15 + w3 * 0.4, -r * 0.60 + w2),
        ],
        window,
    );
    // 右侧翻飞混天绫
    draw_ribbon(
        [
            (r * 0.72, -r * 0.24),
            (r * 1.18, -r * 0.58 - w1 * 0.5),
            (r * 1.38 - w1 * 0.3, r * 0.08 - w2 * 0.5),
            (r * 1.16 - w3 * 0.4, r * 0.54 - w2 * 0.8),
        ],
        window,
    );

    // 暖金火灵星点
    for (ex0, ey0, ph, sr) in [
        (-r * 1.24, -r * 0.30, 0.4_f32, r * 0.042),
        (r * 1.26, r * 0.24, 2.2, r * 0.038),
        (r * 1.08, -r * 0.60, 4.1, r * 0.034),
    ] {
        let t = time_sec * 2.8 + ph;
        let ex = ex0 + t.sin() * r * 0.05;
        let ey = ey0 + (t * 0.9).cos() * r * 0.06;
        let alpha = 0.55 + 0.45 * (t * 1.5).sin();
        c.fill_circle(ex, ey, sr * 1.5, rgba_alpha(0xFB923C, alpha * 0.40), window);
        c.fill_circle(ex, ey, sr, rgba_alpha(0xFEF08A, alpha), window);
    }
}

fn draw_devil_back(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let pulse = 0.75 + 0.25 * (time_sec * 3.8).sin();

    // 1) 左右轻扇赛博暗红蝙蝠翼
    for side in [-1.0_f32, 1.0] {
        let flap = (time_sec * 4.4 + side * 0.25).sin() * 0.14;
        let w = c.sub(side * r * 0.82, -r * 0.12, side * (-0.12 + flap));

        let mut wing = w.fill();
        wing.move_to(0.0, -r * 0.04);
        wing.quad_to(side * r * 0.26, -r * 0.34, side * r * 0.52, -r * 0.24);
        wing.quad_to(side * r * 0.40, -r * 0.06, side * r * 0.44, r * 0.09);
        wing.quad_to(side * r * 0.29, r * 0.03, side * r * 0.24, r * 0.16);
        wing.quad_to(side * r * 0.12, r * 0.07, 0.0, r * 0.13);
        wing.close();
        wing.paint(rgba(0x4C0519FF), window);

        let mut border = w.stroke((r * 0.028).max(0.6));
        border.move_to(0.0, -r * 0.04);
        border.quad_to(side * r * 0.26, -r * 0.34, side * r * 0.52, -r * 0.24);
        border.quad_to(side * r * 0.40, -r * 0.06, side * r * 0.44, r * 0.09);
        border.quad_to(side * r * 0.29, r * 0.03, side * r * 0.24, r * 0.16);
        border.quad_to(side * r * 0.12, r * 0.07, 0.0, r * 0.13);
        border.paint(rgba_alpha(0xF43F5E, 0.85 * pulse), window);

        // 翼骨霓虹细线
        w.stroke_line(side * r * 0.10, -r * 0.09, side * r * 0.42, r * 0.07, (r * 0.018).max(0.5), rgba_alpha(0xFB7185, 0.58 * pulse), window);
        w.stroke_line(side * r * 0.10, -r * 0.05, side * r * 0.23, r * 0.14, (r * 0.018).max(0.5), rgba_alpha(0xFB7185, 0.58 * pulse), window);
    }

    // 2) 右下摇摆桃心尖恶魔尾
    let wag1 = (time_sec * 3.8).sin() * r * 0.075;
    let wag2 = (time_sec * 3.8 - 0.9).sin() * r * 0.13;
    let p0 = (r * 0.70, r * 0.46);
    let p1 = (r * 1.12 + wag1 * 0.4, r * 0.58 + wag1 * 0.25);
    let p2 = (r * 1.32 + wag1, r * 0.26 - wag1 * 0.25);
    let p3 = (r * 1.20 + wag2, r * 0.02 - wag2 * 0.40);
    let tip_ang = (p3.1 - p2.1).atan2(p3.0 - p2.0);

    c.stroke_cubic(p0.0, p0.1, p1.0, p1.1, p2.0, p2.1, p3.0, p3.1, (r * 0.088).max(1.5), rgba(0x3B0718FF), window);
    c.stroke_cubic(p0.0, p0.1, p1.0, p1.1, p2.0, p2.1, p3.0, p3.1, (r * 0.052).max(0.9), rgba(0xE11D48FF), window);

    // 尾端发光桃心矛尖
    let tip = c.sub(p3.0, p3.1, tip_ang);
    let ts = r * 0.135;
    tip.fill_circle(ts * 0.25, 0.0, ts * 0.85, rgba_alpha(0xF43F5E, 0.30 * pulse), window);
    let mut spade = tip.fill();
    spade.move_to(ts * 1.12, 0.0);
    spade.cubic_to(ts * 0.44, -ts * 0.55, -ts * 0.44, -ts * 0.90, -ts * 0.30, -ts * 0.16);
    spade.line_to(-ts * 0.06, 0.0);
    spade.line_to(-ts * 0.30, ts * 0.16);
    spade.cubic_to(-ts * 0.44, ts * 0.90, ts * 0.44, ts * 0.55, ts * 1.12, 0.0);
    spade.close();
    spade.paint(rgba(0xF43F5EFF), window);
}

fn draw_angel_wings_back(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let pulse = 0.80 + 0.20 * (time_sec * 3.4).sin();

    for side in [-1.0_f32, 1.0] {
        let flap = (time_sec * 3.6 + side * 0.2).sin() * 0.13;
        let root = c.sub(side * r * 0.82, -r * 0.06, side * (-0.10 + flap));

        // 冰蓝柔光底层
        root.fill_ellipse(side * r * 0.28, 0.0, r * 0.34, r * 0.24, 0.0, rgba_alpha(0xBAE6FD, 0.26 * pulse), window);

        for (ang, len, w, oy) in [
            (side * 0.36_f32, r * 0.40, r * 0.11, r * 0.09),
            (side * 0.12, r * 0.50, r * 0.125, r * 0.01),
            (side * -0.16, r * 0.58, r * 0.135, -r * 0.07),
        ] {
            let f = root.sub(0.0, oy, ang);
            let mut feather = f.fill();
            feather.move_to(0.0, -w * 0.45);
            feather.cubic_to(side * len * 0.42, -w * 0.95, side * len * 0.94, -w * 0.72, side * len, 0.0);
            feather.cubic_to(side * len * 0.90, w * 0.75, side * len * 0.36, w * 0.84, 0.0, w * 0.45);
            feather.close();
            feather.paint(rgba(0xF0F9FFFF), window);

            let mut rim = f.stroke((r * 0.022).max(0.5));
            rim.move_to(0.0, -w * 0.45);
            rim.cubic_to(side * len * 0.42, -w * 0.95, side * len * 0.94, -w * 0.72, side * len, 0.0);
            rim.cubic_to(side * len * 0.90, w * 0.75, side * len * 0.36, w * 0.84, 0.0, w * 0.45);
            rim.close();
            rim.paint(rgba_alpha(0x93C5FD, 0.78), window);
        }
    }
}

// ============================================================================
// 二、屏内自适应海盗眼罩层（精准贴合 53 款表情左眼坐标与尺寸，不露边不越出表圈）
// ============================================================================

#[inline]
pub(crate) fn expression_uses_hand_row(expr: NomiExpression) -> bool {
    matches!(
        expr,
        NomiExpression::OkGesture
            | NomiExpression::Jubilation
            | NomiExpression::Proud
            | NomiExpression::Gratitude
            | NomiExpression::Triumph
            | NomiExpression::Admiration
            | NomiExpression::Shy
            | NomiExpression::Dreamy
            | NomiExpression::Concerned
            | NomiExpression::MusicGuitarAcoustic
            | NomiExpression::MusicGuitarJoy
            | NomiExpression::MusicGuitarDeep
            | NomiExpression::MusicElectricGuitar
            | NomiExpression::MusicRockStar
            | NomiExpression::MusicViolinJoy
            | NomiExpression::MusicViolinDeep
            | NomiExpression::MusicViolinSerene
            | NomiExpression::MusicCello
            | NomiExpression::MusicSnareDrum
            | NomiExpression::MusicKeyboard
            | NomiExpression::MusicMaracas
            | NomiExpression::MusicTambourine
            | NomiExpression::MusicDjDual
            | NomiExpression::MusicSaxophoneJoy
            | NomiExpression::MusicSaxophoneDeep
            | NomiExpression::MusicVocalJoy
            | NomiExpression::MusicVocalDeep
            | NomiExpression::MusicVocalSerene
            | NomiExpression::MusicConductor
            | NomiExpression::MusicDjSingle
            | NomiExpression::MusicLightsticks
            | NomiExpression::MusicNioSpeaker
            | NomiExpression::MusicRadio
            | NomiExpression::MusicDancingNotes
            | NomiExpression::MusicConfetti
            | NomiExpression::MusicPeaceV
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_pirate_eyepatch_overlay(
    expr: NomiExpression,
    c: &HeadCanvas,
    eye_dx: f32,
    eye_y: f32,
    hand_eye_y: f32,
    offset_x: f32,
    offset_y: f32,
    expr_sx: f32,
    expr_sy: f32,
    window: &mut Window,
) {
    let r = c.r;
    let is_hand_or_music = expression_uses_hand_row(expr);

    let mut lx = -eye_dx;
    let mut ly = if is_hand_or_music { hand_eye_y } else { eye_y };
    let mut pw = r * 0.265;
    let mut ph_top = r * 0.255;
    let mut ph_bot = r * 0.295;

    match expr {
        NomiExpression::Interest => {
            lx = -r * 0.26;
        }
        NomiExpression::Desire => {
            ly = -r * 0.11;
        }
        NomiExpression::Acceptance => {
            ly = -r * 0.09;
        }
        NomiExpression::Sleeping => {
            ly = r * 0.02;
        }
        NomiExpression::Astonishment | NomiExpression::Surprise | NomiExpression::Excitement => {
            pw = r * 0.29;
            ph_top = r * 0.295;
            ph_bot = r * 0.32;
        }
        NomiExpression::Relaxed => {
            pw = r * 0.31;
            ph_top = r * 0.27;
            ph_bot = r * 0.30;
        }
        _ if is_hand_or_music => {
            pw = r * 0.25;
            ph_top = r * 0.21;
            ph_bot = r * 0.23;
        }
        _ => {}
    }

    let px_c = offset_x + lx * expr_sx;
    let py_c = offset_y + ly * expr_sy;

    // 1) 斜跨屏幕的黑色皮带（严格裁在屏幕内圆 R = 0.98r 边界，绝不伸出金属外圈）
    let rim_r = r * 0.98;
    let (s1x, s1y) = ray_circle_end(px_c, py_c, -r * 1.10, py_c - r * 0.22, rim_r);
    let (s2x, s2y) = ray_circle_end(px_c, py_c, r * 0.56, -r * 1.10, rim_r);

    let mut strap = c.stroke((r * 0.055).max(0.95));
    strap.move_to(s1x, s1y);
    strap.line_to(px_c, py_c);
    strap.line_to(s2x, s2y);
    strap.paint(rgba(0x18181BFF), window);

    let mut sheen = c.stroke((r * 0.016).max(0.45));
    sheen.move_to(s1x, s1y - r * 0.015);
    sheen.line_to(px_c, py_c - r * 0.015);
    sheen.line_to(s2x - r * 0.015, s2y);
    sheen.paint(rgba(0xFFFFFF22), window);

    // 2) 加大饱满盾弧形黑皮眼罩主体 + 古铜金包边
    let draw_shield = |scale: f32, fill_col: Rgba, window: &mut Window| {
        let w = pw * scale;
        let ht = ph_top * scale;
        let hb = ph_bot * scale;
        let mut s = c.fill();
        s.move_to(px_c - w, py_c - ht * 0.56);
        s.quad_to(px_c, py_c - ht * 1.05, px_c + w, py_c - ht * 0.56);
        s.cubic_to(px_c + w * 1.04, py_c + hb * 0.52, px_c + w * 0.36, py_c + hb, px_c, py_c + hb);
        s.cubic_to(px_c - w * 0.36, py_c + hb, px_c - w * 1.04, py_c + hb * 0.52, px_c - w, py_c - ht * 0.56);
        s.close();
        s.paint(fill_col, window);
    };
    draw_shield(1.0, rgba(0x121318FF), window);
    // 左上微光皮革质感
    c.fill_ellipse(px_c - pw * 0.22, py_c - ph_top * 0.20, pw * 0.42, ph_top * 0.34, -0.2, rgba(0x52525B66), window);

    let mut rim = c.stroke((r * 0.032).max(0.65));
    rim.move_to(px_c - pw, py_c - ph_top * 0.56);
    rim.quad_to(px_c, py_c - ph_top * 1.05, px_c + pw, py_c - ph_top * 0.56);
    rim.cubic_to(px_c + pw * 1.04, py_c + ph_bot * 0.52, px_c + pw * 0.36, py_c + ph_bot, px_c, py_c + ph_bot);
    rim.cubic_to(px_c - pw * 0.36, py_c + ph_bot, px_c - pw * 1.04, py_c + ph_bot * 0.52, px_c - pw, py_c - ph_top * 0.56);
    rim.close();
    rim.paint(rgba(0xC27803FF), window);
}

/// 屏内绘制爵士跑车半透明冰蓝全息护目镜（贴合眼位且透出镜后双眼神态）
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_jazz_visor_overlay(
    expr: NomiExpression,
    c: &HeadCanvas,
    eye_y: f32,
    hand_eye_y: f32,
    offset_x: f32,
    offset_y: f32,
    expr_sy: f32,
    time_sec: f32,
    window: &mut Window,
) {
    let r = c.r;
    let is_hand_or_music = expression_uses_hand_row(expr);
    let mut ly = if is_hand_or_music { hand_eye_y } else { eye_y };
    if expr == NomiExpression::Desire {
        ly -= r * 0.08;
    } else if expr == NomiExpression::Acceptance {
        ly -= r * 0.04;
    } else if expr == NomiExpression::Sleeping {
        ly += r * 0.05;
    }
    let vx = offset_x;
    let vy = offset_y + ly * expr_sy;
    let vw = r * 0.66;
    let vh = r * 0.205;

    let mut visor = c.fill();
    visor.move_to(vx - vw, vy - vh * 0.85);
    visor.quad_to(vx, vy - vh * 1.05, vx + vw, vy - vh * 0.85);
    visor.line_to(vx + vw * 0.92, vy + vh * 0.75);
    visor.line_to(vx + r * 0.12, vy + vh * 0.88);
    visor.line_to(vx, vy + vh * 0.45);
    visor.line_to(vx - r * 0.12, vy + vh * 0.88);
    visor.line_to(vx - vw * 0.92, vy + vh * 0.75);
    visor.close();
    visor.paint(rgba_alpha(0x38BDF8, 0.26), window);

    // 护目镜上沿斜掠反光亮条与金属外框
    let glint = (time_sec * 1.8).sin() * r * 0.05;
    c.stroke_line(
        vx - r * 0.34 + glint,
        vy - vh * 0.65,
        vx - r * 0.14 + glint,
        vy + vh * 0.52,
        (r * 0.055).max(0.8),
        rgba(0xFFFFFF2A),
        window,
    );

    let mut frame = c.stroke((r * 0.028).max(0.55));
    frame.move_to(vx - vw, vy - vh * 0.85);
    frame.quad_to(vx, vy - vh * 1.05, vx + vw, vy - vh * 0.85);
    frame.line_to(vx + vw * 0.92, vy + vh * 0.75);
    frame.line_to(vx + r * 0.12, vy + vh * 0.88);
    frame.line_to(vx, vy + vh * 0.45);
    frame.line_to(vx - r * 0.12, vy + vh * 0.88);
    frame.line_to(vx - vw * 0.92, vy + vh * 0.75);
    frame.close();
    frame.paint(rgba_alpha(0xE0F2FE, 0.74), window);
}

// ============================================================================
// 三、37 款扩展前置头部配饰
// ============================================================================

pub(crate) fn draw_extended_headwear(
    headwear: NomiHeadwear,
    c: &HeadCanvas,
    time_sec: f32,
    glow_color: Rgba,
    pitch: f32,
    window: &mut Window,
) {
    match headwear {
        NomiHeadwear::Zhuangyuan => draw_zhuangyuan(c, time_sec, window),
        NomiHeadwear::Bainiangzi => draw_bainiangzi(c, time_sec, window),
        NomiHeadwear::VeiledGoddess => draw_veiled_goddess(c, time_sec, window),
        NomiHeadwear::Tiger => draw_tiger(c, time_sec, window),
        NomiHeadwear::OfficerCap => draw_officer_cap(c, window),
        NomiHeadwear::MonkeyKing => draw_monkey_king(c, time_sec, window),
        NomiHeadwear::Nezha => draw_nezha(c, time_sec, window),
        NomiHeadwear::BambooHat => draw_bamboo_hat(c, time_sec, window),
        NomiHeadwear::Caishen => draw_caishen(c, time_sec, window),
        NomiHeadwear::BambooCopter => draw_bamboo_copter(c, time_sec, window),
        NomiHeadwear::RubberDuck => draw_rubber_duck(c, time_sec, window),
        NomiHeadwear::CyberDevil => draw_cyber_devil(c, time_sec, window),
        NomiHeadwear::Wizard => draw_wizard(c, time_sec, window),
        NomiHeadwear::Pirate => draw_pirate(c, time_sec, window),
        NomiHeadwear::AngelHalo => draw_angel_halo(c, time_sec, window),
        NomiHeadwear::Chef => draw_chef(c, time_sec, window),
        NomiHeadwear::Pilot => draw_pilot(c, time_sec, window),
        NomiHeadwear::Doctor => draw_doctor(c, time_sec, window),
        NomiHeadwear::StrawHat => draw_straw_hat(c, time_sec, window),
        NomiHeadwear::Cowboy => draw_cowboy(c, time_sec, window),
        NomiHeadwear::Gege => draw_gege(c, time_sec, window),
        NomiHeadwear::Barrister => draw_barrister(c, time_sec, window),
        NomiHeadwear::Kippah => draw_kippah(c, time_sec, window),
        NomiHeadwear::BaseballCap => draw_baseball_cap(c, window),
        NomiHeadwear::BucketHat => draw_bucket_hat(c, time_sec, window),
        NomiHeadwear::Graduation => draw_graduation(c, time_sec, window),
        NomiHeadwear::Doctoral => draw_doctoral(c, time_sec, window),
        NomiHeadwear::SaudiPrince => draw_saudi_prince(c, time_sec, window),
        NomiHeadwear::TanBeret => draw_tan_beret(c, window),
        NomiHeadwear::Newsboy => draw_newsboy(c, window),
        NomiHeadwear::Captain => draw_captain(c, time_sec, window),
        NomiHeadwear::Hiphop => draw_hiphop(c, time_sec, window),
        NomiHeadwear::Panama => draw_panama(c, window),
        NomiHeadwear::MiniDrone => draw_mini_drone(c, time_sec, glow_color, false, window),
        NomiHeadwear::LaserbeakDrone => draw_laserbeak_drone(c, time_sec, false, window),
        NomiHeadwear::OrbitRing => draw_orbit_ring_half(c, time_sec, glow_color, pitch, false, window),
        NomiHeadwear::CybertronRing => draw_cybertron_ring_half(c, time_sec, glow_color, pitch, false, window),
        NomiHeadwear::Bowtie => draw_bowtie(c, window),
        NomiHeadwear::WinterScarf => draw_winter_scarf(c, time_sec, window),
        NomiHeadwear::BumblebeeHelm => draw_bumblebee_helm(c, time_sec, window),
        NomiHeadwear::OptimusCrest => draw_optimus_crest(c, time_sec, window),
        NomiHeadwear::MegatronCannon => draw_megatron_cannon(c, time_sec, window),
        NomiHeadwear::MatrixPendant => draw_matrix_pendant(c, time_sec, glow_color, window),
        NomiHeadwear::CommsVisor => draw_comms_visor(c, time_sec, window),
        NomiHeadwear::ShockwaveOptic => draw_shockwave_optic(c, time_sec, window),
        NomiHeadwear::JazzVisor => draw_jazz_visor(c, window),
        NomiHeadwear::DriftSamurai => draw_drift_samurai(c, window),
        NomiHeadwear::GrimlockHelm => draw_grimlock_helm(c, time_sec, window),
        NomiHeadwear::ArceeHelm => draw_arcee_helm(c, time_sec, window),
        NomiHeadwear::RatchetHelm => draw_ratchet_helm(c, time_sec, window),
        NomiHeadwear::HotrodHelm => draw_hotrod_helm(c, time_sec, window),
        NomiHeadwear::DevastatorHelm => draw_devastator_helm(c, time_sec, window),
        NomiHeadwear::WheeljackHelm => draw_wheeljack_helm(c, time_sec, window),
        _ => {}
    }
}

/// 1. 明制状元新郎帽 (Zhuangyuan)
fn draw_zhuangyuan(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let head_top = -c.head_r;
    let base_y = head_top + r * 0.24;
    let cap_w = r * 0.66;

    // 1) 左右弹性颤动状元展翅（适配侧边栏收敛至 ±1.50r）
    for side in [-1.0_f32, 1.0] {
        let bounce = (time_sec * 4.8 + side * 0.25).sin() * 0.065;
        let w = c.sub(side * cap_w * 0.84, base_y - r * 0.14, side * (-0.10 + bounce));
        let wlen = r * 0.86;
        let wh = r * 0.14;

        // 鎏金翅柄
        w.stroke_line(0.0, 0.0, side * wlen * 0.24, -r * 0.02, (r * 0.048).max(0.85), rgba(0xF59E0BFF), window);

        // 朱红金边长椭翅叶
        let mut leaf = w.fill();
        leaf.move_to(side * wlen * 0.18, 0.0);
        leaf.cubic_to(side * wlen * 0.36, -wh * 1.15, side * wlen * 0.90, -wh * 1.05, side * wlen, -wh * 0.12);
        leaf.cubic_to(side * wlen * 0.94, wh * 1.05, side * wlen * 0.38, wh * 1.08, side * wlen * 0.18, 0.0);
        leaf.close();
        leaf.paint(rgba(0xDC2626FF), window);

        let mut rim = w.stroke((r * 0.028).max(0.6));
        rim.move_to(side * wlen * 0.18, 0.0);
        rim.cubic_to(side * wlen * 0.36, -wh * 1.15, side * wlen * 0.90, -wh * 1.05, side * wlen, -wh * 0.12);
        rim.cubic_to(side * wlen * 0.94, wh * 1.05, side * wlen * 0.38, wh * 1.08, side * wlen * 0.18, 0.0);
        rim.close();
        rim.paint(rgba(0xFDE047FF), window);

        // 翅面如意云纹金丝
        w.stroke_quad(
            side * wlen * 0.32,
            0.0,
            side * wlen * 0.62,
            -wh * 0.38,
            side * wlen * 0.86,
            -wh * 0.05,
            (r * 0.020).max(0.5),
            rgba(0xFEF08ACC),
            window,
        );

        // 翅下垂坠红流苏与金珠
        let sway = (time_sec * 3.2 + side * 0.7).sin() * 0.14;
        let tsub = w.sub(side * wlen * 0.42, wh * 0.78, sway);
        tsub.stroke_line(0.0, 0.0, 0.0, r * 0.24, (r * 0.025).max(0.5), rgba(0xFACC15FF), window);
        tsub.fill_circle(0.0, r * 0.14, r * 0.045, rgba(0xFDE047FF), window);
        let mut tas = tsub.fill();
        tas.move_to(-r * 0.032, r * 0.18);
        tas.line_to(r * 0.032, r * 0.18);
        tas.line_to(r * 0.058, r * 0.42);
        tas.line_to(-r * 0.058, r * 0.42);
        tas.close();
        tas.paint(rgba(0xEF4444FF), window);
    }

    // 2) 后阶高耸纱帽帽山
    let mut back = c.fill();
    back.move_to(-cap_w * 0.56, base_y - r * 0.12);
    back.cubic_to(-cap_w * 0.58, head_top - r * 0.36, cap_w * 0.58, head_top - r * 0.36, cap_w * 0.56, base_y - r * 0.12);
    back.close();
    back.paint(rgba(0x991B1BFF), window);

    // 3) 前阶大红主帽冠
    let mut front = c.fill();
    front.move_to(-cap_w, base_y);
    front.cubic_to(-cap_w * 0.96, head_top - r * 0.24, cap_w * 0.96, head_top - r * 0.24, cap_w, base_y);
    front.quad_to(0.0, base_y - r * 0.15, -cap_w, base_y);
    front.close();
    front.paint(rgba(0xDC2626FF), window);

    let mut front_rim = c.stroke((r * 0.030).max(0.65));
    front_rim.move_to(-cap_w, base_y);
    front_rim.cubic_to(-cap_w * 0.96, head_top - r * 0.24, cap_w * 0.96, head_top - r * 0.24, cap_w, base_y);
    front_rim.quad_to(0.0, base_y - r * 0.15, -cap_w, base_y);
    front_rim.close();
    front_rim.paint(rgba(0xF59E0BFF), window);

    // 4) 帽口鎏金箍边 + 正中如意金花珠饰
    c.stroke_quad(-cap_w * 0.98, base_y, 0.0, base_y - r * 0.15, cap_w * 0.98, base_y, (r * 0.068).max(1.2), rgba(0xFACC15FF), window);
    let cy_gem = base_y - r * 0.16;
    c.fill_ellipse(0.0, cy_gem, r * 0.17, r * 0.11, 0.0, rgba(0xF59E0BFF), window);
    c.fill_ellipse(0.0, cy_gem, r * 0.12, r * 0.075, 0.0, rgba(0xFEF08AFF), window);
    c.fill_circle(0.0, cy_gem, r * 0.052, rgba(0xEF4444FF), window);
    c.fill_circle(-r * 0.015, cy_gem - r * 0.015, r * 0.018, rgba(0xFFFFFFCC), window);
}

/// 2. 白娘子仙气发髻 (Bainiangzi)
fn draw_bainiangzi(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let head_top = -c.head_r;
    let bun_y = head_top - r * 0.08;
    let wing_w = r * 1.18;
    let sway = (time_sec * 2.2).sin() * r * 0.04;

    // 1) 背后双侧飘逸半透明白纱垂帘
    for side in [-1.0_f32, 1.0] {
        let mut veil = c.fill();
        veil.move_to(side * r * 0.42, head_top + r * 0.08);
        veil.cubic_to(
            side * r * 1.18,
            head_top + r * 0.16,
            side * r * 1.24 + sway * side,
            r * 0.42,
            side * r * 0.96 + sway * side,
            r * 0.88,
        );
        veil.quad_to(side * r * 0.84, r * 0.48, side * r * 0.78, head_top + r * 0.44);
        veil.close();
        veil.paint(rgba(0xF8FAFC48), window);
    }

    // 2) 横贯双翅纤细象牙玉簪 + 尾端垂珠
    c.stroke_line(-wing_w * 1.08, bun_y - r * 0.03, wing_w * 1.08, bun_y - r * 0.03, (r * 0.040).max(0.75), rgba(0xFEFCE8FF), window);
    for side in [-1.0_f32, 1.0] {
        c.fill_circle(side * wing_w * 1.09, bun_y - r * 0.03, r * 0.052, rgba(0xFDE68AFF), window);
        c.fill_circle(side * wing_w * 1.09, bun_y + r * 0.06, r * 0.036, rgba(0xFFFFFFEE), window);
    }

    // 3) 经典横展双翼乌黑云髻（《新白娘子传奇》标志性宽翼造型）
    for side in [-1.0_f32, 1.0] {
        let mut wing = c.fill();
        wing.move_to(0.0, bun_y + r * 0.12);
        wing.cubic_to(
            side * wing_w * 0.36,
            bun_y - r * 0.28,
            side * wing_w * 0.98,
            bun_y - r * 0.24,
            side * wing_w,
            bun_y + r * 0.04,
        );
        wing.cubic_to(
            side * wing_w * 0.96,
            bun_y + r * 0.24,
            side * wing_w * 0.34,
            bun_y + r * 0.22,
            0.0,
            bun_y + r * 0.16,
        );
        wing.close();
        wing.paint(rgba(0x18181BFF), window);

        // 墨黑发丝高光弧线
        c.stroke_quad(
            side * r * 0.22,
            bun_y - r * 0.06,
            side * wing_w * 0.62,
            bun_y - r * 0.14,
            side * wing_w * 0.90,
            bun_y + r * 0.02,
            (r * 0.024).max(0.5),
            rgba(0xE2E8F048),
            window,
        );
    }

    // 4) 发际额前柔和墨发包边
    let mut bangs = c.fill();
    bangs.move_to(-r * 0.78, head_top + r * 0.30);
    bangs.quad_to(0.0, head_top - r * 0.02, r * 0.78, head_top + r * 0.30);
    bangs.quad_to(0.0, head_top + r * 0.12, -r * 0.78, head_top + r * 0.30);
    bangs.close();
    bangs.paint(rgba(0x18181BFF), window);

    // 5) 发髻正中白玉多瓣珠花胸针 + 两侧温润珍珠耳坠
    let by = bun_y + r * 0.04;
    for i in 0..6 {
        let a = (i as f32) * PI / 3.0;
        c.fill_circle(a.cos() * r * 0.095, by + a.sin() * r * 0.075, r * 0.052, rgba(0xFFFBEBFA), window);
    }
    c.fill_circle(0.0, by, r * 0.062, rgba(0xFDE047FF), window);
    c.fill_circle(0.0, by, r * 0.035, rgba(0xFFFFFFFF), window);

    // 两侧玉珠耳坠
    for side in [-1.0_f32, 1.0] {
        let ex = side * (c.head_r + r * 0.02);
        let ey = r * 0.18;
        c.stroke_line(ex, ey - r * 0.12, ex, ey + r * 0.06, (r * 0.020).max(0.45), rgba(0xE2E8F0CC), window);
        c.fill_circle(ex, ey + r * 0.09, r * 0.055, rgba(0xFFFFFFEE), window);
    }
}

/// 3. 蒙面女神白纱 (VeiledGoddess) - 纯粹极简半透明面纱，不遮双眸
fn draw_veiled_goddess(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let top_y = r * 0.16;
    let bot_r = c.head_r * 1.02;
    let ripple = (time_sec * 2.4).sin() * r * 0.03;

    // 1) 超轻透薄纱主体（沿下半圆轮廓自然垂落）
    let mut veil = c.fill();
    veil.move_to(-r * 0.94, top_y + r * 0.03);
    veil.quad_to(-r * 0.45, top_y - r * 0.05, 0.0, top_y + r * 0.04);
    veil.quad_to(r * 0.45, top_y - r * 0.05, r * 0.94, top_y + r * 0.03);
    // 沿圆盘下半部圆弧流畅收底
    let steps = 20;
    for i in 0..=steps {
        let theta = 0.18 + (PI - 0.36) * (i as f32 / steps as f32);
        let wave = (theta * 4.0 + time_sec * 2.4).sin() * r * 0.018;
        veil.line_to(theta.cos() * (bot_r + wave), theta.sin() * (bot_r + wave));
    }
    veil.close();
    veil.paint(rgba(0xF8FAFC30), window);

    // 2) 上缘微垂轻纱滚边（柔和银白细线）
    let mut hem = c.stroke((r * 0.028).max(0.55));
    hem.move_to(-r * 0.94, top_y + r * 0.03);
    hem.quad_to(-r * 0.45, top_y - r * 0.05, 0.0, top_y + r * 0.04);
    hem.quad_to(r * 0.45, top_y - r * 0.05, r * 0.94, top_y + r * 0.03);
    hem.paint(rgba(0xFFFFFF92), window);

    // 3) 几道柔和纵向轻纱褶皱高光
    for (fx, len) in [(-r * 0.46, r * 0.62), (0.0, r * 0.72), (r * 0.46, r * 0.62)] {
        c.stroke_quad(
            fx,
            top_y + r * 0.06,
            fx + ripple,
            top_y + len * 0.55,
            fx - ripple * 0.5,
            top_y + len,
            (r * 0.022).max(0.45),
            rgba(0xFFFFFF36),
            window,
        );
    }
}

/// 4. 福气虎头帽 (Tiger)
fn draw_tiger(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let head_top = -c.head_r;
    let base_y = head_top + r * 0.46;
    let peak_y = head_top - r * 0.18;
    let cap_w = r * 0.88;

    // 1) 两侧垂坠红绳毛绒球
    for side in [-1.0_f32, 1.0] {
        let sway = (time_sec * 3.1 + side * 0.8).sin() * 0.14;
        let cord = c.sub(side * cap_w * 0.92, base_y, sway);
        cord.stroke_line(0.0, 0.0, 0.0, r * 0.28, (r * 0.034).max(0.65), rgba(0xDC2626FF), window);
        cord.fill_circle(0.0, r * 0.30, r * 0.095, rgba(0xEF4444FF), window);
        cord.fill_circle(-r * 0.025, r * 0.27, r * 0.035, rgba(0xFCA5A5AA), window);
    }

    // 2) 左右俏皮抖耳暖橘圆虎耳
    for side in [-1.0_f32, 1.0] {
        let twitch = (time_sec * 3.2 + side * 0.7).sin() * 0.05
            + (time_sec * 1.4 + side).sin().max(0.0).powi(6) * 0.09;
        let ear = c.sub(side * r * 0.56, peak_y + r * 0.10, side * (0.42 + twitch));
        ear.fill_circle(0.0, -r * 0.04, r * 0.21, rgba(0xEA580CFF), window);
        ear.stroke_circle(0.0, -r * 0.04, r * 0.21, (r * 0.028).max(0.55), rgba(0x9A3412FF), window);
        ear.fill_circle(0.0, -r * 0.03, r * 0.115, rgba(0xFEF3C7FF), window);
    }

    // 3) 蓬松暖橘主帽冠
    let mut cap = c.fill();
    cap.move_to(-cap_w, base_y);
    cap.cubic_to(-cap_w * 0.96, peak_y + r * 0.06, -cap_w * 0.45, peak_y, 0.0, peak_y);
    cap.cubic_to(cap_w * 0.45, peak_y, cap_w * 0.96, peak_y + r * 0.06, cap_w, base_y);
    cap.quad_to(0.0, base_y - r * 0.28, -cap_w, base_y);
    cap.close();
    cap.paint(rgba(0xF97316FF), window);

    // 帽顶柔和暖橙高光
    c.fill_ellipse(-r * 0.14, peak_y + r * 0.16, r * 0.36, r * 0.13, -0.15, rgba(0xFDBA7466), window);

    // 4) 两侧虎斑纹 + 额心醒目「王」字刺绣
    for side in [-1.0_f32, 1.0] {
        c.stroke_quad(side * r * 0.62, peak_y + r * 0.20, side * r * 0.46, peak_y + r * 0.24, side * r * 0.36, peak_y + r * 0.26, (r * 0.040).max(0.7), rgba(0x431407FF), window);
        c.stroke_quad(side * r * 0.72, peak_y + r * 0.34, side * r * 0.56, peak_y + r * 0.35, side * r * 0.45, peak_y + r * 0.36, (r * 0.036).max(0.65), rgba(0x431407FF), window);
    }
    let wy = peak_y + r * 0.14;
    let ww = r * 0.15;
    let wh = r * 0.18;
    let sw = (r * 0.040).max(0.75);
    c.stroke_line(-ww, wy, ww, wy, sw, rgba(0x3B1106FF), window);
    c.stroke_line(-ww * 0.78, wy + wh * 0.5, ww * 0.78, wy + wh * 0.5, sw, rgba(0x3B1106FF), window);
    c.stroke_line(-ww * 1.05, wy + wh, ww * 1.05, wy + wh, sw, rgba(0x3B1106FF), window);
    c.stroke_line(0.0, wy, 0.0, wy + wh, sw, rgba(0x3B1106FF), window);

    // 5) 帽口奶油毛绒包边
    c.stroke_quad(-cap_w * 0.96, base_y, 0.0, base_y - r * 0.28, cap_w * 0.96, base_y, (r * 0.090).max(1.5), rgba(0xFEF3C7FF), window);
}

/// 5. 香港警察 PTU 蓝帽子 (OfficerCap)
fn draw_officer_cap(c: &HeadCanvas, window: &mut Window) {
    let r = c.r;
    let head_top = -c.head_r;
    let cap = c.sub(-r * 0.04, head_top + r * 0.22, -0.14);

    // 1) 经典藏青毛呢贝雷帽身（向右优雅微垂）
    let mut dome = cap.fill();
    dome.move_to(-r * 0.78, r * 0.02);
    dome.cubic_to(-r * 0.94, -r * 0.32, -r * 0.28, -r * 0.52, r * 0.26, -r * 0.42);
    dome.cubic_to(r * 0.88, -r * 0.30, r * 1.08, r * 0.02, r * 0.80, r * 0.14);
    dome.quad_to(0.0, -r * 0.04, -r * 0.78, r * 0.02);
    dome.close();
    dome.paint(rgba(0x1E3A8AFF), window);

    // 顶部柔和海军蓝光泽
    cap.fill_ellipse(-r * 0.14, -r * 0.26, r * 0.44, r * 0.14, -0.1, rgba(0x3B82F644), window);

    let mut rim = cap.stroke((r * 0.026).max(0.55));
    rim.move_to(-r * 0.78, r * 0.02);
    rim.cubic_to(-r * 0.94, -r * 0.32, -r * 0.28, -r * 0.52, r * 0.26, -r * 0.42);
    rim.cubic_to(r * 0.88, -r * 0.30, r * 1.08, r * 0.02, r * 0.80, r * 0.14);
    rim.paint(rgba(0x0F172AFF), window);

    // 2) 经典黑色亮皮包边帽口 + 右侧蝴蝶结短飘带
    cap.stroke_quad(-r * 0.78, r * 0.03, 0.0, -r * 0.09, r * 0.78, r * 0.07, (r * 0.095).max(1.6), rgba(0x0F172AFF), window);
    cap.stroke_quad(-r * 0.74, r * 0.01, 0.0, -r * 0.11, r * 0.74, r * 0.05, (r * 0.022).max(0.45), rgba(0x94A3B866), window);

    // 右侧黑色小飘带
    cap.stroke_line(r * 0.74, r * 0.07, r * 0.88, r * 0.24, (r * 0.042).max(0.75), rgba(0x111827FF), window);
    cap.stroke_line(r * 0.72, r * 0.08, r * 0.78, r * 0.28, (r * 0.042).max(0.75), rgba(0x111827FF), window);

    // 3) 左侧正面标志性银蓝香港警徽盾牌
    let bx = -r * 0.30;
    let by = -r * 0.17;
    let bw = r * 0.14;
    let bh = r * 0.16;
    let mut badge = cap.fill();
    badge.move_to(bx, by - bh * 0.78);
    badge.line_to(bx + bw * 0.85, by - bh * 0.36);
    badge.quad_to(bx + bw * 0.80, by + bh * 0.45, bx, by + bh * 0.82);
    badge.quad_to(bx - bw * 0.80, by + bh * 0.45, bx - bw * 0.85, by - bh * 0.36);
    badge.close();
    badge.paint(rgba(0xE2E8F0FF), window);

    cap.fill_circle(bx, by + bh * 0.04, bw * 0.44, rgba(0x1D4ED8FF), window);
    cap.fill_circle(bx, by + bh * 0.04, bw * 0.18, rgba(0xF8FAFCFF), window);
}

/// 6. 齐天大圣凤翅紫金冠 (MonkeyKing)
fn draw_monkey_king(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let head_top = -c.head_r;
    let band_y = head_top + r * 0.22;

    // 1) 双根弹性甩动长雉鸡翎（适配侧边栏高宽比，依旧拉风霸气）
    for side in [-1.0_f32, 1.0] {
        let whip1 = (time_sec * 4.2 + side * 0.4).sin() * r * 0.08;
        let whip2 = (time_sec * 4.2 - 0.7 + side * 0.4).sin() * r * 0.13;
        let p0 = (side * r * 0.18, head_top - r * 0.22);
        let p1 = (side * r * 0.48 + whip1 * 0.4, head_top - r * 0.62);
        let p2 = (side * r * 1.02 + whip1, head_top - r * 0.56 - whip1 * 0.3);
        let p3 = (side * r * 1.32 + whip2, head_top - r * 0.14 + whip2 * 0.35);

        // 翎羽外层橘红金底 + 深棕斑点
        c.stroke_cubic(p0.0, p0.1, p1.0, p1.1, p2.0, p2.1, p3.0, p3.1, (r * 0.095).max(1.6), rgba(0xEA580CFF), window);
        c.stroke_cubic(p0.0, p0.1, p1.0, p1.1, p2.0, p2.1, p3.0, p3.1, (r * 0.052).max(0.9), rgba(0xFDE047FF), window);
        for t in [0.30_f32, 0.52, 0.74, 0.90] {
            let (x, y, ang) = eval_cubic(t, p0, p1, p2, p3);
            let s = c.sub(x, y, ang);
            s.fill_ellipse(0.0, 0.0, r * 0.022, r * 0.044, 0.0, rgba(0x451A03EE), window);
        }
    }

    // 2) 凤翅紫金冠冠座（深紫鎏金三尖冠）
    let mut crown = c.fill();
    crown.move_to(-r * 0.42, band_y - r * 0.06);
    crown.line_to(-r * 0.46, head_top - r * 0.20);
    crown.line_to(-r * 0.22, head_top - r * 0.12);
    crown.line_to(0.0, head_top - r * 0.34);
    crown.line_to(r * 0.22, head_top - r * 0.12);
    crown.line_to(r * 0.46, head_top - r * 0.20);
    crown.line_to(r * 0.42, band_y - r * 0.06);
    crown.close();
    crown.paint(rgba(0x7E22CEFF), window);

    let mut crim = c.stroke((r * 0.034).max(0.7));
    crim.move_to(-r * 0.42, band_y - r * 0.06);
    crim.line_to(-r * 0.46, head_top - r * 0.20);
    crim.line_to(-r * 0.22, head_top - r * 0.12);
    crim.line_to(0.0, head_top - r * 0.34);
    crim.line_to(r * 0.22, head_top - r * 0.12);
    crim.line_to(r * 0.46, head_top - r * 0.20);
    crim.line_to(r * 0.42, band_y - r * 0.06);
    crim.paint(rgba(0xFACC15FF), window);

    // 冠心火红宝珠
    c.fill_circle(0.0, head_top - r * 0.06, r * 0.075, rgba(0xEF4444FF), window);
    c.fill_circle(-r * 0.02, head_top - r * 0.08, r * 0.024, rgba(0xFFFFFFCC), window);

    // 3) 额前经典鎏金紧箍咒 + 居中双卷云头
    c.stroke_quad(-r * 0.84, band_y + r * 0.08, 0.0, band_y - r * 0.14, r * 0.84, band_y + r * 0.08, (r * 0.078).max(1.4), rgba(0xF59E0BFF), window);
    c.stroke_quad(-r * 0.80, band_y + r * 0.06, 0.0, band_y - r * 0.16, r * 0.80, band_y + r * 0.06, (r * 0.028).max(0.5), rgba(0xFEF08AFF), window);
    for side in [-1.0_f32, 1.0] {
        c.stroke_circle(side * r * 0.13, band_y - r * 0.02, r * 0.068, (r * 0.036).max(0.65), rgba(0xFDE047FF), window);
    }
}

/// 7. 哪吒双丸子混天绫 + 脚底风火轮 (Nezha)
fn draw_nezha(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let head_top = -c.head_r;

    // 1) 左右俏皮乌黑双丸子 + 鲜红发带丝结
    for side in [-1.0_f32, 1.0] {
        let bx = side * r * 0.62;
        let by = head_top + r * 0.06;
        let br = r * 0.26;
        let sway = (time_sec * 3.8 + side * 0.8).sin() * 0.16;

        // 发带垂飘短丝带
        let rib = c.sub(bx + side * br * 0.55, by + br * 0.50, side * 0.35 + sway);
        rib.stroke_quad(0.0, 0.0, side * r * 0.12, r * 0.16, side * r * 0.05, r * 0.30, (r * 0.058).max(1.0), rgba(0xEF4444FF), window);

        // 乌黑光亮丸子头
        c.fill_circle(bx, by, br, rgba(0x18181BFF), window);
        c.stroke_circle(bx, by, br, (r * 0.026).max(0.5), rgba(0x3F3F46FF), window);
        c.fill_ellipse(bx - side * br * 0.24, by - br * 0.28, br * 0.34, br * 0.18, -side * 0.4, rgba(0xFFFFFF36), window);

        // 扎丸子鲜红丝带环
        let ring = c.sub(bx, by + br * 0.62, side * 0.48);
        ring.fill_ellipse(0.0, 0.0, br * 0.85, r * 0.062, 0.0, rgba(0xDC2626FF), window);
        ring.stroke_line(-br * 0.72, 0.0, br * 0.72, 0.0, (r * 0.020).max(0.45), rgba(0xFDE047CC), window);
    }

    // 2) 眉心经典哪吒莲花火焰印记
    let fy = head_top + r * 0.24;
    let mut flame = c.fill();
    flame.move_to(0.0, fy - r * 0.11);
    flame.quad_to(r * 0.085, fy, 0.0, fy + r * 0.085);
    flame.quad_to(-r * 0.085, fy, 0.0, fy - r * 0.11);
    flame.close();
    flame.paint(rgba(0xEF4444FF), window);
    c.fill_circle(0.0, fy + r * 0.01, r * 0.032, rgba(0xFEF08AFF), window);

    // 3) 两侧底部向前高速滚转的烈焰「风火轮」
    let spin = -time_sec * 8.2;
    for side in [-1.0_f32, 1.0] {
        let wx = side * r * 0.82;
        let wy = r * 0.86;
        let wr = r * 0.25;

        // 轮后向后曳出的赤金火焰尾流
        let flicker = (time_sec * 16.0 + side * 1.4).sin() * r * 0.035;
        let mut fire = c.fill();
        fire.move_to(wx, wy - wr * 0.85);
        fire.quad_to(wx + wr * 1.45, wy - wr * 0.55 + flicker, wx + wr * 1.80, wy + flicker);
        fire.quad_to(wx + wr * 1.35, wy + wr * 0.70 - flicker, wx, wy + wr * 0.85);
        fire.close();
        fire.paint(rgba(0xF97316AA), window);

        // 轮圈金色发光外环 + 旋转辐条
        c.fill_circle(wx, wy, wr * 1.18, rgba(0xFB923C40), window);
        c.stroke_circle(wx, wy, wr, (r * 0.068).max(1.2), rgba(0xF59E0BFF), window);
        c.stroke_circle(wx, wy, wr * 0.86, (r * 0.024).max(0.45), rgba(0xFEF08AFF), window);

        let wheel = c.sub(wx, wy, spin);
        for i in 0..6 {
            let a = (i as f32) * PI / 3.0;
            wheel.stroke_line(0.0, 0.0, a.cos() * wr * 0.84, a.sin() * wr * 0.84, (r * 0.032).max(0.55), rgba(0xFDE047FF), window);
        }
        c.fill_circle(wx, wy, wr * 0.26, rgba(0xDC2626FF), window);
        c.fill_circle(wx, wy, wr * 0.12, rgba(0xFEF08AFF), window);
    }
}

/// 8. 江湖侠客竹编斗笠 (BambooHat)
fn draw_bamboo_hat(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let head_top = -c.head_r;
    let hat = c.sub(0.0, head_top + r * 0.20, -0.07);
    let brim_w = r * 1.32;
    let peak_y = -r * 0.46;

    // 1) 斗笠两侧微飘江湖深灰轻纱
    let sway = (time_sec * 2.6).sin() * r * 0.05;
    for side in [-1.0_f32, 1.0] {
        let mut gauze = hat.fill();
        gauze.move_to(side * brim_w * 0.72, r * 0.06);
        gauze.quad_to(side * brim_w * 0.82 + sway, r * 0.42, side * brim_w * 0.68 + sway * 1.2, r * 0.76);
        gauze.line_to(side * brim_w * 0.56 + sway, r * 0.70);
        gauze.quad_to(side * brim_w * 0.64, r * 0.36, side * brim_w * 0.58, r * 0.06);
        gauze.close();
        gauze.paint(rgba(0x1E293B52), window);
    }

    // 2) 竹编宽檐斗笠暗影底面 + 暖竹金斗笠锥身
    hat.fill_ellipse(0.0, r * 0.07, brim_w * 0.96, r * 0.16, 0.0, rgba(0x78350FFF), window);

    let mut cone = hat.fill();
    cone.move_to(-brim_w, r * 0.06);
    cone.quad_to(-brim_w * 0.42, peak_y * 0.58, 0.0, peak_y);
    cone.quad_to(brim_w * 0.42, peak_y * 0.58, brim_w, r * 0.06);
    cone.quad_to(0.0, r * 0.20, -brim_w, r * 0.06);
    cone.close();
    cone.paint(rgba(0xD97706FF), window);

    // 亮竹金受光高光侧
    let mut hi = hat.fill();
    hi.move_to(-brim_w * 0.88, r * 0.05);
    hi.quad_to(-brim_w * 0.36, peak_y * 0.60, 0.0, peak_y + r * 0.02);
    hi.line_to(-r * 0.14, r * 0.12);
    hi.close();
    hi.paint(rgba(0xFCD34D77), window);

    // 竹篾编织放射筋线 + 环纹
    for frac in [-0.62_f32, -0.30, 0.0, 0.30, 0.62] {
        hat.stroke_line(0.0, peak_y + r * 0.03, brim_w * frac, r * 0.09, (r * 0.018).max(0.45), rgba(0x78350F88), window);
    }
    hat.stroke_quad(-brim_w * 0.55, -r * 0.14, 0.0, -r * 0.06, brim_w * 0.55, -r * 0.14, (r * 0.020).max(0.45), rgba(0x92400E88), window);
    hat.stroke_quad(-brim_w, r * 0.06, 0.0, r * 0.20, brim_w, r * 0.06, (r * 0.038).max(0.75), rgba(0x451A03FF), window);

    // 斗笠顶玄黑竹结
    hat.fill_ellipse(0.0, peak_y + r * 0.02, r * 0.14, r * 0.065, 0.0, rgba(0x1C1917FF), window);

    // 3) 右侧随风飘落翠绿竹叶
    let leaf_t = (time_sec * 1.4).sin();
    let leaf = hat.sub(brim_w * 0.72 + leaf_t * r * 0.08, -r * 0.18 + leaf_t * r * 0.06, 0.45 + leaf_t * 0.25);
    let mut lf = leaf.fill();
    lf.move_to(-r * 0.16, 0.0);
    lf.quad_to(0.0, -r * 0.08, r * 0.18, 0.0);
    lf.quad_to(0.0, r * 0.08, -r * 0.16, 0.0);
    lf.close();
    lf.paint(rgba(0x22C55EFF), window);
}

/// 9. 喜庆财神招财帽 (Caishen)
fn draw_caishen(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let head_top = -c.head_r;
    let base_y = head_top + r * 0.28;
    let cap_w = r * 0.76;

    // 1) 两侧上翘鎏金如意招财帽翅
    for side in [-1.0_f32, 1.0] {
        let mut wing = c.fill();
        wing.move_to(side * cap_w * 0.78, base_y - r * 0.12);
        wing.cubic_to(side * r * 1.18, base_y - r * 0.12, side * r * 1.32, head_top - r * 0.20, side * r * 1.16, head_top - r * 0.30);
        wing.cubic_to(side * r * 1.00, head_top - r * 0.34, side * r * 0.92, head_top - r * 0.10, side * cap_w * 0.74, base_y - r * 0.22);
        wing.close();
        wing.paint(rgba(0xF59E0BFF), window);
        c.fill_circle(side * r * 1.14, head_top - r * 0.26, r * 0.085, rgba(0xFEF08AFF), window);
        c.fill_circle(side * r * 1.14, head_top - r * 0.26, r * 0.042, rgba(0xDC2626FF), window);

        // 垂挂方孔金铜钱
        let sway = (time_sec * 3.0 + side * 0.9).sin() * 0.12;
        let cord = c.sub(side * cap_w * 0.96, base_y + r * 0.02, sway);
        cord.stroke_line(0.0, 0.0, 0.0, r * 0.24, (r * 0.028).max(0.55), rgba(0xDC2626FF), window);
        cord.fill_circle(0.0, r * 0.32, r * 0.11, rgba(0xFACC15FF), window);
        cord.stroke_circle(0.0, r * 0.32, r * 0.11, (r * 0.024).max(0.5), rgba(0xB45309FF), window);
        // 铜钱中央方孔
        let sq = r * 0.036;
        let mut hole = cord.fill();
        hole.move_to(-sq, r * 0.32 - sq);
        hole.line_to(sq, r * 0.32 - sq);
        hole.line_to(sq, r * 0.32 + sq);
        hole.line_to(-sq, r * 0.32 + sq);
        hole.close();
        hole.paint(rgba(0x78350FFF), window);
    }

    // 2) 大红锦缎财神主帽身
    let mut cap = c.fill();
    cap.move_to(-cap_w, base_y);
    cap.cubic_to(-cap_w * 0.96, head_top - r * 0.24, cap_w * 0.96, head_top - r * 0.24, cap_w, base_y);
    cap.quad_to(0.0, base_y - r * 0.18, -cap_w, base_y);
    cap.close();
    cap.paint(rgba(0xDC2626FF), window);

    // 鎏金帽口滚边与正中福珠
    c.stroke_quad(-cap_w * 0.98, base_y, 0.0, base_y - r * 0.18, cap_w * 0.98, base_y, (r * 0.080).max(1.4), rgba(0xFACC15FF), window);
    c.fill_circle(0.0, base_y - r * 0.18, r * 0.088, rgba(0xEF4444FF), window);
    c.stroke_circle(0.0, base_y - r * 0.18, r * 0.088, (r * 0.026).max(0.55), rgba(0xFEF08AFF), window);

    // 3) 帽顶闪耀招财金元宝 + 闪烁财气星芒
    let iy = head_top - r * 0.20;
    let pulse = 0.72 + 0.28 * (time_sec * 4.2).sin();
    c.fill_ellipse(0.0, iy - r * 0.06, r * 0.36, r * 0.18, 0.0, rgba_alpha(0xFEF08A, 0.32 * pulse), window);
    c.fill_ellipse(0.0, iy - r * 0.09, r * 0.18, r * 0.12, 0.0, rgba(0xFDE047FF), window);
    let mut ingot = c.fill();
    ingot.move_to(-r * 0.32, iy - r * 0.10);
    ingot.quad_to(0.0, iy + r * 0.02, r * 0.32, iy - r * 0.10);
    ingot.quad_to(r * 0.24, iy + r * 0.10, 0.0, iy + r * 0.10);
    ingot.quad_to(-r * 0.24, iy + r * 0.10, -r * 0.32, iy - r * 0.10);
    ingot.close();
    ingot.paint(rgba(0xF59E0BFF), window);
    c.fill_star4(r * 0.28, iy - r * 0.20, r * (0.075 * pulse), rgba(0xFFFBEBFF), window);
}

/// 10. 旋转竹蜻蜓 (BambooCopter)
fn draw_bamboo_copter(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let head_top = -c.head_r;
    let lift = (time_sec * 5.0).sin() * r * 0.02;
    let base_y = head_top + r * 0.02 + lift;
    let hub_y = base_y - r * 0.30;

    // 1) 顶部明黄吸盘底座 + 金属转轴
    let mut cup = c.fill();
    cup.move_to(-r * 0.24, base_y + r * 0.03);
    cup.cubic_to(-r * 0.22, base_y - r * 0.14, r * 0.22, base_y - r * 0.14, r * 0.24, base_y + r * 0.03);
    cup.close();
    cup.paint(rgba(0xFACC15FF), window);
    c.stroke_line(-r * 0.22, base_y + r * 0.03, r * 0.22, base_y + r * 0.03, (r * 0.032).max(0.6), rgba(0xA16207FF), window);

    c.stroke_line(0.0, base_y - r * 0.08, 0.0, hub_y, (r * 0.068).max(1.2), rgba(0xCBD5E1FF), window);
    c.stroke_line(-r * 0.015, base_y - r * 0.08, -r * 0.015, hub_y, (r * 0.020).max(0.45), rgba(0xFFFFFFCC), window);

    // 2) 高速旋转透视残影圆盘 + 明黄桨叶（按余弦透视缩放模拟高速旋转）
    let spin_cos = (time_sec * 24.0).cos();
    let blade_span = r * (0.28 + 0.56 * spin_cos.abs());
    c.fill_ellipse(0.0, hub_y, r * 0.86, r * 0.11, 0.0, rgba(0xFEF08A48), window);
    c.stroke_ellipse(0.0, hub_y, r * 0.84, r * 0.10, 0.0, (r * 0.022).max(0.45), rgba(0xFDE04788), window);

    // 实体明黄双翼桨叶
    c.fill_ellipse(-blade_span * 0.52, hub_y, blade_span * 0.50, r * 0.068, 0.0, rgba(0xFACC15FF), window);
    c.fill_ellipse(blade_span * 0.52, hub_y, blade_span * 0.50, r * 0.068, 0.0, rgba(0xF59E0BFF), window);
    c.fill_circle(0.0, hub_y, r * 0.068, rgba(0xEAB308FF), window);
    c.fill_circle(-r * 0.018, hub_y - r * 0.018, r * 0.022, rgba(0xFFFFFFDD), window);

    // 3) 微弱起飞气流弧线
    let air_a = 0.25 + 0.25 * (time_sec * 8.0).sin();
    for side in [-1.0_f32, 1.0] {
        c.stroke_quad(
            side * r * 0.48,
            hub_y + r * 0.14,
            side * r * 0.58,
            hub_y + r * 0.24,
            side * r * 0.44,
            hub_y + r * 0.34,
            (r * 0.020).max(0.45),
            rgba_alpha(0xE0F2FE, air_a),
            window,
        );
    }
}

/// 11. 头顶趴趴小黄鸭 (RubberDuck)
fn draw_rubber_duck(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let head_top = -c.head_r;
    let sq = (time_sec * 3.4).sin() * 0.04;
    let duck = c.sub(0.0, head_top + r * 0.03, (time_sec * 1.8).sin() * 0.05);

    // 1) 圆滚滚软萌鸭身（趴在球顶）
    let bw = r * (0.46 * (1.0 + sq));
    let bh = r * (0.25 * (1.0 - sq));
    duck.fill_ellipse(0.0, -bh * 0.72, bw, bh, 0.0, rgba(0xFACC15FF), window);
    duck.stroke_ellipse(0.0, -bh * 0.72, bw, bh, 0.0, (r * 0.026).max(0.55), rgba(0xCA8A04FF), window);

    // 翘起的小尾巴
    let mut tail = duck.fill();
    tail.move_to(bw * 0.72, -bh * 0.75);
    tail.line_to(bw * 1.18, -bh * 1.25);
    tail.line_to(bw * 0.85, -bh * 0.32);
    tail.close();
    tail.paint(rgba(0xFACC15FF), window);

    // 2) 圆润小黄鸭脑袋 + 头顶俏皮呆毛
    let hx = -r * 0.16;
    let hy = -bh * 1.55;
    let hr = r * 0.24;
    duck.fill_circle(hx, hy, hr, rgba(0xFDE047FF), window);
    duck.stroke_circle(hx, hy, hr, (r * 0.024).max(0.5), rgba(0xCA8A04FF), window);
    duck.stroke_quad(hx, hy - hr * 0.92, hx + r * 0.06, hy - hr * 1.32, hx + r * 0.14, hy - hr * 1.12, (r * 0.034).max(0.65), rgba(0xFACC15FF), window);

    // 3) 橘红色扁扁小鸭嘴
    duck.fill_ellipse(hx - hr * 0.92, hy + hr * 0.16, r * 0.12, r * 0.068, -0.1, rgba(0xF97316FF), window);
    duck.stroke_ellipse(hx - hr * 0.92, hy + hr * 0.16, r * 0.12, r * 0.068, -0.1, (r * 0.018).max(0.45), rgba(0xC2410CFF), window);

    // 4) 周期自然眨眼的小豆眼 + 粉嫩小腮红
    let is_blink = (time_sec % 3.6) < 0.14;
    let ex = hx - hr * 0.22;
    let ey = hy - hr * 0.08;
    if is_blink {
        duck.stroke_line(ex - r * 0.045, ey, ex + r * 0.045, ey, (r * 0.030).max(0.6), rgba(0x18181BFF), window);
    } else {
        duck.fill_circle(ex, ey, r * 0.044, rgba(0x18181BFF), window);
        duck.fill_circle(ex - r * 0.015, ey - r * 0.015, r * 0.016, rgba(0xFFFFFFFF), window);
    }
    duck.fill_ellipse(hx + hr * 0.05, hy + hr * 0.25, r * 0.055, r * 0.034, 0.0, rgba(0xFB718588), window);

    // 5) 侧面轻扑棱小翅膀
    let flap = (time_sec * 6.5).sin() * 0.22;
    let wing = duck.sub(r * 0.08, -bh * 0.72, -0.18 + flap);
    wing.fill_ellipse(r * 0.10, 0.0, r * 0.18, r * 0.095, 0.12, rgba(0xEAB308FF), window);
    wing.stroke_ellipse(r * 0.10, 0.0, r * 0.18, r * 0.095, 0.12, (r * 0.020).max(0.45), rgba(0xA16207FF), window);
}

/// 12. 赛博小恶魔角 (CyberDevil - 配合后景蝙蝠翼与恶魔尾)
fn draw_cyber_devil(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let head_top = -c.head_r;
    let pulse = 0.75 + 0.25 * (time_sec * 3.8).sin();

    for side in [-1.0_f32, 1.0] {
        let h = c.sub(side * r * 0.56, head_top + r * 0.20, side * 0.34);

        // 暗红霓虹底光晕
        h.fill_ellipse(0.0, -r * 0.20, r * 0.16, r * 0.24, 0.0, rgba_alpha(0xF43F5E, 0.28 * pulse), window);

        // 弯曲锐利小恶魔角
        let mut horn = h.fill();
        horn.move_to(-r * 0.13, r * 0.05);
        horn.cubic_to(-r * 0.14, -r * 0.16, -side * r * 0.02, -r * 0.36, -side * r * 0.08, -r * 0.44);
        horn.cubic_to(side * r * 0.10, -r * 0.30, r * 0.15, -r * 0.10, r * 0.13, r * 0.05);
        horn.close();
        horn.paint(rgba(0x9F1239FF), window);

        // 赛博亮粉霓虹勾边
        let mut glow = h.stroke((r * 0.030).max(0.6));
        glow.move_to(-r * 0.13, r * 0.05);
        glow.cubic_to(-r * 0.14, -r * 0.16, -side * r * 0.02, -r * 0.36, -side * r * 0.08, -r * 0.44);
        glow.cubic_to(side * r * 0.10, -r * 0.30, r * 0.15, -r * 0.10, r * 0.13, r * 0.05);
        glow.paint(rgba_alpha(0xFB7185, 0.90 * pulse), window);

        // 角根暗金属箍环
        h.fill_ellipse(0.0, r * 0.04, r * 0.145, r * 0.052, 0.0, rgba(0x18181BFF), window);
    }
}

/// 13. 星空魔法师尖顶帽 (Wizard)
fn draw_wizard(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let head_top = -c.head_r;
    let wiz = c.sub(0.0, head_top + r * 0.24, -0.08);
    let brim_w = r * 1.24;
    let tip_x = r * 0.36;
    let tip_y = -r * 0.72;

    // 1) 宽大波浪巫师帽檐（深紫丝绒 + 鎏金滚边）
    wiz.fill_ellipse(0.0, r * 0.05, brim_w, r * 0.20, 0.0, rgba(0x3B0764FF), window);
    wiz.stroke_ellipse(0.0, r * 0.05, brim_w, r * 0.20, 0.0, (r * 0.032).max(0.65), rgba(0xFACC15FF), window);

    // 2) 俏皮弯尖星空尖顶帽冠
    let mut cone = wiz.fill();
    cone.move_to(-r * 0.62, r * 0.02);
    cone.cubic_to(-r * 0.50, -r * 0.42, -r * 0.05, -r * 0.80, tip_x, tip_y);
    cone.cubic_to(r * 0.22, -r * 0.56, r * 0.50, -r * 0.32, r * 0.62, r * 0.02);
    cone.quad_to(0.0, r * 0.14, -r * 0.62, r * 0.02);
    cone.close();
    cone.paint(rgba(0x4C1D95FF), window);

    // 帽身梦幻紫罗兰受光面
    let mut hi = wiz.fill();
    hi.move_to(-r * 0.52, 0.0);
    hi.cubic_to(-r * 0.40, -r * 0.36, -r * 0.06, -r * 0.66, tip_x * 0.75, tip_y * 0.88);
    hi.line_to(-r * 0.08, r * 0.04);
    hi.close();
    hi.paint(rgba(0x7C3AED66), window);

    // 3) 鎏金帽带与魔法宝石方扣
    wiz.stroke_quad(-r * 0.60, 0.0, 0.0, r * 0.12, r * 0.60, 0.0, (r * 0.095).max(1.5), rgba(0xF59E0BFF), window);
    wiz.fill_ellipse(0.0, r * 0.05, r * 0.13, r * 0.09, 0.0, rgba(0xFEF08AFF), window);
    wiz.fill_ellipse(0.0, r * 0.05, r * 0.075, r * 0.052, 0.0, rgba(0x38BDF8FF), window);

    // 4) 帽身金色新月与小星
    let mx = -r * 0.12;
    let my = -r * 0.26;
    wiz.fill_circle(mx, my, r * 0.095, rgba(0xFDE047FF), window);
    wiz.fill_circle(mx + r * 0.042, my - r * 0.018, r * 0.082, rgba(0x4C1D95FF), window);
    wiz.fill_star4(r * 0.14, -r * 0.32, r * 0.065, rgba(0xFEF08AFF), window);

    // 5) 绕帽尖 3D 倾斜公转的魔法金星
    let orb_a = time_sec * 2.8;
    let ox = tip_x - r * 0.08 + orb_a.cos() * r * 0.34;
    let oy = tip_y + r * 0.10 + orb_a.sin() * r * 0.11;
    let depth_s = 0.78 + 0.28 * orb_a.sin();
    wiz.fill_circle(ox, oy, r * 0.09 * depth_s, rgba(0xFEF08A48), window);
    wiz.fill_star5(ox, oy, r * 0.095 * depth_s, r * 0.044 * depth_s, orb_a * 0.7, rgba(0xFDE047FF), window);
}

/// 14. 海盗船长帽 (Pirate - 配合屏内自适应独眼眼罩)
fn draw_pirate(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let head_top = -c.head_r;
    let hat = c.sub(0.0, head_top + r * 0.22, -0.06);
    let hw = r * 1.12;

    // 1) 左侧插羽（绯红船长羽毛）
    let f_sway = (time_sec * 3.0).sin() * 0.08;
    let f = hat.sub(-hw * 0.62, -r * 0.28, -0.45 + f_sway);
    let mut feather = f.fill();
    feather.move_to(0.0, r * 0.06);
    feather.cubic_to(-r * 0.14, -r * 0.16, -r * 0.08, -r * 0.36, r * 0.04, -r * 0.40);
    feather.cubic_to(r * 0.16, -r * 0.24, r * 0.12, -r * 0.06, 0.0, r * 0.06);
    feather.close();
    feather.paint(rgba(0xDC2626FF), window);
    f.stroke_line(0.0, r * 0.04, r * 0.03, -r * 0.34, (r * 0.020).max(0.45), rgba(0xFCA5A5CC), window);

    // 2) 绯红海盗头巾包边 + 右侧飘逸系带
    let band_sway = (time_sec * 3.4).sin() * 0.12;
    hat.stroke_quad(-hw * 0.72, r * 0.04, 0.0, -r * 0.08, hw * 0.72, r * 0.06, (r * 0.095).max(1.5), rgba(0xB91C1CFF), window);
    let tie = hat.sub(hw * 0.70, r * 0.06, band_sway);
    tie.stroke_quad(0.0, 0.0, r * 0.18, r * 0.12, r * 0.24, r * 0.30, (r * 0.068).max(1.1), rgba(0xDC2626FF), window);
    tie.stroke_quad(0.0, 0.0, r * 0.10, r * 0.20, r * 0.08, r * 0.34, (r * 0.058).max(0.95), rgba(0x991B1BFF), window);

    // 3) 黑金翻沿海盗双角船长帽主体
    let mut main = hat.fill();
    main.move_to(-hw * 0.76, r * 0.02);
    main.cubic_to(-hw * 1.12, -r * 0.08, -hw * 1.06, -r * 0.36, -hw * 0.56, -r * 0.38);
    main.quad_to(0.0, -r * 0.62, hw * 0.56, -r * 0.38);
    main.cubic_to(hw * 1.06, -r * 0.36, hw * 1.12, -r * 0.08, hw * 0.76, r * 0.02);
    main.quad_to(0.0, -r * 0.12, -hw * 0.76, r * 0.02);
    main.close();
    main.paint(rgba(0x18181BFF), window);

    let mut trim = hat.stroke((r * 0.044).max(0.8));
    trim.move_to(-hw * 0.76, r * 0.02);
    trim.cubic_to(-hw * 1.12, -r * 0.08, -hw * 1.06, -r * 0.36, -hw * 0.56, -r * 0.38);
    trim.quad_to(0.0, -r * 0.62, hw * 0.56, -r * 0.38);
    trim.cubic_to(hw * 1.06, -r * 0.36, hw * 1.12, -r * 0.08, hw * 0.76, r * 0.02);
    trim.quad_to(0.0, -r * 0.12, -hw * 0.76, r * 0.02);
    trim.close();
    trim.paint(rgba(0xD97706FF), window);

    // 4) 帽冠正中纯白骷髅交叉骨徽章
    let sy = -r * 0.25;
    hat.stroke_line(-r * 0.19, sy - r * 0.07, r * 0.19, sy + r * 0.11, (r * 0.040).max(0.7), rgba(0xF8FAFCFF), window);
    hat.stroke_line(r * 0.19, sy - r * 0.07, -r * 0.19, sy + r * 0.11, (r * 0.040).max(0.7), rgba(0xF8FAFCFF), window);
    hat.fill_circle(0.0, sy - r * 0.01, r * 0.105, rgba(0xFFFFFFFF), window);
    hat.fill_ellipse(0.0, sy + r * 0.075, r * 0.068, r * 0.042, 0.0, rgba(0xFFFFFFFF), window);
    hat.fill_circle(-r * 0.042, sy - r * 0.01, r * 0.026, rgba(0x18181BFF), window);
    hat.fill_circle(r * 0.042, sy - r * 0.01, r * 0.026, rgba(0x18181BFF), window);
}

/// 15. 悬浮天使光环 (AngelHalo - 配合后景纯白羽翼)
fn draw_angel_halo(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let head_top = -c.head_r;
    let bob = (time_sec * 2.6).sin() * r * 0.045;
    let pulse = 0.78 + 0.22 * (time_sec * 3.8).sin();
    let hy = head_top - r * 0.26 + bob;
    let hrx = r * 0.56;
    let hry = r * 0.145;

    // 1) 多层流金柔光外晕 + 亮金主光环 + 炽白内芯
    c.stroke_ellipse(0.0, hy, hrx, hry, 0.0, (r * 0.16).max(2.6), rgba_alpha(0xFDE047, 0.28 * pulse), window);
    c.stroke_ellipse(0.0, hy, hrx, hry, 0.0, (r * 0.088).max(1.5), rgba_alpha(0xFACC15, 0.92), window);
    c.stroke_ellipse(0.0, hy, hrx, hry, 0.0, (r * 0.040).max(0.7), rgba(0xFFFBEBFF), window);

    // 2) 环绕光环流转的圣洁星芒
    for (phase, sr) in [(0.0_f32, r * 0.078), (PI, r * 0.062)] {
        let a = time_sec * 2.2 + phase;
        let sx = a.cos() * (hrx + r * 0.04);
        let sy = hy + a.sin() * (hry + r * 0.02);
        let tw = 0.55 + 0.45 * (time_sec * 5.0 + phase).sin();
        c.fill_star4(sx, sy, sr * tw, rgba_alpha(0xFFFBEB, 0.65 + 0.35 * tw), window);
    }
}

/// 16. 米其林主厨高帽 (Chef)
fn draw_chef(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let head_top = -c.head_r;
    let base_y = head_top + r * 0.22;
    let band_w = r * 0.66;
    let band_top = head_top - r * 0.08;
    let puff = (time_sec * 2.6).sin() * r * 0.015;

    // 1) 帽顶袅袅热气
    for i in 0..3 {
        let phase = (time_sec * 0.65 + (i as f32) * 0.33).fract();
        let alpha = (phase * PI).sin() * 0.45;
        let sx = ((i as f32) - 1.0) * r * 0.26 + (time_sec * 2.8 + (i as f32) * 1.7).sin() * r * 0.035;
        let sy = head_top - r * 0.44 - phase * r * 0.16;
        c.stroke_quad(
            sx,
            sy + r * 0.10,
            sx + r * 0.05,
            sy + r * 0.05,
            sx - r * 0.03,
            sy,
            (r * 0.024).max(0.5),
            rgba_alpha(0xFFFFFF, alpha),
            window,
        );
    }

    // 2) 蓬松云朵三泡高帽冠
    let mut cloud = c.fill();
    cloud.move_to(-band_w * 0.92, band_top + r * 0.04);
    cloud.cubic_to(-r * 0.98, band_top - r * 0.08, -r * 0.90, head_top - r * 0.44 - puff, -r * 0.42, head_top - r * 0.42 - puff);
    cloud.cubic_to(-r * 0.32, head_top - r * 0.60 - puff, r * 0.32, head_top - r * 0.60 - puff, r * 0.42, head_top - r * 0.42 - puff);
    cloud.cubic_to(r * 0.90, head_top - r * 0.44 - puff, r * 0.98, band_top - r * 0.08, band_w * 0.92, band_top + r * 0.04);
    cloud.close();
    cloud.paint(rgba(0xF8FAFCFF), window);

    let mut crim = c.stroke((r * 0.026).max(0.55));
    crim.move_to(-band_w * 0.92, band_top + r * 0.04);
    crim.cubic_to(-r * 0.98, band_top - r * 0.08, -r * 0.90, head_top - r * 0.44 - puff, -r * 0.42, head_top - r * 0.42 - puff);
    crim.cubic_to(-r * 0.32, head_top - r * 0.60 - puff, r * 0.32, head_top - r * 0.60 - puff, r * 0.42, head_top - r * 0.42 - puff);
    crim.cubic_to(r * 0.90, head_top - r * 0.44 - puff, r * 0.98, band_top - r * 0.08, band_w * 0.92, band_top + r * 0.04);
    crim.paint(rgba(0x94A3B8FF), window);

    // 百褶竖纹
    for px_f in [-0.36_f32, -0.14, 0.14, 0.36] {
        c.stroke_quad(
            px_f * r * 1.05,
            band_top - r * 0.01,
            px_f * r * 1.18,
            head_top - r * 0.25,
            px_f * r * 0.96,
            head_top - r * 0.40 - puff,
            (r * 0.020).max(0.45),
            rgba(0x94A3B877),
            window,
        );
    }

    // 3) 挺括雪白厨师帽带 + 红色滚边 + 米其林红星徽
    let mut band = c.fill();
    band.move_to(-band_w, base_y);
    band.line_to(-band_w * 0.94, band_top);
    band.quad_to(0.0, band_top - r * 0.10, band_w * 0.94, band_top);
    band.line_to(band_w, base_y);
    band.quad_to(0.0, base_y - r * 0.13, -band_w, base_y);
    band.close();
    band.paint(rgba(0xFFFFFFFF), window);

    c.stroke_quad(-band_w * 0.96, base_y - r * 0.035, 0.0, base_y - r * 0.16, band_w * 0.96, base_y - r * 0.035, (r * 0.032).max(0.65), rgba(0xDC2626FF), window);
    let by = (base_y + band_top) * 0.5 - r * 0.06;
    c.fill_circle(0.0, by, r * 0.105, rgba(0xDC2626FF), window);
    c.stroke_circle(0.0, by, r * 0.105, (r * 0.022).max(0.45), rgba(0xFACC15FF), window);
    c.fill_star5(0.0, by, r * 0.065, r * 0.030, 0.0, rgba(0xFEF08AFF), window);
}

/// 17. 蓝天机长制服帽 (Pilot)
fn draw_pilot(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let head_top = -c.head_r;
    let cap = c.sub(0.0, head_top + r * 0.22, -0.04);
    let cw = r * 0.94;
    let top_y = -r * 0.42;

    // 1) 深藏青高冠
    let mut crown = cap.fill();
    crown.move_to(-cw * 0.78, -r * 0.02);
    crown.cubic_to(-cw * 1.08, -r * 0.24, -cw * 0.94, top_y - r * 0.02, -cw * 0.42, top_y);
    crown.quad_to(0.0, top_y - r * 0.13, cw * 0.42, top_y);
    crown.cubic_to(cw * 0.94, top_y - r * 0.02, cw * 1.08, -r * 0.24, cw * 0.78, -r * 0.02);
    crown.quad_to(0.0, -r * 0.14, -cw * 0.78, -r * 0.02);
    crown.close();
    crown.paint(rgba(0x1E3A8AFF), window);
    cap.fill_ellipse(-r * 0.10, top_y + r * 0.08, cw * 0.44, r * 0.085, -0.06, rgba(0x60A5FA36), window);

    // 2) 黑帽墙 + 亮黑漆皮前檐 + 金色麦穗
    let mut wall = cap.fill();
    wall.move_to(-cw * 0.78, r * 0.05);
    wall.line_to(-cw * 0.76, -r * 0.10);
    wall.quad_to(0.0, -r * 0.19, cw * 0.76, -r * 0.10);
    wall.line_to(cw * 0.78, r * 0.05);
    wall.quad_to(0.0, -r * 0.05, -cw * 0.78, r * 0.05);
    wall.close();
    wall.paint(rgba(0x0F172AFF), window);

    let mut visor = cap.fill();
    visor.move_to(-cw * 0.80, r * 0.03);
    visor.quad_to(0.0, -r * 0.06, cw * 0.80, r * 0.03);
    visor.quad_to(cw * 0.60, r * 0.21, 0.0, r * 0.16);
    visor.quad_to(-cw * 0.60, r * 0.21, -cw * 0.80, r * 0.03);
    visor.close();
    visor.paint(rgba(0x090D16FF), window);
    cap.stroke_quad(-cw * 0.54, r * 0.08, -cw * 0.12, r * 0.03, cw * 0.24, r * 0.06, (r * 0.022).max(0.45), rgba(0xFFFFFF44), window);

    for side in [-1.0_f32, 1.0] {
        cap.stroke_quad(side * cw * 0.16, r * 0.12, side * cw * 0.40, r * 0.14, side * cw * 0.56, r * 0.08, (r * 0.024).max(0.5), rgba(0xFACC15FF), window);
    }

    // 金色双绞帽带与侧扣
    cap.stroke_quad(-cw * 0.76, r * 0.02, 0.0, -r * 0.08, cw * 0.76, r * 0.02, (r * 0.044).max(0.8), rgba(0xF59E0BFF), window);
    for side in [-1.0_f32, 1.0] {
        cap.fill_circle(side * cw * 0.76, r * 0.02, r * 0.046, rgba(0xFDE047FF), window);
    }

    // 3) 金色展翅航空徽章
    let ey = -r * 0.24;
    let glint = 0.75 + 0.25 * (time_sec * 3.4).sin();
    cap.fill_ellipse(0.0, ey, r * 0.36, r * 0.12, 0.0, rgba_alpha(0xFACC15, 0.22 * glint), window);
    for side in [-1.0_f32, 1.0] {
        let mut wing = cap.fill();
        wing.move_to(side * r * 0.08, ey - r * 0.02);
        wing.line_to(side * r * 0.34, ey - r * 0.09);
        wing.line_to(side * r * 0.28, ey + r * 0.03);
        wing.line_to(side * r * 0.08, ey + r * 0.06);
        wing.close();
        wing.paint(rgba(0xFACC15FF), window);
    }
    cap.fill_circle(0.0, ey + r * 0.01, r * 0.092, rgba(0x1D4ED8FF), window);
    cap.stroke_circle(0.0, ey + r * 0.01, r * 0.092, (r * 0.024).max(0.5), rgba(0xFEF08AFF), window);
    cap.fill_circle(0.0, ey + r * 0.01, r * 0.034, rgba(0xFEF08AFF), window);
}

/// 18. 仁心医生额镜帽 (Doctor)
fn draw_doctor(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let head_top = -c.head_r;
    let base_y = head_top + r * 0.24;
    let cap_w = r * 0.76;

    // 1) 纯白折叠医护帽冠 + 冰蓝医院饰条
    let mut cap = c.fill();
    cap.move_to(-cap_w * 0.86, base_y - r * 0.04);
    cap.line_to(-cap_w * 0.96, head_top - r * 0.15);
    cap.quad_to(-cap_w * 0.44, head_top - r * 0.34, 0.0, head_top - r * 0.28);
    cap.quad_to(cap_w * 0.44, head_top - r * 0.34, cap_w * 0.96, head_top - r * 0.15);
    cap.line_to(cap_w * 0.86, base_y - r * 0.04);
    cap.quad_to(0.0, base_y - r * 0.20, -cap_w * 0.86, base_y - r * 0.04);
    cap.close();
    cap.paint(rgba(0xF8FAFCFF), window);

    c.stroke_quad(-cap_w * 0.86, head_top - r * 0.09, 0.0, head_top - r * 0.24, cap_w * 0.86, head_top - r * 0.09, (r * 0.034).max(0.65), rgba(0x38BDF8FF), window);

    // 2) 醒目红十字医疗徽标
    let cx0 = -r * 0.10;
    let cy0 = head_top - r * 0.06;
    let arm = r * 0.115;
    let thick = (r * 0.072).max(1.3);
    c.stroke_line(cx0, cy0 - arm, cx0, cy0 + arm, thick, rgba(0xEF4444FF), window);
    c.stroke_line(cx0 - arm, cy0, cx0 + arm, cy0, thick, rgba(0xEF4444FF), window);

    // 3) 银色金属额镜头带 + 右额圆盘反光额镜
    c.stroke_quad(-cap_w * 0.92, base_y, 0.0, base_y - r * 0.20, cap_w * 0.92, base_y, (r * 0.068).max(1.2), rgba(0x475569FF), window);
    c.stroke_quad(-cap_w * 0.90, base_y - r * 0.005, 0.0, base_y - r * 0.205, cap_w * 0.90, base_y - r * 0.005, (r * 0.034).max(0.65), rgba(0xE2E8F0FF), window);

    let mx = r * 0.35;
    let my = base_y - r * 0.13;
    let mr = r * 0.205;
    let glint = 0.55 + 0.45 * (time_sec * 2.8).sin();
    c.fill_circle(mx, my, mr, rgba(0xCBD5E1FF), window);
    c.fill_circle(mx - mr * 0.14, my - mr * 0.14, mr * 0.72, rgba_alpha(0xE0F2FE, 0.75 + 0.25 * glint), window);
    c.stroke_circle(mx, my, mr, (r * 0.032).max(0.65), rgba(0x334155FF), window);
    c.fill_circle(mx, my, mr * 0.26, rgba(0x1E293BFF), window);
}

/// 19. 航海王红带草帽 (StrawHat)
fn draw_straw_hat(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let head_top = -c.head_r;
    let hat = c.sub(0.0, head_top + r * 0.22, -0.05);
    let brim_w = r * 1.32;
    let dome_w = r * 0.78;
    let dome_h = r * 0.46;

    // 1) 右侧随海风微摇红帽绳
    let sway = (time_sec * 3.1).sin() * 0.14;
    let cord = hat.sub(dome_w * 0.88, r * 0.08, sway);
    cord.stroke_cubic(0.0, 0.0, r * 0.12, r * 0.14, r * 0.02, r * 0.30, r * 0.10, r * 0.44, (r * 0.034).max(0.65), rgba(0xDC2626FF), window);
    cord.fill_circle(r * 0.10, r * 0.44, r * 0.038, rgba(0xFACC15FF), window);

    // 2) 阳光暖金宽大草帽檐
    let mut brim = hat.fill();
    brim.move_to(-brim_w, r * 0.08);
    brim.cubic_to(-brim_w * 0.65, -r * 0.15, brim_w * 0.65, -r * 0.15, brim_w, r * 0.08);
    brim.cubic_to(brim_w * 0.72, r * 0.27, -brim_w * 0.72, r * 0.27, -brim_w, r * 0.08);
    brim.close();
    brim.paint(rgba(0xFACC15FF), window);

    let mut brim_rim = hat.stroke((r * 0.030).max(0.6));
    brim_rim.move_to(-brim_w, r * 0.08);
    brim_rim.cubic_to(-brim_w * 0.65, -r * 0.15, brim_w * 0.65, -r * 0.15, brim_w, r * 0.08);
    brim_rim.cubic_to(brim_w * 0.72, r * 0.27, -brim_w * 0.72, r * 0.27, -brim_w, r * 0.08);
    brim_rim.close();
    brim_rim.paint(rgba(0x92400EFF), window);

    // 3) 圆润半圆草帽顶冠
    let mut dome = hat.fill();
    dome.move_to(-dome_w, r * 0.03);
    dome.cubic_to(-dome_w * 0.98, -dome_h * 1.22, dome_w * 0.98, -dome_h * 1.22, dome_w, r * 0.03);
    dome.quad_to(0.0, r * 0.15, -dome_w, r * 0.03);
    dome.close();
    dome.paint(rgba(0xFDE047FF), window);

    let mut drim = hat.stroke((r * 0.028).max(0.55));
    drim.move_to(-dome_w, r * 0.03);
    drim.cubic_to(-dome_w * 0.98, -dome_h * 1.22, dome_w * 0.98, -dome_h * 1.22, dome_w, r * 0.03);
    drim.paint(rgba(0x92400EFF), window);

    // 4) 标志性鲜红宽绸带环
    let mut rib = hat.fill();
    rib.move_to(-dome_w * 0.99, r * 0.03);
    rib.line_to(-dome_w * 0.95, -r * 0.13);
    rib.quad_to(0.0, -r * 0.02, dome_w * 0.95, -r * 0.13);
    rib.line_to(dome_w * 0.99, r * 0.03);
    rib.quad_to(0.0, r * 0.14, -dome_w * 0.99, r * 0.03);
    rib.close();
    rib.paint(rgba(0xDC2626FF), window);

    // 阳光星芒
    let tw = 0.55 + 0.45 * (time_sec * 3.8).sin();
    hat.fill_star4(-r * 0.40, -dome_h * 0.85, r * 0.075 * tw, rgba_alpha(0xFFFBEB, 0.55 + 0.45 * tw), window);
}

/// 20. 西部警长牛仔帽 (Cowboy)
fn draw_cowboy(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let head_top = -c.head_r;
    let hat = c.sub(0.0, head_top + r * 0.24, -0.06);
    let brim_w = r * 1.30;
    let cw = r * 0.70;
    let top_y = -r * 0.48;

    // 1) 两侧潇洒上卷阔檐
    let mut brim = hat.fill();
    brim.move_to(-brim_w, -r * 0.14);
    brim.cubic_to(-brim_w * 0.70, -r * 0.04, -cw * 0.78, r * 0.02, 0.0, r * 0.04);
    brim.cubic_to(cw * 0.78, r * 0.02, brim_w * 0.70, -r * 0.04, brim_w, -r * 0.14);
    brim.cubic_to(brim_w * 0.86, r * 0.17, cw * 0.54, r * 0.23, 0.0, r * 0.20);
    brim.cubic_to(-cw * 0.54, r * 0.23, -brim_w * 0.86, r * 0.17, -brim_w, -r * 0.14);
    brim.close();
    brim.paint(rgba(0x9A4A1CFF), window);

    let mut brim_rim = hat.stroke((r * 0.030).max(0.6));
    brim_rim.move_to(-brim_w, -r * 0.14);
    brim_rim.cubic_to(-brim_w * 0.86, r * 0.17, -cw * 0.54, r * 0.23, 0.0, r * 0.20);
    brim_rim.cubic_to(cw * 0.54, r * 0.23, brim_w * 0.86, r * 0.17, brim_w, -r * 0.14);
    brim_rim.paint(rgba(0x3B1106FF), window);

    // 2) 中部微凹双峰皮革帽冠
    let mut crown = hat.fill();
    crown.move_to(-cw, r * 0.04);
    crown.cubic_to(-cw * 0.96, top_y + r * 0.08, -cw * 0.62, top_y - r * 0.06, -cw * 0.34, top_y - r * 0.02);
    crown.quad_to(0.0, top_y + r * 0.14, cw * 0.34, top_y - r * 0.02);
    crown.cubic_to(cw * 0.62, top_y - r * 0.06, cw * 0.96, top_y + r * 0.08, cw, r * 0.04);
    crown.quad_to(0.0, r * 0.15, -cw, r * 0.04);
    crown.close();
    crown.paint(rgba(0xB45309FF), window);

    let mut crim = hat.stroke((r * 0.028).max(0.55));
    crim.move_to(-cw, r * 0.04);
    crim.cubic_to(-cw * 0.96, top_y + r * 0.08, -cw * 0.62, top_y - r * 0.06, -cw * 0.34, top_y - r * 0.02);
    crim.quad_to(0.0, top_y + r * 0.14, cw * 0.34, top_y - r * 0.02);
    crim.cubic_to(cw * 0.62, top_y - r * 0.06, cw * 0.96, top_y + r * 0.08, cw, r * 0.04);
    crim.paint(rgba(0x3B1106FF), window);

    // 3) 深棕牛皮帽带 + 闪耀金色西部警长星徽
    let mut band = hat.fill();
    band.move_to(-cw * 1.01, r * 0.05);
    band.line_to(-cw * 0.96, -r * 0.09);
    band.quad_to(0.0, r * 0.02, cw * 0.96, -r * 0.09);
    band.line_to(cw * 1.01, r * 0.05);
    band.quad_to(0.0, r * 0.15, -cw * 1.01, r * 0.05);
    band.close();
    band.paint(rgba(0x3B1106FF), window);

    let pulse = 0.76 + 0.24 * (time_sec * 3.6).sin();
    hat.fill_circle(0.0, -r * 0.04, r * 0.17, rgba_alpha(0xFACC15, 0.25 * pulse), window);
    hat.fill_star5(0.0, -r * 0.04, r * 0.155, r * 0.072, 0.0, rgba(0xFACC15FF), window);
}

/// 21. 清宫格格旗头大拉翅 (Gege)
fn draw_gege(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let head_top = -c.head_r;
    let base_y = head_top + r * 0.22;
    let wing_w = r * 1.18;
    let top_y = head_top - r * 0.42;

    // 1) 双侧飘逸翡翠珠红流苏
    for side in [-1.0_f32, 1.0] {
        let sway = (time_sec * 2.9 + side * 0.7).sin() * 0.11;
        let t = c.sub(side * wing_w * 0.86, top_y + r * 0.32, sway);
        t.stroke_line(0.0, 0.0, 0.0, r * 0.22, (r * 0.024).max(0.5), rgba(0xFACC15FF), window);
        t.fill_circle(0.0, r * 0.13, r * 0.052, rgba(0x10B981FF), window);
        t.fill_ellipse(0.0, r * 0.22, r * 0.048, r * 0.028, 0.0, rgba(0xF59E0BFF), window);
        let mut tas = t.fill();
        tas.move_to(-r * 0.040, r * 0.24);
        tas.line_to(r * 0.040, r * 0.24);
        tas.line_to(r * 0.070, r * 0.62);
        tas.quad_to(0.0, r * 0.68, -r * 0.070, r * 0.62);
        tas.close();
        tas.paint(rgba(0xDC2626FF), window);
    }

    // 2) 玄黑金边扇形大拉翅冠板
    let mut board = c.fill();
    board.move_to(-r * 0.62, base_y - r * 0.06);
    board.cubic_to(-wing_w * 0.96, base_y - r * 0.16, -wing_w * 1.04, top_y + r * 0.04, -wing_w * 0.76, top_y);
    board.quad_to(0.0, top_y - r * 0.11, wing_w * 0.76, top_y);
    board.cubic_to(wing_w * 1.04, top_y + r * 0.04, wing_w * 0.96, base_y - r * 0.16, r * 0.62, base_y - r * 0.06);
    board.quad_to(0.0, base_y - r * 0.22, -r * 0.62, base_y - r * 0.06);
    board.close();
    board.paint(rgba(0x18181BFF), window);

    let mut brim = c.stroke((r * 0.032).max(0.65));
    brim.move_to(-r * 0.62, base_y - r * 0.06);
    brim.cubic_to(-wing_w * 0.96, base_y - r * 0.16, -wing_w * 1.04, top_y + r * 0.04, -wing_w * 0.76, top_y);
    brim.quad_to(0.0, top_y - r * 0.11, wing_w * 0.76, top_y);
    brim.cubic_to(wing_w * 1.04, top_y + r * 0.04, wing_w * 0.96, base_y - r * 0.16, r * 0.62, base_y - r * 0.06);
    brim.paint(rgba(0xD97706FF), window);

    // 3) 翡翠绿叶 + 三朵盛放粉嫩牡丹宫花
    for (lx, ly, ang) in [
        (-r * 0.26, top_y + r * 0.20, -2.5_f32),
        (r * 0.26, top_y + r * 0.20, -0.64),
    ] {
        c.fill_ellipse(lx, ly, r * 0.16, r * 0.065, ang, rgba(0x10B981FF), window);
    }
    for (fx, fy, fr, outer_c, inner_c) in [
        (-r * 0.54, top_y + r * 0.21, r * 0.145, 0xF472B6FF_u32, 0xFBCFE8FF_u32),
        (r * 0.54, top_y + r * 0.21, r * 0.145, 0xF472B6FF, 0xFBCFE8FF),
        (0.0, top_y + r * 0.21, r * 0.195, 0xFB7185FF, 0xFFE4E6FF),
    ] {
        for i in 0..5 {
            let a = (i as f32) * (2.0 * PI / 5.0);
            c.fill_circle(fx + a.cos() * fr * 0.48, fy + a.sin() * fr * 0.44, fr * 0.46, rgba(outer_c), window);
        }
        c.fill_circle(fx, fy, fr * 0.50, rgba(inner_c), window);
        c.fill_circle(fx, fy, fr * 0.24, rgba(0xFEF08AFF), window);
    }
    c.fill_circle(0.0, top_y - r * 0.01, r * 0.052, rgba(0xFACC15FF), window);
}

/// 22. 英国大律师假发 (Barrister)
fn draw_barrister(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let head_top = -c.head_r;
    let base_y = head_top + r * 0.36;
    let peak_y = head_top - r * 0.20;
    let crown_w = r * 0.92;
    let sway = (time_sec * 2.5).sin() * 0.035;

    // 1) 两侧下方探出双小辫尾圈 + 精致小黑丝结
    for side in [-1.0_f32, 1.0] {
        let t = c.sub(side * r * 0.94, r * 0.15, side * (-0.06 + sway));
        t.fill_ellipse(side * r * 0.03, r * 0.16, r * 0.10, r * 0.165, side * 0.12, rgba(0xE2E8F0FF), window);
        t.stroke_ellipse(side * r * 0.03, r * 0.16, r * 0.10, r * 0.165, side * 0.12, (r * 0.022).max(0.45), rgba(0x94A3B8FF), window);
        // 黑丝蝴蝶结
        t.fill_ellipse(-r * 0.05, r * 0.03, r * 0.055, r * 0.032, 0.2, rgba(0x18181BFF), window);
        t.fill_ellipse(r * 0.05, r * 0.03, r * 0.055, r * 0.032, -0.2, rgba(0x18181BFF), window);
    }

    // 2) 饱满象牙白马尾卷发顶冠
    let mut crown = c.fill();
    crown.move_to(-crown_w, base_y + r * 0.04);
    crown.cubic_to(-crown_w * 1.02, peak_y + r * 0.08, -crown_w * 0.48, peak_y - r * 0.04, 0.0, peak_y);
    crown.cubic_to(crown_w * 0.48, peak_y - r * 0.04, crown_w * 1.02, peak_y + r * 0.08, crown_w, base_y + r * 0.04);
    crown.quad_to(0.0, base_y - r * 0.32, -crown_w, base_y + r * 0.04);
    crown.close();
    crown.paint(rgba(0xF8FAFCFF), window);

    // 顶冠横向波浪卷纹 + 中分缝
    for i in 0..3 {
        let y_off = peak_y + r * (0.12 + (i as f32) * 0.095);
        let span = crown_w * (0.58 + (i as f32) * 0.12);
        c.stroke_quad(-span, y_off + r * 0.02, -span * 0.45, y_off - r * 0.05, 0.0, y_off - r * 0.01, (r * 0.022).max(0.45), rgba(0x94A3B888), window);
        c.stroke_quad(0.0, y_off - r * 0.01, span * 0.45, y_off - r * 0.05, span, y_off + r * 0.02, (r * 0.022).max(0.45), rgba(0x94A3B888), window);
    }
    c.stroke_line(0.0, peak_y + r * 0.02, 0.0, base_y - r * 0.22, (r * 0.022).max(0.45), rgba(0x64748B77), window);

    // 3) 两侧标志性 4 层阶梯式圆润马尾大卷筒
    for side in [-1.0_f32, 1.0] {
        let tiers = [
            (side * r * 0.88, head_top + r * 0.34, r * 0.21, r * 0.10, side * 0.26),
            (side * r * 0.96, head_top + r * 0.54, r * 0.22, r * 0.105, side * 0.14),
            (side * r * 1.00, head_top + r * 0.74, r * 0.225, r * 0.108, side * 0.04),
            (side * r * 0.98, head_top + r * 0.94, r * 0.21, r * 0.10, side * -0.06),
        ];
        for (idx, (tx, ty, trx, try_, rot)) in tiers.into_iter().enumerate() {
            let micro = (time_sec * 2.5 - (idx as f32) * 0.45).sin() * r * 0.008;
            let seg = c.sub(tx + side * micro, ty, rot);
            seg.fill_ellipse(0.0, 0.0, trx, try_, 0.0, rgba(0xF1F5F9FF), window);
            seg.stroke_ellipse(0.0, 0.0, trx, try_, 0.0, (r * 0.024).max(0.5), rgba(0x64748BFF), window);
            seg.stroke_circle(side * trx * 0.52, 0.0, try_ * 0.42, (r * 0.018).max(0.45), rgba(0x47556988), window);
        }
    }

    // 4) 额前双波浪卷发际线
    let mut wave = c.stroke((r * 0.068).max(1.2));
    wave.move_to(-r * 0.80, base_y - r * 0.01);
    wave.quad_to(-r * 0.38, base_y - r * 0.23, 0.0, base_y - r * 0.15);
    wave.quad_to(r * 0.38, base_y - r * 0.23, r * 0.80, base_y - r * 0.01);
    wave.paint(rgba(0xFFFFFFFF), window);
}

/// 23. 犹太经典小白帽 (Kippah)
fn draw_kippah(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let head_top = -c.head_r;
    let cap_w = r * 0.58;
    let base_y = head_top + r * 0.14;
    let peak_y = head_top - r * 0.14;

    // 1) 纯白圆顶基帕小圆帽主体
    let mut cap = c.fill();
    cap.move_to(-cap_w, base_y);
    cap.cubic_to(-cap_w * 0.92, peak_y + r * 0.03, -cap_w * 0.42, peak_y, 0.0, peak_y);
    cap.cubic_to(cap_w * 0.42, peak_y, cap_w * 0.92, peak_y + r * 0.03, cap_w, base_y);
    cap.quad_to(0.0, base_y - r * 0.16, -cap_w, base_y);
    cap.close();
    cap.paint(rgba(0xF8FAFCFF), window);

    let mut rim = c.stroke((r * 0.024).max(0.5));
    rim.move_to(-cap_w, base_y);
    rim.cubic_to(-cap_w * 0.92, peak_y + r * 0.03, -cap_w * 0.42, peak_y, 0.0, peak_y);
    rim.cubic_to(cap_w * 0.42, peak_y, cap_w * 0.92, peak_y + r * 0.03, cap_w, base_y);
    rim.paint(rgba(0x94A3B8FF), window);

    // 2) 宝蓝刺绣滚边 + 浅蓝装饰细线
    c.stroke_quad(-cap_w * 0.96, base_y - r * 0.018, 0.0, base_y - r * 0.17, cap_w * 0.96, base_y - r * 0.018, (r * 0.048).max(0.85), rgba(0x1D4ED8FF), window);
    c.stroke_quad(-cap_w * 0.88, base_y - r * 0.056, 0.0, base_y - r * 0.20, cap_w * 0.88, base_y - r * 0.056, (r * 0.020).max(0.45), rgba(0x60A5FAFF), window);

    // 3) 正中宝蓝「大卫之星」刺绣
    let sy = peak_y + r * 0.115;
    let sr = r * 0.068;
    let glint = 0.82 + 0.18 * (time_sec * 3.0).sin();
    let col = rgba_alpha(0x1D4ED8, glint);
    for rot in [-PI * 0.5, PI * 0.5] {
        let mut tri = c.stroke((r * 0.022).max(0.45));
        for i in 0..3 {
            let a = rot + (i as f32) * (2.0 * PI / 3.0);
            let px_v = a.cos() * sr;
            let py_v = sy + a.sin() * sr * 0.86;
            if i == 0 {
                tri.move_to(px_v, py_v);
            } else {
                tri.line_to(px_v, py_v);
            }
        }
        tri.close();
        tri.paint(col, window);
    }
}

/// 24. 潮流街头棒球帽 (BaseballCap)
fn draw_baseball_cap(c: &HeadCanvas, window: &mut Window) {
    let r = c.r;
    let head_top = -c.head_r;
    let cap = c.sub(-r * 0.02, head_top + r * 0.28, -0.08);
    let cw = r * 0.88;
    let top_y = -r * 0.44;

    // 1) 顶扣 + 学院蓝立体六片帽冠
    cap.fill_ellipse(0.0, top_y - r * 0.015, r * 0.082, r * 0.042, 0.0, rgba(0x1D4ED8FF), window);
    let mut crown = cap.fill();
    crown.move_to(-cw, r * 0.04);
    crown.cubic_to(-cw * 1.02, top_y + r * 0.04, -cw * 0.48, top_y, 0.0, top_y);
    crown.cubic_to(cw * 0.48, top_y, cw * 1.02, top_y + r * 0.04, cw, r * 0.04);
    crown.quad_to(0.0, -r * 0.15, -cw, r * 0.04);
    crown.close();
    crown.paint(rgba(0x2563EBFF), window);
    cap.fill_ellipse(-r * 0.16, top_y + r * 0.14, r * 0.34, r * 0.12, -0.15, rgba(0x60A5FA55), window);

    // 拼缝弧线 + 醒目 3D 立体刺绣蔚来 NIO Logo
    for side in [-1.0_f32, 1.0] {
        cap.stroke_quad(0.0, top_y + r * 0.01, side * cw * 0.48, top_y + r * 0.16, side * cw * 0.54, -r * 0.05, (r * 0.018).max(0.45), rgba(0xDBEAFE55), window);
    }
    let by = top_y + r * 0.26;
    let lr = r * 0.145;
    let draw_nio = |ox: f32, oy: f32, col: Rgba, window: &mut Window| {
        let cx0 = ox;
        let cy0 = by + oy;
        // Sky 上半圆拱
        let mut sky = cap.fill();
        let steps = 14;
        for i in 0..=steps {
            let a = PI - (i as f32 / steps as f32) * PI;
            let x = cx0 + a.cos() * lr;
            let y = cy0 - a.sin() * lr;
            if i == 0 {
                sky.move_to(x, y);
            } else {
                sky.line_to(x, y);
            }
        }
        sky.line_to(cx0 + lr * 0.56, cy0);
        sky.quad_to(cx0 + lr * 0.455, cy0, cx0 + lr * 0.448, cy0 - lr * 0.085);
        for i in 0..=steps {
            let a = 0.185 + (i as f32 / steps as f32) * (PI - 0.370);
            sky.line_to(cx0 + a.cos() * lr * 0.455, cy0 - a.sin() * lr * 0.455);
        }
        sky.quad_to(cx0 - lr * 0.455, cy0, cx0 - lr * 0.56, cy0);
        sky.close();
        sky.paint(col, window);

        // Earth 下半折角（平顶梯肩 + 同心外圆弧包边 + 柔和内尖）
        let mut earth = cap.fill();
        let outer_a = 0.435_f32;
        let foot_a = 0.965_f32;
        earth.move_to(cx0 - lr * 0.37, cy0 + lr * 0.005);
        earth.line_to(cx0 + lr * 0.37, cy0 + lr * 0.005);
        earth.quad_to(cx0 + lr * 0.45, cy0 + lr * 0.005, cx0 + lr * 0.50, cy0 + lr * 0.055);
        for i in 0..=6 {
            let a = outer_a + (foot_a - outer_a) * (i as f32 / 6.0);
            earth.line_to(cx0 + a.cos() * lr, cy0 + a.sin() * lr);
        }
        earth.quad_to(cx0 + lr * 0.51, cy0 + lr * 0.855, cx0 + lr * 0.445, cy0 + lr * 0.775);
        earth.line_to(cx0 + lr * 0.055, cy0 + lr * 0.365);
        earth.quad_to(cx0, cy0 + lr * 0.310, cx0 - lr * 0.055, cy0 + lr * 0.365);
        earth.line_to(cx0 - lr * 0.445, cy0 + lr * 0.775);
        earth.quad_to(
            cx0 - lr * 0.51,
            cy0 + lr * 0.855,
            cx0 - foot_a.cos() * lr,
            cy0 + foot_a.sin() * lr,
        );
        for i in 1..=6 {
            let a = (PI - foot_a) + (foot_a - outer_a) * (i as f32 / 6.0);
            earth.line_to(cx0 + a.cos() * lr, cy0 + a.sin() * lr);
        }
        earth.line_to(cx0 - lr * 0.50, cy0 + lr * 0.055);
        earth.quad_to(cx0 - lr * 0.45, cy0 + lr * 0.005, cx0 - lr * 0.37, cy0 + lr * 0.005);
        earth.close();
        earth.paint(col, window);
    };

    // 底层深蓝刺绣阴影 + 顶层雪白刺绣 NIO 标
    draw_nio(r * 0.012, r * 0.012, rgba(0x17255499), window);
    draw_nio(0.0, 0.0, rgba(0xFFFFFFFF), window);

    // 2) 向右前斜伸的立体弯帽檐
    let mut under = cap.fill();
    under.move_to(-cw * 0.84, r * 0.02);
    under.quad_to(r * 0.20, -r * 0.13, cw * 1.32, r * 0.02);
    under.quad_to(r * 0.42, r * 0.18, -cw * 0.84, r * 0.02);
    under.close();
    under.paint(rgba(0x172554FF), window);

    let mut visor = cap.fill();
    visor.move_to(-cw * 0.86, r * 0.01);
    visor.quad_to(0.0, -r * 0.16, cw * 0.92, -r * 0.06);
    visor.quad_to(cw * 1.38, -r * 0.02, cw * 1.30, r * 0.07);
    visor.quad_to(cw * 0.54, r * 0.13, -cw * 0.86, r * 0.01);
    visor.close();
    visor.paint(rgba(0x3B82F6FF), window);

    cap.stroke_quad(-cw * 0.64, -r * 0.02, cw * 0.42, -r * 0.05, cw * 1.20, r * 0.03, (r * 0.018).max(0.45), rgba(0xFFFFFF66), window);
}

/// 25. 日系潮流渔夫帽 (BucketHat)
fn draw_bucket_hat(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let head_top = -c.head_r;
    let hat = c.sub(0.0, head_top + r * 0.22, -0.03);
    let top_w = r * 0.66;
    let mid_w = r * 0.84;
    let brim_w = r * 1.20;
    let top_y = -r * 0.42;
    let band_y = -r * 0.10;
    let flutter = (time_sec * 2.6).sin() * r * 0.014;

    // 1) 奶油卡其平顶筒身
    let mut crown = hat.fill();
    crown.move_to(-mid_w, band_y);
    crown.line_to(-top_w, top_y + r * 0.04);
    crown.quad_to(0.0, top_y - r * 0.06, top_w, top_y + r * 0.04);
    crown.line_to(mid_w, band_y);
    crown.quad_to(0.0, band_y - r * 0.11, -mid_w, band_y);
    crown.close();
    crown.paint(rgba(0xE8D8BCFF), window);

    // 2) 森系橄榄绿织带 + 焦糖橙山峰小织标
    let mut band = hat.fill();
    band.move_to(-mid_w, band_y + r * 0.01);
    band.line_to(-mid_w * 0.96, band_y - r * 0.11);
    band.quad_to(0.0, band_y - r * 0.20, mid_w * 0.96, band_y - r * 0.11);
    band.line_to(mid_w, band_y + r * 0.01);
    band.quad_to(0.0, band_y - r * 0.09, -mid_w, band_y + r * 0.01);
    band.close();
    band.paint(rgba(0x3F6212FF), window);

    let mut tag = hat.fill();
    tag.move_to(-r * 0.11, band_y - r * 0.16);
    tag.line_to(r * 0.11, band_y - r * 0.16);
    tag.line_to(r * 0.11, band_y - r * 0.055);
    tag.line_to(-r * 0.11, band_y - r * 0.055);
    tag.close();
    tag.paint(rgba(0xEA580CFF), window);

    // 3) 垂坠软檐 + 标志性同心车缝线
    let mut brim = hat.fill();
    brim.move_to(-mid_w, band_y);
    brim.quad_to(0.0, band_y - r * 0.10, mid_w, band_y);
    brim.cubic_to(mid_w * 1.16, band_y + r * 0.08, brim_w * 1.02, r * 0.14 - flutter, brim_w, r * 0.23 - flutter);
    brim.cubic_to(brim_w * 0.56, r * 0.10, -brim_w * 0.56, r * 0.10, -brim_w, r * 0.23 + flutter);
    brim.cubic_to(-brim_w * 1.02, r * 0.14 + flutter, -mid_w * 1.16, band_y + r * 0.08, -mid_w, band_y);
    brim.close();
    brim.paint(rgba(0xDFCBB0FF), window);

    let mut rim = hat.stroke((r * 0.026).max(0.55));
    rim.move_to(-mid_w, band_y);
    rim.cubic_to(-mid_w * 1.16, band_y + r * 0.08, -brim_w * 1.02, r * 0.14 + flutter, -brim_w, r * 0.23 + flutter);
    rim.cubic_to(-brim_w * 0.56, r * 0.10, brim_w * 0.56, r * 0.10, brim_w, r * 0.23 - flutter);
    rim.cubic_to(brim_w * 1.02, r * 0.14 - flutter, mid_w * 1.16, band_y + r * 0.08, mid_w, band_y);
    rim.paint(rgba(0x786246FF), window);

    for f in [0.35_f32, 0.72] {
        let xs = mid_w + (brim_w - mid_w) * f;
        let ye = band_y + (r * 0.23 - band_y) * f;
        let ym = band_y - r * 0.10 + (r * 0.11 - (band_y - r * 0.10)) * f;
        hat.stroke_quad(-xs * 0.96, ye + flutter * f, 0.0, ym, xs * 0.96, ye - flutter * f, (r * 0.016).max(0.45), rgba(0x78624666), window);
    }
}

/// 26. 荣誉毕业学士帽 (Graduation)
fn draw_graduation(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let head_top = -c.head_r;
    let hat = c.sub(0.0, head_top + r * 0.22, -0.04);
    let skull_w = r * 0.70;
    let board_w = r * 1.16;
    let board_y = -r * 0.26;
    let board_h = r * 0.23;

    // 1) 玄黑下帽筒 + 暗金荣誉边
    let mut skull = hat.fill();
    skull.move_to(-skull_w, r * 0.04);
    skull.line_to(-skull_w * 0.94, board_y + r * 0.06);
    skull.line_to(skull_w * 0.94, board_y + r * 0.06);
    skull.line_to(skull_w, r * 0.04);
    skull.quad_to(0.0, -r * 0.11, -skull_w, r * 0.04);
    skull.close();
    skull.paint(rgba(0x18181BFF), window);
    hat.stroke_quad(-skull_w * 0.94, r * 0.02, 0.0, -r * 0.12, skull_w * 0.94, r * 0.02, (r * 0.024).max(0.5), rgba(0xFACC1599), window);

    // 2) 菱形大方板 3D 厚度底边 + 顶面
    let thick = r * 0.05;
    let mut edge = hat.fill();
    edge.move_to(-board_w, board_y);
    edge.line_to(-board_w, board_y + thick);
    edge.line_to(0.0, board_y + board_h + thick);
    edge.line_to(board_w, board_y + thick);
    edge.line_to(board_w, board_y);
    edge.line_to(0.0, board_y + board_h);
    edge.close();
    edge.paint(rgba(0x09090BFF), window);

    let mut top = hat.fill();
    top.move_to(0.0, board_y - board_h);
    top.line_to(board_w, board_y);
    top.line_to(0.0, board_y + board_h);
    top.line_to(-board_w, board_y);
    top.close();
    top.paint(rgba(0x27272AFF), window);

    let mut trim = hat.stroke((r * 0.024).max(0.5));
    trim.move_to(0.0, board_y - board_h);
    trim.line_to(board_w, board_y);
    trim.line_to(0.0, board_y + board_h);
    trim.line_to(-board_w, board_y);
    trim.close();
    trim.paint(rgba(0x52525BFF), window);

    // 3) 中央金扣 + 绕右角垂摆的灿金流苏穗
    let drop_x = board_w * 0.90;
    let drop_y = board_y + r * 0.03;
    hat.stroke_quad(0.0, board_y, board_w * 0.48, board_y - r * 0.02, drop_x, drop_y, (r * 0.034).max(0.65), rgba(0xFACC15FF), window);
    hat.fill_ellipse(0.0, board_y, r * 0.072, r * 0.042, 0.0, rgba(0xFDE047FF), window);

    let sway = (time_sec * 3.0).sin() * 0.13;
    let t = hat.sub(drop_x, drop_y, sway);
    t.stroke_line(0.0, 0.0, 0.0, r * 0.30, (r * 0.032).max(0.6), rgba(0xFACC15FF), window);
    t.fill_circle(0.0, r * 0.28, r * 0.048, rgba(0xF59E0BFF), window);
    let mut tas = t.fill();
    tas.move_to(-r * 0.042, r * 0.30);
    tas.line_to(r * 0.042, r * 0.30);
    tas.line_to(r * 0.086, r * 0.60);
    tas.quad_to(0.0, r * 0.65, -r * 0.086, r * 0.60);
    tas.close();
    tas.paint(rgba(0xFACC15FF), window);
}

/// 27. 荣耀博士丝绒帽 (Doctoral)
fn draw_doctoral(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let head_top = -c.head_r;
    let hat = c.sub(0.0, head_top + r * 0.24, -0.05);
    let band_w = r * 0.76;
    let tam_w = r * 1.12;
    let top_y = -r * 0.42;
    let band_top = -r * 0.14;

    // 1) 酒红丝绒八角博士软帽主冠
    let mut tam = hat.fill();
    tam.move_to(-band_w * 0.94, band_top + r * 0.04);
    tam.cubic_to(-tam_w * 1.04, -r * 0.10, -tam_w * 1.02, top_y + r * 0.04, -tam_w * 0.60, top_y - r * 0.01);
    tam.quad_to(0.0, top_y - r * 0.13, tam_w * 0.60, top_y - r * 0.01);
    tam.cubic_to(tam_w * 1.02, top_y + r * 0.04, tam_w * 1.04, -r * 0.10, band_w * 0.94, band_top + r * 0.04);
    tam.close();
    tam.paint(rgba(0x991B1BFF), window);
    hat.fill_ellipse(-r * 0.22, top_y + r * 0.08, r * 0.32, r * 0.08, -0.14, rgba(0xFECACA30), window);

    // 2) 玄黑丝绒帽箍 + 双道鎏金滚边 + 桂冠菱徽
    let mut band = hat.fill();
    band.move_to(-band_w, r * 0.04);
    band.line_to(-band_w * 0.96, band_top);
    band.quad_to(0.0, band_top - r * 0.12, band_w * 0.96, band_top);
    band.line_to(band_w, r * 0.04);
    band.quad_to(0.0, -r * 0.10, -band_w, r * 0.04);
    band.close();
    band.paint(rgba(0x18181BFF), window);

    hat.stroke_quad(-band_w * 0.95, band_top + r * 0.01, 0.0, band_top - r * 0.11, band_w * 0.95, band_top + r * 0.01, (r * 0.026).max(0.55), rgba(0xFACC15FF), window);
    hat.stroke_quad(-band_w * 0.98, r * 0.02, 0.0, -r * 0.115, band_w * 0.98, r * 0.02, (r * 0.024).max(0.5), rgba(0xF59E0BFF), window);
    hat.fill_star4(0.0, (band_top + r * 0.03) * 0.5 - r * 0.055, r * 0.075, rgba(0xFDE047FF), window);

    // 3) 鎏金绞绳 + 右侧丰盈大红金丝博士流苏
    let center_y = top_y - r * 0.04;
    let drop_x = tam_w * 0.86;
    let drop_y = band_top + r * 0.02;
    hat.stroke_cubic(0.0, center_y, tam_w * 0.42, center_y - r * 0.04, tam_w * 0.84, top_y + r * 0.08, drop_x, drop_y, (r * 0.036).max(0.7), rgba(0xFACC15FF), window);
    hat.fill_ellipse(0.0, center_y, r * 0.08, r * 0.04, 0.0, rgba(0xFDE047FF), window);

    let sway = (time_sec * 2.8).sin() * 0.12;
    let t = hat.sub(drop_x, drop_y, sway);
    t.stroke_line(0.0, 0.0, 0.0, r * 0.22, (r * 0.034).max(0.65), rgba(0xFACC15FF), window);
    t.fill_circle(0.0, r * 0.22, r * 0.052, rgba(0xF59E0BFF), window);
    let mut tas = t.fill();
    tas.move_to(-r * 0.048, r * 0.25);
    tas.line_to(r * 0.048, r * 0.25);
    tas.line_to(r * 0.098, r * 0.58);
    tas.quad_to(0.0, r * 0.64, -r * 0.098, r * 0.58);
    tas.close();
    tas.paint(rgba(0xDC2626FF), window);
    t.stroke_line(-r * 0.02, r * 0.27, -r * 0.045, r * 0.56, (r * 0.018).max(0.45), rgba(0xFDE047BB), window);
    t.stroke_line(r * 0.02, r * 0.27, r * 0.045, r * 0.56, (r * 0.018).max(0.45), rgba(0xFDE047BB), window);
}

/// 28. 沙特王子白头巾 (SaudiPrince - 参照正面哈姆丹王子：高圆纯白布冠 + 下弧立体双黑 Agal + 不对称飘逸纯白 Ghutra)
fn draw_saudi_prince(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let head_top = -c.head_r;
    let brow_y = head_top + r * 0.22;
    let agal_y = head_top - r * 0.12;
    let peak_y = head_top - r * 0.50;
    let agal_w = r * 0.84;
    let sway = (time_sec * 2.1).sin() * r * 0.022;

    // 1) 左右披巾内侧柔灰暗部（空气感布帘立体层次）
    let mut inner_l = c.fill();
    inner_l.move_to(-r * 0.84, brow_y);
    inner_l.cubic_to(-r * 1.30, brow_y + r * 0.02, -r * 1.38 + sway, r * 0.22, -r * 1.18 + sway, r * 0.78);
    inner_l.quad_to(-r * 0.98 + sway * 0.5, r * 0.84, -r * 0.86, r * 0.70);
    inner_l.cubic_to(-r * 0.96, r * 0.28, -r * 0.94, -r * 0.12, -r * 0.84, brow_y);
    inner_l.close();
    inner_l.paint(rgba(0x94A3B8CC), window);

    let mut inner_r = c.fill();
    inner_r.move_to(r * 0.82, brow_y);
    inner_r.cubic_to(r * 1.18, brow_y + r * 0.06, r * 1.28 - sway, r * 0.24, r * 1.14 - sway, r * 0.76);
    inner_r.quad_to(r * 0.96 - sway * 0.5, r * 0.82, r * 0.86, r * 0.68);
    inner_r.cubic_to(r * 0.96, r * 0.28, r * 0.94, -r * 0.10, r * 0.82, brow_y);
    inner_r.close();
    inner_r.paint(rgba(0xCBD5E1DD), window);

    // 2) 额前微露针织透气小白内衬帽边 (Taqiyah)
    let mut taq = c.fill();
    taq.move_to(-r * 0.76, brow_y + r * 0.07);
    taq.quad_to(-r * 0.10, brow_y - r * 0.11, r * 0.66, brow_y + r * 0.05);
    taq.quad_to(0.0, brow_y - r * 0.01, -r * 0.76, brow_y + r * 0.07);
    taq.close();
    taq.paint(rgba(0xE2E8F0FF), window);

    // 3) 左侧潇洒宽展纯白大翼 + 右侧优雅垂坠卷折 + 额前自然微斜纯白折檐
    let mut left_w = c.fill();
    left_w.move_to(-agal_w * 0.88, agal_y + r * 0.08);
    left_w.cubic_to(-r * 1.20, agal_y + r * 0.10, -r * 1.36, agal_y + r * 0.16, -r * 1.38 + sway * 0.5, brow_y + r * 0.14);
    left_w.cubic_to(-r * 1.40 + sway * 0.8, r * 0.18, -r * 1.30 + sway, r * 0.54, -r * 1.18 + sway, r * 0.80);
    left_w.quad_to(-r * 1.06 + sway * 0.5, r * 0.80, -r * 1.02 + sway * 0.3, r * 0.70);
    left_w.cubic_to(-r * 1.14 + sway * 0.4, r * 0.32, -r * 1.16, -r * 0.04, -r * 0.84, brow_y - r * 0.01);
    left_w.close();
    left_w.paint(rgba(0xF8FAFCFF), window);

    let mut right_w = c.fill();
    right_w.move_to(agal_w * 0.90, agal_y + r * 0.06);
    right_w.cubic_to(r * 1.14, agal_y + r * 0.10, r * 1.28 - sway * 0.4, brow_y + r * 0.10, r * 1.28 - sway * 0.6, r * 0.12);
    right_w.cubic_to(r * 1.28 - sway * 0.8, r * 0.42, r * 1.18 - sway, r * 0.64, r * 1.10 - sway, r * 0.78);
    right_w.quad_to(r * 0.98 - sway * 0.5, r * 0.80, r * 0.94 - sway * 0.3, r * 0.70);
    right_w.cubic_to(r * 1.08 - sway * 0.3, r * 0.30, r * 1.08, -r * 0.04, r * 0.80, brow_y + r * 0.02);
    right_w.close();
    right_w.paint(rgba(0xFFFFFFFF), window);

    let mut brow = c.fill();
    brow.move_to(-r * 1.18, brow_y - r * 0.04);
    brow.quad_to(-r * 0.52, agal_y + r * 0.04, agal_w * 0.94, agal_y + r * 0.06);
    brow.line_to(r * 1.10, brow_y + r * 0.04);
    brow.cubic_to(r * 0.58, brow_y - r * 0.02, r * 0.14, brow_y - r * 0.15, -r * 0.24, brow_y - r * 0.11);
    brow.quad_to(-r * 0.70, brow_y - r * 0.13, -r * 1.18, brow_y - r * 0.04);
    brow.close();
    brow.paint(rgba(0xFFFFFFFF), window);

    // 4) 3D 环抱双圈纯黑 Agal 后半环 + 高圆纯白布冠 + 下弧微笑曲线前半双黑环
    let ag = c.sub(0.0, agal_y, 0.04);
    for (dy, rw, thick) in [(r * 0.015, agal_w * 1.02, r * 0.11), (-r * 0.082, agal_w * 0.96, r * 0.10)] {
        ag.stroke_quad(-rw, dy, 0.0, dy - r * 0.20, rw, dy, thick.max(1.8), rgba(0x090A0DFF), window);
    }

    let dome_w = agal_w * 0.84;
    let dome_top = peak_y - agal_y;
    let mut dome = ag.fill();
    dome.move_to(-dome_w, r * 0.02);
    dome.cubic_to(-dome_w * 0.96, dome_top + r * 0.04, -dome_w * 0.42, dome_top, 0.0, dome_top);
    dome.cubic_to(dome_w * 0.42, dome_top, dome_w * 0.96, dome_top + r * 0.04, dome_w, r * 0.02);
    dome.quad_to(0.0, r * 0.14, -dome_w, r * 0.02);
    dome.close();
    dome.paint(rgba(0xF8FAFCFF), window);

    // 前半双黑环（下弧微笑曲线环抱白冠）
    for (dy, rw, dip, thick) in [
        (r * 0.022, agal_w * 1.02, r * 0.15, r * 0.115),
        (-r * 0.080, agal_w * 0.96, r * 0.14, r * 0.108),
    ] {
        ag.stroke_quad(-rw, dy, 0.0, dy + dip, rw, dy, thick.max(1.9), rgba(0x111318FF), window);
        ag.stroke_quad(-rw * 0.90, dy - r * 0.020, 0.0, dy + dip - r * 0.022, rw * 0.90, dy - r * 0.020, (thick * 0.30).max(0.55), rgba(0x94A3B848), window);
    }
    ag.stroke_quad(-agal_w * 0.94, -r * 0.026, 0.0, r * 0.118, agal_w * 0.94, -r * 0.026, (r * 0.026).max(0.5), rgba(0x030406FF), window);
}

/// 29. 沙漠特种兵沙色贝雷帽 (TanBeret - 右挺左垂大斜折 + 油亮黑皮包边)
fn draw_tan_beret(c: &HeadCanvas, window: &mut Window) {
    let r = c.r;
    let head_top = -c.head_r;
    let base_y = head_top + r * 0.28;
    let top_y = head_top - r * 0.34;

    // 1) 右前额油亮黑皮包边 + 皮革反光脊线
    c.stroke_quad(-r * 0.56, base_y + r * 0.02, r * 0.24, base_y - r * 0.17, r * 0.90, base_y + r * 0.03, (r * 0.095).max(1.6), rgba(0x09090BFF), window);
    c.stroke_quad(-r * 0.34, base_y - r * 0.04, r * 0.24, base_y - r * 0.185, r * 0.80, base_y - r * 0.01, (r * 0.024).max(0.45), rgba(0xE2E8F077), window);

    // 2) 右侧挺括直立沙色毛呢前立面
    let mut right_p = c.fill();
    right_p.move_to(-r * 0.20, base_y - r * 0.08);
    right_p.cubic_to(r * 0.08, top_y + r * 0.04, r * 0.56, top_y - r * 0.02, r * 0.78, top_y + r * 0.06);
    right_p.cubic_to(r * 0.98, top_y + r * 0.16, r * 0.98, base_y - r * 0.18, r * 0.88, base_y - r * 0.01);
    right_p.quad_to(r * 0.34, base_y - r * 0.19, -r * 0.20, base_y - r * 0.08);
    right_p.close();
    right_p.paint(rgba(0x9A774CFF), window);

    // 3) 自右上峰向左侧大幅潇洒斜拉下垂的沙色羊毛主帽冠
    let mut crown = c.fill();
    crown.move_to(r * 0.74, top_y + r * 0.05);
    crown.cubic_to(r * 0.44, top_y - r * 0.10, -r * 0.26, top_y - r * 0.08, -r * 0.70, top_y + r * 0.12);
    crown.cubic_to(-r * 1.04, top_y + r * 0.28, -r * 1.22, base_y - r * 0.08, -r * 1.18, base_y + r * 0.14);
    crown.cubic_to(-r * 1.14, base_y + r * 0.28, -r * 0.86, base_y + r * 0.22, -r * 0.42, base_y - r * 0.01);
    crown.cubic_to(-r * 0.06, base_y - r * 0.16, r * 0.36, top_y + r * 0.28, r * 0.74, top_y + r * 0.05);
    crown.close();
    crown.paint(rgba(0xC29F74FF), window);

    // 标志性斜折脊深影与受光高光
    c.stroke_cubic(r * 0.68, top_y + r * 0.08, r * 0.34, top_y + r * 0.30, -r * 0.04, base_y - r * 0.14, -r * 0.38, base_y - r * 0.01, (r * 0.044).max(0.8), rgba(0x442E1872), window);
    c.stroke_cubic(r * 0.56, top_y + r * 0.02, r * 0.14, top_y + r * 0.22, -r * 0.32, base_y - r * 0.18, -r * 0.90, base_y + r * 0.06, (r * 0.028).max(0.55), rgba(0xF5E2C666), window);
}

/// 30. 复古英伦报童帽 (Newsboy)
fn draw_newsboy(c: &HeadCanvas, window: &mut Window) {
    let r = c.r;
    let head_top = -c.head_r;
    let hat = c.sub(0.0, head_top + r * 0.28, -0.05);
    let crown_w = r * 1.10;
    let band_w = r * 0.86;
    let top_y = -r * 0.42;

    // 1) 短巧微弧鸭舌前帽檐
    let mut visor = hat.fill();
    visor.move_to(-band_w * 0.88, -r * 0.01);
    visor.quad_to(0.0, -r * 0.11, band_w * 0.88, -r * 0.01);
    visor.quad_to(r * 0.52, r * 0.18, 0.0, r * 0.16);
    visor.quad_to(-r * 0.52, r * 0.18, -band_w * 0.88, -r * 0.01);
    visor.close();
    visor.paint(rgba(0x1E293BFF), window);
    hat.stroke_quad(-band_w * 0.70, r * 0.03, 0.0, r * 0.13, band_w * 0.70, r * 0.03, (r * 0.018).max(0.45), rgba(0xCBD5E14D), window);

    // 2) 煤灰粗花呢八片蓬松帽冠
    let mut crown = hat.fill();
    crown.move_to(-band_w * 0.90, -r * 0.01);
    crown.cubic_to(-crown_w * 1.10, -r * 0.04, -crown_w * 1.04, top_y + r * 0.08, -crown_w * 0.48, top_y + r * 0.01);
    crown.quad_to(0.0, top_y - r * 0.08, crown_w * 0.48, top_y + r * 0.01);
    crown.cubic_to(crown_w * 1.04, top_y + r * 0.08, crown_w * 1.10, -r * 0.04, band_w * 0.90, -r * 0.01);
    crown.quad_to(0.0, r * 0.06, -band_w * 0.90, -r * 0.01);
    crown.close();
    crown.paint(rgba(0x475569FF), window);

    // 放射拼缝弧线 + 顶扣 + 复古古铜小圆铆徽
    let btn_y = top_y - r * 0.015;
    for (cx_p, ex_p, ey_p) in [
        (-crown_w * 0.82, -band_w * 0.82, -r * 0.01),
        (-crown_w * 0.44, -band_w * 0.42, r * 0.025),
        (0.0, 0.0, r * 0.035),
        (crown_w * 0.44, band_w * 0.42, r * 0.025),
        (crown_w * 0.82, band_w * 0.82, -r * 0.01),
    ] {
        hat.stroke_quad(0.0, btn_y, cx_p, top_y + r * 0.17, ex_p, ey_p, (r * 0.022).max(0.5), rgba(0x0F172A99), window);
    }
    hat.fill_ellipse(0.0, btn_y, r * 0.10, r * 0.052, 0.0, rgba(0x334155FF), window);
    hat.stroke_ellipse(0.0, btn_y, r * 0.10, r * 0.052, 0.0, (r * 0.022).max(0.45), rgba(0x0F172AFF), window);
    hat.fill_circle(-band_w * 0.62, -r * 0.055, r * 0.038, rgba(0xD97706FF), window);
}

/// 31. 远洋白顶船长帽 (Captain)
fn draw_captain(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let head_top = -c.head_r;
    let cap = c.sub(0.0, head_top + r * 0.22, -0.04);
    let cw = r * 0.96;
    let top_y = -r * 0.46;

    // 1) 雪白挺括远洋船长高帽冠
    let mut crown = cap.fill();
    crown.move_to(-cw * 0.78, -r * 0.02);
    crown.cubic_to(-cw * 1.12, -r * 0.25, -cw * 0.98, top_y - r * 0.02, -cw * 0.44, top_y);
    crown.quad_to(0.0, top_y - r * 0.14, cw * 0.44, top_y);
    crown.cubic_to(cw * 0.98, top_y - r * 0.02, cw * 1.12, -r * 0.25, cw * 0.78, -r * 0.02);
    crown.quad_to(0.0, -r * 0.14, -cw * 0.78, -r * 0.02);
    crown.close();
    crown.paint(rgba(0xF8FAFCFF), window);

    let mut crim = cap.stroke((r * 0.026).max(0.55));
    crim.move_to(-cw * 0.78, -r * 0.02);
    crim.cubic_to(-cw * 1.12, -r * 0.25, -cw * 0.98, top_y - r * 0.02, -cw * 0.44, top_y);
    crim.quad_to(0.0, top_y - r * 0.14, cw * 0.44, top_y);
    crim.cubic_to(cw * 0.98, top_y - r * 0.02, cw * 1.12, -r * 0.25, cw * 0.78, -r * 0.02);
    crim.paint(rgba(0x94A3B8FF), window);

    // 2) 深海蓝帽墙 + 黑漆皮金穗帽檐 + 鎏金绞绳
    let mut wall = cap.fill();
    wall.move_to(-cw * 0.78, r * 0.05);
    wall.line_to(-cw * 0.76, -r * 0.12);
    wall.quad_to(0.0, -r * 0.21, cw * 0.76, -r * 0.12);
    wall.line_to(cw * 0.78, r * 0.05);
    wall.quad_to(0.0, -r * 0.05, -cw * 0.78, r * 0.05);
    wall.close();
    wall.paint(rgba(0x0F172AFF), window);

    let mut visor = cap.fill();
    visor.move_to(-cw * 0.80, r * 0.03);
    visor.quad_to(0.0, -r * 0.06, cw * 0.80, r * 0.03);
    visor.quad_to(cw * 0.62, r * 0.22, 0.0, r * 0.17);
    visor.quad_to(-cw * 0.62, r * 0.22, -cw * 0.80, r * 0.03);
    visor.close();
    visor.paint(rgba(0x090D16FF), window);

    for side in [-1.0_f32, 1.0] {
        cap.stroke_quad(side * cw * 0.15, r * 0.125, side * cw * 0.42, r * 0.145, side * cw * 0.60, r * 0.08, (r * 0.028).max(0.55), rgba(0xFACC15FF), window);
    }
    cap.stroke_quad(-cw * 0.76, r * 0.015, 0.0, -r * 0.085, cw * 0.76, r * 0.015, (r * 0.044).max(0.8), rgba(0xF59E0BFF), window);
    for side in [-1.0_f32, 1.0] {
        cap.fill_circle(side * cw * 0.76, r * 0.015, r * 0.046, rgba(0xFDE047FF), window);
    }

    // 3) 深蓝金边船锚徽章
    let ey = -r * 0.24;
    let glint = 0.75 + 0.25 * (time_sec * 3.4).sin();
    cap.fill_ellipse(0.0, ey, r * 0.21, r * 0.165, 0.0, rgba(0x0F172AFF), window);
    cap.stroke_ellipse(0.0, ey, r * 0.21, r * 0.165, 0.0, (r * 0.026).max(0.55), rgba_alpha(0xFACC15, glint), window);
    cap.stroke_line(0.0, ey - r * 0.08, 0.0, ey + r * 0.085, (r * 0.030).max(0.6), rgba(0xFEF08AFF), window);
    cap.stroke_line(-r * 0.068, ey - r * 0.032, r * 0.068, ey - r * 0.032, (r * 0.028).max(0.55), rgba(0xFEF08AFF), window);
    cap.stroke_quad(-r * 0.075, ey + r * 0.04, 0.0, ey + r * 0.115, r * 0.075, ey + r * 0.04, (r * 0.028).max(0.55), rgba(0xFEF08AFF), window);
}

/// 32. 街头嘻哈平檐帽 (Hiphop - 水泥灰高挺帽冠 + 3D 立体蔚来 NIO Logo + 碳黑冰薄荷底檐 + 镭射冷银贴纸)
fn draw_hiphop(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let head_top = -c.head_r;
    let cap = c.sub(-r * 0.04, head_top + r * 0.28, -0.13);
    let cw = r * 0.86;
    let top_y = -r * 0.52;

    // 1) 曜石黑顶扣 + 水泥灰高挺方圆帽冠
    cap.fill_ellipse(0.0, top_y - r * 0.012, r * 0.08, r * 0.038, 0.0, rgba(0x18181BFF), window);
    let mut crown = cap.fill();
    crown.move_to(-cw, r * 0.02);
    crown.cubic_to(-cw * 1.02, top_y + r * 0.10, -cw * 0.86, top_y - r * 0.02, 0.0, top_y);
    crown.cubic_to(cw * 0.86, top_y - r * 0.02, cw * 1.02, top_y + r * 0.10, cw, r * 0.02);
    crown.close();
    crown.paint(rgba(0xCBD5E1FF), window);

    let mut crim = cap.stroke((r * 0.024).max(0.5));
    crim.move_to(-cw, r * 0.02);
    crim.cubic_to(-cw * 1.02, top_y + r * 0.10, -cw * 0.86, top_y - r * 0.02, 0.0, top_y);
    crim.cubic_to(cw * 0.86, top_y - r * 0.02, cw * 1.02, top_y + r * 0.10, cw, r * 0.02);
    crim.paint(rgba(0x334155FF), window);

    // 2) 帽冠正前 3D 立体刺绣蔚来 NIO Logo（对齐官方像素比例：厚实半圆拱 + 圆滑内角 + 平顶斜肩外圆弧包边）
    let by = top_y + r * 0.29;
    let lr = r * 0.175;
    let pulse = 0.78 + 0.22 * (time_sec * 3.4).sin();
    let draw_nio = |ox: f32, oy: f32, col: Rgba, window: &mut Window| {
        let cx0 = ox;
        let cy0 = by + oy;
        // Sky 上半圆拱
        let mut sky = cap.fill();
        let steps = 16;
        for i in 0..=steps {
            let a = PI - (i as f32 / steps as f32) * PI;
            let x = cx0 + a.cos() * lr;
            let y = cy0 - a.sin() * lr;
            if i == 0 {
                sky.move_to(x, y);
            } else {
                sky.line_to(x, y);
            }
        }
        sky.line_to(cx0 + lr * 0.56, cy0);
        sky.quad_to(cx0 + lr * 0.455, cy0, cx0 + lr * 0.448, cy0 - lr * 0.085);
        for i in 0..=steps {
            let a = 0.185 + (i as f32 / steps as f32) * (PI - 0.370);
            sky.line_to(cx0 + a.cos() * lr * 0.455, cy0 - a.sin() * lr * 0.455);
        }
        sky.quad_to(cx0 - lr * 0.455, cy0, cx0 - lr * 0.56, cy0);
        sky.close();
        sky.paint(col, window);

        // Earth 下半折角（平顶梯肩 + 同心外圆弧包边 + 柔和内尖）
        let mut earth = cap.fill();
        let outer_a = 0.435_f32;
        let foot_a = 0.965_f32;
        earth.move_to(cx0 - lr * 0.37, cy0 + lr * 0.005);
        earth.line_to(cx0 + lr * 0.37, cy0 + lr * 0.005);
        earth.quad_to(cx0 + lr * 0.45, cy0 + lr * 0.005, cx0 + lr * 0.50, cy0 + lr * 0.055);
        for i in 0..=6 {
            let a = outer_a + (foot_a - outer_a) * (i as f32 / 6.0);
            earth.line_to(cx0 + a.cos() * lr, cy0 + a.sin() * lr);
        }
        earth.quad_to(cx0 + lr * 0.51, cy0 + lr * 0.855, cx0 + lr * 0.445, cy0 + lr * 0.775);
        earth.line_to(cx0 + lr * 0.055, cy0 + lr * 0.365);
        earth.quad_to(cx0, cy0 + lr * 0.310, cx0 - lr * 0.055, cy0 + lr * 0.365);
        earth.line_to(cx0 - lr * 0.445, cy0 + lr * 0.775);
        earth.quad_to(
            cx0 - lr * 0.51,
            cy0 + lr * 0.855,
            cx0 - foot_a.cos() * lr,
            cy0 + foot_a.sin() * lr,
        );
        for i in 1..=6 {
            let a = (PI - foot_a) + (foot_a - outer_a) * (i as f32 / 6.0);
            earth.line_to(cx0 + a.cos() * lr, cy0 + a.sin() * lr);
        }
        earth.line_to(cx0 - lr * 0.50, cy0 + lr * 0.055);
        earth.quad_to(cx0 - lr * 0.45, cy0 + lr * 0.005, cx0 - lr * 0.37, cy0 + lr * 0.005);
        earth.close();
        earth.paint(col, window);
    };
    draw_nio(r * 0.015, r * 0.015, rgba_alpha(0x2DD4BF, 0.88 * pulse), window);
    draw_nio(0.0, 0.0, rgba(0x0F172AFF), window);

    // 3) 碳黑哑光宽平帽檐 + 冰薄荷青撞色底檐
    let bill_l = -cw * 1.02;
    let bill_r = cw * 1.32;
    let bill_y = r * 0.00;
    let bill_h = r * 0.135;
    let bill_w = bill_r - bill_l;
    let bill_cx = (bill_l + bill_r) * 0.5;
    // 冰薄荷底檐
    cap.fill_pill(bill_cx, bill_y + bill_h * 0.72, bill_w, bill_h * 0.82, rgba(0x14B8A6FF), window);
    // 碳黑顶檐
    cap.fill_pill(bill_cx, bill_y + bill_h * 0.42, bill_w, bill_h * 0.82, rgba(0x18181BFF), window);
    cap.stroke_pill(bill_cx, bill_y + bill_h * 0.42, bill_w, bill_h * 0.82, (r * 0.022).max(0.45), rgba(0x3F3F46FF), window);

    // 4) 镭射冷银箔全息圆贴纸 + 动态冷白星芒
    let sx = bill_r - r * 0.32;
    let sy = bill_y + bill_h * 0.44;
    cap.fill_ellipse(sx, sy, r * 0.11, r * 0.058, 0.0, rgba(0xE0F2FEFF), window);
    cap.stroke_ellipse(sx, sy, r * 0.11, r * 0.058, 0.0, (r * 0.018).max(0.45), rgba(0x475569FF), window);
    let tw = 0.55 + 0.45 * (time_sec * 4.6).sin();
    cap.fill_star4(sx + r * 0.065, sy - r * 0.038, r * 0.064 * tw, rgba_alpha(0xFFFFFF, 0.60 + 0.40 * tw), window);
}

/// 33. 绅士巴拿马帽 (Panama)
fn draw_panama(c: &HeadCanvas, window: &mut Window) {
    let r = c.r;
    let head_top = -c.head_r;
    let hat = c.sub(0.0, head_top + r * 0.22, -0.07);
    let brim_w = r * 1.30;
    let cw = r * 0.72;
    let top_y = -r * 0.46;

    // 1) 象牙米白前压后翘优雅帽檐
    let mut brim = hat.fill();
    brim.move_to(-brim_w, r * 0.04);
    brim.cubic_to(-brim_w * 0.66, -r * 0.14, brim_w * 0.66, -r * 0.14, brim_w, r * 0.00);
    brim.cubic_to(brim_w * 0.72, r * 0.20, -brim_w * 0.64, r * 0.22, -brim_w, r * 0.04);
    brim.close();
    brim.paint(rgba(0xF5E8CEFF), window);

    let mut brim_rim = hat.stroke((r * 0.026).max(0.55));
    brim_rim.move_to(-brim_w, r * 0.04);
    brim_rim.cubic_to(-brim_w * 0.66, -r * 0.14, brim_w * 0.66, -r * 0.14, brim_w, r * 0.00);
    brim_rim.cubic_to(brim_w * 0.72, r * 0.20, -brim_w * 0.64, r * 0.22, -brim_w, r * 0.04);
    brim_rim.close();
    brim_rim.paint(rgba(0x9C835CFF), window);

    // 2) 中凹双峰象牙米白捏顶帽冠
    let mut crown = hat.fill();
    crown.move_to(-cw, r * 0.03);
    crown.cubic_to(-cw * 0.94, top_y + r * 0.10, -cw * 0.64, top_y - r * 0.04, -cw * 0.34, top_y);
    crown.quad_to(0.0, top_y + r * 0.10, cw * 0.34, top_y);
    crown.cubic_to(cw * 0.64, top_y - r * 0.04, cw * 0.94, top_y + r * 0.10, cw, r * 0.03);
    crown.quad_to(0.0, r * 0.13, -cw, r * 0.03);
    crown.close();
    crown.paint(rgba(0xFFFDF5FF), window);

    let mut crim = hat.stroke((r * 0.026).max(0.55));
    crim.move_to(-cw, r * 0.03);
    crim.cubic_to(-cw * 0.94, top_y + r * 0.10, -cw * 0.64, top_y - r * 0.04, -cw * 0.34, top_y);
    crim.quad_to(0.0, top_y + r * 0.10, cw * 0.34, top_y);
    crim.cubic_to(cw * 0.64, top_y - r * 0.04, cw * 0.94, top_y + r * 0.10, cw, r * 0.03);
    crim.paint(rgba(0x9C835CFF), window);

    // 两侧前捏阴影
    for side in [-1.0_f32, 1.0] {
        hat.fill_ellipse(side * cw * 0.40, top_y + r * 0.17, r * 0.12, r * 0.068, side * -0.45, rgba(0x9C835C38), window);
    }

    // 3) 经典宽幅玄黑罗纹缎带 + 左侧平折蝴蝶结
    let mut band = hat.fill();
    band.move_to(-cw * 1.005, r * 0.035);
    band.line_to(-cw * 0.96, -r * 0.115);
    band.quad_to(0.0, -r * 0.01, cw * 0.96, -r * 0.115);
    band.line_to(cw * 1.005, r * 0.035);
    band.quad_to(0.0, r * 0.14, -cw * 1.005, r * 0.035);
    band.close();
    band.paint(rgba(0x18181BFF), window);

    hat.fill_ellipse(-cw * 0.56, -r * 0.025, r * 0.12, r * 0.072, 0.0, rgba(0x27272AFF), window);
}

/// 34. 绅士礼服领结 (Bowtie - 洁白燕尾服翼领 + 双层玄黑真丝缎面经典蝴蝶结)
fn draw_bowtie(c: &HeadCanvas, window: &mut Window) {
    let r = c.r;
    let neck = c.sub(0.0, c.head_r - r * 0.02, 0.0);
    let wing_w = r * 0.31;
    let wing_h = r * 0.165;

    // 1) 硬挺洁白燕尾服衬衫小翼领
    for side in [-1.0_f32, 1.0] {
        let mut collar = neck.fill();
        collar.move_to(side * r * 0.03, -r * 0.04);
        collar.line_to(side * r * 0.25, -r * 0.12);
        collar.line_to(side * r * 0.29, r * 0.01);
        collar.line_to(side * r * 0.08, r * 0.065);
        collar.close();
        collar.paint(rgba(0xF8FAFCFF), window);

        let mut crim = neck.stroke((r * 0.020).max(0.45));
        crim.move_to(side * r * 0.03, -r * 0.04);
        crim.line_to(side * r * 0.25, -r * 0.12);
        crim.line_to(side * r * 0.29, r * 0.01);
        crim.line_to(side * r * 0.08, r * 0.065);
        crim.close();
        crim.paint(rgba(0x64748BFF), window);
    }

    // 2) 双层手打领结后折翼 + 玄黑真丝缎面前翼
    for side in [-1.0_f32, 1.0] {
        let mut back = neck.fill();
        back.move_to(side * r * 0.04, -r * 0.01);
        back.quad_to(side * wing_w * 0.62, -wing_h * 1.06, side * wing_w * 1.05, -wing_h * 0.90);
        back.quad_to(side * wing_w * 1.09, 0.0, side * wing_w * 1.04, wing_h * 0.92);
        back.quad_to(side * wing_w * 0.62, wing_h * 1.06, side * r * 0.04, r * 0.02);
        back.close();
        back.paint(rgba(0x09090BFF), window);

        let mut front = neck.fill();
        front.move_to(side * r * 0.02, -r * 0.036);
        front.line_to(side * wing_w * 0.92, -wing_h * 0.92);
        front.quad_to(side * wing_w * 1.02, -wing_h * 0.45, side * wing_w * 0.98, 0.0);
        front.quad_to(side * wing_w * 1.02, wing_h * 0.45, side * wing_w * 0.92, wing_h * 0.92);
        front.line_to(side * r * 0.02, r * 0.036);
        front.close();
        front.paint(rgba(0x27272AFF), window);

        let mut rim = neck.stroke((r * 0.022).max(0.45));
        rim.move_to(side * r * 0.02, -r * 0.036);
        rim.line_to(side * wing_w * 0.92, -wing_h * 0.92);
        rim.quad_to(side * wing_w * 1.02, -wing_h * 0.45, side * wing_w * 0.98, 0.0);
        rim.quad_to(side * wing_w * 1.02, wing_h * 0.45, side * wing_w * 0.92, wing_h * 0.92);
        rim.line_to(side * r * 0.02, r * 0.036);
        rim.close();
        rim.paint(rgba(0x52525BFF), window);

        neck.stroke_quad(side * r * 0.06, -r * 0.025, side * wing_w * 0.50, -wing_h * 0.42, side * wing_w * 0.82, -wing_h * 0.54, (r * 0.016).max(0.45), rgba(0xFFFFFF38), window);
    }

    // 3) 中央窄腰真丝结芯
    let kw = r * 0.054;
    let kh = r * 0.068;
    neck.fill_pill(0.0, 0.0, kw * 2.0, kh * 2.0, rgba(0x18181BFF), window);
    neck.stroke_pill(0.0, 0.0, kw * 2.0, kh * 2.0, (r * 0.020).max(0.45), rgba(0x71717AFF), window);
}

/// 35. 暖冬针织围巾 (WinterScarf - 包裹球底颈圈的红色暖绒围巾 + 奶油条纹与微飘流苏尾)
fn draw_winter_scarf(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let sway = (time_sec * 2.5).sin() * r * 0.036;
    let sway2 = (time_sec * 2.5 - 0.9).sin() * r * 0.052;
    let collar_y = c.head_r * 0.78;
    let collar_w = c.head_r * 0.86;

    // 1) 右下垂飘逸双围巾尾（带奶油白撞色横条纹与流苏）
    let draw_tail = |ox: f32, oy: f32, w: f32, h: f32, tilt_a: f32, sw: f32, col: u32, window: &mut Window| {
        let t = c.sub(ox, oy, tilt_a);
        let mut body = t.fill();
        body.move_to(-w * 0.46, 0.0);
        body.quad_to(-w * 0.50 + sw * 0.5, h * 0.55, -w * 0.52 + sw, h);
        body.line_to(w * 0.52 + sw, h);
        body.quad_to(w * 0.50 + sw * 0.5, h * 0.55, w * 0.46, 0.0);
        body.close();
        body.paint(rgba(col), window);

        // 奶油撞色横条
        let mut stripe = t.fill();
        stripe.move_to(-w * 0.48 + sw * 0.68, h * 0.64);
        stripe.line_to(w * 0.48 + sw * 0.68, h * 0.64);
        stripe.line_to(w * 0.49 + sw * 0.80, h * 0.78);
        stripe.line_to(-w * 0.49 + sw * 0.80, h * 0.78);
        stripe.close();
        stripe.paint(rgba(0xFEF3C7FF), window);

        // 奶油绒线流苏
        for i in -1..=1 {
            let fx = (i as f32) * w * 0.30 + sw;
            t.stroke_line(fx, h, fx + sw * 0.2, h + r * 0.075, (r * 0.022).max(0.45), rgba(0xFDE68AFF), window);
        }
    };
    draw_tail(r * 0.42, collar_y + r * 0.05, r * 0.22, r * 0.36, -0.16, sway2, 0xB91C1CFF, window);
    draw_tail(r * 0.30, collar_y + r * 0.07, r * 0.25, r * 0.42, -0.05, sway, 0xDC2626FF, window);

    // 2) 包裹球体下缘的加厚蓬松围巾领圈
    let mut wrap = c.fill();
    wrap.move_to(-collar_w, collar_y - r * 0.14);
    wrap.quad_to(0.0, collar_y + r * 0.21, collar_w, collar_y - r * 0.14);
    wrap.cubic_to(collar_w * 1.05, collar_y + r * 0.05, collar_w * 0.82, collar_y + r * 0.33, 0.0, collar_y + r * 0.35);
    wrap.cubic_to(-collar_w * 0.82, collar_y + r * 0.33, -collar_w * 1.05, collar_y + r * 0.05, -collar_w, collar_y - r * 0.14);
    wrap.close();
    wrap.paint(rgba(0xDC2626FF), window);

    let mut wrim = c.stroke((r * 0.026).max(0.55));
    wrim.move_to(-collar_w, collar_y - r * 0.14);
    wrim.quad_to(0.0, collar_y + r * 0.21, collar_w, collar_y - r * 0.14);
    wrim.cubic_to(collar_w * 1.05, collar_y + r * 0.05, collar_w * 0.82, collar_y + r * 0.33, 0.0, collar_y + r * 0.35);
    wrim.cubic_to(-collar_w * 0.82, collar_y + r * 0.33, -collar_w * 1.05, collar_y + r * 0.05, -collar_w, collar_y - r * 0.14);
    wrim.close();
    wrim.paint(rgba(0x450A0AFF), window);

    // 罗纹针织斜纹 + 右侧围巾结
    for i in -4..=4 {
        let f = (i as f32) / 5.0;
        let rx0 = f * collar_w * 0.82;
        let ry0 = collar_y + (1.0 - f * f) * r * 0.14;
        c.stroke_line(rx0 - r * 0.035, ry0 - r * 0.02, rx0 + r * 0.035, ry0 + r * 0.13, (r * 0.020).max(0.45), rgba(0xFEF3C744), window);
    }
    c.fill_ellipse(r * 0.34, collar_y + r * 0.12, r * 0.165, r * 0.125, -0.18, rgba(0xEF4444FF), window);
    c.stroke_ellipse(r * 0.34, collar_y + r * 0.12, r * 0.165, r * 0.125, -0.18, (r * 0.022).max(0.45), rgba(0x450A0AFF), window);
}

/// 36. 行星轨道光环 (OrbitRing - 前后穿插球体的清透土星多带环系 + 卡西尼环缝 + 公转牧羊卫星)
fn draw_orbit_ring_half(
    c: &HeadCanvas,
    time_sec: f32,
    glow_color: Rgba,
    pitch: f32,
    is_back: bool,
    window: &mut Window,
) {
    let r = c.r;
    let ring_tilt = -0.34 + (time_sec * 1.1).sin() * 0.02;
    let aspect = (0.36 + pitch * 0.22).clamp(0.22, 0.50) / 1.48;
    let ring = c.sub(0.0, r * 0.10, ring_tilt);

    let (start_a, end_a) = if is_back { (PI, 2.0 * PI) } else { (0.0, PI) };
    let layer_alpha = if is_back { 0.56_f32 } else { 1.0_f32 };

    // 多带同心半透薄纱环（内 C 环 -> 主香槟金 B 环 -> 卡西尼环缝 -> 外银蓝 A 环）
    let bands: [(f32, f32, u32, f32); 5] = [
        (1.23, 0.055, 0xD6CEC2, 0.16),
        (1.31, 0.088, 0xEED6AA, 0.30),
        (1.345, 0.018, 0xFAF0DC, 0.34),
        (1.425, 0.060, 0xD8E4F2, 0.26),
        (1.455, 0.014, 0xF8FAFC, 0.36),
    ];
    for (rx_scale, w_scale, rgb, alpha) in bands {
        let rx = r * rx_scale;
        let ry = rx * aspect;
        ring.stroke_arc(
            0.0,
            0.0,
            rx,
            ry,
            start_a,
            end_a,
            (r * w_scale).max(0.55),
            rgba_alpha(rgb, alpha * layer_alpha),
            window,
        );
    }

    // 贴合 A 环内侧的主题眼神色微光丝线
    ring.stroke_arc(
        0.0,
        0.0,
        r * 1.40,
        r * 1.40 * aspect,
        start_a,
        end_a,
        (r * 0.014).max(0.45),
        color_with_alpha(glow_color, if is_back { 0.14 } else { 0.28 }),
        window,
    );

    // 外环公转微光牧羊卫星（按前后半圈自然受球体遮挡）
    let moon_a = time_sec * 1.05 + 0.65;
    let sin_a = moon_a.sin();
    let in_layer = if is_back { sin_a < 0.0 } else { sin_a >= 0.0 };
    if in_layer {
        let moon_rx = r * 1.46;
        let moon_ry = moon_rx * aspect;
        let mx = moon_a.cos() * moon_rx;
        let my = sin_a * moon_ry;
        let mr = (r * 0.042 * (0.86 + 0.22 * (sin_a + 1.0) * 0.5)).max(0.95);
        ring.fill_circle(mx, my, mr * 1.6, color_with_alpha(glow_color, if is_back { 0.20 } else { 0.38 }), window);
        ring.fill_circle(mx, my, mr, rgba_alpha(0xF8FAFC, if is_back { 0.55 } else { 0.92 }), window);
        ring.fill_star4(mx, my, mr * 1.6, rgba_alpha(0xFFFFFF, if is_back { 0.50 } else { 0.95 }), window);
    }
}

/// 37. 伴飞迷你无人机 (MiniDrone - 绕 NOMI 上方前后景深巡航伴飞的双旋翼机甲小无人机 + 联动发光双眸)
fn draw_mini_drone(
    c: &HeadCanvas,
    time_sec: f32,
    glow_color: Rgba,
    is_back: bool,
    window: &mut Window,
) {
    let orbit_a = time_sec * 1.15 + 0.45;
    let sin_a = orbit_a.sin();
    let cos_a = orbit_a.cos();
    let in_front = sin_a >= 0.0;
    if is_back == in_front {
        return;
    }

    let r = c.r;
    // 世界水平基准画布（保持伴飞无人机自然水平悬停，仅叠加巡航转弯压倾）
    let level = HeadCanvas {
        ux: 1.0,
        uy: 0.0,
        vx: 0.0,
        vy: 1.0,
        ..*c
    };
    let fly_dir = -sin_a;
    let hover_y = (time_sec * 3.4).sin() * r * 0.04;
    let drone_tilt = fly_dir * 0.16 + (time_sec * 2.7).sin() * 0.04;
    let base_x = cos_a * r * 1.12;
    let base_y = -r * 0.96 + sin_a * r * 0.16 + hover_y;
    let depth_s = 1.0 + 0.14 * sin_a;
    let s = r * 0.26 * depth_s;
    let alpha = if in_front { 1.0_f32 } else { 0.84_f32 };

    let d = level.sub(base_x, base_y, drone_tilt);

    // 1) 左右碳钛双悬臂 + 高速旋转双旋翼
    let arm_span = s * 0.96;
    let arm_y = -s * 0.18;
    d.stroke_line(-s * 0.42, -s * 0.04, -arm_span, arm_y, (s * 0.14).max(0.65), rgba_alpha(0x64748B, alpha), window);
    d.stroke_line(s * 0.42, -s * 0.04, arm_span, arm_y, (s * 0.14).max(0.65), rgba_alpha(0x64748B, alpha), window);

    for side in [-1.0_f32, 1.0] {
        let rx_p = side * arm_span;
        let ry_p = arm_y - s * 0.08;
        d.fill_circle(rx_p, ry_p + s * 0.06, s * 0.08, rgba_alpha(0x334155, alpha), window);

        // 旋翼半透明残影圆盘 + 高速动态桨叶
        let rotor_r = s * 0.44;
        d.fill_ellipse(rx_p, ry_p, rotor_r, s * 0.11, 0.0, rgba_alpha(0x38BDF8, 0.30 * alpha), window);
        let blade_w = (time_sec * 26.0 * side).cos().abs().max(0.18) * rotor_r * 0.94;
        d.stroke_line(rx_p - blade_w, ry_p, rx_p + blade_w, ry_p, (s * 0.09).max(0.55), rgba_alpha(0xE2E8F0, alpha), window);
        d.fill_circle(rx_p, ry_p, s * 0.06, rgba_alpha(0xF8FAFC, alpha), window);
    }

    // 2) 顶部微型通信天线 + 呼吸青光信标灯
    d.stroke_line(0.0, -s * 0.34, 0.0, -s * 0.56, (s * 0.08).max(0.45), rgba_alpha(0x94A3B8, alpha), window);
    let beacon_a = (0.55 + 0.45 * (time_sec * 6.0).sin()) * alpha;
    d.fill_circle(0.0, -s * 0.60, (s * 0.14).max(0.8), rgba_alpha(0x22D3EE, beacon_a * 0.40), window);
    d.fill_circle(0.0, -s * 0.60, (s * 0.09).max(0.6), rgba_alpha(0x22D3EE, beacon_a), window);

    // 3) 钛灰金属流线胶囊机身 + 纯黑 AMOLED 面罩
    let body_w = s * 1.20;
    let body_h = s * 0.72;
    d.fill_pill(0.0, 0.0, body_w, body_h, rgba_alpha(0x94A3B8, alpha), window);
    d.stroke_pill(0.0, 0.0, body_w, body_h, (s * 0.08).max(0.5), rgba_alpha(0x1E293B, alpha), window);

    let visor_w = body_w * 0.78;
    let visor_h = body_h * 0.68;
    d.fill_pill(0.0, 0.0, visor_w, visor_h, rgba_alpha(0x06070A, alpha), window);

    // 4) 随 NOMI 主题色同步眨眼的迷你发光双眸 + 底部推进微光条
    let is_blink = time_sec > 0.0 && ((time_sec + 1.1) % 3.6) < 0.13;
    let eye_dx = fly_dir * visor_w * 0.085;
    let eye_col = color_with_alpha(glow_color, alpha);
    for side in [-1.0_f32, 1.0] {
        let ex = side * visor_w * 0.22 + eye_dx;
        let ey = visor_h * 0.02;
        if is_blink {
            d.stroke_line(ex - s * 0.09, ey, ex + s * 0.09, ey, (s * 0.08).max(0.5), eye_col, window);
        } else {
            d.fill_pill(ex, ey, s * 0.14, s * 0.22, eye_col, window);
        }
    }
    d.stroke_line(-s * 0.22, body_h * 0.56, s * 0.22, body_h * 0.56, (s * 0.08).max(0.5), rgba_alpha(0x38BDF8, 0.75 * alpha), window);
}

// ============================================================================
// 四、赛博坦 / 变形金刚主题全套配饰（11 款前沿机甲）
// ============================================================================

/// 38. 大黄蜂机械双角盔 (BumblebeeMechaHelm)
fn draw_bumblebee_helm(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let shell_r = c.head_r * 1.07;
    let pulse = 0.72 + 0.28 * (time_sec * 3.0).sin();

    // 1) 双侧炭黑×明黄机械耳甲护翼
    for side in [-1.0_f32, 1.0] {
        let pod = c.sub(side * shell_r * 0.985, -shell_r * 0.08, side * 0.06);
        pod.fill_pill(0.0, 0.0, r * 0.16, r * 0.40, rgba(0x18181BFF), window);
        let mut arm = pod.fill();
        arm.move_to(side * -r * 0.02, -r * 0.17);
        arm.line_to(side * r * 0.095, -r * 0.12);
        arm.line_to(side * r * 0.11, r * 0.12);
        arm.line_to(side * -r * 0.02, r * 0.17);
        arm.close();
        arm.paint(rgba(0xFACC15FF), window);
        pod.fill_circle(side * r * 0.035, 0.0, r * 0.065, rgba(0x27272AFF), window);
        pod.fill_circle(side * r * 0.035, 0.0, r * 0.032, rgba_alpha(0x38BDF8, 0.90 * pulse), window);
    }

    // 2) 贴合球顶的明黄机甲主盔壳
    let steps = 28;
    let a_l = -0.94 * PI;
    let a_r = -0.06 * PI;
    let out_r = |f: f32| shell_r * (1.03 + 0.09 * (f * PI).sin());
    let in_r = |f: f32| {
        let dc = (f - 0.5).abs();
        let dip = if dc < 0.09 { (1.0 - dc / 0.09) * 0.095 } else { 0.0 };
        shell_r * (0.885 - dip)
    };
    let mut dome = c.fill();
    for i in 0..=steps {
        let f = i as f32 / steps as f32;
        let a = a_l + (a_r - a_l) * f;
        let rr = out_r(f);
        if i == 0 {
            dome.move_to(a.cos() * rr, a.sin() * rr);
        } else {
            dome.line_to(a.cos() * rr, a.sin() * rr);
        }
    }
    for i in (0..=steps).rev() {
        let f = i as f32 / steps as f32;
        let a = a_l + (a_r - a_l) * f;
        let rr = in_r(f);
        dome.line_to(a.cos() * rr, a.sin() * rr);
    }
    dome.close();
    dome.paint(rgba(0xFACC15FF), window);

    let mut d_rim = c.stroke((r * 0.026).max(0.55));
    for i in 0..=steps {
        let f = i as f32 / steps as f32;
        let a = a_l + (a_r - a_l) * f;
        let rr = out_r(f);
        if i == 0 {
            d_rim.move_to(a.cos() * rr, a.sin() * rr);
        } else {
            d_rim.line_to(a.cos() * rr, a.sin() * rr);
        }
    }
    for i in (0..=steps).rev() {
        let f = i as f32 / steps as f32;
        let a = a_l + (a_r - a_l) * f;
        let rr = in_r(f);
        d_rim.line_to(a.cos() * rr, a.sin() * rr);
    }
    d_rim.close();
    d_rim.paint(rgba(0x18181BFF), window);

    // 碳黑双赛道条纹 + 正中护额脊与冰蓝能量指示灯
    for side in [-1.0_f32, 1.0] {
        let mut st = c.fill();
        st.move_to(side * r * 0.13, -shell_r * 0.88);
        st.line_to(side * r * 0.21, -shell_r * 0.87);
        st.line_to(side * r * 0.25, -shell_r * 1.09);
        st.line_to(side * r * 0.16, -shell_r * 1.10);
        st.close();
        st.paint(rgba(0x18181BFF), window);
    }
    let mut crest = c.fill();
    crest.move_to(-r * 0.11, -shell_r * 1.11);
    crest.line_to(r * 0.11, -shell_r * 1.11);
    crest.line_to(r * 0.075, -shell_r * 0.86);
    crest.line_to(0.0, -shell_r * 0.785);
    crest.line_to(-r * 0.075, -shell_r * 0.86);
    crest.close();
    crest.paint(rgba(0x18181BFF), window);
    c.fill_pill(0.0, -shell_r * 0.97, r * 0.048, r * 0.115, rgba_alpha(0x38BDF8, 0.92 * pulse), window);

    // 3) 标志性斜切折叠机械双角
    for side in [-1.0_f32, 1.0] {
        let anc = -PI * 0.5 + side * 0.45;
        let h = c.sub(anc.cos() * shell_r * 1.02, anc.sin() * shell_r * 1.02, side * 0.40);
        h.fill_pill(0.0, 0.0, r * 0.18, r * 0.095, rgba(0x18181BFF), window);

        let mut horn = h.fill();
        horn.move_to(side * -r * 0.075, -r * 0.02);
        horn.line_to(side * -r * 0.055, -r * 0.25);
        horn.line_to(side * -r * 0.125, -r * 0.36);
        horn.line_to(side * r * 0.035, -r * 0.31);
        horn.line_to(side * r * 0.088, -r * 0.12);
        horn.line_to(side * r * 0.075, -r * 0.02);
        horn.close();
        horn.paint(rgba(0xFACC15FF), window);

        let mut hrim = h.stroke((r * 0.022).max(0.5));
        hrim.move_to(side * -r * 0.075, -r * 0.02);
        hrim.line_to(side * -r * 0.055, -r * 0.25);
        hrim.line_to(side * -r * 0.125, -r * 0.36);
        hrim.line_to(side * r * 0.035, -r * 0.31);
        hrim.line_to(side * r * 0.088, -r * 0.12);
        hrim.line_to(side * r * 0.075, -r * 0.02);
        hrim.close();
        hrim.paint(rgba(0x18181BFF), window);

        h.stroke_line(side * -r * 0.055, -r * 0.14, side * r * 0.07, -r * 0.11, (r * 0.022).max(0.5), rgba(0x18181BFF), window);
    }
}

/// 39. 赛博坦领袖战盔 (OptimusCrest - 红蓝经典撞色：炽红卡车领甲双窗格栅 + 钴蓝银冠双天线)
fn draw_optimus_crest(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let shell_r = c.head_r * 1.07;
    let pulse = 0.76 + 0.24 * (time_sec * 3.0).sin();

    // 0) 下缘炽红擎天柱卡车胸甲领圈 + 双冰蓝前风挡胸窗 + 银灰镀铬前格栅
    let c_l = 0.14 * PI;
    let c_r = 0.86 * PI;
    let c_steps = 22;
    let mut chest = c.fill();
    for i in 0..=c_steps {
        let f = i as f32 / c_steps as f32;
        let a = c_l + (c_r - c_l) * f;
        let rr = shell_r * (1.03 + 0.055 * (f * PI).sin());
        if i == 0 {
            chest.move_to(a.cos() * rr, a.sin() * rr);
        } else {
            chest.line_to(a.cos() * rr, a.sin() * rr);
        }
    }
    for i in (0..=c_steps).rev() {
        let f = i as f32 / c_steps as f32;
        let a = c_l + (c_r - c_l) * f;
        let rr = shell_r * (0.87 + 0.025 * (f * PI).sin());
        chest.line_to(a.cos() * rr, a.sin() * rr);
    }
    chest.close();
    chest.paint(rgba(0xDC2626FF), window);

    for side in [-1.0_f32, 1.0] {
        let wa = PI * 0.5 - side * 0.36;
        let w = c.sub(wa.cos() * shell_r * 0.965, wa.sin() * shell_r * 0.965, wa - PI * 0.5);
        w.fill_pill(0.0, 0.0, r * 0.29, r * 0.082, rgba(0x38BDF8FF), window);
        w.stroke_pill(0.0, 0.0, r * 0.29, r * 0.082, (r * 0.018).max(0.45), rgba(0xCBD5E1FF), window);
    }
    let gy = shell_r * 0.985;
    c.fill_pill(0.0, gy, r * 0.23, r * 0.135, rgba(0xE2E8F0FF), window);
    c.stroke_pill(0.0, gy, r * 0.23, r * 0.135, (r * 0.020).max(0.45), rgba(0x334155FF), window);
    for g in [-1.0_f32, 0.0, 1.0] {
        c.stroke_line(-r * 0.08, gy + g * r * 0.032, r * 0.08, gy + g * r * 0.032, (r * 0.018).max(0.45), rgba(0x475569FF), window);
    }

    // 1) 双侧钴蓝耳甲 + 炽红箍环银色天线塔
    for side in [-1.0_f32, 1.0] {
        let ear = c.sub(side * shell_r * 0.99, -shell_r * 0.08, 0.0);
        let mut ebox = ear.fill();
        ebox.move_to(side * -r * 0.03, -r * 0.24);
        ebox.line_to(side * r * 0.11, -r * 0.17);
        ebox.line_to(side * r * 0.125, r * 0.16);
        ebox.line_to(side * r * 0.01, r * 0.24);
        ebox.line_to(side * -r * 0.04, r * 0.18);
        ebox.close();
        ebox.paint(rgba(0x1D4ED8FF), window);
        ear.fill_circle(side * r * 0.045, r * 0.02, r * 0.055, rgba(0x94A3B8FF), window);
        ear.fill_circle(side * r * 0.045, r * 0.02, r * 0.026, rgba(0xEF4444FF), window);

        let mut ant = ear.fill();
        ant.move_to(side * -r * 0.015, -r * 0.16);
        ant.line_to(side * r * 0.095, -r * 0.14);
        ant.line_to(side * r * 0.058, -shell_r * 1.10);
        ant.line_to(side * -r * 0.015, -shell_r * 1.04);
        ant.close();
        ant.paint(rgba(0xE2E8F0FF), window);

        let mut arim = ear.stroke((r * 0.020).max(0.45));
        arim.move_to(side * -r * 0.015, -r * 0.16);
        arim.line_to(side * r * 0.095, -r * 0.14);
        arim.line_to(side * r * 0.058, -shell_r * 1.10);
        arim.line_to(side * -r * 0.015, -shell_r * 1.04);
        arim.close();
        arim.paint(rgba(0x1E293BFF), window);

        ear.fill_pill(side * r * 0.03, -r * 0.59, r * 0.09, r * 0.06, rgba(0xDC2626FF), window);
    }

    // 2) 钴蓝分层头盔 + 炽红肩条纹 + 银白正中脊冠
    let steps = 28;
    let a_l = -0.92 * PI;
    let a_r = -0.08 * PI;
    let out_r = |f: f32| shell_r * (1.03 + 0.095 * (f * PI).sin());
    let in_r = |f: f32| {
        let dc = (f - 0.5).abs();
        let dip = if dc < 0.08 { (1.0 - dc / 0.08) * 0.085 } else { 0.0 };
        shell_r * (0.88 - dip)
    };
    let mut dome = c.fill();
    for i in 0..=steps {
        let f = i as f32 / steps as f32;
        let a = a_l + (a_r - a_l) * f;
        let rr = out_r(f);
        if i == 0 {
            dome.move_to(a.cos() * rr, a.sin() * rr);
        } else {
            dome.line_to(a.cos() * rr, a.sin() * rr);
        }
    }
    for i in (0..=steps).rev() {
        let f = i as f32 / steps as f32;
        let a = a_l + (a_r - a_l) * f;
        let rr = in_r(f);
        dome.line_to(a.cos() * rr, a.sin() * rr);
    }
    dome.close();
    dome.paint(rgba(0x2563EBFF), window);

    for side in [-1.0_f32, 1.0] {
        c.stroke_quad(
            side * r * 0.22,
            -shell_r * 0.95,
            side * r * 0.54,
            -shell_r * 0.84,
            side * r * 0.78,
            -shell_r * 0.58,
            (r * 0.032).max(0.65),
            rgba(0xEF4444FF),
            window,
        );
    }

    let mut crest = c.fill();
    crest.move_to(-r * 0.115, -shell_r * 1.21);
    crest.line_to(r * 0.115, -shell_r * 1.21);
    crest.line_to(r * 0.14, -shell_r * 0.98);
    crest.line_to(r * 0.075, -shell_r * 0.855);
    crest.line_to(0.0, -shell_r * 0.785);
    crest.line_to(-r * 0.075, -shell_r * 0.855);
    crest.line_to(-r * 0.14, -shell_r * 0.98);
    crest.close();
    crest.paint(rgba(0xF1F5F9FF), window);

    c.fill_pill(0.0, -shell_r * 1.05, r * 0.056, r * 0.14, rgba_alpha(0xEF4444, 0.95 * pulse), window);
}

/// 40. 枪铁霸主融合炮 (MegatronCannon)
fn draw_megatron_cannon(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let shell_r = c.head_r * 1.07;
    let pulse = 0.72 + 0.28 * (time_sec * 3.2).sin();

    // 1) 硬朗枪铁灰斜切梯形战盔
    let mut bucket = c.fill();
    bucket.move_to(-shell_r * 0.96, -shell_r * 0.14);
    bucket.line_to(-shell_r * 1.04, -shell_r * 0.62);
    bucket.line_to(-shell_r * 0.62, -shell_r * 1.07);
    bucket.line_to(0.0, -shell_r * 1.12);
    bucket.line_to(shell_r * 0.62, -shell_r * 1.07);
    bucket.line_to(shell_r * 1.04, -shell_r * 0.62);
    bucket.line_to(shell_r * 0.96, -shell_r * 0.14);
    bucket.quad_to(shell_r * 0.85, -shell_r * 0.76, shell_r * 0.44, -shell_r * 0.88);
    bucket.line_to(0.0, -shell_r * 0.80);
    bucket.line_to(-shell_r * 0.44, -shell_r * 0.88);
    bucket.quad_to(-shell_r * 0.85, -shell_r * 0.76, -shell_r * 0.96, -shell_r * 0.14);
    bucket.close();
    bucket.paint(rgba(0x64748BFF), window);

    let mut brim = c.stroke((r * 0.024).max(0.55));
    brim.move_to(-shell_r * 0.96, -shell_r * 0.14);
    brim.line_to(-shell_r * 1.04, -shell_r * 0.62);
    brim.line_to(-shell_r * 0.62, -shell_r * 1.07);
    brim.line_to(0.0, -shell_r * 1.12);
    brim.line_to(shell_r * 0.62, -shell_r * 1.07);
    brim.line_to(shell_r * 1.04, -shell_r * 0.62);
    brim.line_to(shell_r * 0.96, -shell_r * 0.14);
    brim.paint(rgba(0x0F172AFF), window);

    for side in [-1.0_f32, 1.0] {
        c.stroke_line(side * shell_r * 0.22, -shell_r * 0.88, side * shell_r * 0.74, -shell_r * 0.78, (r * 0.020).max(0.45), rgba(0xA855F78C), window);
    }

    // 额前暗钢菱晶紫徽
    let mut badge = c.fill();
    badge.move_to(-r * 0.12, -shell_r * 1.07);
    badge.line_to(r * 0.12, -shell_r * 1.07);
    badge.line_to(r * 0.08, -shell_r * 0.84);
    badge.line_to(0.0, -shell_r * 0.79);
    badge.line_to(-r * 0.08, -shell_r * 0.84);
    badge.close();
    badge.paint(rgba(0x0F172AFF), window);

    let mut gem = c.fill();
    gem.move_to(0.0, -shell_r * 1.01);
    gem.line_to(r * 0.05, -shell_r * 0.92);
    gem.line_to(0.0, -shell_r * 0.845);
    gem.line_to(-r * 0.05, -shell_r * 0.92);
    gem.close();
    gem.paint(rgba_alpha(0xC084FC, 0.92 * pulse), window);

    // 2) 右侧紧凑黑色融合炮管 + 紫晶蓄能核心
    let gun = c.sub(shell_r * 0.97, -shell_r * 0.12, -0.10 + (time_sec * 2.2).sin() * 0.025);
    gun.fill_pill(r * 0.17, 0.0, r * 0.44, r * 0.19, rgba(0x18181BFF), window);
    gun.stroke_pill(r * 0.17, 0.0, r * 0.44, r * 0.19, (r * 0.022).max(0.45), rgba(0x52525BFF), window);
    gun.fill_pill(r * 0.37, 0.0, r * 0.06, r * 0.11, rgba_alpha(0xC084FC, 0.95 * pulse), window);
}

/// 41. 领导模块吊坠 (MatrixPendant)
fn draw_matrix_pendant(c: &HeadCanvas, time_sec: f32, glow_color: Rgba, window: &mut Window) {
    let r = c.r;
    let pulse = 0.78 + 0.22 * (time_sec * 3.0).sin();
    let neck = c.sub(0.0, c.head_r - r * 0.01, 0.0);

    neck.stroke_quad(-r * 0.28, -r * 0.06, 0.0, r * 0.03, r * 0.28, -r * 0.06, (r * 0.022).max(0.45), rgba(0x94A3B8FF), window);
    for side in [-1.0_f32, 1.0] {
        let mut h = neck.fill();
        h.move_to(side * r * 0.11, -r * 0.075);
        h.line_to(side * r * 0.29, -r * 0.105);
        h.line_to(side * r * 0.34, 0.0);
        h.line_to(side * r * 0.29, r * 0.11);
        h.line_to(side * r * 0.11, r * 0.08);
        h.close();
        h.paint(rgba(0xD97706FF), window);
    }
    neck.fill_ellipse(0.0, 0.0, r * 0.145, r * 0.11, 0.0, rgba(0xCBD5E1FF), window);
    neck.stroke_ellipse(0.0, 0.0, r * 0.145, r * 0.11, 0.0, (r * 0.020).max(0.45), rgba(0x334155FF), window);

    let mut spark = neck.fill();
    spark.move_to(0.0, -r * 0.082);
    spark.line_to(r * 0.098, 0.0);
    spark.line_to(0.0, r * 0.082);
    spark.line_to(-r * 0.098, 0.0);
    spark.close();
    spark.paint(color_with_alpha(glow_color, 0.85 * pulse + 0.15), window);
    neck.fill_circle(-r * 0.018, -r * 0.018, r * 0.025, rgba(0xFFFFFFFF), window);
}

/// 42. 声波通讯耳麦 (CommsVisor)
fn draw_comms_visor(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let shell_r = c.head_r * 1.07;
    let pulse = 0.75 + 0.25 * (time_sec * 3.6).sin();

    c.stroke_arc(0.0, 0.0, shell_r + r * 0.012, shell_r + r * 0.012, -PI * 0.86, -PI * 0.14, (r * 0.062).max(1.1), rgba(0x1E3A8AFF), window);
    c.stroke_arc(0.0, 0.0, shell_r + r * 0.025, shell_r + r * 0.025, -PI * 0.80, -PI * 0.20, (r * 0.020).max(0.45), rgba(0xCBD5E1FF), window);

    for side in [-1.0_f32, 1.0] {
        let cup = c.sub(side * shell_r * 0.995, -shell_r * 0.02, 0.0);
        if side < 0.0 {
            let ant_h = shell_r * 0.82;
            cup.fill_pill(0.0, -r * 0.20 - ant_h * 0.5, r * 0.048, ant_h, rgba(0xE2E8F0FF), window);
            cup.fill_circle(0.0, -r * 0.22 - ant_h, r * 0.036, rgba_alpha(0x38BDF8, 0.95 * pulse), window);
        }
        cup.fill_pill(0.0, 0.0, r * 0.17, r * 0.44, rgba(0x1D4ED8FF), window);
        cup.stroke_pill(0.0, 0.0, r * 0.17, r * 0.44, (r * 0.022).max(0.45), rgba(0x0F172AFF), window);
        for b in 0..3 {
            let bp = 0.45 + 0.55 * (time_sec * 5.2 + b as f32 * 1.1 + side).sin().abs();
            cup.fill_pill(0.0, -r * 0.09 + b as f32 * r * 0.085, r * 0.072 * bp, r * 0.032, rgba(0x38BDF8FF), window);
        }
    }
}

/// 43. 震荡波机械独眼盔 (ShockwaveOptic)
fn draw_shockwave_optic(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let shell_r = c.head_r * 1.07;
    let pulse = 0.76 + 0.24 * (time_sec * 3.0).sin();

    for side in [-1.0_f32, 1.0] {
        let fin = c.sub(side * shell_r * 0.97, -shell_r * 0.14, side * 0.16);
        let mut fp = fin.fill();
        fp.move_to(side * -r * 0.02, -r * 0.12);
        fp.line_to(side * r * 0.11, -r * 0.08);
        fp.line_to(side * r * 0.135, -r * 0.88);
        fp.line_to(side * r * 0.02, -r * 0.98);
        fp.line_to(side * -r * 0.04, -r * 0.70);
        fp.close();
        fp.paint(rgba(0x7C3AEDFF), window);
        fin.fill_circle(side * r * 0.045, -r * 0.24, r * 0.028, rgba_alpha(0xFACC15, 0.90 * pulse), window);
    }

    let mut helm = c.fill();
    helm.move_to(-shell_r * 0.96, -shell_r * 0.18);
    helm.line_to(-shell_r * 1.01, -shell_r * 0.62);
    helm.line_to(-shell_r * 0.56, -shell_r * 1.07);
    helm.line_to(0.0, -shell_r * 1.15);
    helm.line_to(shell_r * 0.56, -shell_r * 1.07);
    helm.line_to(shell_r * 1.01, -shell_r * 0.62);
    helm.line_to(shell_r * 0.96, -shell_r * 0.18);
    helm.quad_to(shell_r * 0.80, -shell_r * 0.78, shell_r * 0.36, -shell_r * 0.88);
    helm.line_to(0.0, -shell_r * 0.81);
    helm.line_to(-shell_r * 0.36, -shell_r * 0.88);
    helm.quad_to(-shell_r * 0.80, -shell_r * 0.78, -shell_r * 0.96, -shell_r * 0.18);
    helm.close();
    helm.paint(rgba(0x6D28D9FF), window);

    let mut crest = c.fill();
    crest.move_to(-r * 0.09, -shell_r * 1.14);
    crest.line_to(r * 0.09, -shell_r * 1.14);
    crest.line_to(r * 0.055, -shell_r * 0.84);
    crest.line_to(0.0, -shell_r * 0.79);
    crest.line_to(-r * 0.055, -shell_r * 0.84);
    crest.close();
    crest.paint(rgba(0xE2E8F0FF), window);
}

/// 44. 爵士跑车护目镜盔 (JazzVisor)
fn draw_jazz_visor(c: &HeadCanvas, window: &mut Window) {
    let r = c.r;
    let shell_r = c.head_r * 1.07;

    for side in [-1.0_f32, 1.0] {
        let ear = c.sub(side * shell_r * 0.99, -shell_r * 0.10, 0.0);
        let mut wp = ear.fill();
        wp.move_to(side * -r * 0.02, -r * 0.18);
        wp.line_to(side * r * 0.12, -r * 0.38);
        wp.line_to(side * r * 0.145, -r * 0.02);
        wp.line_to(side * r * 0.085, r * 0.22);
        wp.line_to(side * -r * 0.03, r * 0.16);
        wp.close();
        wp.paint(rgba(0xE2E8F0FF), window);
    }

    let steps = 26;
    let a_l = -0.92 * PI;
    let a_r = -0.08 * PI;
    let mut dome = c.fill();
    for i in 0..=steps {
        let f = i as f32 / steps as f32;
        let a = a_l + (a_r - a_l) * f;
        let rr = shell_r * (1.03 + 0.085 * (f * PI).sin());
        if i == 0 {
            dome.move_to(a.cos() * rr, a.sin() * rr);
        } else {
            dome.line_to(a.cos() * rr, a.sin() * rr);
        }
    }
    for i in (0..=steps).rev() {
        let f = i as f32 / steps as f32;
        let a = a_l + (a_r - a_l) * f;
        dome.line_to(a.cos() * shell_r * 0.89, a.sin() * shell_r * 0.89);
    }
    dome.close();
    dome.paint(rgba(0xF8FAFCFF), window);

    c.stroke_line(-r * 0.06, -shell_r * 0.90, -r * 0.08, -shell_r * 1.10, (r * 0.026).max(0.5), rgba(0x2563EBFF), window);
    c.stroke_line(r * 0.06, -shell_r * 0.90, r * 0.08, -shell_r * 1.10, (r * 0.022).max(0.45), rgba(0xEF4444FF), window);
}

/// 45. 漂移武士双刀盔 (DriftSamurai) - 亮银白武士兜冠 + 舒展双锹形金角 + 两侧吹返护颊
fn draw_drift_samurai(c: &HeadCanvas, window: &mut Window) {
    let r = c.r;
    let shell_r = c.head_r * 1.07;

    // 1) 左右外展赛博武士吹返面甲 (Fukikaeshi Wing Guards)
    for side in [-1.0_f32, 1.0] {
        let fuki = c.sub(side * shell_r * 0.985, -shell_r * 0.12, side * 0.08);
        let mut plate = fuki.fill();
        plate.move_to(side * -r * 0.02, -r * 0.20);
        plate.line_to(side * r * 0.13, -r * 0.14);
        plate.line_to(side * r * 0.15, r * 0.16);
        plate.line_to(side * r * 0.02, r * 0.22);
        plate.line_to(side * -r * 0.03, r * 0.18);
        plate.close();
        plate.paint(rgba(0xE2E8F0FF), window);

        let mut trim = fuki.stroke((r * 0.024).max(0.5));
        trim.move_to(side * -r * 0.02, -r * 0.20);
        trim.line_to(side * r * 0.13, -r * 0.14);
        trim.line_to(side * r * 0.15, r * 0.16);
        trim.line_to(side * r * 0.02, r * 0.22);
        trim.line_to(side * -r * 0.03, r * 0.18);
        trim.close();
        trim.paint(rgba(0x1E293BFF), window);

        // 吹返内嵌钴蓝赛车饰条
        let mut blue_bar = fuki.fill();
        blue_bar.move_to(side * r * 0.02, -r * 0.12);
        blue_bar.line_to(side * r * 0.10, -r * 0.08);
        blue_bar.line_to(side * r * 0.11, r * 0.10);
        blue_bar.line_to(side * r * 0.03, r * 0.14);
        blue_bar.close();
        blue_bar.paint(rgba(0x1D4ED8FF), window);
    }

    // 2) 贴合球顶的亮银珍珠白分层武士兜盔 (Pearl White Kabuto Dome)
    let steps = 30;
    let a_l = -0.93 * PI;
    let a_r = -0.07 * PI;
    let mut kabuto = c.fill();
    for i in 0..=steps {
        let f = i as f32 / steps as f32;
        let a = a_l + (a_r - a_l) * f;
        let rr = shell_r * (1.04 + 0.09 * (f * PI).sin());
        if i == 0 {
            kabuto.move_to(a.cos() * rr, a.sin() * rr);
        } else {
            kabuto.line_to(a.cos() * rr, a.sin() * rr);
        }
    }
    for i in (0..=steps).rev() {
        let f = i as f32 / steps as f32;
        let a = a_l + (a_r - a_l) * f;
        let dist = (f - 0.5).abs();
        let dip = if dist < 0.10 { (1.0 - dist / 0.10) * 0.09 } else { 0.0 };
        let rr = shell_r * (0.875 - dip);
        kabuto.line_to(a.cos() * rr, a.sin() * rr);
    }
    kabuto.close();
    kabuto.paint(rgba(0xF8FAFCFF), window);

    let mut k_rim = c.stroke((r * 0.028).max(0.6));
    for i in 0..=steps {
        let f = i as f32 / steps as f32;
        let a = a_l + (a_r - a_l) * f;
        let rr = shell_r * (1.04 + 0.09 * (f * PI).sin());
        if i == 0 {
            k_rim.move_to(a.cos() * rr, a.sin() * rr);
        } else {
            k_rim.line_to(a.cos() * rr, a.sin() * rr);
        }
    }
    k_rim.paint(rgba(0x334155FF), window);

    // 兜身双侧钴蓝与炽红纵向装甲筋条
    for side in [-1.0_f32, 1.0] {
        let col = if side < 0.0 { rgba(0x1D4ED8FF) } else { rgba(0xDC2626FF) };
        let mut rib = c.fill();
        rib.move_to(side * r * 0.18, -shell_r * 0.88);
        rib.line_to(side * r * 0.25, -shell_r * 0.87);
        rib.line_to(side * r * 0.32, -shell_r * 1.10);
        rib.line_to(side * r * 0.24, -shell_r * 1.11);
        rib.close();
        rib.paint(col, window);
    }

    // 兜顶鎏金八幡座六角金属顶纽
    c.fill_ellipse(0.0, -shell_r * 1.12, r * 0.09, r * 0.045, 0.0, rgba(0xFACC15FF), window);
    c.stroke_ellipse(0.0, -shell_r * 1.12, r * 0.09, r * 0.045, 0.0, (r * 0.02).max(0.45), rgba(0x78350FFF), window);

    // 3) 额前玄黑折角眉庇护檐 (Mabisashi Visor Brow)
    let mut brow = c.fill();
    brow.move_to(-r * 0.44, -shell_r * 0.87);
    brow.line_to(0.0, -shell_r * 0.95);
    brow.line_to(r * 0.44, -shell_r * 0.87);
    brow.line_to(0.0, -shell_r * 0.78);
    brow.close();
    brow.paint(rgba(0x0F172AFF), window);

    let mut b_rim = c.stroke((r * 0.022).max(0.5));
    b_rim.move_to(-r * 0.44, -shell_r * 0.87);
    b_rim.line_to(0.0, -shell_r * 0.78);
    b_rim.line_to(r * 0.44, -shell_r * 0.87);
    b_rim.paint(rgba(0xDC2626FF), window);

    // 4) 昂扬展翼大号流线型鎏金双锹形大角 (Grand Kuwagata V-Crest)
    for side in [-1.0_f32, 1.0] {
        let mut horn = c.fill();
        horn.move_to(0.0, -shell_r * 0.88);
        horn.quad_to(side * r * 0.38, -shell_r * 1.02, side * r * 0.65, -shell_r * 1.36);
        horn.quad_to(side * r * 0.46, -shell_r * 1.12, side * r * 0.05, -shell_r * 0.98);
        horn.close();
        horn.paint(rgba(0xFACC15FF), window);

        let mut h_rim = c.stroke((r * 0.022).max(0.45));
        h_rim.move_to(0.0, -shell_r * 0.88);
        h_rim.quad_to(side * r * 0.38, -shell_r * 1.02, side * r * 0.65, -shell_r * 1.36);
        h_rim.quad_to(side * r * 0.46, -shell_r * 1.12, side * r * 0.05, -shell_r * 0.98);
        h_rim.close();
        h_rim.paint(rgba(0x78350FFF), window);
    }

    // 5) 前立正中博派立体六角金红战标 (Maedate Crest Badge)
    let mut badge = c.fill();
    for v in 0..6 {
        let a = (v as f32 * PI) / 3.0;
        let hx = a.cos() * r * 0.095;
        let hy = -shell_r * 0.90 + a.sin() * r * 0.095;
        if v == 0 {
            badge.move_to(hx, hy);
        } else {
            badge.line_to(hx, hy);
        }
    }
    badge.close();
    badge.paint(rgba(0xFACC15FF), window);
    c.fill_circle(0.0, -shell_r * 0.90, r * 0.045, rgba(0xDC2626FF), window);
}

/// 46. 探索者战机双翼（后景层）
fn draw_seeker_wings_back(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let lift = (time_sec * 3.0).sin() * 0.025;
    let jet = 0.72 + 0.28 * (time_sec * 8.5).sin();

    for side in [-1.0_f32, 1.0] {
        let w = c.sub(side * r * 0.78, -r * 0.04, side * lift);
        let nx = side * r * 0.24;
        let ny = r * 0.34;
        w.fill_pill(nx, ny, r * 0.16, r * 0.12, rgba(0x334155FF), window);
        let mut flame = w.fill();
        flame.move_to(nx - side * r * 0.05, ny + r * 0.06);
        flame.line_to(nx + side * r * 0.09, ny + r * 0.06);
        flame.line_to(nx + side * r * 0.02, ny + r * (0.20 + 0.07 * jet));
        flame.close();
        flame.paint(rgba_alpha(0x38BDF8, 0.85 * jet), window);

        let mut wing = w.fill();
        wing.move_to(0.0, -r * 0.44);
        wing.line_to(side * r * 0.84, -r * 0.54);
        wing.line_to(side * r * 0.78, -r * 0.24);
        wing.line_to(side * r * 0.42, r * 0.26);
        wing.line_to(0.0, r * 0.32);
        wing.close();
        wing.paint(rgba(0xCBD5E1FF), window);

        let mut red = w.fill();
        red.move_to(side * r * 0.16, -r * 0.32);
        red.line_to(side * r * 0.68, -r * 0.39);
        red.line_to(side * r * 0.63, -r * 0.29);
        red.line_to(side * r * 0.14, -r * 0.22);
        red.close();
        red.paint(rgba(0xEF4444FF), window);

        let mut blue = w.fill();
        blue.move_to(side * r * 0.14, -r * 0.20);
        blue.line_to(side * r * 0.61, -r * 0.27);
        blue.line_to(side * r * 0.57, -r * 0.20);
        blue.line_to(side * r * 0.12, -r * 0.13);
        blue.close();
        blue.paint(rgba(0x2563EBFF), window);
    }
}

/// 47. 漂移背后双武士刀（后景层）
fn draw_drift_swords_back(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let pulse = 0.75 + 0.25 * (time_sec * 3.4).sin();
    for side in [-1.0_f32, 1.0] {
        let sw = c.sub(0.0, 0.0, side * 0.56);
        sw.fill_pill(0.0, -r * 1.12, r * 0.08, r * 0.55, rgba(0x334155FF), window);
        let blade_col = if side < 0.0 {
            rgba_alpha(0x38BDF8, 0.85 * pulse)
        } else {
            rgba_alpha(0xFB923C, 0.85 * pulse)
        };
        sw.stroke_line(-r * 0.04, -r * 1.32, -r * 0.04, -r * 0.95, (r * 0.020).max(0.45), blade_col, window);
        sw.fill_pill(0.0, -r * 1.40, r * 0.22, r * 0.05, rgba(0xFACC15FF), window);
        sw.fill_pill(0.0, -r * 1.57, r * 0.09, r * 0.30, rgba(0x18181BFF), window);
        sw.stroke_pill(0.0, -r * 1.57, r * 0.09, r * 0.30, (r * 0.018).max(0.45), rgba(0xDC2626FF), window);
    }
}

/// 48. 赛博坦齿轮星环 (CybertronRing)
fn draw_cybertron_ring_half(
    c: &HeadCanvas,
    time_sec: f32,
    glow_color: Rgba,
    pitch: f32,
    is_back: bool,
    window: &mut Window,
) {
    let r = c.r;
    let aspect = (0.36 + pitch * 0.22).clamp(0.22, 0.50) / 1.48;
    let ring = c.sub(0.0, r * 0.10, -0.32);
    let (start_a, end_a) = if is_back { (PI, 2.0 * PI) } else { (0.0, PI) };
    let layer_alpha = if is_back { 0.48_f32 } else { 1.0_f32 };

    ring.stroke_arc(0.0, 0.0, r * 1.42, r * 1.42 * aspect, start_a, end_a, (r * 0.032).max(0.6), rgba_alpha(0x94A3B8, 0.36 * layer_alpha), window);
    ring.stroke_arc(0.0, 0.0, r * 1.34, r * 1.34 * aspect, start_a, end_a, (r * 0.016).max(0.45), color_with_alpha(glow_color, if is_back { 0.22 } else { 0.58 }), window);

    // 齿轨分段齿痕
    for seg in 0..8 {
        let sa = start_a + (end_a - start_a) * (seg as f32 / 8.0);
        let ea = sa + (end_a - start_a) * 0.065;
        ring.stroke_arc(0.0, 0.0, r * 1.48, r * 1.48 * aspect, sa, ea, (r * 0.022).max(0.5), rgba_alpha(0xE2E8F0, 0.45 * layer_alpha), window);
    }

    let nodes = [
        (time_sec * 1.25 + 0.35, 1.42_f32, r * 0.050, glow_color),
        (-time_sec * 0.85 + 2.40, 1.34_f32, r * 0.038, rgba(0x38BDF8FF)),
    ];
    for (na, scale, ns, col) in nodes {
        let sin_a = na.sin();
        if is_back != (sin_a < 0.0) {
            continue;
        }
        let mx = na.cos() * r * scale;
        let my = sin_a * r * scale * aspect;
        let sub = ring.sub(mx, my, time_sec * 2.2);
        sub.fill_pill(0.0, 0.0, ns, ns, color_with_alpha(col, if is_back { 0.60 } else { 0.96 }), window);
    }
}

/// 49. 伴飞机械激光鸟 (LaserbeakDrone - Y 轴真 3D 转向不突变)
fn draw_laserbeak_drone(c: &HeadCanvas, time_sec: f32, is_back: bool, window: &mut Window) {
    let orbit_a = time_sec * 1.15 + 0.55;
    let sin_a = orbit_a.sin();
    let cos_a = orbit_a.cos();
    let in_front = sin_a >= 0.0;
    if is_back == in_front {
        return;
    }

    let r = c.r;
    let level = HeadCanvas {
        ux: 1.0,
        uy: 0.0,
        vx: 0.0,
        vy: 1.0,
        ..*c
    };
    let hover_y = (time_sec * 3.8).sin() * r * 0.04;
    let bank_tilt = -sin_a * 0.14 + (time_sec * 3.0).sin() * 0.04;
    let base_x = cos_a * r * 1.12;
    let base_y = -r * 0.96 + sin_a * r * 0.16 + hover_y;
    let depth_s = 1.0 + 0.14 * sin_a;
    let s = r * 0.27 * depth_s;
    let alpha = if in_front { 1.0_f32 } else { 0.86_f32 };

    let d = level.sub(base_x, base_y, bank_tilt);
    let cos_h = -sin_a;
    let sin_h = cos_a;
    let proj = |lx: f32, ly: f32, lz: f32| -> (f32, f32) {
        (lx * cos_h - lz * sin_h, ly + lx * sin_h * 0.12)
    };
    let flap_lift = (time_sec * 9.5).sin() * 0.26 * s;

    let draw_wing = |wing_side: f32, near: bool, window: &mut Window| {
        let p0 = proj(s * 0.14, -s * 0.02, wing_side * s * 0.12);
        let p1 = proj(-s * 0.78, -s * 0.28 - flap_lift, wing_side * s * 1.08);
        let p2 = proj(-s * 0.82, -s * 0.04 - flap_lift * 0.6, wing_side * s * 0.72);
        let p3 = proj(-s * 0.42, s * 0.12, wing_side * s * 0.10);
        let mut wf = d.fill();
        wf.move_to(p0.0, p0.1);
        wf.line_to(p1.0, p1.1);
        wf.line_to(p2.0, p2.1);
        wf.line_to(p3.0, p3.1);
        wf.close();
        wf.paint(rgba_alpha(if near { 0xDC2626 } else { 0x991B1B }, alpha), window);

        let s0 = proj(-s * 0.12, -s * 0.09 - flap_lift * 0.25, wing_side * s * 0.34);
        let s1 = proj(-s * 0.68, -s * 0.22 - flap_lift * 0.85, wing_side * s * 0.90);
        d.stroke_line(s0.0, s0.1, s1.0, s1.1, (s * 0.045).max(0.45), rgba_alpha(0xCBD5E1, alpha), window);
    };

    let (far_side, near_side) = if cos_h >= 0.0 { (-1.0, 1.0) } else { (1.0, -1.0) };
    draw_wing(far_side, false, window);

    // 双肩银炮管 + 机械机身 + 银喙红眼
    for lz in [-s * 0.14, s * 0.14] {
        let g0 = proj(-s * 0.28, -s * 0.22, lz);
        let g1 = proj(s * 0.24, -s * 0.22, lz);
        d.stroke_line(g0.0, g0.1, g1.0, g1.1, (s * 0.09).max(0.5), rgba_alpha(0x64748B, alpha), window);
        d.fill_circle(g1.0, g1.1, (s * 0.05).max(0.5), rgba_alpha(0xEF4444, alpha), window);
    }

    let pts = [
        proj(-s * 0.72, s * 0.10, 0.0),
        proj(-s * 0.32, -s * 0.14, 0.0),
        proj(s * 0.44, -s * 0.16, 0.0),
        proj(s * 0.76, -s * 0.05, 0.0),
        proj(s * 0.98, s * 0.04, 0.0),
        proj(s * 0.62, s * 0.10, 0.0),
        proj(s * 0.18, s * 0.17, 0.0),
        proj(-s * 0.45, s * 0.08, 0.0),
    ];
    let mut body = d.fill();
    body.move_to(pts[0].0, pts[0].1);
    for pt in &pts[1..] {
        body.line_to(pt.0, pt.1);
    }
    body.close();
    body.paint(rgba_alpha(0x27272A, alpha), window);

    let b0 = proj(s * 0.74, -s * 0.04, 0.0);
    let b1 = proj(s * 0.72, s * 0.07, 0.0);
    let mut beak = d.fill();
    beak.move_to(b0.0, b0.1);
    beak.line_to(pts[4].0, pts[4].1);
    beak.line_to(b1.0, b1.1);
    beak.close();
    beak.paint(rgba_alpha(0xE2E8F0, alpha), window);

    for ez in [-s * 0.06, s * 0.06] {
        let ep = proj(s * 0.66, -s * 0.03, ez);
        d.fill_circle(ep.0, ep.1, (s * 0.055).max(0.55), rgba_alpha(0xF87171, alpha), window);
    }

    draw_wing(near_side, true, window);
}

/// 50. 钢索霸王龙折翼背甲 (后景层)
fn draw_grimlock_wings_back(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let pulse = 0.72 + 0.28 * (time_sec * 3.2).sin();
    for side in [-1.0_f32, 1.0] {
        let w = c.sub(side * r * 0.72, -r * 0.12, side * 0.04);
        // 暗银/枪铁霸王龙背翼外甲
        let mut frame = w.fill();
        frame.move_to(side * r * 0.10, -r * 0.28);
        frame.line_to(side * r * 0.65, -r * 0.98);
        frame.line_to(side * r * 0.88, -r * 0.82);
        frame.line_to(side * r * 0.82, -r * 0.44);
        frame.line_to(side * r * 0.89, -r * 0.32);
        frame.line_to(side * r * 0.74, r * 0.16);
        frame.line_to(side * r * 0.42, r * 0.30);
        frame.line_to(side * r * 0.10, r * 0.18);
        frame.close();
        frame.paint(rgba(0x334155FF), window);

        let mut gold = w.fill();
        gold.move_to(side * r * 0.22, -r * 0.24);
        gold.line_to(side * r * 0.62, -r * 0.80);
        gold.line_to(side * r * 0.76, -r * 0.68);
        gold.line_to(side * r * 0.68, r * 0.05);
        gold.line_to(side * r * 0.44, r * 0.16);
        gold.close();
        gold.paint(rgba(0xFACC15FF), window);

        // 熔岩能量排气线
        w.stroke_line(side * r * 0.32, -r * 0.38, side * r * 0.64, -r * 0.58, (r * 0.022).max(0.5), rgba_alpha(0xEF4444, 0.88 * pulse), window);
        w.stroke_line(side * r * 0.30, -r * 0.16, side * r * 0.62, -r * 0.34, (r * 0.022).max(0.5), rgba_alpha(0xEF4444, 0.88 * pulse), window);
    }
}

/// 51. 钢索 · 霸王龙战盔 (前景层)
fn draw_grimlock_helm(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let head_r = c.head_r;
    let pulse = 0.76 + 0.24 * (time_sec * 3.2).sin();

    // 1) 顶部暗银/枪铁重甲半幅头盔 (高对比金属质感，绝不黑成一片)
    let steps = 24;
    let a_left = -0.94 * PI;
    let a_right = -0.06 * PI;
    let mut dome = c.fill();
    for i in 0..=steps {
        let f = i as f32 / steps as f32;
        let a = a_left + (a_right - a_left) * f;
        let rr = head_r * (1.065 + 0.075 * (f * PI).sin());
        let (s, co) = a.sin_cos();
        if i == 0 {
            dome.move_to(co * rr, s * rr);
        } else {
            dome.line_to(co * rr, s * rr);
        }
    }
    for i in (0..=steps).rev() {
        let f = i as f32 / steps as f32;
        let a = a_left + (a_right - a_left) * f;
        let rr = head_r * (0.875 + 0.015 * (f * PI).sin());
        let (s, co) = a.sin_cos();
        dome.line_to(co * rr, s * rr);
    }
    dome.close();
    dome.paint(rgba(0x334155FF), window);

    let mut rim = c.stroke((r * 0.024).max(0.6));
    for i in 0..=steps {
        let f = i as f32 / steps as f32;
        let a = a_left + (a_right - a_left) * f;
        let rr = head_r * (1.065 + 0.075 * (f * PI).sin());
        let (s, co) = a.sin_cos();
        if i == 0 { rim.move_to(co * rr, s * rr); } else { rim.line_to(co * rr, s * rr); }
    }
    rim.paint(rgba(0x64748BFF), window);

    // 2) 双侧银金斜挑霸王龙耳角
    for side in [-1.0_f32, 1.0] {
        let mut ear = c.fill();
        ear.move_to(side * head_r * 0.90, -head_r * 0.44);
        ear.line_to(side * head_r * 1.19, -head_r * 0.76);
        ear.line_to(side * head_r * 1.16, -head_r * 0.24);
        ear.line_to(side * head_r * 1.03, head_r * 0.08);
        ear.line_to(side * head_r * 0.92, -head_r * 0.02);
        ear.close();
        ear.paint(rgba(0x94A3B8FF), window);

        let mut ear_gold = c.fill();
        ear_gold.move_to(side * head_r * 0.96, -head_r * 0.40);
        ear_gold.line_to(side * head_r * 1.13, -head_r * 0.63);
        ear_gold.line_to(side * head_r * 1.11, -head_r * 0.29);
        ear_gold.line_to(side * head_r * 0.98, -head_r * 0.14);
        ear_gold.close();
        ear_gold.paint(rgba(0xFACC15FF), window);
    }

    // 3) 额心鎏金王冠折带 + 中央钛银头冠脊
    let mut brow = c.fill();
    brow.move_to(-r * 0.56, -head_r * 0.84);
    brow.line_to(-r * 0.24, -head_r * 0.96);
    brow.line_to(r * 0.24, -head_r * 0.96);
    brow.line_to(r * 0.56, -head_r * 0.84);
    brow.line_to(r * 0.44, -head_r * 0.78);
    brow.line_to(0.0, -head_r * 0.86);
    brow.line_to(-r * 0.44, -head_r * 0.78);
    brow.close();
    brow.paint(rgba(0xFACC15FF), window);

    let mut crest = c.fill();
    crest.move_to(-r * 0.18, -head_r * 0.90);
    crest.line_to(-r * 0.14, -head_r * 1.24);
    crest.line_to(r * 0.14, -head_r * 1.24);
    crest.line_to(r * 0.18, -head_r * 0.90);
    crest.line_to(0.0, -head_r * 0.82);
    crest.close();
    crest.paint(rgba(0xE2E8F0FF), window);

    c.fill_pill(0.0, -head_r * 1.08, r * 0.045, r * 0.12, rgba(0xDC2626FF), window);
    c.fill_circle(0.0, -head_r * 0.90, (r * 0.065).max(1.0), rgba_alpha(0xF97316, 0.95 * pulse), window);
}

/// 52. 阿尔茜双侧流线背翼舱 (后景层)
fn draw_arcee_pods_back(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let pulse = 0.75 + 0.25 * (time_sec * 3.4).sin();
    for side in [-1.0_f32, 1.0] {
        let w = c.sub(side * r * 0.74, -r * 0.16, side * 0.05);
        let mut pod = w.fill();
        pod.move_to(0.0, -r * 0.35);
        pod.line_to(side * r * 0.58, -r * 0.98);
        pod.line_to(side * r * 0.68, -r * 0.74);
        pod.line_to(side * r * 0.48, -r * 0.12);
        pod.line_to(0.0, 0.0);
        pod.close();
        pod.paint(rgba(0xF472B6FF), window);

        let mut trim = w.fill();
        trim.move_to(side * r * 0.12, -r * 0.32);
        trim.line_to(side * r * 0.48, -r * 0.82);
        trim.line_to(side * r * 0.54, -r * 0.65);
        trim.line_to(side * r * 0.26, -r * 0.15);
        trim.close();
        trim.paint(rgba(0xF8FAFCFF), window);

        w.fill_circle(side * r * 0.52, -r * 0.82, (r * 0.045).max(0.6), rgba_alpha(0x38BDF8, 0.92 * pulse), window);
    }
}

/// 53. 阿尔茜 · 赛博双髻盔 (前景层)
fn draw_arcee_helm(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let head_r = c.head_r;
    let pulse = 0.78 + 0.22 * (time_sec * 3.4).sin();

    // 1) 珍珠白半幅底座
    let steps = 24;
    let a_left = -0.94 * PI;
    let a_right = -0.06 * PI;
    let mut dome = c.fill();
    for i in 0..=steps {
        let f = i as f32 / steps as f32;
        let a = a_left + (a_right - a_left) * f;
        let rr = head_r * (1.055 + 0.065 * (f * PI).sin());
        let (s, co) = a.sin_cos();
        if i == 0 { dome.move_to(co * rr, s * rr); } else { dome.line_to(co * rr, s * rr); }
    }
    for i in (0..=steps).rev() {
        let f = i as f32 / steps as f32;
        let a = a_left + (a_right - a_left) * f;
        let rr = head_r * (0.885 + 0.012 * (f * PI).sin());
        let (s, co) = a.sin_cos();
        dome.line_to(co * rr, s * rr);
    }
    dome.close();
    dome.paint(rgba(0xF8FAFCFF), window);

    // 2) 樱花粉覆顶冠层
    let p_left = -0.82 * PI;
    let p_right = -0.18 * PI;
    let mut pink = c.fill();
    for i in 0..=18 {
        let f = i as f32 / 18.0;
        let a = p_left + (p_right - p_left) * f;
        let rr = head_r * (1.07 + 0.075 * (f * PI).sin());
        let (s, co) = a.sin_cos();
        if i == 0 { pink.move_to(co * rr, s * rr); } else { pink.line_to(co * rr, s * rr); }
    }
    for i in (0..=18).rev() {
        let f = i as f32 / 18.0;
        let a = p_left + (p_right - p_left) * f;
        let rr = head_r * (0.94 + 0.03 * (f * PI).sin());
        let (s, co) = a.sin_cos();
        pink.line_to(co * rr, s * rr);
    }
    pink.close();
    pink.paint(rgba(0xF472B6FF), window);

    // 3) 标志性双侧赛博机械圆润耳包 (Side Buns)
    for side in [-1.0_f32, 1.0] {
        let bun = c.sub(side * head_r * 1.03, -head_r * 0.25, side * 0.16);
        bun.fill_pill(0.0, 0.0, r * 0.21, r * 0.31, rgba(0xEC4899FF), window);
        bun.fill_pill(side * r * 0.02, 0.0, r * 0.13, r * 0.21, rgba(0xF8FAFCFF), window);
        bun.fill_pill(side * r * 0.04, 0.0, r * 0.075, r * 0.13, rgba(0x1E293BFF), window);
        bun.stroke_arc(side * r * 0.04, 0.0, r * 0.055, r * 0.10, -PI * 0.65, PI * 0.65, (r * 0.020).max(0.45), rgba_alpha(0x7DD3FC, 0.88 * pulse), window);
    }

    // 4) 额心粉红顶饰与冰蓝晶核
    let mut v_crest = c.fill();
    v_crest.move_to(-r * 0.16, -head_r * 0.92);
    v_crest.line_to(-r * 0.10, -head_r * 1.18);
    v_crest.line_to(0.0, -head_r * 1.22);
    v_crest.line_to(r * 0.10, -head_r * 1.18);
    v_crest.line_to(r * 0.16, -head_r * 0.92);
    v_crest.line_to(0.0, -head_r * 0.84);
    v_crest.close();
    v_crest.paint(rgba(0xF472B6FF), window);

    c.fill_circle(0.0, -head_r * 0.96, (r * 0.055).max(0.9), rgba_alpha(0x7DD3FC, 0.95 * pulse), window);
}

/// 54. 救护车 · 医官战盔 (前景层)
fn draw_ratchet_helm(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let head_r = c.head_r;
    let beacon_l = 0.45 + 0.55 * (time_sec * 5.2).sin().max(0.0);
    let beacon_r = 0.45 + 0.55 * (time_sec * 5.2 + PI).sin().max(0.0);

    // 1) 侧方通讯耳甲
    for side in [-1.0_f32, 1.0] {
        let ear = c.sub(side * head_r * 0.99, -head_r * 0.12, 0.0);
        ear.fill_pill(side * r * 0.04, 0.0, r * 0.08, r * 0.18, rgba(0xE2E8F0FF), window);
        ear.fill_pill(if side > 0.0 { r * 0.04 } else { -r * 0.04 }, 0.0, r * 0.04, r * 0.18, rgba(0xDC2626FF), window);
        ear.fill_circle(side * r * 0.05, -r * 0.14, (r * 0.025).max(0.5), rgba(0x2DD4BFFF), window);
    }

    // 2) 陶瓷白半幅战盔
    let steps = 24;
    let a_left = -0.94 * PI;
    let a_right = -0.06 * PI;
    let mut dome = c.fill();
    for i in 0..=steps {
        let f = i as f32 / steps as f32;
        let a = a_left + (a_right - a_left) * f;
        let rr = head_r * (1.055 + 0.065 * (f * PI).sin());
        let (s, co) = a.sin_cos();
        if i == 0 { dome.move_to(co * rr, s * rr); } else { dome.line_to(co * rr, s * rr); }
    }
    for i in (0..=steps).rev() {
        let f = i as f32 / steps as f32;
        let a = a_left + (a_right - a_left) * f;
        let rr = head_r * (0.88 + 0.014 * (f * PI).sin());
        let (s, co) = a.sin_cos();
        dome.line_to(co * rr, s * rr);
    }
    dome.close();
    dome.paint(rgba(0xF8FAFCFF), window);

    // 3) 左右救护车红/青交替警灯
    for side in [-1.0_f32, 1.0] {
        let b = c.sub(side * head_r * 0.56, -head_r * 0.97, side * 0.44);
        b.fill_pill(0.0, 0.0, r * 0.09, r * 0.035, rgba(0x334155FF), window);
        let b_glow = if side < 0.0 { beacon_l } else { beacon_r };
        let b_col = if side < 0.0 { 0xEF4444 } else { 0x38BDF8 };
        b.fill_pill(0.0, -r * 0.04, r * 0.075, r * 0.032, rgba_alpha(b_col, 0.65 + 0.35 * b_glow), window);
    }

    // 4) 标志性鲜红大 V 字医官额饰冠角 + 医疗小红十字
    for side in [-1.0_f32, 1.0] {
        let mut v_wing = c.fill();
        v_wing.move_to(0.0, -head_r * 0.87);
        v_wing.line_to(side * r * 0.45, -head_r * 1.26);
        v_wing.line_to(side * r * 0.38, -head_r * 1.32);
        v_wing.line_to(0.0, -head_r * 1.00);
        v_wing.close();
        v_wing.paint(rgba(0xEF4444FF), window);
    }

    let mut clasp = c.fill();
    clasp.move_to(-r * 0.10, -head_r * 1.02);
    clasp.line_to(r * 0.10, -head_r * 1.02);
    clasp.line_to(r * 0.08, -head_r * 0.85);
    clasp.line_to(0.0, -head_r * 0.81);
    clasp.line_to(-r * 0.08, -head_r * 0.85);
    clasp.close();
    clasp.paint(rgba(0xFFFFFFFF), window);

    c.fill_pill(0.0, -head_r * 0.93, r * 0.036, r * 0.10, rgba(0xDC2626FF), window);
    c.fill_pill(0.0, -head_r * 0.93, r * 0.10, r * 0.036, rgba(0xDC2626FF), window);
}

/// 55. 热破鎏金双尖赛车扰流尾翼 (后景层)
fn draw_hotrod_spoiler_back(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let pulse = 0.76 + 0.24 * (time_sec * 3.8).sin();

    // 1) 鎏金高位扰流横翼梁
    let mut bar = c.fill();
    bar.move_to(-r * 1.34, -r * 1.06);
    bar.line_to(r * 1.34, -r * 1.06);
    bar.line_to(r * 1.22, -r * 0.80);
    bar.line_to(-r * 1.22, -r * 0.80);
    bar.close();
    bar.paint(rgba(0xFACC15FF), window);

    // 2) 左右高挑锋利双尖尾翼
    for side in [-1.0_f32, 1.0] {
        let mut fin = c.fill();
        fin.move_to(side * r * 0.72, -r * 0.24);
        fin.line_to(side * r * 1.18, -r * 1.06);
        fin.line_to(side * r * 1.48, -r * 1.34);
        fin.line_to(side * r * 1.54, -r * 0.92);
        fin.line_to(side * r * 1.26, -r * 0.14);
        fin.line_to(side * r * 0.80, r * 0.08);
        fin.close();
        fin.paint(rgba(0xFACC15FF), window);

        c.stroke_line(side * r * 1.16, -r * 0.88, side * r * 1.41, -r * 1.14, (r * 0.024).max(0.6), rgba_alpha(0xF97316, 0.88 * pulse), window);
    }
}

/// 56. 热破 · 烈焰尾翼盔 (前景层)
fn draw_hotrod_helm(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let head_r = c.head_r;
    let pulse = 0.78 + 0.22 * (time_sec * 3.6).sin();

    // 1) 镀铬排气管耳甲
    for side in [-1.0_f32, 1.0] {
        let p = c.sub(side * head_r * 0.99, -head_r * 0.10, 0.0);
        p.fill_pill(0.0, 0.0, r * 0.07, r * 0.20, rgba(0x1E293BFF), window);
        for off in [-r * 0.025, r * 0.035] {
            p.fill_pill(side * off, 0.0, r * 0.022, r * 0.16, rgba(0xE2E8F0FF), window);
        }
    }

    // 2) 炽红半幅流线跑车战盔
    let steps = 24;
    let a_left = -0.94 * PI;
    let a_right = -0.06 * PI;
    let mut dome = c.fill();
    for i in 0..=steps {
        let f = i as f32 / steps as f32;
        let a = a_left + (a_right - a_left) * f;
        let rr = head_r * (1.055 + 0.068 * (f * PI).sin());
        let (s, co) = a.sin_cos();
        if i == 0 { dome.move_to(co * rr, s * rr); } else { dome.line_to(co * rr, s * rr); }
    }
    for i in (0..=steps).rev() {
        let f = i as f32 / steps as f32;
        let a = a_left + (a_right - a_left) * f;
        let rr = head_r * (0.882 + 0.014 * (f * PI).sin());
        let (s, co) = a.sin_cos();
        dome.line_to(co * rr, s * rr);
    }
    dome.close();
    dome.paint(rgba(0xDC2626FF), window);

    // 3) 额侧烈焰金橙拉花
    for side in [-1.0_f32, 1.0] {
        let mut flame = c.fill();
        flame.move_to(side * r * 0.24, -head_r * 0.98);
        flame.line_to(side * r * 0.76, -head_r * 0.64);
        flame.line_to(side * r * 0.54, -head_r * 0.78);
        flame.line_to(side * r * 0.22, -head_r * 0.89);
        flame.close();
        flame.paint(rgba(0xFACC15FF), window);
    }

    // 4) 中央三叉火焰顶饰
    let mut crest = c.fill();
    crest.move_to(-r * 0.20, -head_r * 0.89);
    crest.line_to(-r * 0.13, -head_r * 1.22);
    crest.line_to(0.0, -head_r * 1.15);
    crest.line_to(r * 0.13, -head_r * 1.22);
    crest.line_to(r * 0.20, -head_r * 0.89);
    crest.line_to(0.0, -head_r * 0.82);
    crest.close();
    crest.paint(rgba(0x991B1BFF), window);

    let mut tri = c.fill();
    tri.move_to(0.0, -head_r * 1.16);
    tri.line_to(r * 0.055, -head_r * 0.99);
    tri.line_to(r * 0.130, -head_r * 1.07);
    tri.line_to(r * 0.090, -head_r * 0.90);
    tri.line_to(0.0, -head_r * 0.85);
    tri.line_to(-r * 0.090, -head_r * 0.90);
    tri.line_to(-r * 0.130, -head_r * 1.07);
    tri.line_to(-r * 0.055, -head_r * 0.99);
    tri.close();
    tri.paint(rgba_alpha(0xFACC15, 0.95 * pulse), window);
}

/// 57. 大力神工程机械肩塔 (后景层)
fn draw_devastator_shoulders_back(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let pulse = 0.75 + 0.25 * (time_sec * 3.1).sin();
    for side in [-1.0_f32, 1.0] {
        let w = c.sub(side * r * 0.74, -r * 0.16, side * 0.02);
        // 工程毒绿重装肩塔
        let mut tower = w.fill();
        tower.move_to(0.0, -r * 0.32);
        tower.line_to(side * r * 0.35, -r * 0.96);
        tower.line_to(side * r * 0.72, -r * 0.90);
        tower.line_to(side * r * 0.76, -r * 0.18);
        tower.line_to(side * r * 0.60, r * 0.36);
        tower.line_to(0.0, r * 0.32);
        tower.close();
        tower.paint(rgba(0x84CC16FF), window);

        let mut purple = w.fill();
        purple.move_to(side * r * 0.16, -r * 0.26);
        purple.line_to(side * r * 0.40, -r * 0.76);
        purple.line_to(side * r * 0.60, -r * 0.72);
        purple.line_to(side * r * 0.62, -r * 0.18);
        purple.line_to(side * r * 0.44, r * 0.18);
        purple.close();
        purple.paint(rgba(0x7E22CEFF), window);

        // 履带刻线 + 红色指示灯
        for k in 0..3 {
            let ly = -r * 0.52 + k as f32 * r * 0.20;
            w.stroke_line(side * r * 0.28, ly, side * r * 0.58, ly + r * 0.04, (r * 0.024).max(0.6), rgba(0x18181BFF), window);
        }
        w.fill_circle(side * r * 0.54, -r * 0.81, (r * 0.045).max(0.6), rgba_alpha(0xEF4444, 0.88 * pulse), window);
    }
}

/// 58. 大力神 · 挖地虎重甲 (前景层)
fn draw_devastator_helm(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let head_r = c.head_r;
    let pulse = 0.78 + 0.22 * (time_sec * 3.1).sin();

    // 1) 侧方暗紫机械耳块与工程绿铆钉
    for side in [-1.0_f32, 1.0] {
        let ear = c.sub(side * head_r * 1.00, -head_r * 0.14, 0.0);
        ear.fill_pill(side * r * 0.05, 0.0, r * 0.08, r * 0.22, rgba(0x7E22CEFF), window);
        ear.fill_pill(if side > 0.0 { r * 0.05 } else { -r * 0.05 }, 0.0, r * 0.05, r * 0.16, rgba(0xA3E635FF), window);
    }

    // 2) 工程毒绿多棱角半幅重装战盔
    let steps = 24;
    let a_left = -0.95 * PI;
    let a_right = -0.05 * PI;
    let mut dome = c.fill();
    for i in 0..=steps {
        let f = i as f32 / steps as f32;
        let a = a_left + (a_right - a_left) * f;
        let rr = head_r * (1.065 + 0.072 * (f * PI).sin());
        let (s, co) = a.sin_cos();
        if i == 0 { dome.move_to(co * rr, s * rr); } else { dome.line_to(co * rr, s * rr); }
    }
    for i in (0..=steps).rev() {
        let f = i as f32 / steps as f32;
        let a = a_left + (a_right - a_left) * f;
        let rr = head_r * (0.88 + 0.014 * (f * PI).sin());
        let (s, co) = a.sin_cos();
        dome.line_to(co * rr, s * rr);
    }
    dome.close();
    dome.paint(rgba(0x84CC16FF), window);

    // 盔顶两侧炭黑护板
    for side in [-1.0_f32, 1.0] {
        let mut plate = c.fill();
        plate.move_to(side * r * 0.22, -head_r * 1.13);
        plate.line_to(side * r * 0.62, -head_r * 0.98);
        plate.line_to(side * r * 0.56, -head_r * 0.87);
        plate.line_to(side * r * 0.20, -head_r * 0.94);
        plate.close();
        plate.paint(rgba(0x18181BFF), window);
    }

    // 3) 阶梯重型暗紫顶冠 + 发光紫晶
    let mut crest = c.fill();
    crest.move_to(-r * 0.24, -head_r * 0.90);
    crest.line_to(-r * 0.19, -head_r * 1.24);
    crest.line_to(r * 0.19, -head_r * 1.24);
    crest.line_to(r * 0.24, -head_r * 0.90);
    crest.line_to(0.0, -head_r * 0.83);
    crest.close();
    crest.paint(rgba(0x7E22CEFF), window);

    c.fill_pill(0.0, -head_r * 1.18, r * 0.20, r * 0.05, rgba(0xA3E635FF), window);
    c.fill_circle(0.0, -head_r * 0.98, (r * 0.065).max(1.0), rgba_alpha(0xC084FC, 0.95 * pulse), window);
}

/// 59. 千斤顶拉力赛车肩翼 (后景层)
fn draw_wheeljack_wings_back(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let pulse = 0.72 + 0.28 * (time_sec * 4.5).sin();
    for side in [-1.0_f32, 1.0] {
        let w = c.sub(side * r * 0.74, -r * 0.14, side * 0.04);
        // 陶瓷白上扬肩翼
        let mut fin = w.fill();
        fin.move_to(0.0, -r * 0.26);
        fin.line_to(side * r * 0.46, -r * 0.94);
        fin.line_to(side * r * 0.64, -r * 0.80);
        fin.line_to(side * r * 0.52, 0.0);
        fin.line_to(0.0, r * 0.16);
        fin.close();
        fin.paint(rgba(0xF8FAFCFF), window);

        // Alitalia 绿 + 红拉力条纹
        w.stroke_line(side * r * 0.18, -r * 0.32, side * r * 0.50, -r * 0.70, (r * 0.035).max(0.7), rgba(0x16A34AFF), window);
        w.stroke_line(side * r * 0.22, -r * 0.14, side * r * 0.52, -r * 0.50, (r * 0.026).max(0.6), rgba(0xDC2626FF), window);
        w.fill_circle(side * r * 0.50, -r * 0.81, (r * 0.04).max(0.55), rgba_alpha(0x38BDF8, 0.88 * pulse), window);
    }
}

/// 60. 千斤顶 · 电光耳翼盔 (前景层)
fn draw_wheeljack_helm(c: &HeadCanvas, time_sec: f32, window: &mut Window) {
    let r = c.r;
    let head_r = c.head_r;
    let ear_pulse = 0.58 + 0.42 * (0.5 + 0.5 * (time_sec * 6.4).sin() * (time_sec * 2.3).cos());

    // 1) 经典半幅科学家战盔
    let steps = 24;
    let a_left = -0.94 * PI;
    let a_right = -0.06 * PI;
    let mut dome = c.fill();
    for i in 0..=steps {
        let f = i as f32 / steps as f32;
        let a = a_left + (a_right - a_left) * f;
        let rr = head_r * (1.055 + 0.065 * (f * PI).sin());
        let (s, co) = a.sin_cos();
        if i == 0 { dome.move_to(co * rr, s * rr); } else { dome.line_to(co * rr, s * rr); }
    }
    for i in (0..=steps).rev() {
        let f = i as f32 / steps as f32;
        let a = a_left + (a_right - a_left) * f;
        let rr = head_r * (0.882 + 0.014 * (f * PI).sin());
        let (s, co) = a.sin_cos();
        dome.line_to(co * rr, s * rr);
    }
    dome.close();
    dome.paint(rgba(0xF8FAFCFF), window);

    // 2) 意大利经典红/绿拉力赛车斜条纹
    for side in [-1.0_f32, 1.0] {
        let mut green = c.fill();
        green.move_to(side * r * 0.22, -head_r * 1.11);
        green.line_to(side * r * 0.44, -head_r * 1.04);
        green.line_to(side * r * 0.36, -head_r * 0.89);
        green.line_to(side * r * 0.18, -head_r * 0.93);
        green.close();
        green.paint(rgba(0x16A34AFF), window);

        let mut red = c.fill();
        red.move_to(side * r * 0.44, -head_r * 1.04);
        red.line_to(side * r * 0.62, -head_r * 0.92);
        red.line_to(side * r * 0.54, -head_r * 0.81);
        red.line_to(side * r * 0.36, -head_r * 0.89);
        red.close();
        red.paint(rgba(0xDC2626FF), window);
    }

    // 3) 中央发明家头脊与青蓝感应灯
    let mut crest = c.fill();
    crest.move_to(-r * 0.18, -head_r * 0.90);
    crest.line_to(-r * 0.13, -head_r * 1.22);
    crest.line_to(r * 0.13, -head_r * 1.22);
    crest.line_to(r * 0.18, -head_r * 0.90);
    crest.line_to(0.0, -head_r * 0.83);
    crest.close();
    crest.paint(rgba(0xE2E8F0FF), window);

    c.fill_pill(0.0, -head_r * 1.06, r * 0.05, r * 0.07, rgba_alpha(0x38BDF8, 0.92 * ear_pulse), window);

    // 4) 标志性律动发光半透明电光蓝大耳翼
    for side in [-1.0_f32, 1.0] {
        let ear = c.sub(side * head_r * 0.96, -head_r * 0.18, 0.0);
        ear.fill_pill(0.0, 0.0, r * 0.06, r * 0.22, rgba(0xF1F5F9FF), window);

        let mut fin = ear.fill();
        fin.move_to(side * r * 0.05, -r * 0.20);
        fin.line_to(side * r * 0.33, -r * 0.33);
        fin.line_to(side * r * 0.37, -r * 0.08);
        fin.line_to(side * r * 0.30, r * 0.23);
        fin.line_to(side * r * 0.06, r * 0.18);
        fin.close();
        fin.paint(rgba_alpha(0x38BDF8, 0.65 + 0.30 * ear_pulse), window);

        // 3 条律动声波灯栅
        for b in [-1.0_f32, 0.0, 1.0] {
            let by = b * r * 0.10 - r * 0.02;
            ear.stroke_line(side * r * 0.10, by, side * r * 0.28, by - r * 0.03, (r * 0.020).max(0.45), rgba_alpha(0xFFFFFF, 0.50 + 0.45 * ear_pulse), window);
        }
    }
}
