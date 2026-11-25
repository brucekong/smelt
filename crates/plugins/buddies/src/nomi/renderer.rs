//! NOMI 2D 矢量渲染核心（55 款官方表情 + 方案 2 大尺寸 + 原厂钛灰金属边框 + 原厂冷白眼神）。
//!
//! 行为设计：
//! - **静默态 (`Act::Rest`)**：确定性展示 55 款官方表情之一（27 款经典日常 + 28 款音乐季），
//!   静静陪伴、不乱晃，仅保留微弱呼吸与自然眨眼，列表一眼可辨每个会话个性；
//! - **运行态 (`Act::Work`)**：休眠/闭眼类表情自动唤醒，配合 4 种独立工作动态（左右转头 3D 透视、
//!   歪头晃脑、点头律动、舞台摇摆）与道具动画，直观区分正在工作中的会话。

use std::cell::Cell;
use std::f32::consts::PI;

use gpui::{
    Bounds, Corners, Edges, PaintQuad, PathBuilder, Pixels, Point, Rgba, Window,
    linear_color_stop, linear_gradient, point, px, rgba,
};

use super::genome::{
    NomiCyberOpticMode, NomiExpression, NomiGenome, NomiHeadwear, NomiLookMode, NomiWorkAnim,
    NomiWorkStyle,
};
use super::headwear::{self, HeadCanvas};
use super::physics::{NomiFaceChange, NomiFaceChangeStyle, NomiTransform};
use crate::buddy::Act;

/// 方案 2：基础半径占 36pt 画布比例（外圈直径约 35.5pt，饱满对齐 3D 小人）
const BASE_RADIUS_RATIO: f32 = 0.45;
/// 金属外边框宽度占半径比例
const BEZEL_WIDTH_RATIO: f32 = 0.095;
/// 垂直微下移偏移量（pt）：底座齐平小人脚底，顶部自然利用 44pt 行高上边距
const CENTER_Y_BIAS_PT: f32 = 1.2;

/// 工作中动态姿态参数
#[derive(Clone, Copy)]
struct WorkMotion {
    yaw: f32,
    pitch: f32,
    sway_x: f32,
    tilt: f32,
    pitch_y: f32,
    bob_y: f32,
    scale_x: f32,
    scale_y: f32,
    wink_left: bool,
    wink_right: bool,
}

impl Default for WorkMotion {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.0,
            sway_x: 0.0,
            tilt: 0.0,
            pitch_y: 0.0,
            bob_y: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            wink_left: false,
            wink_right: false,
        }
    }
}

#[inline]
fn scale_alpha(color: Rgba, factor: f32) -> Rgba {
    Rgba {
        a: (color.a * factor).clamp(0.0, 1.0),
        ..color
    }
}

/// 3D 切平面投影绘制上下文：在真实正交基底 `(ux, uy, vx, vy)` 上绘制无胶囊畸变的倾斜椭圆与圆弧
#[derive(Clone, Copy)]
struct ProjPlane {
    center: Point<Pixels>,
    ux: f32,
    uy: f32,
    vx: f32,
    vy: f32,
}

impl ProjPlane {
    #[inline]
    fn pt(&self, lx: f32, ly: f32) -> Point<Pixels> {
        point(
            self.center.x + px(lx * self.ux + ly * self.vx),
            self.center.y + px(lx * self.uy + ly * self.vy),
        )
    }

    fn fill_ellipse(&self, cx: f32, cy: f32, rx: f32, ry: f32, color: Rgba, window: &mut Window) {
        let steps = 40;
        let mut path = PathBuilder::fill();
        for i in 0..=steps {
            let theta = (i as f32 / steps as f32) * 2.0 * PI;
            let p = self.pt(cx + rx * theta.cos(), cy + ry * theta.sin());
            if i == 0 {
                path.move_to(p);
            } else {
                path.line_to(p);
            }
        }
        path.close();
        if let Ok(built) = path.build() {
            window.paint_path(built, color);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn stroke_arc(
        &self,
        rx: f32,
        ry: f32,
        start_angle: f32,
        end_angle: f32,
        width: Pixels,
        color: Rgba,
        window: &mut Window,
    ) {
        let steps = 36;
        let is_closed = (end_angle - start_angle).abs() >= 2.0 * PI - 1e-3;
        let count = if is_closed { steps } else { steps + 1 };
        let mut path = PathBuilder::stroke(width);
        for i in 0..count {
            let frac = i as f32 / steps as f32;
            let theta = start_angle + (end_angle - start_angle) * frac;
            let p = self.pt(rx * theta.cos(), ry * theta.sin());
            if i == 0 {
                path.move_to(p);
            } else {
                path.line_to(p);
            }
        }
        if is_closed {
            path.close();
        }
        if let Ok(built) = path.build() {
            window.paint_path(built, color);
        }
    }
}

#[inline]
fn mix_u64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// 4 项变脸方式的确定性洗牌排列（按 4 轮分块独立随机洗牌，保证各会话花式各异且均匀覆盖 4 种特效）
#[inline]
fn face_change_block_perm(seed: u64, block: usize) -> [usize; 4] {
    let mut perm = [0usize, 1, 2, 3];
    let mut h = mix_u64(seed ^ (block as u64).wrapping_mul(0xD6E8_FEB8_6659_FD93));
    for i in (1..4).rev() {
        let j = (h as usize) % (i + 1);
        perm.swap(i, j);
        h = mix_u64(h);
    }
    perm
}

/// 3D 运行态自动循环川剧变脸：按各会话基因独立错峰节拍 + 分块洗牌伪随机抽取（挥扇 / 机甲 / 甩头 / 眨眼），
/// 彻底避免多会话同时进行时齐步同步或花式雷同，同时保证同一会话绝不连续两轮重复同款特效。
fn compute_work_face_change(
    genome: &NomiGenome,
    time_sec: f32,
) -> (NomiExpression, Option<NomiFaceChange>) {
    let base_idx = NomiExpression::ALL
        .iter()
        .position(|&e| e == genome.expression)
        .unwrap_or(0);
    let hat_idx = NomiHeadwear::ALL
        .iter()
        .position(|&h| h == genome.headwear)
        .unwrap_or(0) as u64;
    let work_style_idx = NomiWorkStyle::ALL
        .iter()
        .position(|&s| s == genome.work_style)
        .unwrap_or(0) as u64;

    // 融合该会话的表情、配饰、性格相位与速率构造独立 64 位指纹
    let seed = mix_u64(
        (base_idx as u64)
            ^ hat_idx.wrapping_mul(0x9E37_79B9)
            ^ work_style_idx.wrapping_mul(0x517C_C1B7)
            ^ u64::from(genome.temper.phase.to_bits()).wrapping_mul(0xA5A5_5A5A)
            ^ u64::from(genome.temper.pace.to_bits()).wrapping_mul(0x3C6E_F372)
            ^ u64::from(genome.temper.spring_stiffness.to_bits()),
    );

    // 表情遍历步长同样按会话错开（全部与 53 互质，确保无重复遍历全部 53 款表情）
    const EXPR_STRIDES: [usize; 8] = [7, 11, 13, 17, 19, 23, 29, 31];
    let expr_stride = EXPR_STRIDES[((seed >> 32) as usize) % EXPR_STRIDES.len()];
    let expr_at = |step: usize| -> NomiExpression {
        let len = NomiExpression::ALL.len();
        NomiExpression::ALL[(base_idx + step * expr_stride) % len].wake_for_work()
    };

    // 每个会话拥有独立的变脸周期（1.70s ~ 2.70s）与初始相位，打破全局 2.0s 整齐节拍
    let dur_frac = ((seed & 0xFFFF) as f32) / 65535.0;
    let phase_frac = (((seed >> 16) & 0xFFFF) as f32) / 65535.0;
    let cycle_dur = 1.70 + dur_frac * 1.00;
    let anim_dur = 0.60_f32;
    let work_t = (time_sec + phase_frac * cycle_dur * 3.0).max(0.0);
    let cycle_idx = (work_t / cycle_dur).floor() as usize;
    let local_t = work_t - (cycle_idx as f32) * cycle_dur;

    let from_expr = expr_at(cycle_idx);
    let to_expr = expr_at(cycle_idx + 1);
    if local_t >= anim_dur {
        return (to_expr, None);
    }

    let styles = [
        NomiFaceChangeStyle::FanSwipe,
        NomiFaceChangeStyle::TransformSnap,
        NomiFaceChangeStyle::HeadWhip,
        NomiFaceChangeStyle::BlinkMorph,
    ];
    let block = cycle_idx / styles.len();
    let pos = cycle_idx % styles.len();
    let mut perm = face_change_block_perm(seed, block);
    // 跨块边界去重：若本块首项与上一块末项相同则交换 perm[0]/perm[1]（不改变 perm[3]，O(1) 严格保证永不连抽同款）
    if block > 0 && perm[0] == face_change_block_perm(seed, block - 1)[3] {
        perm.swap(0, 1);
    }
    let dir_hash = mix_u64(seed ^ (cycle_idx as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
    let fc = NomiFaceChange {
        from_expression: from_expr,
        from_headwear: genome.headwear,
        progress: (local_t / anim_dur).clamp(0.0, 1.0),
        style: styles[perm[pos]],
        direction: if (dir_hash & 1) == 0 { 1.0 } else { -1.0 },
    };
    let shown = if fc.progress < 0.5 { from_expr } else { to_expr };
    (shown, Some(fc))
}

/// 较强波峰压缩：让头部左右歪/看在节拍上干脆卡点并略微停留，呈现“贱萌”神态
#[inline]
fn cheeky_snap(angle: f32) -> f32 {
    let k = 2.1_f32;
    (angle.sin() * k).tanh() / k.tanh()
}

/// 原地脚底蹦迪曲线：快起轻落 + 触地瞬间 Q 弹微压扁（无左右跑步横移）
#[inline]
fn disco_hop(step_angle: f32, hop_pt: f32, squash_amp: f32) -> (f32, f32, f32) {
    let air = step_angle.sin().abs();
    // 较快起跳、顶部略作停顿的蹦迪弧线
    let hop_up = air.powf(0.58) * hop_pt;
    // 踩拍触地瞬间收紧压扁
    let land = (-air * 8.0).exp();
    let stretch = (air - 0.40).max(0.0) * 0.035;
    let scale_y = 1.0 - squash_amp * land + stretch;
    let scale_x = 1.0 + squash_amp * 0.85 * land - stretch * 0.7;
    (hop_up, scale_x, scale_y)
}

impl WorkMotion {
    fn compute(
        genome: &NomiGenome,
        look_mode: NomiLookMode,
        work_anim: NomiWorkAnim,
        act: Act,
        time_sec: f32,
    ) -> Self {
        if time_sec <= 0.0 {
            return Self::default();
        }

        match act {
            Act::Rest | Act::Call => Self::default(),
            // 选中打招呼 / 欢呼：原地轻快小弹蹦 + 微微歪头
            Act::Greet => {
                let (hop_up, sx, sy) = disco_hop(time_sec * 6.4, 1.55, 0.068);
                Self {
                    tilt: (time_sec * 6.4).sin() * 0.14,
                    bob_y: -hop_up,
                    scale_x: sx,
                    scale_y: sy,
                    ..Self::default()
                }
            }
            Act::Cheer => {
                let (hop_up, sx, sy) = disco_hop(time_sec * 8.4, 2.65, 0.11);
                Self {
                    yaw: (time_sec * 4.2).sin() * 0.30,
                    sway_x: (time_sec * 4.2).sin() * 0.75,
                    tilt: (time_sec * 8.4).sin() * 0.24,
                    bob_y: -hop_up,
                    scale_x: sx,
                    scale_y: sy,
                    ..Self::default()
                }
            }
            Act::Work => {
                if work_anim == NomiWorkAnim::FaceChange {
                    // 变脸模式改由 `compute_work_face_change` 驱动自动循环变脸特效，不叠加额外蹦跳
                    return Self::default();
                }

                // 节拍蹦跳模式：2D / 3D 均可使用（3D 保持圆球刚性 scale = 1.0，仅做立体弹跳与转头）
                let t = time_sec * genome.temper.pace + genome.temper.phase;
                let rigid_scale = |sx: f32, sy: f32| {
                    if look_mode == NomiLookMode::Face2D {
                        (sx, sy)
                    } else {
                        (1.0, 1.0)
                    }
                };
                match genome.work_style {
                    NomiWorkStyle::LookAround => {
                        let w = 6.0;
                        let snap = cheeky_snap(t * w);
                        let (hop_up, sx, sy) = disco_hop(t * w, 1.85, 0.080);
                        let (scale_x, scale_y) = rigid_scale(sx, sy);
                        Self {
                            yaw: snap * 0.48,
                            sway_x: snap * 0.65,
                            tilt: snap * 0.17,
                            bob_y: -hop_up,
                            scale_x,
                            scale_y,
                            wink_right: (t % 3.2) > 2.85,
                            ..Self::default()
                        }
                    }
                    NomiWorkStyle::GrooveTilt => {
                        let w = 6.4;
                        let snap = cheeky_snap(t * w);
                        let (hop_up, sx, sy) = disco_hop(t * w, 2.05, 0.090);
                        let (scale_x, scale_y) = rigid_scale(sx, sy);
                        Self {
                            yaw: (t * w).sin() * 0.34,
                            sway_x: snap * 0.60,
                            tilt: snap * 0.19,
                            bob_y: -hop_up,
                            scale_x,
                            scale_y,
                            ..Self::default()
                        }
                    }
                    NomiWorkStyle::BounceNod => {
                        let w = 6.2;
                        let snap = cheeky_snap(t * w * 0.5);
                        let (hop_up, sx, sy) = disco_hop(t * w, 1.95, 0.085);
                        let (scale_x, scale_y) = rigid_scale(sx, sy);
                        Self {
                            yaw: snap * 0.44,
                            sway_x: snap * 0.55,
                            tilt: snap * 0.14,
                            pitch_y: -hop_up * 0.28,
                            bob_y: -hop_up,
                            scale_x,
                            scale_y,
                            wink_left: (t % 3.0) > 2.65,
                            ..Self::default()
                        }
                    }
                    NomiWorkStyle::StageVibe => {
                        let w = 6.6;
                        let snap = cheeky_snap(t * w);
                        let (hop_up, sx, sy) = disco_hop(t * w, 2.15, 0.095);
                        let (scale_x, scale_y) = rigid_scale(sx, sy);
                        Self {
                            yaw: snap * 0.46,
                            sway_x: snap * 0.80,
                            tilt: cheeky_snap(t * w + 0.3) * 0.18,
                            bob_y: -hop_up,
                            scale_x,
                            scale_y,
                            wink_right: (t % 3.6) > 3.25,
                            ..Self::default()
                        }
                    }
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CyberOpticTheme {
    Bumblebee,
    Optimus,
    Megatron,
    Comms,
    Shockwave,
    Jazz,
    Drift,
    Grimlock,
    Arcee,
    Ratchet,
    Hotrod,
    Devastator,
    Wheeljack,
    Cyber,
}

fn resolve_cyber_optic_theme(
    headwear: NomiHeadwear,
    mode: NomiCyberOpticMode,
) -> Option<CyberOpticTheme> {
    if mode == NomiCyberOpticMode::Off {
        return None;
    }
    match headwear {
        NomiHeadwear::BumblebeeHelm => Some(CyberOpticTheme::Bumblebee),
        NomiHeadwear::OptimusCrest => Some(CyberOpticTheme::Optimus),
        NomiHeadwear::MegatronCannon => Some(CyberOpticTheme::Megatron),
        NomiHeadwear::CommsVisor => Some(CyberOpticTheme::Comms),
        NomiHeadwear::ShockwaveOptic => Some(CyberOpticTheme::Shockwave),
        NomiHeadwear::JazzVisor => Some(CyberOpticTheme::Jazz),
        NomiHeadwear::DriftSamurai => Some(CyberOpticTheme::Drift),
        NomiHeadwear::GrimlockHelm => Some(CyberOpticTheme::Grimlock),
        NomiHeadwear::ArceeHelm => Some(CyberOpticTheme::Arcee),
        NomiHeadwear::RatchetHelm => Some(CyberOpticTheme::Ratchet),
        NomiHeadwear::HotrodHelm => Some(CyberOpticTheme::Hotrod),
        NomiHeadwear::DevastatorHelm => Some(CyberOpticTheme::Devastator),
        NomiHeadwear::WheeljackHelm => Some(CyberOpticTheme::Wheeljack),
        NomiHeadwear::SeekerWings | NomiHeadwear::LaserbeakDrone => Some(CyberOpticTheme::Megatron), // 狂派猩红机械眼
        NomiHeadwear::MatrixPendant | NomiHeadwear::CybertronRing => Some(CyberOpticTheme::Cyber),
        _ => (mode == NomiCyberOpticMode::On).then_some(CyberOpticTheme::Cyber),
    }
}

/// 头部局部坐标变换上下文（统一处理 3D 球面正交投影、歪头倾角与眼神跟随）
struct FaceCtx {
    face_center: Point<Pixels>,
    r: f32,
    eye_dx: f32,
    eye_y: f32,
    hand_eye_y: f32,
    tilt: f32,
    yaw_sin: f32,
    ux: f32,
    uy: f32,
    vx: f32,
    vy: f32,
    left_w_scale: f32,
    right_w_scale: f32,
    expr_scale_x: f32,
    expr_scale_y: f32,
    glow: Rgba,
    cyber_theme: Option<CyberOpticTheme>,
    shockwave_drawn: Cell<bool>,
    is_blinking: bool,
    wink_left: bool,
    wink_right: bool,
    anim_t: f32,
}

impl FaceCtx {
    fn pt(&self, dx: f32, dy: f32) -> Point<Pixels> {
        let lx = dx * self.expr_scale_x;
        let ly = dy * self.expr_scale_y;
        point(
            self.face_center.x + px(lx * self.ux + ly * self.vx),
            self.face_center.y + px(lx * self.uy + ly * self.vy),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn try_cyber_eye(
        &self,
        dx: f32,
        dy: f32,
        eye_r: f32,
        tilt: f32,
        shutter_top: f32,
        happy_lift: bool,
        is_blinking: bool,
        alpha: f32,
        window: &mut Window,
    ) -> bool {
        let Some(theme) = self.cyber_theme else {
            return false;
        };
        let (target_dx, target_tilt, target_shutter, target_blink, target_r) =
            if theme == CyberOpticTheme::Shockwave {
                if self.shockwave_drawn.replace(true) {
                    return true;
                }
                (0.0, 0.0, 0.0, false, (eye_r * 1.24).max(self.r * 0.245))
            } else {
                (dx, tilt, shutter_top, is_blinking, eye_r)
            };
        let center = self.pt(target_dx, dy);
        draw_cyber_optic_unit(
            center,
            self.r,
            target_r,
            target_tilt,
            target_shutter,
            happy_lift,
            target_blink,
            alpha,
            self.anim_t,
            theme,
            window,
        );
        true
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_nomi(
    genome: &NomiGenome,
    look_mode: NomiLookMode,
    work_anim: NomiWorkAnim,
    transform: &NomiTransform,
    act: Act,
    time_sec: f32,
    bounds: Bounds<Pixels>,
    window: &mut Window,
) {
    let size = bounds.size.width.min(bounds.size.height).as_f32();
    let motion = WorkMotion::compute(genome, look_mode, work_anim, act, time_sec);

    let smooth01 = |t: f32| -> f32 {
        let c = t.clamp(0.0, 1.0);
        c * c * (3.0 - 2.0 * c)
    };
    let ease_out_back = |t: f32| -> f32 {
        let c = t.clamp(0.0, 1.0);
        let c1 = 1.70158_f32;
        let c3 = c1 + 1.0;
        1.0 + c3 * (c - 1.0).powi(3) + c1 * (c - 1.0).powi(2)
    };

    let (
        mut fc_yaw,
        mut fc_pitch,
        mut fc_tilt,
        mut fc_breath_y,
        mut fc_head_sx,
        mut fc_head_sy,
        mut fc_expr_sx,
        mut fc_expr_sy,
        mut fc_expr_dx,
        mut fc_blink,
    ) = (0.0_f32, 0.0_f32, 0.0_f32, 0.0_f32, 1.0_f32, 1.0_f32, 1.0_f32, 1.0_f32, 0.0_f32, false);

    let (work_expr_override, work_face_change) =
        if work_anim == NomiWorkAnim::FaceChange
            && act == Act::Work
            && time_sec > 0.0
            && transform.face_change.is_none()
            && !transform.is_squished
            && !transform.is_dizzy
        {
            let (expr, fc) = compute_work_face_change(genome, time_sec);
            (Some(expr), fc)
        } else {
            (None, None)
        };
    let active_face_change = transform.face_change.or(work_face_change);

    if let Some(fc) = active_face_change {
        let p = fc.progress.clamp(0.0, 1.0);
        let dir = fc.direction;
        match fc.style {
            NomiFaceChangeStyle::HeadWhip => {
                let whip_yaw = if p < 0.18 {
                    -dir * 0.14 * smooth01(p / 0.18)
                } else if p < 0.50 {
                    dir * (-0.14 + 0.82 * smooth01((p - 0.18) / 0.32))
                } else if p < 0.78 {
                    dir * (0.68 - 0.82 * smooth01((p - 0.50) / 0.28))
                } else {
                    -dir * 0.14 * (1.0 - smooth01((p - 0.78) / 0.22))
                };
                fc_yaw = whip_yaw;
                fc_tilt = whip_yaw * 0.24;
                fc_pitch = (p * 2.0 * PI).sin() * 0.08;
                fc_breath_y = -(p * PI).sin() * 0.55;
                let mid_flash = (-((p - 0.5) / 0.14).powi(2)).exp();
                fc_expr_sx = (1.0 - 0.45 * mid_flash).max(0.35);
                let shift_phase = if p < 0.5 {
                    -smooth01(p / 0.5)
                } else {
                    1.0 - smooth01((p - 0.5) / 0.5)
                };
                fc_expr_dx = shift_phase * dir * size * BASE_RADIUS_RATIO * 0.22;
            }
            NomiFaceChangeStyle::FanSwipe => {
                let sweep_arc = (p * PI).sin();
                fc_yaw = -dir * 0.20 * sweep_arc;
                fc_tilt = -dir * 0.08 * sweep_arc;
                fc_pitch = (p * 2.0 * PI).sin() * 0.05;
                if p < 0.5 {
                    let s1 = smooth01(p / 0.5);
                    fc_expr_sx = 1.0 - 0.15 * s1;
                    fc_expr_sy = 1.0 - 0.15 * s1;
                } else {
                    let s2 = ease_out_back((p - 0.5) / 0.5);
                    fc_expr_sx = 0.85 + 0.15 * s2;
                    fc_expr_sy = 0.85 + 0.15 * s2;
                }
            }
            NomiFaceChangeStyle::TransformSnap => {
                if p < 0.5 {
                    let s1 = smooth01(p / 0.5);
                    fc_head_sx = 1.0 - 0.035 * s1;
                    fc_head_sy = 1.0 - 0.035 * s1;
                    fc_yaw = (p * 38.0).sin() * 0.025 * (1.0 - s1 * 0.3);
                    fc_expr_sx = 1.0 - 0.22 * s1;
                    fc_expr_sy = 1.0 - 0.22 * s1;
                } else {
                    let t2 = (p - 0.5) / 0.5;
                    let recoil = (t2 * PI).sin() * (-t2 * 1.6).exp();
                    let pop = ease_out_back(t2);
                    fc_head_sx = 1.0 + 0.045 * recoil;
                    fc_head_sy = 1.0 + 0.045 * recoil;
                    fc_expr_sx = 0.78 + 0.22 * pop;
                    fc_expr_sy = 0.78 + 0.22 * pop;
                }
            }
            NomiFaceChangeStyle::BlinkMorph => {
                if p < 0.5 {
                    let s1 = smooth01(p / 0.5);
                    fc_head_sx = 1.0 + 0.10 * s1;
                    fc_head_sy = 1.0 - 0.13 * s1;
                    fc_breath_y = 0.85 * s1;
                    fc_pitch = 0.20 * s1;
                    fc_expr_sy = (1.0 - 0.90 * s1).max(0.10);
                    fc_expr_sx = 1.0 + 0.14 * s1;
                    fc_blink = p >= 0.34;
                } else {
                    let t2 = (p - 0.5) / 0.5;
                    let spring = (t2 * PI).sin() * (-t2 * 1.3).exp();
                    let pop = ease_out_back(t2);
                    fc_head_sx = 1.0 - 0.07 * spring;
                    fc_head_sy = 1.0 + 0.09 * spring;
                    fc_breath_y = -0.55 * spring;
                    fc_pitch = -0.11 * spring;
                    fc_expr_sy = pop.max(0.10);
                    fc_expr_sx = 1.12 - 0.12 * pop;
                    fc_blink = p <= 0.58;
                }
            }
        }
    }

    let breath_y = if transform.is_squished || time_sec == 0.0 || act == Act::Work {
        0.0
    } else {
        (time_sec * 1.8 * genome.temper.pace + genome.temper.phase).sin() * 0.22
    };

    let base_r = size * BASE_RADIUS_RATIO;
    let rx = px(base_r * transform.scale_x * motion.scale_x * fc_head_sx);
    let ry = px(base_r * transform.scale_y * motion.scale_y * fc_head_sy);
    // 脚底锚点补偿：压扁时圆底贴稳地面、顶部下沉，产生真实的“脚底弹性弹跳”感
    let foot_anchor_dy = (1.0 - transform.scale_y * motion.scale_y * fc_head_sy) * base_r;

    let origin_x = bounds.origin.x + bounds.size.width / 2.0 + px(transform.offset_x + motion.sway_x);
    let origin_y = bounds.origin.y
        + bounds.size.height / 2.0
        + px(CENTER_Y_BIAS_PT + transform.offset_y + breath_y + motion.bob_y + fc_breath_y + foot_anchor_dy);

    let rx_val = rx.as_f32();
    let ry_val = ry.as_f32();
    let r = (rx_val + ry_val) * 0.5;
    let glow_color = genome.glow.color();

    // 3D 模式下给每只静止态 NOMI 赋予微弱固有立体偏转角（对齐 Web 端克制 3/4 透视）；hover 时平滑过渡
    let (rest_yaw, rest_pitch) = if look_mode == NomiLookMode::Head3D
        && act != Act::Work
        && active_face_change.is_none()
    {
        let phase_norm = (genome.temper.phase / std::f32::consts::TAU).rem_euclid(1.0);
        let sign = if ((genome.temper.phase * 10.0) as i32) & 1 == 0 {
            1.0
        } else {
            -1.0
        };
        let gaze_fade = (1.0 - transform.gaze_x.abs().max(transform.gaze_y.abs())).clamp(0.0, 1.0);
        let base_yaw = sign * (0.10 + phase_norm * 0.08);
        let base_pitch = ((genome.temper.pace - 0.98) * 0.9).clamp(-0.07, 0.07);
        (base_yaw * gaze_fade, base_pitch * gaze_fade)
    } else {
        (0.0, 0.0)
    };

    // 按注视模式计算鼠标互动：3D 按 15° 转头跟随幅度（与 Web 端 headFollowDeg=15° 比例一致）；2D 保持表情平移跟随
    let hover_follow = 15.0_f32.to_radians();
    let (mouse_yaw, mouse_pitch, mouse_tilt, gaze_x, gaze_y) = match look_mode {
        NomiLookMode::Head3D => (
            transform.gaze_x * hover_follow,
            transform.gaze_y * hover_follow,
            transform.gaze_x * hover_follow * 0.44,
            0.0,
            0.0,
        ),
        NomiLookMode::Face2D => (
            0.0,
            0.0,
            0.0,
            transform.gaze_x * rx_val * 0.23,
            transform.gaze_y * ry_val * 0.19,
        ),
    };

    let persp = 1.00_f32;
    let total_yaw = (rest_yaw + motion.yaw + mouse_yaw + fc_yaw).clamp(-0.75, 0.75) * persp;
    let total_pitch = (rest_pitch + motion.pitch + mouse_pitch + fc_pitch).clamp(-0.65, 0.65) * persp;
    let total_tilt = motion.tilt + mouse_tilt + fc_tilt;
    let (sin_y, cos_y) = total_yaw.sin_cos();
    let (sin_p, cos_p) = total_pitch.sin_cos();
    let (ts, tc) = total_tilt.sin_cos();
    let yaw_sin = sin_y;
    let show_3d = look_mode == NomiLookMode::Head3D || fc_yaw.abs() > 0.01 || fc_pitch.abs() > 0.01;

    // 真实 3D 球面切平面正交基底与球面外法线（与 Web 端投影矩阵一致，无直边胶囊畸变）
    let ux0 = cos_y;
    let uy0 = 0.0_f32;
    let vx0 = -sin_y * sin_p;
    let vy0 = cos_p;
    let nx0 = sin_y * cos_p;
    let ny0 = sin_p;

    let sx_scale = rx_val / r.max(1e-3);
    let sy_scale = ry_val / r.max(1e-3);
    let ux = (ux0 * tc - uy0 * ts) * sx_scale;
    let uy = (ux0 * ts + uy0 * tc) * sy_scale;
    let vx = (vx0 * tc - vy0 * ts) * sx_scale;
    let vy = (vx0 * ts + vy0 * tc) * sy_scale;
    let nx = nx0 * tc - ny0 * ts;
    let ny = nx0 * ts + ny0 * tc;

    // 3D 外环保证 36pt 小尺寸下全 360° 清晰可辨，同时预留安全前移深度不顶边
    let bezel_w = if show_3d {
        r * 0.082
    } else {
        r * BEZEL_WIDTH_RATIO
    };
    let ring_w = px(bezel_w);
    let outer_r = r + bezel_w;
    let shell_r = r * 1.155;
    let max_safe_depth = (shell_r * shell_r - outer_r * outer_r).max(0.0).sqrt();
    let face_depth = (r * 0.48).min(max_safe_depth * 0.96);
    let (head_shift_x, head_shift_y) = if show_3d {
        (nx * face_depth, ny * face_depth)
    } else {
        (0.0, 0.0)
    };
    let center_x = origin_x + px(head_shift_x);
    let center_y = origin_y + px(head_shift_y);

    let (selected_expr, selected_headwear) = if let Some(fc) = transform.face_change {
        if fc.progress < 0.5 {
            (fc.from_expression, fc.from_headwear)
        } else {
            (genome.expression, genome.headwear)
        }
    } else if let Some(work_expr) = work_expr_override {
        (work_expr, genome.headwear)
    } else {
        (genome.expression, genome.headwear)
    };

    // ==========================================
    // 0.0 后景配饰层（置于球体后壳与外圈后方：虎尾、混天绫、恶魔翼尾、天使羽翼、行星星轨背弧、伴飞无人机后景）
    // ==========================================
    let back_canvas = HeadCanvas {
        center: point(origin_x, origin_y),
        r,
        head_r: outer_r,
        ux: tc,
        uy: ts,
        vx: -ts,
        vy: tc,
    };
    headwear::draw_headwear_back(selected_headwear, &back_canvas, time_sec, glow_color, total_pitch, window);

    // ==========================================
    // 0. 3D 球体哑光深邃炭黑后壳（对齐 Web 端径向球面明暗：中心偏暗炭黑，仅左上保留克制缘光）
    // ==========================================
    if show_3d {
        let shell_alpha = if look_mode == NomiLookMode::Head3D {
            1.0
        } else {
            let rot_mag = total_yaw.hypot(total_pitch);
            let t = ((rot_mag - 0.025) / 0.095).clamp(0.0, 1.0);
            t * t * (3.0 - 2.0 * t)
        };
        if shell_alpha > 0.01 {
            let shell_rx = px(shell_r * sx_scale);
            let shell_ry = px(shell_r * sy_scale);
            let a_byte = (shell_alpha * 255.0) as u32;
            window.paint_quad(PaintQuad {
                bounds: Bounds {
                    origin: point(origin_x - shell_rx, origin_y - shell_ry),
                    size: gpui::size(shell_rx * 2.0, shell_ry * 2.0),
                },
                corner_radii: Corners::all(shell_rx.max(shell_ry)),
                background: linear_gradient(
                    135.0,
                    linear_color_stop(rgba((0x20232B << 8) | a_byte), 0.0),
                    linear_color_stop(rgba((0x0A0B0E << 8) | a_byte), 1.0),
                ),
                border_widths: Edges::all(px(0.75)),
                border_color: rgba((0x565C68 << 8) | ((shell_alpha * 115.0) as u32)).into(),
                border_style: gpui::BorderStyle::Solid,
            });
            // 叠加左上球面柔和哑光高光点，模拟 Web 端径向球体渐变而不把整半边染灰
            let spec_r_x = px(shell_r * 0.54 * sx_scale);
            let spec_r_y = px(shell_r * 0.54 * sy_scale);
            let spec_cx = origin_x - px(shell_r * 0.24 * sx_scale);
            let spec_cy = origin_y - px(shell_r * 0.24 * sy_scale);
            window.paint_quad(PaintQuad {
                bounds: Bounds {
                    origin: point(spec_cx - spec_r_x, spec_cy - spec_r_y),
                    size: gpui::size(spec_r_x * 2.0, spec_r_y * 2.0),
                },
                corner_radii: Corners::all(spec_r_x.max(spec_r_y)),
                background: linear_gradient(
                    135.0,
                    linear_color_stop(rgba((0x3E434E << 8) | ((shell_alpha * 54.0) as u32)), 0.0),
                    linear_color_stop(rgba(0x00000000), 1.0),
                ),
                border_widths: Edges::all(px(0.0)),
                border_color: rgba(0x00000000).into(),
                border_style: gpui::BorderStyle::Solid,
            });
        }
    }

    // ==========================================
    // 1. 外圈金属边框工艺 & 2. AMOLED 纯黑屏幕核心
    // ==========================================
    let work_pulse = if act == Act::Work && time_sec > 0.0 {
        0.5 + 0.5 * (time_sec * 4.5 + genome.temper.phase).sin()
    } else {
        0.0
    };

    if show_3d {
        // 真实 3D 切平面向量椭圆投影：金属环、倒角壁、黑屏与顶部弧形反光均与表情 100% 同平面
        let front_plane = ProjPlane {
            center: point(center_x, center_y),
            ux,
            uy,
            vx,
            vy,
        };
        if head_shift_x.hypot(head_shift_y) > 0.20 {
            let wall_plane = ProjPlane {
                center: point(
                    center_x - px(nx * bezel_w * 0.50),
                    center_y - px(ny * bezel_w * 0.50),
                ),
                ux,
                uy,
                vx,
                vy,
            };
            wall_plane.fill_ellipse(0.0, 0.0, outer_r, outer_r, genome.bezel.ring_dark(), window);
            wall_plane.stroke_arc(
                outer_r,
                outer_r,
                0.0,
                2.0 * PI,
                px(0.60),
                rgba(0x00000070),
                window,
            );
        }

        // 外圈精工金属环：中灰底 + 随偏航/俯仰联动扫光弧 + 背光暗面弧 + 内倒角亮弧
        front_plane.fill_ellipse(0.0, 0.0, outer_r, outer_r, genome.bezel.ring_mid(), window);
        let light_angle = -0.75 * PI + total_yaw * 0.42 - total_pitch * 0.36;
        let ring_mid_r = r + bezel_w * 0.50;
        front_plane.stroke_arc(
            ring_mid_r,
            ring_mid_r,
            light_angle - 0.62 * PI,
            light_angle + 0.62 * PI,
            px(bezel_w * 0.96),
            scale_alpha(genome.bezel.ring_light(), 0.72),
            window,
        );
        front_plane.stroke_arc(
            ring_mid_r,
            ring_mid_r,
            light_angle - 0.28 * PI,
            light_angle + 0.28 * PI,
            px(bezel_w * 0.92),
            scale_alpha(genome.bezel.ring_light(), 0.96),
            window,
        );
        front_plane.stroke_arc(
            ring_mid_r,
            ring_mid_r,
            light_angle + 0.60 * PI,
            light_angle + 1.40 * PI,
            px(bezel_w * 0.88),
            scale_alpha(genome.bezel.ring_dark(), 0.40),
            window,
        );
        // 补齐 360° 金属外轮廓描边，确保背光侧（右下方）依然与深色球壳清晰分离、不会“消失一半”
        front_plane.stroke_arc(
            outer_r - 0.25,
            outer_r - 0.25,
            0.0,
            2.0 * PI,
            px(0.65),
            genome.bezel.ring_mid(),
            window,
        );
        front_plane.stroke_arc(
            r + 0.35,
            r + 0.35,
            light_angle - 0.48 * PI,
            light_angle + 0.48 * PI,
            px(0.75),
            genome.bezel.ring_highlight(),
            window,
        );

        if act == Act::Work && time_sec > 0.0 {
            let sweep_start = time_sec * 4.4 + genome.temper.phase;
            front_plane.stroke_arc(
                ring_mid_r,
                ring_mid_r,
                sweep_start,
                sweep_start + 0.65 * PI,
                px((bezel_w * 0.88).max(1.10)),
                scale_alpha(glow_color, 0.48 + 0.30 * work_pulse),
                window,
            );
            front_plane.stroke_arc(
                ring_mid_r,
                ring_mid_r,
                sweep_start + 0.46 * PI,
                sweep_start + 0.68 * PI,
                px((bezel_w * 1.08).max(1.30)),
                scale_alpha(glow_color, 0.86 + 0.12 * work_pulse),
                window,
            );
            front_plane.stroke_arc(
                ring_mid_r,
                ring_mid_r,
                sweep_start + PI,
                sweep_start + 1.32 * PI,
                px((bezel_w * 0.55).max(0.80)),
                scale_alpha(glow_color, 0.24 + 0.14 * work_pulse),
                window,
            );
        }

        // 纯黑 AMOLED 屏幕 + 内嵌边缘暗影（严格收在黑屏内侧，不压占金属环）
        front_plane.fill_ellipse(0.0, 0.0, r, r, rgba(0x06070AFF), window);
        front_plane.stroke_arc(
            (r - 0.55).max(1.0),
            (r - 0.55).max(1.0),
            0.0,
            2.0 * PI,
            px(0.85),
            rgba(0x00000088),
            window,
        );

        // 球面高透玻璃穹顶柔光与顶部弧面高光条（对齐 Web 端 drawGlassReflections）
        let glare_dx = -sin_y * r * 0.14;
        let glare_dy = sin_p * r * 0.12;
        front_plane.fill_ellipse(
            glare_dx * 0.6,
            -r * 0.16 + glare_dy * 0.6,
            r * 0.76,
            r * 0.64,
            rgba(0xFFFFFF07),
            window,
        );
        front_plane.fill_ellipse(
            glare_dx * 0.4,
            -r * 0.74 + glare_dy * 0.25,
            r * 0.52,
            r * 0.115,
            rgba(0xFFFFFF10),
            window,
        );
    } else {
        // 2D 平面正圆分支：保留原生 PaintQuad 线性渐变与硬件抗锯齿边框
        let bezel_rx = rx + ring_w;
        let bezel_ry = ry + ring_w;
        let bezel_light_angle = 135.0 + total_yaw * 32.0 - total_pitch * 26.0;
        window.paint_quad(PaintQuad {
            bounds: Bounds {
                origin: point(center_x - bezel_rx, center_y - bezel_ry),
                size: gpui::size(bezel_rx * 2.0, bezel_ry * 2.0),
            },
            corner_radii: Corners::all(bezel_rx.max(bezel_ry)),
            background: linear_gradient(
                bezel_light_angle,
                linear_color_stop(genome.bezel.ring_light(), 0.0),
                linear_color_stop(genome.bezel.ring_dark(), 1.0),
            ),
            border_widths: Edges::all(px(0.65)),
            border_color: genome.bezel.ring_mid().into(),
            border_style: gpui::BorderStyle::Solid,
        });

        draw_arc_stroke(
            point(center_x, center_y),
            rx_val + 0.35,
            ry_val + 0.35,
            -0.78 * PI,
            0.18 * PI,
            px(0.70),
            genome.bezel.ring_highlight(),
            window,
        );

        window.paint_quad(PaintQuad {
            bounds: Bounds {
                origin: point(center_x - rx, center_y - ry),
                size: gpui::size(rx * 2.0, ry * 2.0),
            },
            corner_radii: Corners::all(rx.max(ry)),
            background: rgba(0x06070AFF).into(),
            border_widths: Edges::all(px(0.0)),
            border_color: rgba(0x00000000).into(),
            border_style: gpui::BorderStyle::Solid,
        });

        window.paint_quad(PaintQuad {
            bounds: Bounds {
                origin: point(center_x - rx * 0.56, center_y - ry * 0.80),
                size: gpui::size(rx * 1.12, ry * 0.38),
            },
            corner_radii: Corners::all(rx * 0.56),
            background: linear_gradient(
                180.0,
                linear_color_stop(rgba(0xFFFFFF18), 0.0),
                linear_color_stop(rgba(0xFFFFFF00), 1.0),
            ),
            border_widths: Edges::all(px(0.0)),
            border_color: rgba(0x00000000).into(),
            border_style: gpui::BorderStyle::Solid,
        });
    }

    // ==========================================
    // 3. 55 款官方表情 (与 3D 球面屏幕严格保持在同一透视投影平面)
    // ==========================================
    let (work_eye_scan_x, work_eye_scan_y) = if show_3d
        && act == Act::Work
        && !transform.is_squished
        && !transform.is_dizzy
        && gaze_x.abs() < 0.01
    {
        (
            (motion.yaw * 0.15).clamp(-0.08, 0.08) * rx_val,
            (motion.pitch * 0.14).clamp(-0.07, 0.07) * ry_val,
        )
    } else {
        (0.0, 0.0)
    };
    let (proj_ux, proj_uy, proj_vx, proj_vy, base_eye_dx, left_w, right_w, fx, fy) = if show_3d {
        let depth_l = (1.0 - yaw_sin * 0.10).clamp(0.86, 1.08);
        let depth_r = (1.0 + yaw_sin * 0.10).clamp(0.86, 1.08);
        let w_base = cos_y.max(0.72);
        (
            ux,
            uy,
            vx,
            vy,
            r * 0.39,
            (w_base * depth_l).clamp(0.70, 1.05),
            (w_base * depth_r).clamp(0.70, 1.05),
            gaze_x + work_eye_scan_x + fc_expr_dx,
            motion.pitch_y + gaze_y + work_eye_scan_y,
        )
    } else {
        let yaw_cos_2d = cos_y.max(0.55);
        (
            tc,
            ts,
            -ts,
            tc,
            rx_val * 0.39 * yaw_cos_2d,
            (1.0 - (-yaw_sin).max(0.0) * 0.25).clamp(0.75, 1.0),
            (1.0 - yaw_sin.max(0.0) * 0.25).clamp(0.75, 1.0),
            motion.yaw.sin() * rx_val * 0.24 + gaze_x + fc_expr_dx,
            motion.pitch_y + gaze_y,
        )
    };

    let blink_period = if act == Act::Work { 3.2 } else { 4.2 };
    let fctx = FaceCtx {
        face_center: point(
            center_x + px(fx * proj_ux + fy * proj_vx),
            center_y + px(fx * proj_uy + fy * proj_vy),
        ),
        r,
        eye_dx: base_eye_dx,
        eye_y: -ry_val * 0.03,
        hand_eye_y: -ry_val * 0.17,
        tilt: total_tilt,
        yaw_sin,
        ux: proj_ux,
        uy: proj_uy,
        vx: proj_vx,
        vy: proj_vy,
        left_w_scale: left_w,
        right_w_scale: right_w,
        expr_scale_x: fc_expr_sx,
        expr_scale_y: fc_expr_sy,
        glow: glow_color,
        cyber_theme: resolve_cyber_optic_theme(selected_headwear, genome.cyber_optics),
        shockwave_drawn: Cell::new(false),
        is_blinking: fc_blink
            || (time_sec > 0.0 && ((time_sec + genome.temper.phase) % blink_period) < 0.13),
        wink_left: motion.wink_left,
        wink_right: motion.wink_right,
        anim_t: if matches!(act, Act::Work | Act::Cheer) {
            time_sec * 1.45
        } else {
            time_sec
        },
    };

    let active_expr = if matches!(act, Act::Work | Act::Cheer) {
        selected_expr.wake_for_work()
    } else {
        selected_expr
    };

    if transform.is_dizzy {
        let s = r * 0.135;
        draw_dizzy_eye(fctx.pt(-fctx.eye_dx, fctx.eye_y), s, glow_color, window);
        draw_dizzy_eye(fctx.pt(fctx.eye_dx, fctx.eye_y), s, glow_color, window);
    } else if transform.is_squished {
        let s = r * 0.135;
        draw_squish_eye(fctx.pt(-fctx.eye_dx, fctx.eye_y), s, true, glow_color, window);
        draw_squish_eye(fctx.pt(fctx.eye_dx, fctx.eye_y), s, false, glow_color, window);
    } else {
        draw_expression(active_expr, &fctx, window);
    }

    let hat_canvas = HeadCanvas {
        center: point(center_x, center_y),
        r,
        head_r: outer_r,
        ux: proj_ux,
        uy: proj_uy,
        vx: proj_vx,
        vy: proj_vy,
    };

    if selected_headwear == NomiHeadwear::Pirate {
        let patch_expr = if transform.is_dizzy || transform.is_squished {
            NomiExpression::Pleased
        } else {
            active_expr
        };
        headwear::draw_pirate_eyepatch_overlay(
            patch_expr,
            &hat_canvas,
            fctx.eye_dx,
            fctx.eye_y,
            fctx.hand_eye_y,
            fx,
            fy,
            fctx.expr_scale_x,
            fctx.expr_scale_y,
            window,
        );
    } else if selected_headwear == NomiHeadwear::JazzVisor {
        let visor_expr = if transform.is_dizzy || transform.is_squished {
            NomiExpression::Pleased
        } else {
            active_expr
        };
        headwear::draw_jazz_visor_overlay(
            visor_expr,
            &hat_canvas,
            fctx.eye_y,
            fctx.hand_eye_y,
            fx,
            fy,
            fctx.expr_scale_y,
            time_sec,
            window,
        );
    }

    if let Some(fc) = active_face_change {
        draw_face_change_screen_overlay(point(center_x, center_y), r, fc, glow_color, window);
    }

    // 状态外层提示（呼唤角标；进行中思考能量脉冲点；任务完成欢呼撒花）
    match act {
        Act::Call => {
            draw_alert_badge_bounded(center_x, center_y, rx_val, ry_val, bounds, time_sec, window);
        }
        Act::Work if time_sec > 0.0 => {
            draw_work_pulse_dots(
                center_x,
                center_y,
                rx_val,
                ry_val,
                bounds,
                time_sec + genome.temper.phase,
                glow_color,
                window,
            );
        }
        Act::Cheer => {
            draw_cheer_confetti(center_x, center_y, r, bounds, time_sec, window);
        }
        Act::Greet | Act::Work | Act::Rest => {}
    }

    // ==========================================
    // 4. 头部配饰 + 挥扇变脸前端遮面折扇
    // ==========================================
    let work_sprout_lag = if let Some(fc) = active_face_change {
        let wave = (fc.progress * std::f32::consts::PI).sin();
        -fc.direction * wave * 5.5
    } else {
        0.0
    };
    draw_headwear(
        selected_headwear,
        center_x,
        center_y,
        rx_val,
        ry_val,
        total_tilt,
        yaw_sin * r * 0.14,
        transform.sprout_angle + work_sprout_lag,
        &hat_canvas,
        time_sec,
        glow_color,
        total_pitch,
        window,
    );

    if let Some(fc) = active_face_change
        && fc.style == NomiFaceChangeStyle::FanSwipe
    {
        draw_face_change_fan(point(center_x, center_y), r, fc.progress, fc.direction, total_tilt, window);
    }
}

// ==========================================
// 55 款表情全量渲染路由
// ==========================================

fn draw_expression(expr: NomiExpression, f: &FaceCtx, window: &mut Window) {
    let r = f.r;
    let csize = r * 0.135;

    match expr {
        // === 1. 快乐与礼赞 (10) ===
        NomiExpression::OkGesture => {
            draw_eye_pair_crescent(f, f.hand_eye_y, csize, 0.0, window);
            draw_ok_hand(f.pt(-r * 0.32, r * 0.30), r * 0.54, f.tilt, window);
        }
        NomiExpression::Jubilation => {
            draw_eye_pair_crescent(f, f.hand_eye_y, csize, 0.0, window);
            let wave = (f.anim_t * 6.0).sin() * (r * 0.035);
            let wave_rot = (f.anim_t * 6.0).sin() * 0.08;
            draw_shy_palm_hand(f.pt(-r * 0.40, r * 0.24 + wave), r * 0.50, 1.0, f.tilt - 0.08 + wave_rot, window);
            draw_shy_palm_hand(f.pt(r * 0.40, r * 0.24 - wave), r * 0.50, -1.0, f.tilt + 0.24 - wave_rot, window);
        }
        NomiExpression::Proud => {
            draw_eye_pair_crescent(f, f.hand_eye_y, csize, 0.0, window);
            let proud_tilt = (f.anim_t * 4.2).sin() * 0.04;
            draw_thumbs_up_hand(f.pt(-r * 0.41, r * 0.27), r * 0.45, 1.0, f.tilt - 0.38 - proud_tilt, window);
            draw_thumbs_up_hand(f.pt(r * 0.41, r * 0.27), r * 0.45, -1.0, f.tilt + 0.38 + proud_tilt, window);
        }
        NomiExpression::Gratitude => {
            draw_eye_pair_crescent(f, f.hand_eye_y, csize, 0.0, window);
            draw_pray_hands(f.pt(r * 0.02, r * 0.30), r * 0.74, f.tilt, f.anim_t, window);
        }
        NomiExpression::Triumph => {
            draw_eye_pair_crescent(f, f.hand_eye_y, csize, 0.0, window);
            draw_mini_crown(f.pt(-f.eye_dx * 0.95, f.hand_eye_y - r * 0.34), r * 0.24, f.tilt - 0.18, window);
            draw_thumbs_up_hand(f.pt(-r * 0.38, r * 0.28), r * 0.50, 1.0, f.tilt - 0.28, window);
        }
        NomiExpression::Admiration => {
            draw_eye_pair_crescent(f, f.hand_eye_y, csize, 0.0, window);
            let cheek_sway = (f.anim_t * 3.2).sin() * 0.04;
            let cheek_lift = (f.anim_t * 3.2).cos() * (r * 0.014);
            draw_shy_palm_hand(f.pt(-r * 0.34, r * 0.25 + cheek_lift), r * 0.50, 1.0, f.tilt - 0.18 - cheek_sway, window);
            draw_shy_palm_hand(f.pt(r * 0.34, r * 0.25 + cheek_lift), r * 0.50, -1.0, f.tilt + 0.26 + cheek_sway, window);
            let star_pulse = 1.0 + (f.anim_t * 5.0).sin() * 0.15;
            draw_sparkle(f.pt(f.eye_dx * 1.05, f.hand_eye_y - r * 0.30), r * 0.10 * star_pulse, rgba(0xFFFFFFFF), window);
        }
        NomiExpression::Pleased => {
            draw_eye_pair_pill(f, f.eye_y, r * 0.095, r * 0.175, 0.0, window);
        }
        NomiExpression::Amused => {
            draw_eye_pair_crescent(f, f.eye_y, r * 0.14, 0.12, window);
        }
        NomiExpression::Excitement => {
            let jump_y = (f.anim_t * 7.0).sin().abs() * r * 0.045;
            draw_eye_pair_pill(f, f.eye_y - jump_y, r * 0.10, r * 0.19, 0.0, window);
        }

        // === 2. 惬意与休眠 (6) ===
        NomiExpression::Relaxed => {
            draw_glasses(f.pt(0.0, f.eye_y + r * 0.04), f.eye_dx, r * 0.22, f.tilt, f.glow, window);
            draw_sparkle(f.pt(-r * 0.12, f.eye_y - r * 0.34), r * 0.08, f.glow, window);
            draw_sparkle(f.pt(r * 0.14, f.eye_y - r * 0.42), r * 0.10, f.glow, window);
        }
        NomiExpression::Shy => {
            // 柔粉腮红外移微沉置于腕掌下方 + 捂眼小手轻晃周期
            let shy_cycle = 4.4_f32;
            let shy_t = if f.anim_t > 0.0 { f.anim_t % shy_cycle } else { 1.8 };
            let smooth = |v: f32| {
                let c = v.clamp(0.0, 1.0);
                c * c * (3.0 - 2.0 * c)
            };
            let cover_p = if shy_t < 0.85 {
                smooth(shy_t / 0.85)
            } else if shy_t < 3.55 {
                1.0
            } else {
                1.0 - smooth((shy_t - 3.55) / 0.85)
            };
            let blush_amt = smooth(((cover_p - 0.12) / 0.88).max(0.0));
            if blush_amt > 0.01 {
                let a = (blush_amt * 108.0) as u32;
                for sign in [-1.0_f32, 1.0_f32] {
                    let c = f.pt(sign * r * 0.54, f.hand_eye_y + r * 0.23);
                    let br = px(r * 0.145);
                    window.paint_quad(PaintQuad {
                        bounds: Bounds {
                            origin: point(c.x - br, c.y - br),
                            size: gpui::size(br * 2.0, br * 2.0),
                        },
                        corner_radii: Corners::all(br),
                        background: rgba((0xFF7387 << 8) | a).into(),
                        border_widths: Edges::all(px(0.0)),
                        border_color: rgba(0x00000000).into(),
                        border_style: gpui::BorderStyle::Solid,
                    });
                }
            }
            if cover_p < 0.54 {
                let eye_a_f = (1.0 - cover_p * 1.85).max(0.0);
                let eye_a = (eye_a_f * 255.0) as u32;
                let eye_col = rgba(((glow_rgb_u32(f.glow)) << 8) | eye_a);
                let eye_r_shy = (r * 0.165).max(csize * 1.15);
                if !f.try_cyber_eye(-r * 0.36, f.hand_eye_y + r * 0.02, eye_r_shy, f.tilt + 0.08, 0.0, true, false, eye_a_f, window) {
                    draw_crescent_eye(f.pt(-r * 0.36, f.hand_eye_y + r * 0.02), csize * 0.88 * f.left_w_scale, f.tilt + 0.08, eye_col, window);
                }
                if !f.try_cyber_eye(r * 0.36, f.hand_eye_y + r * 0.02, eye_r_shy, f.tilt - 0.08, 0.0, true, false, eye_a_f, window) {
                    draw_crescent_eye(f.pt(r * 0.36, f.hand_eye_y + r * 0.02), csize * 0.88 * f.right_w_scale, f.tilt - 0.08, eye_col, window);
                }
            }
            let peek = (f.anim_t * 3.8).sin() * r * 0.014 * cover_p;
            let sway_rot = (f.anim_t * 3.8).sin() * 0.045 * cover_p;
            let hx = r * (0.45 - 0.05 * cover_p);
            let hy = f.hand_eye_y + r * (0.28 - 0.33 * cover_p) + peek;
            let hrot = 1.18 + 0.37 * cover_p;
            draw_shy_palm_hand(f.pt(-hx, hy), r * 0.58, -1.0, f.tilt + hrot + sway_rot, window);
            draw_shy_palm_hand(f.pt(hx, hy), r * 0.58, 1.0, f.tilt - hrot - sway_rot, window);
        }
        NomiExpression::Acceptance => {
            draw_eye_pair_sleepy_arc(f, f.eye_y - r * 0.04, r * 0.14, window);
        }
        NomiExpression::Sleeping => {
            draw_eye_pair_sleepy_arc(f, f.eye_y + r * 0.05, r * 0.135, window);
            let z_float = if f.anim_t > 0.0 {
                (f.anim_t * 2.4).sin() * r * 0.035
            } else {
                0.0
            };
            draw_zzz(f.pt(-r * 0.06, f.eye_y - r * 0.24 - z_float), r * 0.28, window);
        }
        NomiExpression::Sleepy => {
            if !f.try_cyber_eye(-f.eye_dx, f.eye_y, r * 0.185, f.tilt, 0.50, false, false, 1.0, window) {
                draw_half_closed_eye(f.pt(-f.eye_dx, f.eye_y), r * 0.115 * f.left_w_scale, f.tilt, f.glow, window);
            }
            if !f.try_cyber_eye(f.eye_dx, f.eye_y, r * 0.185, f.tilt, 0.50, false, false, 1.0, window) {
                draw_half_closed_eye(f.pt(f.eye_dx, f.eye_y), r * 0.115 * f.right_w_scale, f.tilt, f.glow, window);
            }
        }
        NomiExpression::Tired => {
            if !f.try_cyber_eye(-f.eye_dx, f.eye_y, r * 0.185, f.tilt - 0.18, 0.34, false, false, 1.0, window) {
                draw_tilted_pill_eye(f.pt(-f.eye_dx, f.eye_y), r * 0.095 * f.left_w_scale, r * 0.175, f.tilt - 0.18, f.glow, window);
            }
            if !f.try_cyber_eye(f.eye_dx, f.eye_y, r * 0.185, f.tilt + 0.18, 0.34, false, false, 1.0, window) {
                draw_tilted_pill_eye(f.pt(f.eye_dx, f.eye_y), r * 0.095 * f.right_w_scale, r * 0.175, f.tilt + 0.18, f.glow, window);
            }
        }

        // === 3. 思考与探索 (11) ===
        NomiExpression::Dreamy => {
            let dream_float = (f.anim_t * 2.2).sin() * r * 0.012;
            if !f.try_cyber_eye(-r * 0.21, -r * 0.18 + dream_float, r * 0.19, f.tilt - 0.07, 0.0, false, false, 1.0, window) {
                draw_tilted_pill_eye(f.pt(-r * 0.21, -r * 0.18 + dream_float), r * 0.105 * f.left_w_scale, r * 0.195, f.tilt - 0.07, f.glow, window);
            }
            if !f.try_cyber_eye(r * 0.45, -r * 0.24 + dream_float, r * 0.19, f.tilt - 0.08, 0.0, false, false, 1.0, window) {
                draw_tilted_pill_eye(f.pt(r * 0.45, -r * 0.24 + dream_float), r * 0.102 * f.right_w_scale, r * 0.195, f.tilt - 0.08, f.glow, window);
            }
            draw_chin_rest_hand(f.pt(-r * 0.48, r * 0.27 - dream_float * 0.4), r * 0.60, f.tilt, window);
        }
        NomiExpression::Doubt => {
            if !f.try_cyber_eye(-f.eye_dx, f.eye_y + r * 0.04, r * 0.185, f.tilt - 0.22, 0.68, false, false, 1.0, window) {
                draw_tilted_pill_eye(f.pt(-f.eye_dx, f.eye_y + r * 0.04), r * 0.12 * f.left_w_scale, r * 0.055, f.tilt - 0.22, f.glow, window);
            }
            if !f.try_cyber_eye(f.eye_dx, f.eye_y - r * 0.04, r * 0.19, f.tilt - 0.12, 0.0, false, false, 1.0, window) {
                draw_tilted_pill_eye(f.pt(f.eye_dx, f.eye_y - r * 0.04), r * 0.095 * f.right_w_scale, r * 0.18, f.tilt - 0.12, f.glow, window);
            }
        }
        NomiExpression::Concerned => {
            let think_bob = (f.anim_t * 2.6).sin() * r * 0.012;
            if !f.try_cyber_eye(-r * 0.12, -r * 0.17 + think_bob * 0.4, r * 0.21, f.tilt, 0.0, false, f.is_blinking, 1.0, window) {
                draw_pill_eye(f.pt(-r * 0.12, -r * 0.17 + think_bob * 0.4), r * 0.095 * f.left_w_scale, r * 0.25, f.tilt, f.is_blinking, f.glow, window);
            }
            if !f.try_cyber_eye(r * 0.48, -r * 0.19 + think_bob * 0.4, r * 0.185, f.tilt, 0.0, false, f.is_blinking, 1.0, window) {
                draw_pill_eye(f.pt(r * 0.48, -r * 0.19 + think_bob * 0.4), r * 0.092 * f.right_w_scale, r * 0.18, f.tilt, f.is_blinking, f.glow, window);
            }
            draw_thinking_finger_hand(f.pt(-r * 0.46, r * 0.19 - think_bob), r * 0.54, f.tilt, window);
        }
        NomiExpression::Concentration => {
            draw_eye_pair_pill(f, f.eye_y, r * 0.09, r * 0.175, 0.0, window);
        }
        NomiExpression::Interest => {
            let close_dx = r * 0.26 * (1.0 - f.yaw_sin.abs() * 0.15);
            if !f.try_cyber_eye(-close_dx, f.eye_y, r * 0.188, f.tilt, 0.0, false, f.is_blinking || f.wink_left, 1.0, window) {
                draw_pill_eye(f.pt(-close_dx, f.eye_y), r * 0.095 * f.left_w_scale, r * 0.18, f.tilt, f.is_blinking || f.wink_left, f.glow, window);
            }
            if !f.try_cyber_eye(close_dx, f.eye_y, r * 0.188, f.tilt, 0.0, false, f.is_blinking || f.wink_right, 1.0, window) {
                draw_pill_eye(f.pt(close_dx, f.eye_y), r * 0.095 * f.right_w_scale, r * 0.18, f.tilt, f.is_blinking || f.wink_right, f.glow, window);
            }
        }
        NomiExpression::Desire => {
            draw_eye_pair_pill(f, f.eye_y - r * 0.08, r * 0.095, r * 0.175, 0.0, window);
        }
        NomiExpression::Attraction => {
            if !f.try_cyber_eye(-f.eye_dx, f.eye_y, r * 0.185, f.tilt + 0.15, 0.0, false, false, 1.0, window) {
                draw_tilted_pill_eye(f.pt(-f.eye_dx, f.eye_y), r * 0.09 * f.left_w_scale, r * 0.175, f.tilt + 0.15, f.glow, window);
            }
            if !f.try_cyber_eye(f.eye_dx, f.eye_y, r * 0.185, f.tilt + 0.15, 0.0, false, false, 1.0, window) {
                draw_tilted_pill_eye(f.pt(f.eye_dx, f.eye_y), r * 0.09 * f.right_w_scale, r * 0.175, f.tilt + 0.15, f.glow, window);
            }
        }
        NomiExpression::Surprise => {
            if !f.try_cyber_eye(-f.eye_dx, f.eye_y, r * 0.20, f.tilt, 0.0, false, false, 1.0, window) {
                draw_pill_eye(f.pt(-f.eye_dx, f.eye_y), r * 0.11 * f.left_w_scale, r * 0.19, f.tilt, false, f.glow, window);
            }
            if !f.try_cyber_eye(f.eye_dx, f.eye_y, r * 0.20, f.tilt, 0.0, false, false, 1.0, window) {
                draw_pill_eye(f.pt(f.eye_dx, f.eye_y), r * 0.11 * f.right_w_scale, r * 0.19, f.tilt, false, f.glow, window);
            }
        }
        NomiExpression::Astonishment => {
            let shake = (f.anim_t * 11.5).sin() * r * 0.008;
            if !f.try_cyber_eye(-f.eye_dx + shake, f.eye_y, r * 0.22, f.tilt, 0.0, false, false, 1.0, window) {
                draw_pill_eye(f.pt(-f.eye_dx + shake, f.eye_y), r * 0.13 * f.left_w_scale, r * 0.215, f.tilt, false, f.glow, window);
            }
            if !f.try_cyber_eye(f.eye_dx + shake, f.eye_y, r * 0.22, f.tilt, 0.0, false, false, 1.0, window) {
                draw_pill_eye(f.pt(f.eye_dx + shake, f.eye_y), r * 0.13 * f.right_w_scale, r * 0.215, f.tilt, false, f.glow, window);
            }
        }
        NomiExpression::Complicity => {
            if !f.try_cyber_eye(-f.eye_dx, f.eye_y, r * 0.185, f.tilt + 0.08, 0.92, false, false, 1.0, window) {
                draw_wink_eye(f.pt(-f.eye_dx, f.eye_y), r * 0.14 * f.left_w_scale, f.tilt, f.glow, window);
            }
            if !f.try_cyber_eye(f.eye_dx, f.eye_y, r * 0.185, f.tilt, 0.0, false, false, 1.0, window) {
                draw_pill_eye(f.pt(f.eye_dx, f.eye_y), r * 0.095 * f.right_w_scale, r * 0.175, f.tilt, false, f.glow, window);
            }
        }
        NomiExpression::Impressed => {
            draw_eye_pair_pill(f, f.eye_y + r * 0.03, r * 0.105, r * 0.185, 0.0, window);
        }

        // === 4. 官方音乐季 (27) ===
        NomiExpression::MusicGuitarAcoustic => {
            draw_eye_pair_sleepy_arc(f, f.hand_eye_y + r * 0.04, csize, window);
            draw_acoustic_guitar(f.pt(-r * 0.05, r * 0.37), r * 0.94, false, false, f.anim_t, window);
        }
        NomiExpression::MusicGuitarJoy => {
            draw_eye_pair_crescent(f, f.hand_eye_y, csize, 0.0, window);
            draw_acoustic_guitar(f.pt(-r * 0.05, r * 0.37), r * 0.94, true, false, f.anim_t, window);
        }
        NomiExpression::MusicGuitarDeep => {
            draw_eye_pair_sleepy_arc(f, f.hand_eye_y + r * 0.04, csize, window);
            draw_acoustic_guitar(f.pt(-r * 0.05, r * 0.37), r * 0.94, false, true, f.anim_t, window);
        }
        NomiExpression::MusicElectricGuitar => {
            draw_eye_pair_crescent(f, f.hand_eye_y, csize, 0.0, window);
            draw_electric_guitar(f.pt(r * 0.12, r * 0.345), r * 1.16, false, f.anim_t, window);
        }
        NomiExpression::MusicRockStar => {
            let rock_bounce = (f.anim_t * 8.0).sin() * r * 0.014;
            if !f.try_cyber_eye(-r * 0.28, f.hand_eye_y - r * 0.12 + rock_bounce, r * 0.17, f.tilt + 0.12, 0.0, true, f.is_blinking, 1.0, window) {
                draw_crescent_eye(f.pt(-r * 0.28, f.hand_eye_y - r * 0.12 + rock_bounce), csize * f.left_w_scale, f.tilt + 0.12, f.glow, window);
            }
            if !f.try_cyber_eye(r * 0.45, f.hand_eye_y - r * 0.06 - rock_bounce * 0.6, r * 0.17, f.tilt + 0.16, 0.0, true, f.is_blinking, 1.0, window) {
                draw_crescent_eye(f.pt(r * 0.45, f.hand_eye_y - r * 0.06 - rock_bounce * 0.6), csize * f.right_w_scale, f.tilt + 0.16, f.glow, window);
            }
            draw_electric_guitar(f.pt(r * 0.06, r * 0.27), r * 1.12, true, f.anim_t, window);
        }
        NomiExpression::MusicViolinJoy => {
            let sway = (f.anim_t * 4.5).sin() * r * 0.010;
            if !f.try_cyber_eye(-r * 0.27, f.hand_eye_y - r * 0.02 + sway, r * 0.17, f.tilt - 0.02, 0.0, true, f.is_blinking, 1.0, window) {
                draw_crescent_eye(f.pt(-r * 0.27, f.hand_eye_y - r * 0.02 + sway), csize * f.left_w_scale, f.tilt - 0.02, f.glow, window);
            }
            if !f.try_cyber_eye(r * 0.39, f.hand_eye_y + r * 0.01 - sway * 0.6, r * 0.17, f.tilt + 0.06, 0.0, true, f.is_blinking, 1.0, window) {
                draw_crescent_eye(f.pt(r * 0.39, f.hand_eye_y + r * 0.01 - sway * 0.6), csize * f.right_w_scale, f.tilt + 0.06, f.glow, window);
            }
            draw_violin(f.pt(r * 0.14, r * 0.36), r * 0.86, false, false, f.anim_t, window);
        }
        NomiExpression::MusicViolinDeep => {
            if !f.try_cyber_eye(-r * 0.30, f.hand_eye_y - r * 0.065, r * 0.17, f.tilt + 0.09, 0.72, false, false, 1.0, window) {
                draw_gentle_sleepy_eye(f.pt(-r * 0.30, f.hand_eye_y - r * 0.065), csize * 1.05 * f.left_w_scale, f.tilt + 0.09, f.glow, window);
            }
            if !f.try_cyber_eye(r * 0.45, f.hand_eye_y - r * 0.025, r * 0.17, f.tilt - 0.04, 0.72, false, false, 1.0, window) {
                draw_gentle_sleepy_eye(f.pt(r * 0.45, f.hand_eye_y - r * 0.025), csize * 1.05 * f.right_w_scale, f.tilt - 0.04, f.glow, window);
            }
            draw_violin(f.pt(r * 0.14, r * 0.36), r * 0.86, false, true, f.anim_t, window);
        }
        NomiExpression::MusicViolinSerene => {
            draw_eye_pair_sleepy_arc(f, f.hand_eye_y + r * 0.04, csize, window);
            draw_violin(f.pt(r * 0.14, r * 0.36), r * 0.86, false, false, f.anim_t, window);
        }
        NomiExpression::MusicCello => {
            draw_eye_pair_crescent(f, f.hand_eye_y - r * 0.02, csize, 0.0, window);
            draw_violin(f.pt(r * 0.12, r * 0.40), r * 0.98, true, false, f.anim_t, window);
        }
        NomiExpression::MusicSnareDrum => {
            draw_eye_pair_crescent(f, f.hand_eye_y - r * 0.04, csize, 0.0, window);
            draw_drums(f.pt(0.0, r * 0.37), r * 0.84, f.anim_t, window);
        }
        NomiExpression::MusicKeyboard => {
            draw_eye_pair_crescent(f, f.hand_eye_y - r * 0.04, csize, 0.0, window);
            draw_keyboard(f.pt(0.0, r * 0.40), r * 1.02, f.anim_t, window);
        }
        NomiExpression::MusicMaracas => {
            draw_eye_pair_crescent(f, f.hand_eye_y - r * 0.02, csize, 0.0, window);
            draw_maracas(f.pt(0.0, r * 0.35), r * 0.68, f.anim_t, window);
        }
        NomiExpression::MusicTambourine => {
            let sway = (f.anim_t * 5.5).sin() * r * 0.012;
            if !f.try_cyber_eye(-f.eye_dx * 1.04, f.hand_eye_y + r * 0.01 + sway * 0.5, r * 0.17, f.tilt - 0.13, 0.0, true, f.is_blinking, 1.0, window) {
                draw_crescent_eye(f.pt(-f.eye_dx * 1.04, f.hand_eye_y + r * 0.01 + sway * 0.5), csize * f.left_w_scale, f.tilt - 0.13, f.glow, window);
            }
            if !f.try_cyber_eye(f.eye_dx * 1.00, f.hand_eye_y - r * 0.06 - sway * 0.5, r * 0.17, f.tilt - 0.09, 0.0, true, f.is_blinking, 1.0, window) {
                draw_crescent_eye(f.pt(f.eye_dx * 1.00, f.hand_eye_y - r * 0.06 - sway * 0.5), csize * f.right_w_scale, f.tilt - 0.09, f.glow, window);
            }
            draw_tambourine(f.pt(-r * 0.10, r * 0.34), r * 0.86, f.anim_t, window);
        }
        NomiExpression::MusicDjDual => {
            draw_eye_pair_crescent(f, f.hand_eye_y - r * 0.02, csize, 0.0, window);
            draw_dual_dj(f.pt(0.0, r * 0.37), r * 0.94, f.anim_t, window);
        }
        NomiExpression::MusicSaxophoneJoy => {
            let gy = (f.anim_t * 4.0).sin() * r * 0.014;
            let gt = (f.anim_t * 4.0).sin() * 0.028;
            if !f.try_cyber_eye(-r * 0.24, f.hand_eye_y - r * 0.08 + gy, r * 0.17, f.tilt + gt * 0.6, 0.0, true, f.is_blinking, 1.0, window) {
                draw_crescent_eye(f.pt(-r * 0.24, f.hand_eye_y - r * 0.08 + gy), csize * f.left_w_scale, f.tilt + gt * 0.6, f.glow, window);
            }
            if !f.try_cyber_eye(r * 0.49, f.hand_eye_y - r * 0.005 - gy, r * 0.165, f.tilt + 0.13 + gt, 0.0, true, f.is_blinking, 1.0, window) {
                draw_crescent_eye(f.pt(r * 0.49, f.hand_eye_y - r * 0.005 - gy), csize * 0.95 * f.right_w_scale, f.tilt + 0.13 + gt, f.glow, window);
            }
            draw_saxophone(f.pt(r * 0.24, r * 0.38), r * 0.72, false, f.anim_t, window);
        }
        NomiExpression::MusicSaxophoneDeep => {
            let breathe = (f.anim_t * 2.5).sin() * r * 0.008;
            if !f.try_cyber_eye(-r * 0.24, -r * 0.35 + breathe, r * 0.165, f.tilt + 0.06, 0.72, false, false, 1.0, window) {
                draw_gentle_sleepy_eye(f.pt(-r * 0.24, -r * 0.35 + breathe), csize * 0.96 * f.left_w_scale, f.tilt + 0.06, f.glow, window);
            }
            if !f.try_cyber_eye(r * 0.49, -r * 0.28 - breathe * 0.5, r * 0.165, f.tilt + 0.04, 0.72, false, false, 1.0, window) {
                draw_gentle_sleepy_eye(f.pt(r * 0.49, -r * 0.28 - breathe * 0.5), csize * 0.96 * f.right_w_scale, f.tilt + 0.04, f.glow, window);
            }
            draw_saxophone(f.pt(r * 0.23, r * 0.38), r * 0.72, true, f.anim_t, window);
        }
        NomiExpression::MusicVocalJoy => {
            if !f.try_cyber_eye(-f.eye_dx * 1.04, f.hand_eye_y + r * 0.01, r * 0.17, f.tilt - 0.04, 0.0, true, f.is_blinking, 1.0, window) {
                draw_crescent_eye(f.pt(-f.eye_dx * 1.04, f.hand_eye_y + r * 0.01), csize * f.left_w_scale, f.tilt - 0.04, f.glow, window);
            }
            if !f.try_cyber_eye(f.eye_dx * 1.04, f.hand_eye_y - r * 0.04, r * 0.17, f.tilt - 0.04, 0.0, true, f.is_blinking, 1.0, window) {
                draw_crescent_eye(f.pt(f.eye_dx * 1.04, f.hand_eye_y - r * 0.04), csize * f.right_w_scale, f.tilt - 0.04, f.glow, window);
            }
            draw_microphone(f.pt(-r * 0.22, r * 0.42), r * 0.64, false, f.anim_t, window);
        }
        NomiExpression::MusicVocalDeep => {
            if !f.try_cyber_eye(-f.eye_dx, f.hand_eye_y + r * 0.02, r * 0.17, f.tilt, 0.72, false, false, 1.0, window) {
                draw_gentle_sleepy_eye(f.pt(-f.eye_dx, f.hand_eye_y + r * 0.02), csize * f.left_w_scale, f.tilt, f.glow, window);
            }
            if !f.try_cyber_eye(f.eye_dx, f.hand_eye_y + r * 0.02, r * 0.17, f.tilt, 0.72, false, false, 1.0, window) {
                draw_gentle_sleepy_eye(f.pt(f.eye_dx, f.hand_eye_y + r * 0.02), csize * f.right_w_scale, f.tilt, f.glow, window);
            }
            draw_microphone(f.pt(-r * 0.22, r * 0.42), r * 0.64, true, f.anim_t, window);
        }
        NomiExpression::MusicVocalSerene => {
            if !f.try_cyber_eye(-f.eye_dx, f.hand_eye_y + r * 0.02, r * 0.17, f.tilt, 0.72, false, false, 1.0, window) {
                draw_gentle_sleepy_eye(f.pt(-f.eye_dx, f.hand_eye_y + r * 0.02), csize * f.left_w_scale, f.tilt, f.glow, window);
            }
            if !f.try_cyber_eye(f.eye_dx, f.hand_eye_y + r * 0.02, r * 0.17, f.tilt, 0.72, false, false, 1.0, window) {
                draw_gentle_sleepy_eye(f.pt(f.eye_dx, f.hand_eye_y + r * 0.02), csize * f.right_w_scale, f.tilt, f.glow, window);
            }
            draw_microphone(f.pt(-r * 0.22, r * 0.42), r * 0.64, false, f.anim_t, window);
        }
        NomiExpression::MusicConductor => {
            let beat_y = (f.anim_t * 4.5).sin() * r * 0.012;
            if !f.try_cyber_eye(-f.eye_dx * 1.16, f.hand_eye_y - r * 0.04 + beat_y * 0.4, r * 0.17, f.tilt - 0.03, 0.0, true, f.is_blinking, 1.0, window) {
                draw_crescent_eye(f.pt(-f.eye_dx * 1.16, f.hand_eye_y - r * 0.04 + beat_y * 0.4), csize * 1.04 * f.left_w_scale, f.tilt - 0.03, f.glow, window);
            }
            if !f.try_cyber_eye(f.eye_dx * 1.06, f.hand_eye_y - r * 0.10 - beat_y * 0.6, r * 0.17, f.tilt - 0.05, 0.0, true, f.is_blinking, 1.0, window) {
                draw_crescent_eye(f.pt(f.eye_dx * 1.06, f.hand_eye_y - r * 0.10 - beat_y * 0.6), csize * f.right_w_scale, f.tilt - 0.05, f.glow, window);
            }
            draw_conductor(f.pt(0.0, 0.0), r, f.anim_t, window);
        }
        NomiExpression::MusicDjSingle => {
            draw_eye_pair_crescent(f, f.hand_eye_y, csize, 0.0, window);
            let scratch = (f.anim_t * 8.0).sin() * 0.05;
            draw_single_dj(f.pt(-f.eye_dx * 0.58, r * 0.34), r * 0.72, scratch, window);
        }
        NomiExpression::MusicLightsticks => {
            draw_eye_pair_crescent(f, f.hand_eye_y, csize, 0.0, window);
            draw_lightsticks(f.pt(r * 0.08, r * 0.44), r * 0.84, f.anim_t, window);
        }
        NomiExpression::MusicNioSpeaker => {
            draw_eye_pair_crescent(f, f.hand_eye_y, csize, 0.0, window);
            draw_spectrum(f.pt(-f.eye_dx * 0.54, r * 0.33), r * 0.72, f.anim_t, window);
        }
        NomiExpression::MusicRadio => {
            draw_eye_pair_crescent(f, f.hand_eye_y, csize, 0.0, window);
            draw_retro_radio(f.pt(-f.eye_dx * 0.54, r * 0.33), r * 0.72, f.anim_t, window);
        }
        NomiExpression::MusicDancingNotes => {
            draw_eye_pair_crescent(f, -r * 0.14, r * 0.15, 0.0, window);
            draw_dancing_notes(f.pt(0.0, 0.0), r, f.anim_t, window);
        }
        NomiExpression::MusicConfetti => {
            draw_eye_pair_crescent(f, f.hand_eye_y, csize, 0.0, window);
            draw_confetti_burst(f.pt(0.0, r * 0.34), r * 0.80, f.anim_t, window);
        }
        NomiExpression::MusicPeaceV => {
            if !f.try_cyber_eye(-f.eye_dx * 1.15, -r * 0.01, r * 0.195, f.tilt, 0.0, false, f.is_blinking || f.wink_left, 1.0, window) {
                draw_pill_eye(f.pt(-f.eye_dx * 1.15, -r * 0.01), r * 0.095 * f.left_w_scale, r * 0.195, f.tilt, f.is_blinking || f.wink_left, f.glow, window);
            }
            draw_peace_sign(f.pt(r * 0.36, r * 0.01), r * 0.52, f.tilt, f.anim_t, window);
        }
    }
}

// ==========================================
// 眼神组合与基础眼型原子函数
// ==========================================

fn draw_eye_pair_crescent(f: &FaceCtx, y: f32, size: f32, extra_tilt: f32, window: &mut Window) {
    let eye_r = (f.r * 0.17).max(size * 1.24);
    if f.cyber_theme.is_some() {
        f.try_cyber_eye(-f.eye_dx, y, eye_r * f.left_w_scale, f.tilt + extra_tilt, 0.0, true, f.is_blinking || f.wink_left, 1.0, window);
        f.try_cyber_eye(f.eye_dx, y, eye_r * f.right_w_scale, f.tilt - extra_tilt, 0.0, true, f.is_blinking || f.wink_right, 1.0, window);
        return;
    }
    let lp = f.pt(-f.eye_dx, y);
    let rp = f.pt(f.eye_dx, y);
    if f.is_blinking || f.wink_left {
        draw_blink_eye(lp, size * f.left_w_scale, f.tilt + extra_tilt, f.glow, window);
    } else {
        draw_crescent_eye(lp, size * f.left_w_scale, f.tilt + extra_tilt, f.glow, window);
    }
    if f.is_blinking || f.wink_right {
        draw_blink_eye(rp, size * f.right_w_scale, f.tilt - extra_tilt, f.glow, window);
    } else {
        draw_crescent_eye(rp, size * f.right_w_scale, f.tilt - extra_tilt, f.glow, window);
    }
}

fn draw_eye_pair_sleepy_arc(f: &FaceCtx, y: f32, size: f32, window: &mut Window) {
    let eye_r = (f.r * 0.17).max(size * 1.22);
    if f.cyber_theme.is_some() {
        f.try_cyber_eye(-f.eye_dx, y, eye_r * f.left_w_scale, f.tilt, 0.84, false, false, 1.0, window);
        f.try_cyber_eye(f.eye_dx, y, eye_r * f.right_w_scale, f.tilt, 0.84, false, false, 1.0, window);
        return;
    }
    draw_sleepy_arc_eye(f.pt(-f.eye_dx, y), size * f.left_w_scale, f.tilt, f.glow, window);
    draw_sleepy_arc_eye(f.pt(f.eye_dx, y), size * f.right_w_scale, f.tilt, f.glow, window);
}

fn draw_eye_pair_pill(
    f: &FaceCtx,
    y: f32,
    rx: f32,
    ry: f32,
    _tilt: f32,
    window: &mut Window,
) {
    if f.cyber_theme.is_some() {
        let eye_r = (ry * 1.04).clamp(f.r * 0.178, f.r * 0.22);
        f.try_cyber_eye(-f.eye_dx, y, eye_r * f.left_w_scale, f.tilt, 0.0, false, f.is_blinking || f.wink_left, 1.0, window);
        f.try_cyber_eye(f.eye_dx, y, eye_r * f.right_w_scale, f.tilt, 0.0, false, f.is_blinking || f.wink_right, 1.0, window);
        return;
    }
    draw_pill_eye(f.pt(-f.eye_dx, y), rx * f.left_w_scale, ry, f.tilt, f.is_blinking || f.wink_left, f.glow, window);
    draw_pill_eye(f.pt(f.eye_dx, y), rx * f.right_w_scale, ry, f.tilt, f.is_blinking || f.wink_right, f.glow, window);
}

/// 赛博坦机械光学眼核心矢量绘制（适配 6 套角色光效、36pt 微缩高可读与机械开合眼睑）
#[allow(clippy::too_many_arguments)]
fn draw_cyber_optic_unit(
    center: Point<Pixels>,
    r: f32,
    eye_r: f32,
    tilt: f32,
    shutter_top: f32,
    happy_lift: bool,
    is_blinking: bool,
    alpha: f32,
    time_sec: f32,
    theme: CyberOpticTheme,
    window: &mut Window,
) {
    let pulse = 0.88 + 0.12 * (time_sec * 3.8).sin();
    let (aura_rgb, outer_rgb, mid_rgb, core_rgb, lid_rgb): (u32, u32, u32, u32, u32) = match theme {
        CyberOpticTheme::Bumblebee => (0x38BDF8, 0x0369A1, 0x0EA5E9, 0x7DD3FC, 0x38BDF8),
        CyberOpticTheme::Optimus => (0x60A5FA, 0x1D4ED8, 0x2563EB, 0x93C5FD, 0x60A5FA),
        CyberOpticTheme::Megatron => (0xEF4444, 0x991B1B, 0xEF4444, 0xFCA5A5, 0xF87171),
        CyberOpticTheme::Comms => (0xEF4444, 0x991B1B, 0xEF4444, 0xFCA5A5, 0xEF4444),
        CyberOpticTheme::Shockwave => (0xFACC15, 0xB45309, 0xFACC15, 0xFEF08A, 0xFACC15),
        CyberOpticTheme::Jazz => (0x38BDF8, 0x0284C7, 0x38BDF8, 0xBAE6FD, 0x7DD3FC),
        CyberOpticTheme::Drift => (0x38BDF8, 0x0284C7, 0x38BDF8, 0xFFFFFF, 0xFACC15),
        CyberOpticTheme::Grimlock => (0x38BDF8, 0x0284C7, 0x0EA5E9, 0xF0F9FF, 0xFACC15),
        CyberOpticTheme::Arcee => (0x38BDF8, 0x0284C7, 0xF472B6, 0xFFFFFF, 0xF472B6),
        CyberOpticTheme::Ratchet => (0x38BDF8, 0x0284C7, 0x0EA5E9, 0xF0F9FF, 0x38BDF8),
        CyberOpticTheme::Hotrod => (0x38BDF8, 0x0284C7, 0x38BDF8, 0xF0F9FF, 0x38BDF8),
        CyberOpticTheme::Devastator => (0xEF4444, 0x991B1B, 0xEF4444, 0xFEF2F2, 0xA3E635),
        CyberOpticTheme::Wheeljack => (0x38BDF8, 0x0284C7, 0x38BDF8, 0xFFFFFF, 0x4ADE80),
        CyberOpticTheme::Cyber => (0x22D3EE, 0x0E7490, 0x06B6D4, 0x67E8F9, 0x22D3EE),
    };
    let with_a = |rgb: u32, a: f32| -> Rgba {
        let ab = ((a * alpha).clamp(0.0, 1.0) * 255.0).round() as u32;
        rgba((rgb << 8) | ab)
    };
    let (s, c) = tilt.sin_cos();
    let rot = |dx: f32, dy: f32| point(center.x + px(dx * c - dy * s), center.y + px(dx * s + dy * c));

    // 声波与爵士专属：全息跑车/通讯机械护目眼条
    if theme == CyberOpticTheme::Comms || theme == CyberOpticTheme::Jazz {
        let is_jazz = theme == CyberOpticTheme::Jazz;
        let bw = eye_r * if is_jazz { 2.25 } else { 2.10 };
        let close_f = if is_blinking { 0.18 } else { (1.0 - shutter_top * 0.78).max(0.22) };
        let bh = eye_r * 0.76 * close_f;
        window.paint_quad(PaintQuad {
            bounds: Bounds {
                origin: point(center.x - px(bw * 0.54), center.y - px(bh * 0.56)),
                size: gpui::size(px(bw * 1.08), px(bh * 1.12)),
            },
            corner_radii: Corners::all(px(bh * 0.28)),
            background: with_a(if is_jazz { 0x090D16 } else { 0x0F172A }, 1.0).into(),
            border_widths: Edges::all(px((r * 0.020).max(0.45))),
            border_color: with_a(if is_jazz { 0x94A3B8 } else { 0x64748B }, 0.9).into(),
            border_style: gpui::BorderStyle::Solid,
        });
        window.paint_quad(PaintQuad {
            bounds: Bounds {
                origin: point(center.x - px(bw * 0.48), center.y - px(bh * 0.42)),
                size: gpui::size(px(bw * 0.96), px(bh * 0.84)),
            },
            corner_radii: Corners::all(px(bh * 0.20)),
            background: with_a(mid_rgb, pulse).into(),
            border_widths: Edges::all(px(0.0)),
            border_color: rgba(0x00000000).into(),
            border_style: gpui::BorderStyle::Solid,
        });
        return;
    }

    // 1) 光晕背景
    let aura_r = px(eye_r * 1.38);
    window.paint_quad(PaintQuad {
        bounds: Bounds {
            origin: point(center.x - aura_r, center.y - aura_r),
            size: gpui::size(aura_r * 2.0, aura_r * 2.0),
        },
        corner_radii: Corners::all(aura_r),
        background: with_a(aura_rgb, 0.24 * pulse).into(),
        border_widths: Edges::all(px(0.0)),
        border_color: rgba(0x00000000).into(),
        border_style: gpui::BorderStyle::Solid,
    });

    // 2) 外壳框（震荡波六角独眼 vs 其余多瓣圆环）
    if theme == CyberOpticTheme::Shockwave {
        let mut hex = PathBuilder::fill();
        for v in 0..6 {
            let a = (v as f32 * PI) / 3.0 + PI / 6.0;
            let p = rot(a.cos() * eye_r * 1.06, a.sin() * eye_r * 1.06);
            if v == 0 {
                hex.move_to(p);
            } else {
                hex.line_to(p);
            }
        }
        hex.close();
        if let Ok(built) = hex.build() {
            window.paint_path(built, with_a(0x1E1B4B, 1.0));
        }
        let mut hrim = PathBuilder::stroke(px((r * 0.024).max(0.5)));
        for v in 0..6 {
            let a = (v as f32 * PI) / 3.0 + PI / 6.0;
            let p = rot(a.cos() * eye_r * 1.06, a.sin() * eye_r * 1.06);
            if v == 0 {
                hrim.move_to(p);
            } else {
                hrim.line_to(p);
            }
        }
        hrim.close();
        if let Ok(built) = hrim.build() {
            window.paint_path(built, with_a(0xA855F7, 1.0));
        }
    } else {
        let er = px(eye_r);
        window.paint_quad(PaintQuad {
            bounds: Bounds {
                origin: point(center.x - er, center.y - er),
                size: gpui::size(er * 2.0, er * 2.0),
            },
            corner_radii: Corners::all(er),
            background: with_a(0x0F172A, 1.0).into(),
            border_widths: Edges::all(px((r * 0.022).max(0.45))),
            border_color: with_a(0x475569, 0.95).into(),
            border_style: gpui::BorderStyle::Solid,
        });
    }

    // 若处于眨眼 / 单眼 Wink 闭合状态（震荡波除外），渲染高科技发光机械闭眼缝 / 优雅微弧笑眼
    // 杜绝粗暴纯黑涂抹导致微缩头像下“眼睛缺失 / 空洞黑圈”
    if theme != CyberOpticTheme::Shockwave && is_blinking {
        let clip_r = eye_r * 0.86;
        let ir = px(clip_r);
        window.paint_quad(PaintQuad {
            bounds: Bounds {
                origin: point(center.x - ir, center.y - ir),
                size: gpui::size(ir * 2.0, ir * 2.0),
            },
            corner_radii: Corners::all(ir),
            background: with_a(0x0F172A, 1.0).into(),
            border_widths: Edges::all(px(0.0)),
            border_color: rgba(0x00000000).into(),
            border_style: gpui::BorderStyle::Solid,
        });

        // 机械闭合护甲中缝暗缝
        let mut seam = PathBuilder::stroke(px((r * 0.016).max(0.45)));
        seam.move_to(rot(-clip_r * 0.88, 0.0));
        seam.line_to(rot(clip_r * 0.88, 0.0));
        if let Ok(p) = seam.build() {
            window.paint_path(p, with_a(0x334155, 0.85));
        }

        let arc_lift = if happy_lift { -clip_r * 0.28 } else { 0.0 };

        // 1) 柔和外晕光层
        let mut aura_path = PathBuilder::stroke(px((clip_r * 0.38).max(2.5)));
        aura_path.move_to(rot(-clip_r * 0.82, 0.0));
        if happy_lift {
            aura_path.curve_to(rot(clip_r * 0.82, 0.0), rot(0.0, arc_lift * 1.4));
        } else {
            aura_path.line_to(rot(clip_r * 0.82, 0.0));
        }
        if let Ok(p) = aura_path.build() {
            window.paint_path(p, with_a(aura_rgb, 0.45 * pulse));
        }

        // 2) 主能量闭目电光层
        let mut mid_path = PathBuilder::stroke(px((r * 0.038).max(1.8)));
        mid_path.move_to(rot(-clip_r * 0.82, 0.0));
        if happy_lift {
            mid_path.curve_to(rot(clip_r * 0.82, 0.0), rot(0.0, arc_lift));
        } else {
            mid_path.line_to(rot(clip_r * 0.82, 0.0));
        }
        if let Ok(p) = mid_path.build() {
            window.paint_path(p, with_a(mid_rgb, pulse));
        }

        // 3) 耀白极芯光斑
        let mut core_path = PathBuilder::stroke(px((r * 0.018).max(0.9)));
        core_path.move_to(rot(-clip_r * 0.60, 0.0));
        if happy_lift {
            core_path.curve_to(rot(clip_r * 0.60, 0.0), rot(0.0, arc_lift * 0.85));
        } else {
            core_path.line_to(rot(clip_r * 0.60, 0.0));
        }
        if let Ok(p) = core_path.build() {
            window.paint_path(p, with_a(0xFFFFFF, 0.96));
        }
        return;
    }

    // 3) 虹膜层：外环 + 亮心 + 深瞳 + 灵动晶体双高光
    let lr = eye_r * 0.78;
    for (scale, rgb, al) in [
        (1.0, outer_rgb, 1.0),
        (0.76, mid_rgb, pulse),
        (0.50, core_rgb, pulse),
        (0.28, 0x041E30, 1.0),
    ] {
        let cr = px(lr * scale);
        window.paint_quad(PaintQuad {
            bounds: Bounds {
                origin: point(center.x - cr, center.y - cr),
                size: gpui::size(cr * 2.0, cr * 2.0),
            },
            corner_radii: Corners::all(cr),
            background: with_a(rgb, al).into(),
            border_widths: Edges::all(px(0.0)),
            border_color: rgba(0x00000000).into(),
            border_style: gpui::BorderStyle::Solid,
        });
    }
    let gl_r = px((lr * 0.17).max(0.65));
    let gl_p = rot(-lr * 0.28, -lr * 0.30);
    window.paint_quad(PaintQuad {
        bounds: Bounds {
            origin: point(gl_p.x - gl_r, gl_p.y - gl_r),
            size: gpui::size(gl_r * 2.0, gl_r * 2.0),
        },
        corner_radii: Corners::all(gl_r),
        background: with_a(0xFFFFFF, 0.96).into(),
        border_widths: Edges::all(px(0.0)),
        border_color: rgba(0x00000000).into(),
        border_style: gpui::BorderStyle::Solid,
    });

    // 4) 机械眼睑动态遮罩（非眨眼状态下的半闭微阖 / 微笑弧下睑）
    // 震荡波为冷酷逻辑独眼，无眼睑遮罩，永不下垂眼皮
    if theme != CyberOpticTheme::Shockwave {
        let clip_r = eye_r * 0.86;
        if shutter_top > 0.02 {
            let cy = (-clip_r + shutter_top * clip_r * 1.65).clamp(-clip_r * 0.97, clip_r * 0.97);
            let half_chord = (clip_r * clip_r - cy * cy).max(0.0).sqrt();
            let a_right = (cy / clip_r).clamp(-1.0, 1.0).asin();
            let a_left = -PI - a_right;
            let arc_steps = 16;

            let mut lid = PathBuilder::fill();
            for i in 0..=arc_steps {
                let a = a_right + (a_left - a_right) * (i as f32 / arc_steps as f32);
                let p = rot(a.cos() * clip_r, a.sin() * clip_r);
                if i == 0 {
                    lid.move_to(p);
                } else {
                    lid.line_to(p);
                }
            }
            lid.close();
            if let Ok(built) = lid.build() {
                window.paint_path(built, with_a(0x1E293B, 1.0));
            }

            if half_chord > 0.25 {
                let mut lrim = PathBuilder::stroke(px((r * 0.022).max(0.45)));
                lrim.move_to(rot(-half_chord, cy));
                lrim.line_to(rot(half_chord, cy));
                if let Ok(built) = lrim.build() {
                    window.paint_path(built, with_a(lid_rgb, 1.0));
                }
            }
        }
        if happy_lift && shutter_top < 0.3 {
            let y_end = clip_r * 0.56;
            let half_w = (clip_r * clip_r - y_end * y_end).max(0.0).sqrt();
            let a_start = (y_end / clip_r).clamp(-1.0, 1.0).asin();
            let a_end = PI - a_start;
            let arc_steps = 12;

            // 严丝合缝贴合镜框下缘的月牙微笑下眼睑，不溢出圆外
            let mut lower_lid = PathBuilder::fill();
            for i in 0..=arc_steps {
                let a = a_start + (a_end - a_start) * (i as f32 / arc_steps as f32);
                let p = rot(a.cos() * clip_r, a.sin() * clip_r);
                if i == 0 {
                    lower_lid.move_to(p);
                } else {
                    lower_lid.line_to(p);
                }
            }
            lower_lid.curve_to(rot(half_w, y_end), rot(0.0, eye_r * 0.18));
            lower_lid.close();
            if let Ok(built) = lower_lid.build() {
                window.paint_path(built, with_a(0x1E293B, 1.0));
            }

            let mut lrim = PathBuilder::stroke(px((r * 0.022).max(0.45)));
            lrim.move_to(rot(-half_w, y_end));
            lrim.curve_to(rot(half_w, y_end), rot(0.0, eye_r * 0.18));
            if let Ok(built) = lrim.build() {
                window.paint_path(built, with_a(core_rgb, 0.92));
            }
        }
    }
    if theme == CyberOpticTheme::Optimus && !is_blinking {
        let mut bx = PathBuilder::stroke(px((r * 0.024).max(0.5)));
        bx.move_to(rot(-eye_r * 0.85, eye_r * 0.54));
        bx.line_to(rot(-eye_r * 0.42, eye_r * 0.80));
        bx.line_to(rot(eye_r * 0.42, eye_r * 0.80));
        bx.line_to(rot(eye_r * 0.85, eye_r * 0.54));
        if let Ok(built) = bx.build() {
            window.paint_path(built, with_a(0x60A5FA, 0.95));
        }
    }
}

fn draw_crescent_eye(center: Point<Pixels>, size: f32, tilt: f32, glow: Rgba, window: &mut Window) {
    let lw = size * 0.44;
    let (s, c) = tilt.sin_cos();
    let rot = |dx: f32, dy: f32| point(center.x + px(dx * c - dy * s), center.y + px(dx * s + dy * c));
    let mut path = PathBuilder::stroke(px(lw));
    path.move_to(rot(-size, 0.0));
    path.curve_to(rot(size, 0.0), rot(0.0, -size * 0.85));
    if let Ok(p) = path.build() {
        window.paint_path(p, glow);
    }
}

#[inline]
fn glow_rgb_u32(glow: Rgba) -> u32 {
    let r = (glow.r.clamp(0.0, 1.0) * 255.0).round() as u32;
    let g = (glow.g.clamp(0.0, 1.0) * 255.0).round() as u32;
    let b = (glow.b.clamp(0.0, 1.0) * 255.0).round() as u32;
    (r << 16) | (g << 8) | b
}

fn draw_sleepy_arc_eye(center: Point<Pixels>, size: f32, tilt: f32, glow: Rgba, window: &mut Window) {
    let lw = size * 0.42;
    let (s, c) = tilt.sin_cos();
    let rot = |dx: f32, dy: f32| point(center.x + px(dx * c - dy * s), center.y + px(dx * s + dy * c));
    let mut path = PathBuilder::stroke(px(lw));
    path.move_to(rot(-size, 0.0));
    path.curve_to(rot(size, 0.0), rot(0.0, size * 0.82));
    if let Ok(p) = path.build() {
        window.paint_path(p, glow);
    }
}

fn draw_gentle_sleepy_eye(center: Point<Pixels>, size: f32, tilt: f32, glow: Rgba, window: &mut Window) {
    let lw = size * 0.40;
    let (s, c) = tilt.sin_cos();
    let rot = |dx: f32, dy: f32| point(center.x + px(dx * c - dy * s), center.y + px(dx * s + dy * c));
    let mut path = PathBuilder::stroke(px(lw));
    path.move_to(rot(-size * 0.98, 0.0));
    path.curve_to(rot(size * 0.98, 0.0), rot(0.0, size * 0.48));
    if let Ok(p) = path.build() {
        window.paint_path(p, glow);
    }
}

fn draw_pill_eye(
    center: Point<Pixels>,
    rx: f32,
    ry: f32,
    tilt: f32,
    is_blinking: bool,
    glow: Rgba,
    window: &mut Window,
) {
    if is_blinking {
        draw_blink_eye(center, rx * 1.4, tilt, glow, window);
        return;
    }
    if tilt.abs() > 0.04 {
        draw_tilted_pill_eye(center, rx, ry, tilt, glow, window);
        return;
    }
    let lw = (rx * 0.62).max(1.1);
    let w = rx * 2.0;
    let h = ry * 2.0;
    window.paint_quad(PaintQuad {
        bounds: Bounds {
            origin: point(center.x - px(rx), center.y - px(ry)),
            size: gpui::size(px(w), px(h)),
        },
        corner_radii: Corners::all(px(rx.min(ry))),
        background: rgba(0x00000000).into(),
        border_widths: Edges::all(px(lw)),
        border_color: glow.into(),
        border_style: gpui::BorderStyle::Solid,
    });
}

fn draw_tilted_pill_eye(
    center: Point<Pixels>,
    rx: f32,
    ry: f32,
    rot: f32,
    glow: Rgba,
    window: &mut Window,
) {
    let lw = (rx.min(ry) * 0.65).max(1.0);
    let (s, c) = rot.sin_cos();
    let steps = 16;
    let mut path = PathBuilder::stroke(px(lw));
    for i in 0..=steps {
        let theta = (i as f32 / steps as f32) * 2.0 * PI;
        let lx = rx * theta.cos();
        let ly = ry * theta.sin();
        let pt = point(center.x + px(lx * c - ly * s), center.y + px(lx * s + ly * c));
        if i == 0 {
            path.move_to(pt);
        } else {
            path.line_to(pt);
        }
    }
    if let Ok(p) = path.build() {
        window.paint_path(p, glow);
    }
}

fn draw_half_closed_eye(center: Point<Pixels>, size: f32, tilt: f32, glow: Rgba, window: &mut Window) {
    let lw = (size * 0.42).max(1.0);
    let (s, c) = tilt.sin_cos();
    let rot = |dx: f32, dy: f32| point(center.x + px(dx * c - dy * s), center.y + px(dx * s + dy * c));
    let mut path = PathBuilder::stroke(px(lw));
    path.move_to(rot(-size, -size * 0.20));
    path.line_to(rot(size, -size * 0.20));
    path.curve_to(rot(-size, -size * 0.20), rot(0.0, size * 0.95));
    if let Ok(p) = path.build() {
        window.paint_path(p, glow);
    }
}

fn draw_wink_eye(center: Point<Pixels>, size: f32, tilt: f32, glow: Rgba, window: &mut Window) {
    let lw = (size * 0.42).max(1.0);
    let (s, c) = tilt.sin_cos();
    let rot = |dx: f32, dy: f32| point(center.x + px(dx * c - dy * s), center.y + px(dx * s + dy * c));
    let mut path = PathBuilder::stroke(px(lw));
    path.move_to(rot(-size * 0.85, -size * 0.10));
    path.line_to(rot(size * 0.85, size * 0.10));
    if let Ok(p) = path.build() {
        window.paint_path(p, glow);
    }
}

fn draw_blink_eye(center: Point<Pixels>, size: f32, tilt: f32, glow: Rgba, window: &mut Window) {
    let lw = (size * 0.38).max(1.0);
    let (s, c) = tilt.sin_cos();
    let rot = |dx: f32, dy: f32| point(center.x + px(dx * c - dy * s), center.y + px(dx * s + dy * c));
    let mut path = PathBuilder::stroke(px(lw));
    path.move_to(rot(-size * 0.9, 0.0));
    path.line_to(rot(size * 0.9, 0.0));
    if let Ok(p) = path.build() {
        window.paint_path(p, glow);
    }
}

fn draw_squish_eye(center: Point<Pixels>, size: f32, is_left: bool, glow: Rgba, window: &mut Window) {
    let lw = size * 0.40;
    let mut path = PathBuilder::stroke(px(lw));
    let dir = if is_left { 1.0 } else { -1.0 };
    path.move_to(point(center.x - px(dir * size * 0.8), center.y - px(size * 0.6)));
    path.line_to(point(center.x + px(dir * size * 0.6), center.y));
    path.line_to(point(center.x - px(dir * size * 0.8), center.y + px(size * 0.6)));
    if let Ok(p) = path.build() {
        window.paint_path(p, glow);
    }
}

fn draw_dizzy_eye(center: Point<Pixels>, size: f32, glow: Rgba, window: &mut Window) {
    let lw = size * 0.32;
    window.paint_quad(PaintQuad {
        bounds: Bounds {
            origin: point(center.x - px(size * 0.7), center.y - px(size * 0.7)),
            size: gpui::size(px(size * 1.4), px(size * 1.4)),
        },
        corner_radii: Corners::all(px(size * 0.7)),
        background: rgba(0x00000000).into(),
        border_widths: Edges::all(px(lw)),
        border_color: glow.into(),
        border_style: gpui::BorderStyle::Solid,
    });
}

// ==========================================
// 软萌手势与微表情道具
// ==========================================

fn rot_pt_at(origin: Point<Pixels>, dx: f32, dy: f32, rot: f32) -> Point<Pixels> {
    let (s, c) = rot.sin_cos();
    point(origin.x + px(dx * c - dy * s), origin.y + px(dx * s + dy * c))
}

fn draw_ok_hand(pos: Point<Pixels>, s: f32, tilt: f32, window: &mut Window) {
    let p = |dx: f32, dy: f32| rot_pt_at(pos, dx * s, dy * s, tilt - 0.12);
    let mut hand = PathBuilder::fill();
    hand.move_to(p(-0.03, 0.40));
    hand.cubic_bezier_to(p(-0.30, 0.08), p(-0.20, 0.40), p(-0.29, 0.26));
    hand.cubic_bezier_to(p(-0.29, -0.14), p(-0.36, -0.02), p(-0.36, -0.13));
    hand.cubic_bezier_to(p(-0.21, -0.05), p(-0.24, -0.15), p(-0.22, -0.08));
    hand.line_to(p(-0.24, -0.22));
    hand.cubic_bezier_to(p(-0.145, -0.23), p(-0.25, -0.29), p(-0.16, -0.30));
    hand.line_to(p(-0.12, -0.13));
    hand.line_to(p(-0.135, -0.33));
    hand.cubic_bezier_to(p(-0.04, -0.33), p(-0.14, -0.41), p(-0.045, -0.41));
    hand.line_to(p(-0.02, -0.16));
    hand.cubic_bezier_to(p(0.36, -0.04), p(0.08, -0.28), p(0.33, -0.26));
    hand.cubic_bezier_to(p(-0.03, 0.40), p(0.39, 0.16), p(0.24, 0.36));
    hand.close();
    if let Ok(path) = hand.build() {
        window.paint_path(path, rgba(0xFFFFFFFF));
    }
    let mut hole = PathBuilder::fill();
    let (hs, hc) = 0.16_f32.sin_cos();
    for i in 0..=14 {
        let th = 2.0 * PI * (i as f32 / 14.0);
        let ex = 0.115 * th.cos();
        let ey = 0.165 * th.sin();
        let pt = p(0.14 + ex * hc - ey * hs, 0.02 + ex * hs + ey * hc);
        if i == 0 {
            hole.move_to(pt);
        } else {
            hole.line_to(pt);
        }
    }
    hole.close();
    if let Ok(path) = hole.build() {
        window.paint_path(path, rgba(0x06070AFF));
    }
}

fn draw_thumbs_up_hand(pos: Point<Pixels>, s: f32, dir: f32, rot: f32, window: &mut Window) {
    let p = |dx: f32, dy: f32| rot_pt_at(pos, dx * dir * s, dy * s, rot);
    let mut hand = PathBuilder::fill();
    hand.move_to(p(-0.20, 0.30));
    hand.line_to(p(0.26, 0.30));
    hand.cubic_bezier_to(p(0.31, 0.19), p(0.35, 0.30), p(0.37, 0.21));
    hand.cubic_bezier_to(p(0.31, 0.06), p(0.37, 0.17), p(0.37, 0.08));
    hand.cubic_bezier_to(p(0.31, -0.07), p(0.37, 0.04), p(0.37, -0.05));
    hand.cubic_bezier_to(p(0.26, -0.18), p(0.37, -0.09), p(0.35, -0.18));
    hand.line_to(p(-0.12, -0.18));
    hand.curve_to(p(-0.16, -0.24), p(-0.16, -0.18));
    hand.line_to(p(-0.16, -0.41));
    hand.cubic_bezier_to(p(-0.32, -0.41), p(-0.16, -0.50), p(-0.32, -0.50));
    hand.line_to(p(-0.32, 0.18));
    hand.cubic_bezier_to(p(-0.20, 0.30), p(-0.32, 0.27), p(-0.27, 0.30));
    hand.close();
    if let Ok(path) = hand.build() {
        window.paint_path(path, rgba(0xFFFFFFFF));
    }
}

fn draw_pray_hands(pos: Point<Pixels>, s: f32, tilt: f32, anim_t: f32, window: &mut Window) {
    let smoothstep = |t: f32| -> f32 {
        let c = t.clamp(0.0, 1.0);
        c * c * (3.0 - 2.0 * c)
    };
    let open_amt = if anim_t <= 0.0 {
        0.0
    } else {
        let phase_t = anim_t.rem_euclid(4.2);
        if phase_t < 1.15 {
            1.0 - smoothstep(phase_t / 1.15).powf(0.85)
        } else if phase_t < 3.55 {
            0.0
        } else {
            smoothstep((phase_t - 3.55) / 0.65)
        }
    };

    let bow_y = (1.0 - open_amt) * s * 0.035;
    let prayer_tilt = tilt + 0.44 - open_amt * 0.04;
    let base_pos = rot_pt_at(pos, 0.0, bow_y, tilt);

    for side in [-1.0_f32, 1.0_f32] {
        let gap_x = side * (s * 0.036 + open_amt * s * 0.088);
        let open_rot = side * open_amt * 0.19;
        let pivot = rot_pt_at(base_pos, gap_x, s * 0.14, prayer_tilt);
        let p = |dx: f32, dy: f32| rot_pt_at(pivot, side * dx * s, (dy - 0.14) * s, prayer_tilt + open_rot);

        let mut hand = PathBuilder::fill();
        hand.move_to(p(0.0, 0.14));
        hand.line_to(p(0.0, -0.26));
        hand.cubic_bezier_to(p(0.11, -0.24), p(0.0, -0.34), p(0.105, -0.34));
        hand.line_to(p(0.225, 0.01));
        hand.curve_to(p(0.30, 0.085), p(0.245, 0.055));
        hand.line_to(p(0.40, 0.14));
        hand.cubic_bezier_to(p(0.33, 0.28), p(0.46, 0.175), p(0.42, 0.30));
        hand.line_to(p(0.13, 0.21));
        hand.curve_to(p(0.0, 0.14), p(0.03, 0.185));
        hand.close();
        if let Ok(path) = hand.build() {
            window.paint_path(path, rgba(0xFFFFFFFF));
        }
    }
}

fn push_arc_points(
    path: &mut PathBuilder,
    pt_fn: &impl Fn(f32, f32) -> Point<Pixels>,
    center: (f32, f32),
    radius: f32,
    start: f32,
    end: f32,
    steps: usize,
) {
    for i in 1..=steps {
        let theta = start + (end - start) * (i as f32 / steps as f32);
        path.line_to(pt_fn(center.0 + radius * theta.cos(), center.1 + radius * theta.sin()));
    }
}

fn draw_chin_rest_hand(pos: Point<Pixels>, s: f32, tilt: f32, window: &mut Window) {
    let p = |dx: f32, dy: f32| rot_pt_at(pos, dx * s, dy * s, tilt);
    let mut hand = PathBuilder::fill();
    hand.move_to(p(-0.16, -0.46));
    hand.cubic_bezier_to(p(0.05, -0.14), p(-0.09, -0.46), p(-0.02, -0.32));
    hand.cubic_bezier_to(p(0.12, 0.22), p(0.18, -0.02), p(0.22, 0.10));
    hand.line_to(p(-0.08, 0.42));
    hand.cubic_bezier_to(p(-0.22, 0.31), p(-0.16, 0.48), p(-0.27, 0.41));
    hand.cubic_bezier_to(p(-0.12, -0.04), p(-0.15, 0.18), p(-0.08, 0.08));
    hand.cubic_bezier_to(p(-0.16, -0.46), p(-0.17, -0.20), p(-0.24, -0.44));
    hand.close();
    if let Ok(path) = hand.build() {
        window.paint_path(path, rgba(0xFFFFFFFF));
    }
}

fn draw_thinking_finger_hand(pos: Point<Pixels>, s: f32, tilt: f32, window: &mut Window) {
    let p = |dx: f32, dy: f32| rot_pt_at(pos, dx * s, dy * s, tilt);
    let mut hand = PathBuilder::fill();
    hand.move_to(p(-0.45, 0.22));
    hand.line_to(p(-0.45, -0.04));
    push_arc_points(&mut hand, &p, (-0.37, -0.04), 0.08, PI, 0.0, 6);
    hand.line_to(p(-0.29, -0.09));
    push_arc_points(&mut hand, &p, (-0.21, -0.09), 0.08, PI, 0.0, 6);
    hand.line_to(p(-0.13, -0.14));
    push_arc_points(&mut hand, &p, (-0.05, -0.14), 0.08, PI, 0.0, 6);
    hand.line_to(p(0.03, -0.42));
    push_arc_points(&mut hand, &p, (0.11, -0.42), 0.08, PI, 0.0, 6);
    hand.line_to(p(0.19, 0.19));
    hand.curve_to(p(0.25, 0.21), p(0.20, 0.24));
    hand.cubic_bezier_to(p(0.41, 0.25), p(0.32, 0.17), p(0.42, 0.18));
    hand.cubic_bezier_to(p(-0.10, 0.43), p(0.39, 0.33), p(0.22, 0.43));
    hand.cubic_bezier_to(p(-0.45, 0.22), p(-0.35, 0.43), p(-0.45, 0.35));
    hand.close();
    if let Ok(path) = hand.build() {
        window.paint_path(path, rgba(0xFFFFFFFF));
    }
}

fn draw_shy_palm_hand(pos: Point<Pixels>, s: f32, dir: f32, rot: f32, window: &mut Window) {
    let p = |dx: f32, dy: f32| rot_pt_at(pos, dx * dir * s, dy * s, rot);
    let mut hand = PathBuilder::fill();
    hand.move_to(p(-0.03, 0.32));
    hand.line_to(p(-0.14, 0.32));
    hand.curve_to(p(-0.25, 0.18), p(-0.25, 0.32));
    hand.line_to(p(-0.25, -0.14));
    hand.cubic_bezier_to(p(-0.155, -0.14), p(-0.25, -0.21), p(-0.155, -0.21));
    hand.line_to(p(-0.145, -0.09));
    hand.line_to(p(-0.145, -0.28));
    hand.cubic_bezier_to(p(-0.048, -0.28), p(-0.145, -0.35), p(-0.048, -0.35));
    hand.line_to(p(-0.038, -0.13));
    hand.line_to(p(-0.038, -0.35));
    hand.cubic_bezier_to(p(0.062, -0.35), p(-0.038, -0.42), p(0.062, -0.42));
    hand.line_to(p(0.072, -0.13));
    hand.line_to(p(0.072, -0.26));
    hand.cubic_bezier_to(p(0.165, -0.26), p(0.072, -0.33), p(0.165, -0.33));
    hand.line_to(p(0.165, 0.07));
    hand.curve_to(p(0.20, 0.07), p(0.175, 0.10));
    hand.line_to(p(0.24, 0.02));
    hand.cubic_bezier_to(p(0.295, 0.09), p(0.285, -0.03), p(0.34, 0.035));
    hand.line_to(p(0.21, 0.21));
    hand.curve_to(p(-0.03, 0.32), p(0.15, 0.32));
    hand.close();
    if let Ok(path) = hand.build() {
        window.paint_path(path, rgba(0xFFFFFFFF));
    }
}

fn draw_peace_sign(pos: Point<Pixels>, s: f32, tilt: f32, anim_t: f32, window: &mut Window) {
    let groove_y = (anim_t * 6.5).sin() * s * 0.025;
    let groove_rot = tilt + (anim_t * 6.5).sin() * 0.05;
    let center = point(pos.x, pos.y + px(groove_y));
    let p = |dx: f32, dy: f32| rot_pt_at(center, dx * s, dy * s, groove_rot);
    let mut hand = PathBuilder::fill();
    hand.move_to(p(0.50, -0.04));
    hand.cubic_bezier_to(p(0.02, -0.27), p(0.50, -0.22), p(0.30, -0.28));
    hand.cubic_bezier_to(p(-0.08, -0.17), p(-0.03, -0.27), p(-0.03, -0.19));
    hand.cubic_bezier_to(p(-0.16, -0.09), p(-0.13, -0.16), p(-0.13, -0.11));
    hand.line_to(p(-0.37, -0.13));
    hand.cubic_bezier_to(p(-0.37, -0.01), p(-0.46, -0.14), p(-0.47, -0.03));
    hand.line_to(p(-0.17, 0.04));
    hand.line_to(p(-0.42, 0.09));
    hand.cubic_bezier_to(p(-0.40, 0.23), p(-0.52, 0.11), p(-0.52, 0.23));
    hand.line_to(p(-0.15, 0.23));
    hand.cubic_bezier_to(p(0.18, 0.26), p(-0.03, 0.23), p(0.04, 0.27));
    hand.cubic_bezier_to(p(0.50, -0.04), p(0.40, 0.25), p(0.50, 0.14));
    hand.close();
    if let Ok(path) = hand.build() {
        window.paint_path(path, rgba(0xFFFFFFFF));
    }

    let twinkle = 1.0 + (anim_t * 8.0).sin() * 0.12;
    draw_sparkle(point(pos.x + px(s * 0.72), pos.y + px(s * 0.31)), s * 0.17 * twinkle, rgba(0xFFD447FF), window);
}

fn draw_mini_crown(pos: Point<Pixels>, s: f32, rot: f32, window: &mut Window) {
    let p = |dx: f32, dy: f32| rot_pt_at(pos, dx * s, dy * s, rot);
    let mut crown = PathBuilder::fill();
    crown.move_to(p(-0.52, 0.30));
    crown.line_to(p(-0.58, -0.22));
    crown.line_to(p(-0.22, 0.02));
    crown.line_to(p(0.0, -0.40));
    crown.line_to(p(0.22, 0.02));
    crown.line_to(p(0.58, -0.22));
    crown.line_to(p(0.52, 0.30));
    crown.close();
    if let Ok(path) = crown.build() {
        window.paint_path(path, rgba(0xFFFFFFFF));
    }
}

fn draw_glasses(center: Point<Pixels>, eye_dx: f32, radius: f32, tilt: f32, glow: Rgba, window: &mut Window) {
    let lw = (radius * 0.28).max(1.0);
    let lens_w = radius * 1.90;
    let lens_h = radius * 1.32;
    let gap = (eye_dx - lens_w * 0.5).max(radius * 0.12);
    let p = |dx: f32, dy: f32| rot_pt_at(center, dx, dy, tilt);

    for sign in [-1.0_f32, 1.0_f32] {
        let lc = p(sign * (gap + lens_w * 0.5), 0.0);
        window.paint_quad(PaintQuad {
            bounds: Bounds {
                origin: point(lc.x - px(lens_w * 0.5), lc.y - px(lens_h * 0.5)),
                size: gpui::size(px(lens_w), px(lens_h)),
            },
            corner_radii: Corners {
                top_left: px(radius * 0.28),
                top_right: px(radius * 0.28),
                bottom_right: px(radius * 0.62),
                bottom_left: px(radius * 0.62),
            },
            background: rgba(0x00000000).into(),
            border_widths: Edges::all(px(lw)),
            border_color: glow.into(),
            border_style: gpui::BorderStyle::Solid,
        });
    }
    let mut bridge = PathBuilder::stroke(px(lw));
    bridge.move_to(p(-gap, -lens_h * 0.30));
    bridge.line_to(p(gap, -lens_h * 0.30));
    if let Ok(path) = bridge.build() {
        window.paint_path(path, glow);
    }
}

fn draw_sparkle(center: Point<Pixels>, r: f32, color: Rgba, window: &mut Window) {
    let sr = px(r);
    let mut star = PathBuilder::fill();
    star.move_to(point(center.x, center.y - sr));
    star.curve_to(point(center.x + sr, center.y), center);
    star.curve_to(point(center.x, center.y + sr), center);
    star.curve_to(point(center.x - sr, center.y), center);
    star.curve_to(point(center.x, center.y - sr), center);
    star.close();
    if let Ok(path) = star.build() {
        window.paint_path(path, color);
    }
}

fn draw_lightning(center: Point<Pixels>, s: f32, color: Rgba, window: &mut Window) {
    let mut bolt = PathBuilder::fill();
    bolt.move_to(point(center.x, center.y - px(s * 0.5)));
    bolt.line_to(point(center.x - px(s * 0.28), center.y));
    bolt.line_to(point(center.x - px(s * 0.05), center.y));
    bolt.line_to(point(center.x - px(s * 0.18), center.y + px(s * 0.5)));
    bolt.line_to(point(center.x + px(s * 0.28), center.y - px(s * 0.05)));
    bolt.line_to(point(center.x + px(s * 0.05), center.y - px(s * 0.05)));
    bolt.close();
    if let Ok(path) = bolt.build() {
        window.paint_path(path, color);
    }
}

fn draw_zzz(pos: Point<Pixels>, s: f32, window: &mut Window) {
    let mut z = PathBuilder::stroke(px((s * 0.18).max(0.85)));
    // 大 Z
    z.move_to(point(pos.x, pos.y));
    z.line_to(point(pos.x + px(s * 0.55), pos.y));
    z.line_to(point(pos.x, pos.y + px(s * 0.52)));
    z.line_to(point(pos.x + px(s * 0.55), pos.y + px(s * 0.52)));
    // 右上小 z
    let zx = pos.x + px(s * 0.72);
    let zy = pos.y - px(s * 0.36);
    z.move_to(point(zx, zy));
    z.line_to(point(zx + px(s * 0.38), zy));
    z.line_to(point(zx, zy + px(s * 0.36)));
    z.line_to(point(zx + px(s * 0.38), zy + px(s * 0.36)));
    if let Ok(path) = z.build() {
        window.paint_path(path, rgba(0xFFFFFFCC));
    }
}

fn draw_stardust(pos: Point<Pixels>, s: f32, time_sec: f32, window: &mut Window) {
    for (dx, dy, speed) in [(-0.35_f32, 0.12_f32, 2.2_f32), (0.0, 0.24, 3.0), (0.32, 0.16, 2.6)] {
        let pulse = 0.55 + 0.45 * (time_sec * speed + dx * 6.0).sin();
        let dot_r = px((s * 0.09 * pulse).max(0.7));
        let cx = pos.x + px(s * dx);
        let cy = pos.y + px(s * dy);
        window.paint_quad(PaintQuad {
            bounds: Bounds {
                origin: point(cx - dot_r, cy - dot_r),
                size: gpui::size(dot_r * 2.0, dot_r * 2.0),
            },
            corner_radii: Corners::all(dot_r),
            background: rgba(0x4EF0FFCC).into(),
            border_widths: Edges::all(px(0.0)),
            border_color: rgba(0x00000000).into(),
            border_style: gpui::BorderStyle::Solid,
        });
    }
}

// ==========================================
// 乐器与舞台道具
// ==========================================

fn draw_single_dj(pos: Point<Pixels>, s: f32, scratch_rad: f32, window: &mut Window) {
    let lw = s * 0.082;
    let dx = s * 0.06;
    let dy = s * 0.14;
    let drx = s * 0.42;
    let dry = s * 0.26;
    let center = point(pos.x + px(dx), pos.y + px(dy));

    draw_arc_stroke(center, drx, dry, 1.45 * PI + scratch_rad, 2.95 * PI + scratch_rad, px(lw), rgba(0xFFFFFFFF), window);
    draw_arc_stroke(center, drx * 0.62, dry * 0.62, 0.22 * PI + scratch_rad, 0.82 * PI + scratch_rad, px(lw * 0.75), rgba(0xFFFFFFFF), window);

    let donut_w = px(s * 0.184);
    let donut_h = px(s * 0.116);
    window.paint_quad(PaintQuad {
        bounds: Bounds {
            origin: point(center.x - donut_w / 2.0, center.y - donut_h / 2.0),
            size: gpui::size(donut_w, donut_h),
        },
        corner_radii: Corners::all(donut_h / 2.0),
        background: rgba(0x00000000).into(),
        border_widths: Edges::all(px(lw)),
        border_color: rgba(0xFFFFFFFF).into(),
        border_style: gpui::BorderStyle::Solid,
    });

    let mut hand = PathBuilder::fill();
    hand.move_to(point(pos.x + px(-s * 0.34), pos.y + px(-s * 0.14)));
    hand.cubic_bezier_to(point(pos.x + px(-s * 0.04), pos.y + px(-s * 0.25)), point(pos.x + px(-s * 0.34), pos.y + px(-s * 0.24)), point(pos.x + px(-s * 0.22), pos.y + px(-s * 0.28)));
    hand.line_to(point(pos.x + px(s * 0.14), pos.y + px(-s * 0.21)));
    hand.cubic_bezier_to(point(pos.x + px(s * 0.14), pos.y + px(-s * 0.09)), point(pos.x + px(s * 0.21), pos.y + px(-s * 0.19)), point(pos.x + px(s * 0.21), pos.y + px(-s * 0.11)));
    hand.line_to(point(pos.x + px(-s * 0.04), pos.y + px(-s * 0.09)));
    hand.cubic_bezier_to(point(pos.x + px(-s * 0.15), pos.y + px(s * 0.12)), point(pos.x + px(-s * 0.05), pos.y + px(-s * 0.02)), point(pos.x + px(-s * 0.08), pos.y + px(s * 0.06)));
    hand.cubic_bezier_to(point(pos.x + px(-s * 0.33), pos.y + px(s * 0.02)), point(pos.x + px(-s * 0.22), pos.y + px(s * 0.16)), point(pos.x + px(-s * 0.30), pos.y + px(s * 0.10)));
    hand.cubic_bezier_to(point(pos.x + px(-s * 0.34), pos.y + px(-s * 0.14)), point(pos.x + px(-s * 0.36), pos.y + px(-s * 0.04)), point(pos.x + px(-s * 0.36), pos.y + px(-s * 0.08)));
    hand.close();
    if let Ok(p) = hand.build() {
        window.paint_path(p, rgba(0xFFFFFFFF));
    }

    draw_sparkle(point(pos.x + px(s * 0.34), pos.y + px(-s * 0.20)), s * 0.082, rgba(0xFFD166FF), window);
}

fn draw_dual_dj(pos: Point<Pixels>, s: f32, time_sec: f32, window: &mut Window) {
    let scratch_wave = (time_sec * 8.0).sin();
    let disc_dx = scratch_wave * s * 0.008;
    let disc_c = point(pos.x + px(disc_dx), pos.y + px(s * 0.14));
    let drx = s * 0.275;
    let dry = s * 0.132;

    // 1. 实心白色透视椭圆盘
    window.paint_quad(PaintQuad {
        bounds: Bounds {
            origin: point(disc_c.x - px(drx), disc_c.y - px(dry)),
            size: gpui::size(px(drx * 2.0), px(dry * 2.0)),
        },
        corner_radii: Corners::all(px(dry)),
        background: rgba(0xFFFFFFFF).into(),
        border_widths: Edges::all(px(0.0)),
        border_color: rgba(0x00000000).into(),
        border_style: gpui::BorderStyle::Solid,
    });

    // 2. 盘面上下双黑环圆洞
    window.paint_quad(PaintQuad {
        bounds: Bounds {
            origin: point(disc_c.x - px(s * 0.050), disc_c.y - px(s * 0.102 + s * 0.026)),
            size: gpui::size(px(s * 0.100), px(s * 0.052)),
        },
        corner_radii: Corners::all(px(s * 0.026)),
        background: rgba(0x00000000).into(),
        border_widths: Edges::all(px((s * 0.028).max(0.75))),
        border_color: rgba(0x06070AFF).into(),
        border_style: gpui::BorderStyle::Solid,
    });
    window.paint_quad(PaintQuad {
        bounds: Bounds {
            origin: point(disc_c.x - px(s * 0.054), disc_c.y + px(s * 0.028 - s * 0.048)),
            size: gpui::size(px(s * 0.108), px(s * 0.096)),
        },
        corner_radii: Corners::all(px(s * 0.048)),
        background: rgba(0x00000000).into(),
        border_widths: Edges::all(px((s * 0.036).max(0.85))),
        border_color: rgba(0x06070AFF).into(),
        border_style: gpui::BorderStyle::Solid,
    });

    // 3. 左右对称搓盘扇贝手（大拇指向内伸出）
    let move_l = scratch_wave * s * 0.014;
    let bob_l = (time_sec * 8.0).cos() * s * 0.012;
    for (dir, hx, hy, rot) in [
        (1.0_f32, -s * 0.215 + move_l, -s * 0.020 + bob_l, 0.04 + scratch_wave * 0.04),
        (-1.0_f32, s * 0.215 - move_l, -s * 0.020 - bob_l, -0.04 - scratch_wave * 0.04),
    ] {
        let h_center = point(pos.x + px(hx), pos.y + px(hy));
        let hs = s * 0.285;
        let p = |dx: f32, dy: f32| rot_pt_at(h_center, dx * dir * hs, dy * hs, rot);
        let trace = || {
            let mut h = PathBuilder::fill();
            h.move_to(p(-0.56, 0.06));
            h.cubic_bezier_to(p(-0.04, -0.33), p(-0.58, -0.18), p(-0.34, -0.33));
            h.line_to(p(0.13, -0.31));
            h.cubic_bezier_to(p(0.12, -0.17), p(0.25, -0.30), p(0.25, -0.17));
            h.line_to(p(0.01, -0.17));
            h.curve_to(p(-0.02, -0.07), p(-0.04, -0.16));
            h.cubic_bezier_to(p(0.16, 0.18), p(0.16, -0.02), p(0.28, 0.10));
            h.cubic_bezier_to(p(-0.03, 0.23), p(0.18, 0.26), p(0.04, 0.31));
            h.cubic_bezier_to(p(-0.23, 0.26), p(-0.03, 0.32), p(-0.18, 0.35));
            h.cubic_bezier_to(p(-0.44, 0.22), p(-0.25, 0.35), p(-0.42, 0.33));
            h.cubic_bezier_to(p(-0.56, 0.06), p(-0.54, 0.20), p(-0.58, 0.12));
            h.close();
            h
        };
        let _ = trace;
        if let Ok(path) = {
            let mut h = PathBuilder::fill();
            h.move_to(p(-0.56, 0.06));
            h.cubic_bezier_to(p(-0.04, -0.33), p(-0.58, -0.18), p(-0.34, -0.33));
            h.line_to(p(0.13, -0.31));
            h.cubic_bezier_to(p(0.12, -0.17), p(0.25, -0.30), p(0.25, -0.17));
            h.line_to(p(0.01, -0.17));
            h.curve_to(p(-0.02, -0.07), p(-0.04, -0.16));
            h.cubic_bezier_to(p(0.16, 0.18), p(0.16, -0.02), p(0.28, 0.10));
            h.cubic_bezier_to(p(-0.03, 0.23), p(0.18, 0.26), p(0.04, 0.31));
            h.cubic_bezier_to(p(-0.23, 0.26), p(-0.03, 0.32), p(-0.18, 0.35));
            h.cubic_bezier_to(p(-0.44, 0.22), p(-0.25, 0.35), p(-0.42, 0.33));
            h.cubic_bezier_to(p(-0.56, 0.06), p(-0.54, 0.20), p(-0.58, 0.12));
            h.close();
            h.build()
        } {
            window.paint_path(path, rgba(0xFFFFFFFF));
        }
    }

    // 4. 左侧白色双音符 ♫ + 右侧珊瑚红与闪烁青蓝单音符 ♪
    let float_y = (time_sec * 5.5).sin() * s * 0.025;
    draw_beamed_eighth_note(point(pos.x - px(s * 0.49), pos.y + px(-s * 0.06 + float_y)), s * 0.165, -0.08, rgba(0xFFFFFFFF), window);
    draw_crisp_eighth_note(point(pos.x + px(s * 0.48), pos.y + px(-s * 0.14 - float_y)), s * 0.175, 0.04, rgba(0xD94F64FF), window);
    let pulse_a = ((0.28 + ((time_sec * 6.5).sin() + 1.0) * 0.36) * 255.0) as u32;
    draw_crisp_eighth_note(point(pos.x + px(s * 0.45), pos.y + px(s * 0.10 + float_y * 0.8)), s * 0.155, 0.06, rgba((0x84DED8 << 8) | pulse_a), window);
}

fn draw_lightsticks(pos: Point<Pixels>, s: f32, time_sec: f32, window: &mut Window) {
    let wave = (time_sec * 4.5).sin() * 0.12;
    let ang = 0.28 + wave;
    for (dx, dy, side, top_col, bot_col) in [
        (-s * 0.15, s * 0.01, -1.0_f32, 0xFF9F43FF_u32, 0xFF5454FF_u32),
        (s * 0.16, 0.0, 1.0_f32, 0x5CE1E6FF_u32, 0x00A8E8FF_u32),
    ] {
        let bx = pos.x + px(dx);
        let by = pos.y + px(dy);
        let rot = |lx: f32, ly: f32| -> Point<Pixels> {
            point(bx + px(lx * ang.cos() - ly * ang.sin()), by + px(lx * ang.sin() + ly * ang.cos()))
        };
        let stick_w = s * 0.095;
        let stick_h = s * 0.42;
        let mut stick_bot = PathBuilder::stroke(px(stick_w));
        stick_bot.move_to(rot(0.0, 0.0));
        stick_bot.line_to(rot(0.0, -stick_h * 0.55));
        if let Ok(p) = stick_bot.build() {
            window.paint_path(p, rgba(bot_col));
        }
        let mut stick_top = PathBuilder::stroke(px(stick_w));
        stick_top.move_to(rot(0.0, -stick_h * 0.45));
        stick_top.line_to(rot(0.0, -stick_h));
        if let Ok(p) = stick_top.build() {
            window.paint_path(p, rgba(top_col));
        }
        let palm_c = rot(side * s * 0.014, s * 0.035);
        window.paint_quad(PaintQuad {
            bounds: Bounds {
                origin: point(palm_c.x - px(s * 0.078), palm_c.y - px(s * 0.09)),
                size: gpui::size(px(s * 0.156), px(s * 0.18)),
            },
            corner_radii: Corners::all(px(s * 0.078)),
            background: rgba(0xFFFFFFFF).into(),
            border_widths: Edges::all(px(0.0)),
            border_color: rgba(0x00000000).into(),
            border_style: gpui::BorderStyle::Solid,
        });
        for ky in [-s * 0.024, s * 0.012, s * 0.048, s * 0.082] {
            let kc = rot(-side * s * 0.060, ky);
            let kr = px(s * 0.024);
            window.paint_quad(PaintQuad {
                bounds: Bounds {
                    origin: point(kc.x - kr, kc.y - kr),
                    size: gpui::size(kr * 2.0, kr * 2.0),
                },
                corner_radii: Corners::all(kr),
                background: rgba(0xFFFFFFFF).into(),
                border_widths: Edges::all(px(0.0)),
                border_color: rgba(0x00000000).into(),
                border_style: gpui::BorderStyle::Solid,
            });
        }
    }
}

fn draw_drums(pos: Point<Pixels>, s: f32, time_sec: f32, window: &mut Window) {
    let lw = (s * 0.052).max(0.8);
    let drx = s * 0.40;
    let dry = s * 0.125;
    let shell_h = s * 0.315;

    // 1. 军鼓上表面椭圆鼓面
    draw_arc_stroke(pos, drx, dry, 0.0, 2.0 * PI, px(lw), rgba(0xFFFFFFFF), window);

    // 2. 鼓身侧壁与圆弧底缘
    let mut shell = PathBuilder::stroke(px(lw));
    shell.move_to(point(pos.x - px(drx), pos.y));
    shell.line_to(point(pos.x - px(drx), pos.y + px(shell_h)));
    let steps = 16;
    for i in 1..=steps {
        let theta = PI * (1.0 - i as f32 / steps as f32);
        shell.line_to(point(
            pos.x + px(drx * theta.cos()),
            pos.y + px(shell_h + dry * theta.sin()),
        ));
    }
    shell.line_to(point(pos.x + px(drx), pos.y));
    if let Ok(path) = shell.build() {
        window.paint_path(path, rgba(0xFFFFFFFF));
    }

    // 3. 左右拉杆 + 中央小圆点
    let mut rods = PathBuilder::stroke(px((lw * 0.58).max(0.6)));
    for rx_off in [-s * 0.18, s * 0.18] {
        rods.move_to(point(pos.x + px(rx_off), pos.y + px(s * 0.112)));
        rods.line_to(point(pos.x + px(rx_off), pos.y + px(shell_h + dry * 0.88)));
    }
    if let Ok(path) = rods.build() {
        window.paint_path(path, rgba(0xFFFFFFFF));
    }
    let dot_r = px((s * 0.025).max(0.6));
    window.paint_quad(PaintQuad {
        bounds: Bounds {
            origin: point(pos.x - dot_r, pos.y + px(s * 0.275) - dot_r),
            size: gpui::size(dot_r * 2.0, dot_r * 2.0),
        },
        corner_radii: Corners::all(dot_r),
        background: rgba(0xFFFFFFFF).into(),
        border_widths: Edges::all(px(0.0)),
        border_color: rgba(0x00000000).into(),
        border_style: gpui::BorderStyle::Solid,
    });

    // 4. 双鼓槌交替轻快敲击
    let beat = (time_sec * 8.5).sin();
    let left_lift = beat.max(0.0) * s * 0.04;
    let right_tap = (-beat).max(0.0) * 0.08;

    let l_c = point(pos.x - px(s * 0.22), pos.y - px(s * 0.24 + left_lift));
    let l_rot = -0.14 + beat * 0.06;
    let mut l_stick = PathBuilder::stroke(px((lw * 0.88).max(0.75)));
    l_stick.move_to(rot_pt_at(l_c, -s * 0.18, 0.0, l_rot));
    l_stick.line_to(rot_pt_at(l_c, s * 0.15, 0.0, l_rot));
    if let Ok(path) = l_stick.build() {
        window.paint_path(path, rgba(0xFFFFFFFF));
    }

    let r_c = point(pos.x + px(s * 0.23), pos.y - px(s * 0.09));
    let r_rot = -0.46 + right_tap;
    let mut r_mask = PathBuilder::stroke(px(lw * 1.85));
    r_mask.move_to(rot_pt_at(r_c, -s * 0.19, 0.0, r_rot));
    r_mask.line_to(rot_pt_at(r_c, s * 0.18, 0.0, r_rot));
    if let Ok(path) = r_mask.build() {
        window.paint_path(path, rgba(0x06070AFF));
    }
    let mut r_stick = PathBuilder::stroke(px((lw * 0.90).max(0.75)));
    r_stick.move_to(rot_pt_at(r_c, -s * 0.19, 0.0, r_rot));
    r_stick.line_to(rot_pt_at(r_c, s * 0.18, 0.0, r_rot));
    if let Ok(path) = r_stick.build() {
        window.paint_path(path, rgba(0xFFFFFFFF));
    }

    // 5. 周围 3 枚轻盈弹跳音符
    let float_y = (time_sec * 6.0).sin() * s * 0.045;
    draw_crisp_eighth_note(point(pos.x - px(s * 0.56), pos.y + px(-s * 0.03 + float_y)), s * 0.19, -0.18, rgba(0xFFFFFFFF), window);
    draw_crisp_eighth_note(point(pos.x + px(s * 0.54), pos.y + px(-s * 0.21 - float_y)), s * 0.18, 0.10, rgba(0xE85D5DFF), window);
    draw_crisp_eighth_note(point(pos.x + px(s * 0.52), pos.y + px(s * 0.08 + float_y)), s * 0.18, 0.12, rgba(0x7CE3E8FF), window);
}

fn draw_keyboard(pos: Point<Pixels>, s: f32, time_sec: f32, window: &mut Window) {
    let lw = (s * 0.032).max(0.8);
    let top_w = s * 1.02;
    let bot_w = s * 1.10;
    let kh = s * 0.285;
    let top_y = -kh * 0.5;
    let bot_y = kh * 0.5;
    let corner_r = s * 0.042;
    let p = |dx: f32, dy: f32| point(pos.x + px(dx), pos.y + px(dy));

    // 1. 三段 U 形白色外框（靠手的上边留空）
    let mut u_frame = PathBuilder::stroke(px(lw));
    u_frame.move_to(p(-top_w * 0.5, top_y + s * 0.01));
    u_frame.line_to(p(-bot_w * 0.5 + s * 0.004, bot_y - corner_r));
    u_frame.curve_to(p(-bot_w * 0.5 + corner_r, bot_y), p(-bot_w * 0.5, bot_y));
    u_frame.line_to(p(bot_w * 0.5 - corner_r, bot_y));
    u_frame.curve_to(p(bot_w * 0.5 - s * 0.004, bot_y - corner_r), p(bot_w * 0.5, bot_y));
    u_frame.line_to(p(top_w * 0.5, top_y + s * 0.01));
    if let Ok(path) = u_frame.build() {
        window.paint_path(path, rgba(0xFFFFFFFF));
    }

    // 2. 加粗黑框分隔后的开放式白键琴床
    let black_gap = s * 0.036;
    let pad_side = lw * 0.5 + black_gap;
    let in_top_w = top_w - pad_side * 2.0;
    let in_bot_w = bot_w - pad_side * 2.0;
    let in_top_y = top_y;
    let in_bot_y = bot_y - pad_side;
    let in_kh = in_bot_y - in_top_y;

    let mut bed = PathBuilder::fill();
    bed.move_to(p(-in_top_w * 0.5, in_top_y));
    bed.line_to(p(in_top_w * 0.5, in_top_y));
    bed.line_to(p(in_bot_w * 0.5, in_bot_y));
    bed.line_to(p(-in_bot_w * 0.5, in_bot_y));
    bed.close();
    if let Ok(path) = bed.build() {
        window.paint_path(path, rgba(0xFFFFFFFF));
    }

    // 8 枚白键纤细分隔线 + 底部 2 + 3 黑键
    let key_count = 8;
    let mut dividers = PathBuilder::stroke(px((s * 0.008).max(0.75)));
    for i in 1..key_count {
        let ratio = i as f32 / key_count as f32 - 0.5;
        dividers.move_to(p(ratio * in_top_w, in_top_y));
        dividers.line_to(p(ratio * in_bot_w, in_bot_y));
    }
    if let Ok(path) = dividers.build() {
        window.paint_path(path, rgba(0x06070AFF));
    }

    let bk_w = px(s * 0.058);
    let bk_h = px(in_kh * 0.56);
    let bk_center_w = in_top_w + (in_bot_w - in_top_w) * (1.0 - 0.28);
    for slot in [1, 2, 4, 5, 6] {
        let ratio = slot as f32 / key_count as f32 - 0.5;
        let bx = pos.x + px(ratio * bk_center_w) - bk_w / 2.0;
        window.paint_quad(PaintQuad {
            bounds: Bounds {
                origin: point(bx, pos.y + px(in_bot_y) - bk_h),
                size: gpui::size(bk_w, bk_h + px(0.6)),
            },
            corner_radii: Corners {
                top_left: bk_w * 0.34,
                top_right: bk_w * 0.34,
                bottom_right: px(0.0),
                bottom_left: px(0.0),
            },
            background: rgba(0x06070AFF).into(),
            border_widths: Edges::all(px(0.0)),
            border_color: rgba(0x00000000).into(),
            border_style: gpui::BorderStyle::Solid,
        });
    }

    // 3. 弹琴扇贝小手（轻微上下起伏与防碰撞漂移）
    let lift_l = (1.0 - (time_sec * 6.8).cos()) * 0.5 * s * 0.022;
    let lift_r = (1.0 - (time_sec * 8.2 + 2.1).cos()) * 0.5 * s * 0.024;
    let drift_l = (time_sec * 1.45 + 0.3).sin() * 0.55 + (time_sec * 2.35 + 1.7).sin() * 0.45;
    let drift_r = (time_sec * 1.65 + 2.4).sin() * 0.55 + (time_sec * 2.75 + 0.6).sin() * 0.45;
    let mut lx = -top_w * 0.27 + drift_l * s * 0.085;
    let mut rx = top_w * 0.27 + drift_r * s * 0.085;
    let min_gap = s * 0.38;
    if rx - lx < min_gap {
        let mid = (lx + rx) * 0.5;
        lx = mid - min_gap * 0.5;
        rx = mid + min_gap * 0.5;
    }

    for (hx, hy, dir, rot) in [
        (lx, top_y - s * 0.030 - lift_l, 1.0_f32, 0.04 + drift_l * 0.035),
        (rx, top_y - s * 0.034 - lift_r, -1.0_f32, -0.05 + drift_r * 0.035),
    ] {
        let h_center = p(hx, hy);
        let hs = s * 0.225;
        let hp = |dx: f32, dy: f32| rot_pt_at(h_center, dx * dir * hs, dy * hs, rot);
        let mut hand = PathBuilder::fill();
        hand.move_to(hp(-0.56, 0.08));
        hand.cubic_bezier_to(hp(0.52, 0.04), hp(-0.58, -0.34), hp(0.36, -0.36));
        hand.cubic_bezier_to(hp(0.39, 0.16), hp(0.56, 0.19), hp(0.45, 0.27));
        hand.cubic_bezier_to(hp(0.13, 0.18), hp(0.34, 0.01), hp(0.17, -0.01));
        hand.cubic_bezier_to(hp(-0.08, 0.18), hp(0.12, 0.33), hp(-0.06, 0.33));
        hand.cubic_bezier_to(hp(-0.30, 0.17), hp(-0.10, 0.33), hp(-0.28, 0.33));
        hand.cubic_bezier_to(hp(-0.56, 0.08), hp(-0.34, 0.30), hp(-0.58, 0.24));
        hand.close();
        if let Ok(path) = hand.build() {
            window.paint_path(path, rgba(0xFFFFFFFF));
        }
    }
}

fn draw_acoustic_guitar(
    pos: Point<Pixels>,
    s: f32,
    has_notes: bool,
    has_stardust: bool,
    time_sec: f32,
    window: &mut Window,
) {
    let strum = (time_sec * 8.0).sin() * 0.018;
    let ang = 0.24 + strum;
    let lw = (s * 0.043).max(0.8);
    let p = |dx: f32, dy: f32| rot_pt_at(pos, dx * s, dy * s, ang);

    // 1. 对称葫芦形空心琴身
    let mut body = PathBuilder::stroke(px(lw));
    body.move_to(p(-0.09, -0.034));
    body.cubic_bezier_to(p(0.03, -0.170), p(-0.08, -0.145), p(-0.02, -0.175));
    body.cubic_bezier_to(p(0.14, -0.135), p(0.08, -0.165), p(0.10, -0.128));
    body.cubic_bezier_to(p(0.32, -0.228), p(0.20, -0.148), p(0.24, -0.228));
    body.cubic_bezier_to(p(0.485, 0.0), p(0.42, -0.228), p(0.485, -0.135));
    body.cubic_bezier_to(p(0.32, 0.228), p(0.485, 0.135), p(0.42, 0.228));
    body.cubic_bezier_to(p(0.14, 0.135), p(0.24, 0.228), p(0.20, 0.148));
    body.cubic_bezier_to(p(0.03, 0.170), p(0.10, 0.128), p(0.08, 0.165));
    body.cubic_bezier_to(p(-0.09, 0.034), p(-0.02, 0.175), p(-0.08, 0.145));
    if let Ok(path) = body.build() {
        window.paint_path(path, rgba(0xFFFFFFFF));
    }

    // 2. 音孔 + 琴桥 + 平行空心琴颈 + 镂空琴头与旋钮
    let hole_x = 0.075;
    let hole_r = 0.068;
    draw_arc_stroke_rotated(pos, hole_x * s, 0.0, hole_r * s, hole_r * s, 0.0, 2.0 * PI, ang, px(lw), rgba(0xFFFFFFFF), window);

    let neck_half_h = 0.034;
    let neck_meet_x = hole_x - (hole_r * hole_r - neck_half_h * neck_half_h).max(0.0).sqrt();
    let mut details = PathBuilder::stroke(px(lw));
    details.move_to(p(0.295, -0.062));
    details.line_to(p(0.295, 0.062));
    details.move_to(p(-0.37, -neck_half_h));
    details.line_to(p(neck_meet_x, -neck_half_h));
    details.move_to(p(-0.37, neck_half_h));
    details.line_to(p(neck_meet_x, neck_half_h));
    // 琴头框
    details.move_to(p(-0.485, -0.056));
    details.line_to(p(-0.37, -0.056));
    details.line_to(p(-0.37, 0.056));
    details.line_to(p(-0.485, 0.056));
    details.close();
    if let Ok(path) = details.build() {
        window.paint_path(path, rgba(0xFFFFFFFF));
    }

    let mut pegs = PathBuilder::stroke(px((lw * 0.72).max(0.65)));
    for px_val in [-0.458_f32, -0.427, -0.396] {
        pegs.move_to(p(px_val, -0.056));
        pegs.line_to(p(px_val, -0.082));
        pegs.move_to(p(px_val, 0.056));
        pegs.line_to(p(px_val, 0.082));
    }
    if let Ok(path) = pegs.build() {
        window.paint_path(path, rgba(0xFFFFFFFF));
    }

    if has_notes {
        let float_y = (time_sec * 5.0).sin() * s * 0.04;
        draw_crisp_eighth_note(point(pos.x + px(s * 0.56), pos.y + px(-s * 0.12 + float_y)), s * 0.15, 0.20, rgba(0x00E5FFFF), window);
        draw_crisp_eighth_note(point(pos.x + px(s * 0.61), pos.y + px(s * 0.12 - float_y)), s * 0.14, -0.15, rgba(0xFF4D6DFF), window);
    }
    if has_stardust {
        draw_stardust(point(pos.x + px(s * 0.10), pos.y + px(s * 0.27)), s * 0.40, time_sec, window);
    }
}

fn draw_electric_guitar(
    pos: Point<Pixels>,
    s: f32,
    is_rock_star: bool,
    time_sec: f32,
    window: &mut Window,
) {
    let lw = (s * 0.034).max(0.75);
    if !is_rock_star {
        let strum = (time_sec * 9.5).sin() * 0.022;
        let ang = -0.22 + strum;
        let slide = (time_sec * 3.5).sin() * 0.08;
        let p = |dx: f32, dy: f32| rot_pt_at(pos, dx * s, dy * s, ang);
        let neck_half_h = 0.026;
        let neck_join_x = -0.195;
        let neck_end_x = 0.345;

        // 1. Stratocaster 双切角电吉他琴身
        let mut body = PathBuilder::stroke(px(lw));
        body.move_to(p(neck_join_x, -neck_half_h));
        body.cubic_bezier_to(p(-0.115, -0.172), p(-0.18, -0.095), p(-0.115, -0.132));
        body.cubic_bezier_to(p(-0.215, -0.162), p(-0.115, -0.205), p(-0.170, -0.202));
        body.cubic_bezier_to(p(-0.415, -0.190), p(-0.265, -0.125), p(-0.325, -0.195));
        body.cubic_bezier_to(p(-0.570, 0.015), p(-0.515, -0.184), p(-0.570, -0.085));
        body.cubic_bezier_to(p(-0.395, 0.208), p(-0.570, 0.125), p(-0.505, 0.212));
        body.cubic_bezier_to(p(-0.205, 0.128), p(-0.305, 0.204), p(-0.255, 0.122));
        body.cubic_bezier_to(p(-0.158, 0.068), p(-0.150, 0.135), p(-0.138, 0.098));
        body.cubic_bezier_to(p(neck_join_x, neck_half_h), p(-0.178, 0.038), p(-0.190, 0.035));
        // 拾音器 + 双线琴颈 + 单侧琴头
        body.move_to(p(-0.415, -0.048));
        body.line_to(p(-0.415, 0.048));
        body.move_to(p(neck_join_x, -neck_half_h));
        body.line_to(p(neck_end_x, -neck_half_h));
        body.move_to(p(neck_join_x, neck_half_h));
        body.line_to(p(neck_end_x, neck_half_h));
        body.move_to(p(neck_end_x, -neck_half_h));
        body.cubic_bezier_to(p(0.465, -0.072), p(0.375, -0.065), p(0.420, -0.082));
        body.cubic_bezier_to(p(0.485, 0.012), p(0.505, -0.062), p(0.515, -0.015));
        body.cubic_bezier_to(p(neck_end_x, neck_half_h), p(0.450, 0.038), p(0.385, 0.028));
        if let Ok(path) = body.build() {
            window.paint_path(path, rgba(0xFFFFFFFF));
        }

        // 音箱线尾
        let mut cable = PathBuilder::stroke(px((lw * 0.72).max(0.65)));
        cable.move_to(p(-0.562, 0.070));
        cable.cubic_bezier_to(p(-0.580, 0.218), p(-0.602, 0.110), p(-0.602, 0.178));
        if let Ok(path) = cable.build() {
            window.paint_path(path, rgba(0xFFFFFFFF));
        }

        // 握弦小白手 + 闪烁金色闪电 ⚡
        let hand_c = p(0.245 + slide, 0.042);
        window.paint_quad(PaintQuad {
            bounds: Bounds {
                origin: point(hand_c.x - px(s * 0.062), hand_c.y - px(s * 0.046)),
                size: gpui::size(px(s * 0.124), px(s * 0.092)),
            },
            corner_radii: Corners::all(px(s * 0.046)),
            background: rgba(0xFFFFFFFF).into(),
            border_widths: Edges::all(px(0.6)),
            border_color: rgba(0x06070AFF).into(),
            border_style: gpui::BorderStyle::Solid,
        });
        let bolt_pulse = 1.0 + (time_sec * 8.5).sin() * 0.08;
        draw_lightning(point(pos.x + px(s * 0.31), pos.y + px(s * 0.08)), s * 0.125 * bolt_pulse, rgba(0xFFD166FF), window);
    } else {
        // 摇滚之星星形飞翼电吉他
        let strum = (time_sec * 9.5).sin() * 0.025;
        let ang = -0.21 + strum;
        let p = |dx: f32, dy: f32| rot_pt_at(pos, dx * s, dy * s, ang);
        let mut star_body = PathBuilder::stroke(px(lw));
        star_body.move_to(p(-0.105, -0.005));
        star_body.line_to(p(-0.165, -0.168));
        star_body.line_to(p(-0.305, -0.088));
        star_body.line_to(p(-0.655, -0.122));
        star_body.line_to(p(-0.408, 0.052));
        star_body.line_to(p(-0.435, 0.295));
        star_body.line_to(p(-0.200, 0.162));
        star_body.line_to(p(0.025, 0.185));
        star_body.line_to(p(-0.085, 0.055));
        // 拾音器与指板
        star_body.move_to(p(-0.340, -0.028));
        star_body.line_to(p(-0.312, 0.042));
        star_body.move_to(p(-0.195, -0.012));
        star_body.line_to(p(0.192, -0.095));
        star_body.line_to(p(0.214, -0.025));
        star_body.line_to(p(-0.162, 0.058));
        star_body.close();
        // 梯形琴头
        star_body.move_to(p(0.192, -0.095));
        star_body.line_to(p(0.245, -0.142));
        star_body.line_to(p(0.365, -0.132));
        star_body.line_to(p(0.365, -0.062));
        star_body.line_to(p(0.214, -0.025));
        if let Ok(path) = star_body.build() {
            window.paint_path(path, rgba(0xFFFFFFFF));
        }

        let bolt_pulse = 1.0 + (time_sec * 8.5).sin() * 0.08;
        draw_lightning(point(pos.x + px(s * 0.42), pos.y - px(s * 0.20)), s * 0.105 * bolt_pulse, rgba(0xE9C46AFF), window);
        draw_lightning(point(pos.x + px(s * 0.28), pos.y + px(s * 0.06)), s * 0.155 * bolt_pulse, rgba(0xFFD166FF), window);
    }
}

fn draw_saxophone(pos: Point<Pixels>, s: f32, has_stardust: bool, time_sec: f32, window: &mut Window) {
    let sway = (time_sec * 4.0).sin() * 0.032;
    let lw = (s * 0.056).max(0.9);
    let p = |dx: f32, dy: f32| rot_pt_at(pos, dx * s, dy * s, sway);

    // 1. 空心斜体萨克斯管身（扁平吹嘴 + 紧密贴壁 U 弯 + 上扬喇叭口连接）
    let mut inner_wall = PathBuilder::stroke(px(lw));
    inner_wall.move_to(p(-0.41, -0.48));
    inner_wall.cubic_bezier_to(p(-0.07, -0.14), p(-0.20, -0.47), p(-0.13, -0.32));
    inner_wall.line_to(p(0.040, 0.26));
    inner_wall.cubic_bezier_to(p(0.085, 0.22), p(0.055, 0.31), p(0.082, 0.30));
    inner_wall.line_to(p(0.082, 0.075));
    if let Ok(path) = inner_wall.build() {
        window.paint_path(path, rgba(0xFFFFFFFF));
    }

    let mut outer_wall = PathBuilder::stroke(px(lw));
    outer_wall.move_to(p(-0.41, -0.48));
    outer_wall.curve_to(p(-0.40, -0.428), p(-0.445, -0.458));
    outer_wall.cubic_bezier_to(p(-0.18, -0.11), p(-0.27, -0.39), p(-0.22, -0.25));
    outer_wall.line_to(p(-0.080, 0.25));
    outer_wall.cubic_bezier_to(p(0.265, 0.29), p(-0.030, 0.50), p(0.215, 0.505));
    outer_wall.curve_to(p(0.345, 0.140), p(0.295, 0.195));
    if let Ok(path) = outer_wall.build() {
        window.paint_path(path, rgba(0xFFFFFFFF));
    }

    // 2. 高位斜切椭圆喇叭口
    let bell_c = p(0.214, 0.102);
    let bell_steps = 18;
    let mut bell = PathBuilder::stroke(px(lw));
    let (bs, bc) = (0.30_f32 + sway).sin_cos();
    for i in 0..=bell_steps {
        let theta = (i as f32 / bell_steps as f32) * 2.0 * PI;
        let lx = 0.138 * s * theta.cos();
        let ly = 0.074 * s * theta.sin();
        let pt = point(bell_c.x + px(lx * bc - ly * bs), bell_c.y + px(lx * bs + ly * bc));
        if i == 0 {
            bell.move_to(pt);
        } else {
            bell.line_to(pt);
        }
    }
    if let Ok(path) = bell.build() {
        window.paint_path(path, rgba(0xFFFFFFFF));
    }

    // 3. 管身左侧 2 枚外延音键
    let mut keys = PathBuilder::stroke(px(lw * 0.88));
    for t in [0.18_f32, 0.56] {
        let kx = -0.18 + 0.100 * t;
        let ky = -0.11 + 0.36 * t;
        keys.move_to(p(kx, ky));
        keys.line_to(p(kx - 0.082, ky + 0.030));
        keys.move_to(p(kx - 0.090, ky + 0.004));
        keys.line_to(p(kx - 0.072, ky + 0.056));
    }
    if let Ok(path) = keys.build() {
        window.paint_path(path, rgba(0xFFFFFFFF));
    }

    if has_stardust {
        draw_stardust(point(pos.x - px(s * 0.30), pos.y + px(s * 0.38)), s * 0.65, time_sec, window);
    }
}

fn draw_violin(
    pos: Point<Pixels>,
    s: f32,
    is_cello: bool,
    has_stardust: bool,
    time_sec: f32,
    window: &mut Window,
) {
    let dir = if is_cello { -1.0_f32 } else { 1.0_f32 };
    let bw = if is_cello { 1.20 } else { 1.0 };
    let body_tilt = (time_sec * if is_cello { 3.4 } else { 4.5 }).sin() * 0.02;
    let ang = (if is_cello { 0.45 } else { -0.58 }) + body_tilt;
    let lw = (s * 0.046).max(0.75);
    let p = |dx: f32, dy: f32| rot_pt_at(pos, dx * s, dy * s, ang);

    // 1. 葫芦镂空琴箱
    let waist_x = if is_cello { 1.08 } else { 1.0 };
    let inner_x = if is_cello { 0.96 } else { 1.0 };
    let mut body = PathBuilder::stroke(px(lw));
    body.move_to(p(0.024, dir * 0.265));
    body.cubic_bezier_to(p(0.152 * waist_x, dir * 0.065), p(0.140 * bw, dir * 0.262), p(0.182 * bw, dir * 0.155));
    body.cubic_bezier_to(p(0.172 * bw, -dir * 0.115), p(0.128 * inner_x, dir * 0.010), p(0.128 * inner_x, -dir * 0.055));
    body.cubic_bezier_to(p(0.0, -dir * 0.305), p(0.215 * bw, -dir * 0.195), p(0.152 * bw, -dir * 0.305));
    body.cubic_bezier_to(p(-0.172 * bw, -dir * 0.115), p(-0.152 * bw, -dir * 0.305), p(-0.215 * bw, -dir * 0.195));
    body.cubic_bezier_to(p(-0.152 * waist_x, dir * 0.065), p(-0.128 * inner_x, -dir * 0.055), p(-0.128 * inner_x, dir * 0.010));
    body.cubic_bezier_to(p(-0.024, dir * 0.265), p(-0.182 * bw, dir * 0.155), p(-0.140 * bw, dir * 0.262));
    if let Ok(path) = body.build() {
        window.paint_path(path, rgba(0xFFFFFFFF));
    }

    // 大提琴尾柱脚撑与 F 音孔
    if is_cello {
        let mut cello_extra = PathBuilder::stroke(px((lw * 0.65).max(0.65)));
        cello_extra.move_to(p(0.0, 0.334));
        cello_extra.line_to(p(0.0, 0.435));
        for side in [-1.0_f32, 1.0] {
            cello_extra.move_to(p(side * 0.088, -0.035));
            cello_extra.curve_to(p(side * 0.082, 0.082), p(side * 0.106, 0.025));
        }
        if let Ok(path) = cello_extra.build() {
            window.paint_path(path, rgba(0xFFFFFFFF));
        }
    }

    // 2. 系弦板、琴桥与渐收指板
    let mut detail = PathBuilder::stroke(px((lw * 0.46).max(0.65)));
    detail.move_to(p(-0.030, -dir * 0.334));
    detail.line_to(p(0.030, -dir * 0.334));
    detail.line_to(p(0.025, -dir * 0.145));
    detail.line_to(p(-0.025, -dir * 0.145));
    detail.close();
    detail.move_to(p(-0.045, -dir * 0.088));
    detail.line_to(p(0.045, -dir * 0.088));
    detail.move_to(p(-0.032, dir * 0.038));
    detail.line_to(p(0.032, dir * 0.038));
    detail.line_to(p(0.024, dir * 0.355));
    detail.line_to(p(-0.024, dir * 0.355));
    detail.close();
    // 琴头与琴轸
    detail.move_to(p(-0.024, dir * 0.355));
    detail.cubic_bezier_to(p(0.0, dir * 0.468), p(-0.044, dir * 0.378), p(-0.039, dir * 0.432));
    detail.cubic_bezier_to(p(0.024, dir * 0.355), p(0.039, dir * 0.432), p(0.044, dir * 0.378));
    detail.move_to(p(-0.031, dir * 0.386));
    detail.line_to(p(-0.059, dir * 0.382));
    detail.move_to(p(0.031, dir * 0.386));
    detail.line_to(p(0.059, dir * 0.382));
    if let Ok(path) = detail.build() {
        window.paint_path(path, rgba(0xFFFFFFFF));
    }

    // 3. 90° 琴弓与饱满握弓小白手
    let bow_slide = (time_sec * if is_cello { 3.6 } else { 5.0 }).sin() * if is_cello { 0.062 } else { 0.050 };
    let bow_y = -dir * 0.025;
    let bow_left = if is_cello { -0.62 } else { -0.56 };

    let mut bow_mask = PathBuilder::stroke(px(lw * 1.45));
    bow_mask.move_to(p(bow_left + bow_slide, bow_y));
    bow_mask.line_to(p(0.005 + bow_slide, bow_y));
    if let Ok(path) = bow_mask.build() {
        window.paint_path(path, rgba(0x06070AFF));
    }

    let mut bow = PathBuilder::stroke(px((lw * 0.84).max(0.75)));
    bow.move_to(p(bow_left + bow_slide, bow_y));
    bow.line_to(p(0.005 + bow_slide, bow_y));
    if let Ok(path) = bow.build() {
        window.paint_path(path, rgba(0xFFFFFFFF));
    }

    let hx = (if is_cello { -0.56 } else { -0.50 }) + bow_slide;
    let mut bow_hand = PathBuilder::fill();
    bow_hand.move_to(p(hx - 0.115, bow_y - 0.020));
    bow_hand.cubic_bezier_to(p(hx + 0.105, bow_y - 0.015), p(hx - 0.125, bow_y - 0.105), p(hx + 0.085, bow_y - 0.105));
    bow_hand.cubic_bezier_to(p(hx + 0.045, bow_y + 0.058), p(hx + 0.115, bow_y + 0.068), p(hx + 0.068, bow_y + 0.092));
    bow_hand.cubic_bezier_to(p(hx - 0.024, bow_y + 0.058), p(hx + 0.032, bow_y + 0.096), p(hx - 0.012, bow_y + 0.096));
    bow_hand.cubic_bezier_to(p(hx - 0.080, bow_y + 0.052), p(hx - 0.036, bow_y + 0.092), p(hx - 0.080, bow_y + 0.090));
    bow_hand.cubic_bezier_to(p(hx - 0.115, bow_y - 0.020), p(hx - 0.102, bow_y + 0.070), p(hx - 0.138, bow_y + 0.036));
    bow_hand.close();
    if let Ok(path) = bow_hand.build() {
        window.paint_path(path, rgba(0xFFFFFFFF));
    }

    if has_stardust {
        draw_stardust(point(pos.x - px(s * 0.08), pos.y + px(s * 0.18)), s * 0.56, time_sec, window);
    }
}

fn draw_conductor(pos: Point<Pixels>, r: f32, time_sec: f32, window: &mut Window) {
    let beat_wave = (time_sec * 4.5).sin();
    let baton_sway = beat_wave * 0.14;
    let hand_bob = (time_sec * 4.5).cos() * r * 0.015;
    let hand_c = point(pos.x - px(r * 0.45), pos.y + px(r * 0.31 + hand_bob));

    // 1. 指挥棒（从 OK 捏合缘斜向右上挥舞，不遮挡 O 环镂空洞）
    let baton_origin = rot_pt_at(hand_c, r * 0.095, r * 0.055, baton_sway * 0.55);
    let baton_tip = rot_pt_at(baton_origin, r * 0.42, 0.0, -0.44 + baton_sway);
    let mut baton = PathBuilder::stroke(px((r * 0.034).max(0.8)));
    baton.move_to(baton_origin);
    baton.line_to(baton_tip);
    if let Ok(path) = baton.build() {
        window.paint_path(path, rgba(0xFFFFFFFF));
    }
    draw_ok_hand(hand_c, r * 0.42, baton_sway * 0.55, window);

    // 2. 下方居中圆角镂空领结 (Bow Tie)
    let bx = pos.x;
    let by = pos.y + px(r * 0.54);
    let bw = r * 0.185;
    let bh = r * 0.10;
    let kr = r * 0.046;
    let lw = (r * 0.036).max(0.8);
    let mut bow = PathBuilder::stroke(px(lw));
    // 左蝶翼
    bow.move_to(point(bx - px(kr * 0.75), by - px(bh * 0.38)));
    bow.curve_to(point(bx - px(bw), by - px(bh * 0.45)), point(bx - px(bw * 0.85), by - px(bh * 1.10)));
    bow.line_to(point(bx - px(bw), by + px(bh * 0.45)));
    bow.curve_to(point(bx - px(kr * 0.75), by + px(bh * 0.38)), point(bx - px(bw * 0.85), by + px(bh * 1.10)));
    // 右蝶翼
    bow.move_to(point(bx + px(kr * 0.75), by - px(bh * 0.38)));
    bow.curve_to(point(bx + px(bw), by - px(bh * 0.45)), point(bx + px(bw * 0.85), by - px(bh * 1.10)));
    bow.line_to(point(bx + px(bw), by + px(bh * 0.45)));
    bow.curve_to(point(bx + px(kr * 0.75), by + px(bh * 0.38)), point(bx + px(bw * 0.85), by + px(bh * 1.10)));
    if let Ok(path) = bow.build() {
        window.paint_path(path, rgba(0xFFFFFFFF));
    }
    window.paint_quad(PaintQuad {
        bounds: Bounds {
            origin: point(bx - px(kr), by - px(kr)),
            size: gpui::size(px(kr * 2.0), px(kr * 2.0)),
        },
        corner_radii: Corners::all(px(kr)),
        background: rgba(0x00000000).into(),
        border_widths: Edges::all(px(lw)),
        border_color: rgba(0xFFFFFFFF).into(),
        border_style: gpui::BorderStyle::Solid,
    });

    // 3. 右侧跳动珊瑚红音符 ♪
    let float_y = (time_sec * 5.2).sin() * r * 0.032;
    draw_crisp_eighth_note(point(pos.x + px(r * 0.52), pos.y + px(r * 0.22 + float_y)), r * 0.19, 0.02, rgba(0xD94F64FF), window);
}

fn draw_maracas(pos: Point<Pixels>, s: f32, time_sec: f32, window: &mut Window) {
    let phase = time_sec * 9.0;
    let lw = (s * 0.068).max(0.85);
    let handle_lw = (s * 0.058).max(0.8);

    for (dx, dy, ang) in [
        (-s * 0.245, phase.sin() * s * 0.026, -0.02 + phase.sin() * 0.055),
        (s * 0.245, -phase.sin() * s * 0.026, 0.02 - phase.sin() * 0.055),
    ] {
        let mc = point(pos.x + px(dx), pos.y + px(dy));
        let mp = |mx: f32, my: f32| rot_pt_at(mc, mx * s, my * s, ang);

        // 1. 一体水滴形镂空沙锤头
        let mut head = PathBuilder::stroke(px(lw));
        head.move_to(mp(0.0, -0.465));
        head.cubic_bezier_to(mp(0.060, 0.055), mp(0.235, -0.465), mp(0.245, -0.16));
        head.curve_to(mp(-0.060, 0.055), mp(0.0, 0.095));
        head.cubic_bezier_to(mp(0.0, -0.465), mp(-0.245, -0.16), mp(-0.235, -0.465));
        if let Ok(path) = head.build() {
            window.paint_path(path, rgba(0xFFFFFFFF));
        }

        // 2. 镂空 U 形长柄
        let hw = 0.060;
        let mut handle = PathBuilder::stroke(px(handle_lw));
        handle.move_to(mp(-hw, 0.075));
        handle.line_to(mp(-hw, 0.41));
        push_arc_points(&mut handle, &mp, (0.0, 0.41), hw, PI, 0.0, 6);
        handle.line_to(mp(hw, 0.075));
        if let Ok(path) = handle.build() {
            window.paint_path(path, rgba(0xFFFFFFFF));
        }

        // 3. 锤头左上角弧形高光短线
        draw_arc_stroke_rotated(mc, 0.015 * s, -0.245 * s, 0.085 * s, 0.085 * s, 1.15 * PI, 1.48 * PI, ang, px((s * 0.030).max(0.6)), rgba(0xFFFFFFFF), window);
    }
}

fn draw_tambourine(pos: Point<Pixels>, s: f32, time_sec: f32, window: &mut Window) {
    let shake_rot = (time_sec * 8.5).sin() * 0.10;
    let shake_bob = (time_sec * 12.0).sin() * s * 0.012;
    let center = point(pos.x, pos.y + px(shake_bob));
    let lw = (s * 0.048).max(0.8);
    let tr = s * 0.235;

    // 1. 铃鼓外框圆环
    draw_arc_stroke_rotated(center, 0.0, 0.0, tr, tr, 0.0, 2.0 * PI, shake_rot, px(lw), rgba(0xFFFFFFFF), window);

    // 2. 外圈 5 个镂空弧形金属响铃
    let jingle_dist = tr + lw * 0.86;
    let jingle_r = s * 0.066;
    for ang in [-2.54_f32, -1.67, -0.80, 0.07, 0.94] {
        let jx = ang.cos() * jingle_dist;
        let jy = ang.sin() * jingle_dist;
        draw_arc_stroke_rotated(
            center,
            jx,
            jy,
            jingle_r,
            jingle_r,
            ang - PI * 0.48,
            ang + PI * 0.48,
            shake_rot,
            px(lw * 0.84),
            rgba(0xFFFFFFFF),
            window,
        );
    }

    // 3. 左下方抓握铃鼓扇贝小手
    let hand_c = rot_pt_at(center, -tr * 0.34, tr * 0.38, shake_rot);
    let hs = s * 0.285;
    let hp = |dx: f32, dy: f32| rot_pt_at(hand_c, dx * hs, dy * hs, shake_rot + 0.72);
    let mut grip = PathBuilder::fill();
    grip.move_to(hp(-0.48, -0.05));
    grip.cubic_bezier_to(hp(0.12, 0.56), hp(-0.54, 0.42), hp(-0.18, 0.62));
    grip.cubic_bezier_to(hp(0.46, -0.04), hp(0.42, 0.50), hp(0.54, 0.22));
    grip.cubic_bezier_to(hp(0.20, -0.08), hp(0.40, -0.22), hp(0.24, -0.22));
    grip.cubic_bezier_to(hp(-0.03, -0.10), hp(0.15, -0.26), hp(-0.01, -0.26));
    grip.cubic_bezier_to(hp(-0.25, -0.10), hp(-0.07, -0.28), hp(-0.23, -0.27));
    grip.cubic_bezier_to(hp(-0.48, -0.05), hp(-0.30, -0.25), hp(-0.45, -0.21));
    grip.close();
    if let Ok(path) = grip.build() {
        window.paint_path(path, rgba(0xFFFFFFFF));
    }

    // 4. 右侧轻快飘动青蓝与珊瑚红音符 ♪
    let f1 = (time_sec * 6.0).sin() * s * 0.035;
    let f2 = (time_sec * 6.0 + 0.8).cos() * s * 0.035;
    draw_crisp_eighth_note(point(pos.x + px(s * 0.46), pos.y + px(-s * 0.08 + f1)), s * 0.20, -0.10, rgba(0x84DED8FF), window);
    draw_crisp_eighth_note(point(pos.x + px(s * 0.60), pos.y + px(s * 0.14 + f2)), s * 0.21, 0.04, rgba(0xD94F64FF), window);
}

fn draw_microphone(pos: Point<Pixels>, s: f32, has_stardust: bool, time_sec: f32, window: &mut Window) {
    let sway = (time_sec * 4.5).sin() * 0.045;
    let p = |dx: f32, dy: f32| rot_pt_at(pos, dx * s, dy * s, sway);

    // 1. 麦柄细直杆（底部探出握拳下方，上方连接斜倾实心麦头颈部）
    let mut stem = PathBuilder::stroke(px((s * 0.068).max(0.9)));
    stem.move_to(p(0.0, -0.08));
    stem.line_to(p(0.035, 0.45));
    if let Ok(path) = stem.build() {
        window.paint_path(path, rgba(0xFFFFFFFF));
    }

    // 2. 右上斜倾饱满实心胶囊麦头（严格对齐 Web 端 roundRect 胶囊轮廓，避免椭圆顶部过尖）
    let head_c = p(0.025, -0.21);
    let mw = s * 0.235;
    let mh = s * 0.390;
    let rad = mw * 0.5;
    let straight_half = ((mh - mw) * 0.5).max(0.0);
    let hp = |mx: f32, my: f32| rot_pt_at(head_c, mx, my, sway + 0.76);

    let mut mic_head = PathBuilder::fill();
    let cap_steps = 10;
    // 上半圆拱 (PI -> 2*PI)
    for i in 0..=cap_steps {
        let th = PI + (i as f32 / cap_steps as f32) * PI;
        let pt = hp(rad * th.cos(), -straight_half + rad * th.sin());
        if i == 0 {
            mic_head.move_to(pt);
        } else {
            mic_head.line_to(pt);
        }
    }
    // 下半圆拱 (0 -> PI)
    for i in 0..=cap_steps {
        let th = (i as f32 / cap_steps as f32) * PI;
        mic_head.line_to(hp(rad * th.cos(), straight_half + rad * th.sin()));
    }
    mic_head.close();
    if let Ok(path) = mic_head.build() {
        window.paint_path(path, rgba(0xFFFFFFFF));
    }

    // 左上侧 3 道纤细独立防喷格栅齿口（每道独立成路径并控制细线宽，避免 36pt 小尺寸下粘连成黑块“咬缺”麦头）
    let grille_w = (s * 0.026).clamp(0.28, 0.42);
    for gy in [-0.082_f32, -0.014, 0.054] {
        let mut slit = PathBuilder::stroke(px(grille_w));
        slit.move_to(hp(-mw * 0.52, gy * s));
        slit.line_to(hp(-mw * 0.10, gy * s));
        if let Ok(path) = slit.build() {
            window.paint_path(path, rgba(0x06070AFF));
        }
    }

    // 3. 握持麦柄的可爱扇贝握拳小手（先画暗黑分隔描边再填充纯白，对齐 Web 端清晰手杆层次）
    let hand_c = p(0.018, 0.185);
    let hs = s * 0.255;
    let gp_fn = |dx: f32, dy: f32| rot_pt_at(hand_c, dx * hs, dy * hs, sway - 0.08);
    let trace_fist = |path: &mut PathBuilder| {
        path.move_to(gp_fn(0.14, -0.50));
        path.cubic_bezier_to(gp_fn(-0.48, 0.34), gp_fn(-0.42, -0.54), gp_fn(-0.62, -0.08));
        path.cubic_bezier_to(gp_fn(0.28, 0.42), gp_fn(-0.36, 0.64), gp_fn(0.10, 0.64));
        path.cubic_bezier_to(gp_fn(0.28, 0.18), gp_fn(0.42, 0.38), gp_fn(0.44, 0.20));
        path.cubic_bezier_to(gp_fn(0.29, -0.06), gp_fn(0.45, 0.14), gp_fn(0.46, -0.04));
        path.cubic_bezier_to(gp_fn(0.27, -0.28), gp_fn(0.45, -0.10), gp_fn(0.44, -0.28));
        path.cubic_bezier_to(gp_fn(0.14, -0.50), gp_fn(0.39, -0.34), gp_fn(0.34, -0.50));
        path.close();
    };

    let mut fist_outline = PathBuilder::stroke(px((s * 0.046).max(0.55)));
    trace_fist(&mut fist_outline);
    if let Ok(path) = fist_outline.build() {
        window.paint_path(path, rgba(0x06070AFF));
    }

    let mut fist = PathBuilder::fill();
    trace_fist(&mut fist);
    if let Ok(path) = fist.build() {
        window.paint_path(path, rgba(0xFFFFFFFF));
    }

    if has_stardust {
        draw_stardust(point(pos.x + px(s * 0.28), pos.y + px(s * 0.34)), s * 0.55, time_sec, window);
    }
}

fn draw_retro_radio(pos: Point<Pixels>, s: f32, time_sec: f32, window: &mut Window) {
    let rw = s * 0.72;
    let rh = s * 0.44;
    let lw = (s * 0.072).max(0.95);

    // 1. 收音机圆角外框（与官方 RetroRadio 比例一致）
    window.paint_quad(PaintQuad {
        bounds: Bounds {
            origin: point(pos.x - px(rw * 0.5), pos.y),
            size: gpui::size(px(rw), px(rh)),
        },
        corner_radii: Corners::all(px(s * 0.095)),
        background: rgba(0x00000000).into(),
        border_widths: Edges::all(px(lw)),
        border_color: rgba(0xFFFFFFFF).into(),
        border_style: gpui::BorderStyle::Solid,
    });

    // 2. 左侧圆润实体调频旋钮 + 右侧单条出声短横线
    let knob_r = px((s * 0.090).max(0.9));
    let ant_x = pos.x - px(rw * 0.24);
    window.paint_quad(PaintQuad {
        bounds: Bounds {
            origin: point(ant_x - knob_r, pos.y + px(rh * 0.5) - knob_r),
            size: gpui::size(knob_r * 2.0, knob_r * 2.0),
        },
        corner_radii: Corners::all(knob_r),
        background: rgba(0xFFFFFFFF).into(),
        border_widths: Edges::all(px(0.0)),
        border_color: rgba(0x00000000).into(),
        border_style: gpui::BorderStyle::Solid,
    });

    let mut slit = PathBuilder::stroke(px((s * 0.062).max(0.85)));
    slit.move_to(point(pos.x + px(rw * 0.08), pos.y + px(rh * 0.5)));
    slit.line_to(point(pos.x + px(rw * 0.28), pos.y + px(rh * 0.5)));
    if let Ok(path) = slit.build() {
        window.paint_path(path, rgba(0xFFFFFFFF));
    }

    // 3. 垂直拉杆天线与顶端小白球
    let ant_top_y = pos.y - px(s * 0.21);
    let mut antenna = PathBuilder::stroke(px((s * 0.050).max(0.75)));
    antenna.move_to(point(ant_x, pos.y));
    antenna.line_to(point(ant_x, ant_top_y));
    if let Ok(path) = antenna.build() {
        window.paint_path(path, rgba(0xFFFFFFFF));
    }
    let ball_r = px((s * 0.056).max(0.75));
    window.paint_quad(PaintQuad {
        bounds: Bounds {
            origin: point(ant_x - ball_r, ant_top_y - ball_r),
            size: gpui::size(ball_r * 2.0, ball_r * 2.0),
        },
        corner_radii: Corners::all(ball_r),
        background: rgba(0xFFFFFFFF).into(),
        border_widths: Edges::all(px(0.0)),
        border_color: rgba(0x00000000).into(),
        border_style: gpui::BorderStyle::Solid,
    });

    // 4. 天线右上方彩色电波小光点/短芒（依次弹出亮起再同步收隐）
    let spark_cycle = 1.8_f32;
    let spark_phase = ((time_sec % spark_cycle) + spark_cycle) % spark_cycle / spark_cycle;
    let spark_state = |idx: f32| -> (f32, f32) {
        let start = 0.03 + idx * 0.052;
        let pop_dur = 0.085;
        if spark_phase < start || spark_phase >= 0.95 {
            return (0.0, 0.4);
        }
        if spark_phase < start + pop_dur {
            let u = (spark_phase - start) / pop_dur;
            let alpha = (u * 1.6).min(1.0);
            let scale = if u < 0.6 {
                0.35 + (u / 0.6) * 1.03
            } else {
                1.38 - ((u - 0.6) / 0.4) * 0.38
            };
            return (alpha, scale);
        }
        if spark_phase < 0.84 {
            return (1.0, 1.0 + 0.05 * (time_sec * 8.0 + idx * 1.1).sin());
        }
        let u = (spark_phase - 0.84) / 0.11;
        (1.0 - u * u, 1.0 - u * 0.25)
    };

    let (a0, sc0) = spark_state(0.0);
    if a0 > 0.02 {
        let dot_r = px((s * 0.024 * sc0).max(0.55));
        let cx = ant_x + px(s * 0.07);
        let cy = ant_top_y - px(s * 0.13);
        window.paint_quad(PaintQuad {
            bounds: Bounds {
                origin: point(cx - dot_r, cy - dot_r),
                size: gpui::size(dot_r * 2.0, dot_r * 2.0),
            },
            corner_radii: Corners::all(dot_r),
            background: rgba((0xF7D794 << 8) | ((a0.clamp(0.0, 1.0) * 255.0) as u32)).into(),
            border_widths: Edges::all(px(0.0)),
            border_color: rgba(0x00000000).into(),
            border_style: gpui::BorderStyle::Solid,
        });
    }

    let (a1, sc1) = spark_state(1.0);
    if a1 > 0.02 {
        let sx = ant_x + px(s * 0.165);
        let sy = ant_top_y - px(s * 0.09);
        let mut s1 = PathBuilder::stroke(px((s * 0.036).max(0.65)));
        s1.move_to(point(sx - px(s * 0.025 * sc1), sy + px(s * 0.01 * sc1)));
        s1.line_to(point(sx + px(s * 0.025 * sc1), sy - px(s * 0.01 * sc1)));
        if let Ok(path) = s1.build() {
            window.paint_path(path, rgba((0xFF7A8A << 8) | ((a1.clamp(0.0, 1.0) * 255.0) as u32)));
        }
    }

    let (a2, sc2) = spark_state(2.0);
    if a2 > 0.02 {
        let sx = ant_x + px(s * 0.162);
        let sy = ant_top_y + px(s * 0.025);
        let mut s2 = PathBuilder::stroke(px((s * 0.040).max(0.65)));
        s2.move_to(point(sx - px(s * 0.032 * sc2), sy - px(s * 0.01 * sc2)));
        s2.line_to(point(sx + px(s * 0.033 * sc2), sy + px(s * 0.01 * sc2)));
        if let Ok(path) = s2.build() {
            window.paint_path(path, rgba((0x84DED8 << 8) | ((a2.clamp(0.0, 1.0) * 255.0) as u32)));
        }
    }
}

/// 13. NIO 车载复古音响与频谱柱 (NIO Boombox Speaker)
fn draw_spectrum(pos: Point<Pixels>, s: f32, time_sec: f32, window: &mut Window) {
    let sw = s * 0.76;
    let sh = s * 0.42;
    let lw = (s * 0.072).max(0.95);

    // 1. 音响圆角外框
    window.paint_quad(PaintQuad {
        bounds: Bounds {
            origin: point(pos.x - px(sw * 0.5), pos.y),
            size: gpui::size(px(sw), px(sh)),
        },
        corner_radii: Corners::all(px(s * 0.088)),
        background: rgba(0x00000000).into(),
        border_widths: Edges::all(px(lw)),
        border_color: rgba(0xFFFFFFFF).into(),
        border_style: gpui::BorderStyle::Solid,
    });

    // 2. "NIO" 矢量字母居中标识
    let cy = pos.y + px(sh * 0.50);
    let hh = s * 0.066;
    let hw = s * 0.042;
    let letter_lw = (s * 0.036).max(0.75);

    let nx = pos.x - px(s * 0.165);
    let mut n_and_i = PathBuilder::stroke(px(letter_lw));
    // N
    n_and_i.move_to(point(nx - px(hw), cy + px(hh)));
    n_and_i.line_to(point(nx - px(hw), cy - px(hh)));
    n_and_i.line_to(point(nx + px(hw), cy + px(hh)));
    n_and_i.line_to(point(nx + px(hw), cy - px(hh)));
    // I
    n_and_i.move_to(point(pos.x, cy - px(hh)));
    n_and_i.line_to(point(pos.x, cy + px(hh)));
    if let Ok(path) = n_and_i.build() {
        window.paint_path(path, rgba(0xFFFFFFFF));
    }

    // O
    let ox = pos.x + px(s * 0.165);
    window.paint_quad(PaintQuad {
        bounds: Bounds {
            origin: point(ox - px(hw * 1.05), cy - px(hh * 1.025)),
            size: gpui::size(px(hw * 2.1), px(hh * 2.05)),
        },
        corner_radii: Corners::all(px(hw * 1.05)),
        background: rgba(0x00000000).into(),
        border_widths: Edges::all(px(letter_lw)),
        border_color: rgba(0xFFFFFFFF).into(),
        border_style: gpui::BorderStyle::Solid,
    });

    // 3. 顶部 5 根淡青色圆头跳动均衡器频谱柱（与下方白框保持黑色间隙）
    let bar_heights = [
        0.55 + 0.35 * (time_sec * 7.5 + 0.4).sin(),
        0.34 + 0.28 * (time_sec * 10.5 - 0.8).sin(),
        0.52 + 0.34 * (time_sec * 8.8 + 0.1).sin(),
        0.68 + 0.30 * (time_sec * 11.2 + 0.9).sin(),
        0.54 + 0.34 * (time_sec * 9.4 + 0.25).sin(),
    ];
    let bar_step = s * 0.132;
    let bar_start_x = -bar_step * 2.0;
    let bar_base_y = -s * 0.14;
    let max_h = s * 0.20;
    let bar_w = px((s * 0.048).max(0.8));

    for (i, h_raw) in bar_heights.into_iter().enumerate() {
        let bx = pos.x + px(bar_start_x + i as f32 * bar_step);
        let bh = px(h_raw.max(0.12) * max_h);
        let top_y = pos.y + px(bar_base_y) - bh;
        window.paint_quad(PaintQuad {
            bounds: Bounds {
                origin: point(bx - bar_w / 2.0, top_y),
                size: gpui::size(bar_w, bh),
            },
            corner_radii: Corners::all(bar_w / 2.0),
            background: rgba(0x84DED8FF).into(),
            border_widths: Edges::all(px(0.0)),
            border_color: rgba(0x00000000).into(),
            border_style: gpui::BorderStyle::Solid,
        });
    }
}

fn draw_crisp_eighth_note(center: Point<Pixels>, size: f32, rot: f32, color: Rgba, window: &mut Window) {
    let p = |dx: f32, dy: f32| rot_pt_at(center, dx, dy, rot);
    let hx = -size * 0.07;
    let hy = size * 0.27;
    let rx = size * 0.175;
    let ry = size * 0.135;
    let (hs, hc) = (-0.35_f32).sin_cos();
    let mut head = PathBuilder::fill();
    for i in 0..=14 {
        let th = (i as f32 / 14.0) * 2.0 * PI;
        let lx = rx * th.cos();
        let ly = ry * th.sin();
        let pt = p(hx + lx * hc - ly * hs, hy + lx * hs + ly * hc);
        if i == 0 {
            head.move_to(pt);
        } else {
            head.line_to(pt);
        }
    }
    head.close();
    if let Ok(path) = head.build() {
        window.paint_path(path, color);
    }

    let mut stem = PathBuilder::stroke(px((size * 0.105).max(0.7)));
    stem.move_to(p(size * 0.055, size * 0.23));
    stem.line_to(p(size * 0.055, -size * 0.36));
    stem.curve_to(p(size * 0.25, -size * 0.05), p(size * 0.27, -size * 0.24));
    if let Ok(path) = stem.build() {
        window.paint_path(path, color);
    }
}

fn draw_beamed_eighth_note(center: Point<Pixels>, size: f32, rot: f32, color: Rgba, window: &mut Window) {
    let p = |dx: f32, dy: f32| rot_pt_at(center, dx, dy, rot);
    let rx = size * 0.175;
    let ry = size * 0.135;
    let (hs, hc) = (-0.32_f32).sin_cos();
    for (hx, hy) in [(-size * 0.30, size * 0.27), (size * 0.26, size * 0.20)] {
        let mut head = PathBuilder::fill();
        for i in 0..=14 {
            let th = (i as f32 / 14.0) * 2.0 * PI;
            let lx = rx * th.cos();
            let ly = ry * th.sin();
            let pt = p(hx + lx * hc - ly * hs, hy + lx * hs + ly * hc);
            if i == 0 {
                head.move_to(pt);
            } else {
                head.line_to(pt);
            }
        }
        head.close();
        if let Ok(path) = head.build() {
            window.paint_path(path, color);
        }
    }

    let mut stems = PathBuilder::stroke(px((size * 0.095).max(0.7)));
    stems.move_to(p(-size * 0.16, size * 0.23));
    stems.line_to(p(-size * 0.16, -size * 0.29));
    stems.move_to(p(size * 0.40, size * 0.16));
    stems.line_to(p(size * 0.40, -size * 0.36));
    if let Ok(path) = stems.build() {
        window.paint_path(path, color);
    }

    let mut beam = PathBuilder::stroke(px((size * 0.145).max(0.95)));
    beam.move_to(p(-size * 0.16, -size * 0.29));
    beam.line_to(p(size * 0.40, -size * 0.36));
    if let Ok(path) = beam.build() {
        window.paint_path(path, color);
    }
}

fn draw_dancing_notes(center: Point<Pixels>, r: f32, time_sec: f32, window: &mut Window) {
    let f1 = (time_sec * 4.8).sin() * r * 0.028;
    let f2 = (time_sec * 4.2 + 1.1).sin() * r * 0.025;
    let f3 = (time_sec * 5.2 + 2.3).sin() * r * 0.028;

    draw_crisp_eighth_note(point(center.x - px(r * 0.56), center.y + px(r * 0.17 + f1)), r * 0.20, -0.04, rgba(0xF2544EFF), window);
    draw_beamed_eighth_note(point(center.x - px(r * 0.03), center.y + px(r * 0.41 + f2)), r * 0.34, 0.0, rgba(0xFFFFFFFF), window);
    draw_crisp_eighth_note(point(center.x + px(r * 0.37), center.y + px(r * 0.32 + f3)), r * 0.23, -0.22, rgba(0x87E3F5FF), window);
}

fn draw_confetti_burst(center: Point<Pixels>, s: f32, time_sec: f32, window: &mut Window) {
    let lw = (s * 0.048).max(0.75);
    for (ly, w, phase) in [
        (-s * 0.16_f32, s * 0.11_f32, 2.0_f32),
        (0.0, s * 0.19, 1.0),
        (s * 0.16, s * 0.27, 0.0),
    ] {
        let pulse = 0.78 + 0.22 * (time_sec * 6.0 - phase * 0.8).sin();
        let a = (pulse.clamp(0.0, 1.0) * 255.0) as u32;
        let mut chev = PathBuilder::stroke(px(lw));
        chev.move_to(point(center.x - px(w), center.y + px(ly + s * 0.085)));
        chev.line_to(point(center.x, center.y + px(ly)));
        chev.line_to(point(center.x + px(w), center.y + px(ly + s * 0.085)));
        if let Ok(path) = chev.build() {
            window.paint_path(path, rgba((0xFFFFFF << 8) | a));
        }
    }

    let blink = ((0.72 + 0.28 * (time_sec * 7.5).sin()).clamp(0.0, 1.0) * 255.0) as u32;
    let sc = rgba((0xFFFFFF << 8) | blink);
    draw_sparkle(point(center.x, center.y - px(s * 0.32)), s * 0.075, sc, window);
    draw_sparkle(point(center.x - px(s * 0.28), center.y + px(s * 0.06)), s * 0.055, sc, window);
    draw_sparkle(point(center.x + px(s * 0.28), center.y + px(s * 0.06)), s * 0.055, sc, window);
}

// ==========================================
// 通用圆弧与配饰渲染
// ==========================================

#[allow(clippy::too_many_arguments)]
fn draw_arc_stroke(
    center: Point<Pixels>,
    rx: f32,
    ry: f32,
    start_angle: f32,
    end_angle: f32,
    width: Pixels,
    color: Rgba,
    window: &mut Window,
) {
    let steps = 20;
    let mut path = PathBuilder::stroke(width);
    for i in 0..=steps {
        let frac = i as f32 / steps as f32;
        let theta = start_angle + (end_angle - start_angle) * frac;
        let pt = point(center.x + px(rx * theta.cos()), center.y + px(ry * theta.sin()));
        if i == 0 {
            path.move_to(pt);
        } else {
            path.line_to(pt);
        }
    }
    if let Ok(p) = path.build() {
        window.paint_path(p, color);
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_arc_fill_rotated(
    pivot: Point<Pixels>,
    offset_x: f32,
    offset_y: f32,
    rx: f32,
    ry: f32,
    start_angle: f32,
    end_angle: f32,
    tilt: f32,
    color: Rgba,
    window: &mut Window,
) {
    let steps = 22;
    let (s, c) = tilt.sin_cos();
    let mut path = PathBuilder::fill();
    for i in 0..=steps {
        let frac = i as f32 / steps as f32;
        let theta = start_angle + (end_angle - start_angle) * frac;
        let lx = offset_x + rx * theta.cos();
        let ly = offset_y + ry * theta.sin();
        let pt = point(pivot.x + px(lx * c - ly * s), pivot.y + px(lx * s + ly * c));
        if i == 0 {
            path.move_to(pt);
        } else {
            path.line_to(pt);
        }
    }
    path.close();
    if let Ok(p) = path.build() {
        window.paint_path(p, color);
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_arc_stroke_rotated(
    pivot: Point<Pixels>,
    offset_x: f32,
    offset_y: f32,
    rx: f32,
    ry: f32,
    start_angle: f32,
    end_angle: f32,
    tilt: f32,
    width: Pixels,
    color: Rgba,
    window: &mut Window,
) {
    let steps = 20;
    let (s, c) = tilt.sin_cos();
    let mut path = PathBuilder::stroke(width);
    for i in 0..=steps {
        let frac = i as f32 / steps as f32;
        let theta = start_angle + (end_angle - start_angle) * frac;
        let lx = offset_x + rx * theta.cos();
        let ly = offset_y + ry * theta.sin();
        let pt = point(pivot.x + px(lx * c - ly * s), pivot.y + px(lx * s + ly * c));
        if i == 0 {
            path.move_to(pt);
        } else {
            path.line_to(pt);
        }
    }
    if let Ok(p) = path.build() {
        window.paint_path(p, color);
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_headwear(
    headwear: NomiHeadwear,
    cx: Pixels,
    cy: Pixels,
    rx: f32,
    ry: f32,
    tilt: f32,
    yaw_dx: f32,
    sprout_angle_deg: f32,
    hat_canvas: &HeadCanvas,
    time_sec: f32,
    glow_color: Rgba,
    pitch: f32,
    window: &mut Window,
) {
    let r = (rx + ry) * 0.5;
    let head_r = r * (1.0 + BEZEL_WIDTH_RATIO);
    let pivot = point(cx, cy);
    let (ts, tc) = tilt.sin_cos();
    let rot_local = |lx: f32, ly: f32| -> Point<Pixels> {
        let x = lx + yaw_dx * 0.5;
        point(cx + px(x * tc - ly * ts), cy + px(x * ts + ly * tc))
    };

    match headwear {
        NomiHeadwear::None => {}
        NomiHeadwear::Sprout => {
            let angle_rad = sprout_angle_deg.to_radians() + tilt;
            let base_p = rot_local(0.0, -head_r);
            let stalk_h = r * 0.24;
            let top_x = base_p.x + px(angle_rad.sin() * stalk_h);
            let top_y = base_p.y - px(angle_rad.cos() * stalk_h);

            window.paint_quad(PaintQuad {
                bounds: Bounds {
                    origin: point(base_p.x - px(r * 0.08), base_p.y - px(r * 0.025)),
                    size: gpui::size(px(r * 0.16), px(r * 0.05)),
                },
                corner_radii: Corners::all(px(r * 0.025)),
                background: rgba(0x38A169FF).into(),
                border_widths: Edges::all(px(0.0)),
                border_color: rgba(0x00000000).into(),
                border_style: gpui::BorderStyle::Solid,
            });
            let mut stalk = PathBuilder::stroke(px(r * 0.065));
            stalk.move_to(base_p);
            stalk.curve_to(point(top_x, top_y), point((base_p.x + top_x) / 2.0 + px(0.5), (base_p.y + top_y) / 2.0));
            if let Ok(p) = stalk.build() {
                window.paint_path(p, rgba(0x48BB78FF));
            }
            let leaf_w = px(r * 0.15);
            let leaf_h = px(r * 0.08);
            for (dx, color) in [(-leaf_w * 1.02, 0x68D391FF_u32), (px(0.6), 0x48BB78FF)] {
                window.paint_quad(PaintQuad {
                    bounds: Bounds {
                        origin: point(top_x + dx, top_y - leaf_h * 0.9),
                        size: gpui::size(leaf_w, leaf_h),
                    },
                    corner_radii: Corners::all(leaf_h / 2.0),
                    background: rgba(color).into(),
                    border_widths: Edges::all(px(0.0)),
                    border_color: rgba(0x00000000).into(),
                    border_style: gpui::BorderStyle::Solid,
                });
            }
        }
        NomiHeadwear::Detective => {
            let side_y = -head_r + r * 0.60;
            let peak_y = -head_r - r * 0.16;
            let cap_w = r * 0.86;

            // 1. 左右护耳侧边小翻扣 (位于帽檐两侧太阳穴处)
            for sign in [-1.0_f32, 1.0_f32] {
                let ecx = sign * (cap_w + r * 0.02);
                let ecy = side_y + r * 0.02;
                let erx = r * 0.075;
                let ery = r * 0.048;
                let (rs, rc) = (sign * 0.35).sin_cos();
                let mut tab_f = PathBuilder::fill();
                let mut tab_s = PathBuilder::stroke(px((r * 0.028).max(0.55)));
                for i in 0..=14 {
                    let th = 2.0 * PI * (i as f32 / 14.0);
                    let ex = erx * th.cos();
                    let ey = ery * th.sin();
                    let pt = rot_local(ecx + ex * rc - ey * rs, ecy + ex * rs + ey * rc);
                    if i == 0 {
                        tab_f.move_to(pt);
                        tab_s.move_to(pt);
                    } else {
                        tab_f.line_to(pt);
                        tab_s.line_to(pt);
                    }
                }
                tab_f.close();
                if let Ok(path) = tab_f.build() {
                    window.paint_path(path, rgba(0x9E6B47FF));
                }
                if let Ok(path) = tab_s.build() {
                    window.paint_path(path, rgba(0x5A3A25FF));
                }
            }

            // 2. 猎鹿帽主穹顶 (暖棕斜纹软呢，饱满贴合球体上沿)
            let mut crown = PathBuilder::fill();
            crown.move_to(rot_local(-cap_w, side_y));
            crown.cubic_bezier_to(
                rot_local(0.0, peak_y),
                rot_local(-cap_w * 0.96, peak_y + r * 0.08),
                rot_local(-r * 0.38, peak_y),
            );
            crown.cubic_bezier_to(
                rot_local(cap_w, side_y),
                rot_local(r * 0.38, peak_y),
                rot_local(cap_w * 0.96, peak_y + r * 0.08),
            );
            crown.curve_to(rot_local(-cap_w, side_y), rot_local(0.0, side_y - r * 0.22));
            crown.close();
            if let Ok(path) = crown.build() {
                window.paint_path(path, rgba(0x886043FF));
            }

            // 3. 帽身竖向细密织纹 (限定在穹顶曲率范围以内)
            for i in -7..=7 {
                let t = (i as f32 * 0.115).clamp(-0.92, 0.92);
                let top_x = t * cap_w * 0.72;
                let bot_x = t * cap_w * 0.92;
                let arch = 1.0 - (1.0 - t * t).sqrt();
                let sy0 = peak_y + r * 0.04 + (side_y - peak_y) * arch * 0.85;
                let sy1 = side_y - r * 0.11 * (1.0 - t * t);
                if sy1 > sy0 + r * 0.04 {
                    let mut rib = PathBuilder::stroke(px((r * 0.022).max(0.45)));
                    rib.move_to(rot_local(top_x, sy0));
                    rib.line_to(rot_local(bot_x, sy1));
                    if let Ok(path) = rib.build() {
                        window.paint_path(path, rgba(0x3A23145C));
                    }
                }
            }

            let mut crown_edge = PathBuilder::stroke(px((r * 0.032).max(0.65)));
            crown_edge.move_to(rot_local(-cap_w, side_y));
            crown_edge.cubic_bezier_to(
                rot_local(0.0, peak_y),
                rot_local(-cap_w * 0.96, peak_y + r * 0.08),
                rot_local(-r * 0.38, peak_y),
            );
            crown_edge.cubic_bezier_to(
                rot_local(cap_w, side_y),
                rot_local(r * 0.38, peak_y),
                rot_local(cap_w * 0.96, peak_y + r * 0.08),
            );
            if let Ok(path) = crown_edge.build() {
                window.paint_path(path, rgba(0x523421FF));
            }

            // 4. 左右两侧从帽顶发散的立体弧形滚边缝线
            for sign in [-1.0_f32, 1.0_f32] {
                let mut seam_dark = PathBuilder::stroke(px((r * 0.058).max(1.0)));
                seam_dark.move_to(rot_local(0.0, peak_y + r * 0.02));
                seam_dark.curve_to(
                    rot_local(sign * (cap_w - r * 0.03), side_y - r * 0.01),
                    rot_local(sign * r * 0.54, peak_y + r * 0.14),
                );
                if let Ok(path) = seam_dark.build() {
                    window.paint_path(path, rgba(0x523421FF));
                }

                let mut seam_light = PathBuilder::stroke(px((r * 0.032).max(0.65)));
                seam_light.move_to(rot_local(0.0, peak_y + r * 0.02));
                seam_light.curve_to(
                    rot_local(sign * (cap_w - r * 0.03), side_y - r * 0.01),
                    rot_local(sign * r * 0.54, peak_y + r * 0.14),
                );
                if let Ok(path) = seam_light.build() {
                    window.paint_path(path, rgba(0xB8845EFF));
                }
            }

            // 5. 帽顶扁平双翼蝴蝶结
            for sign in [-1.0_f32, 1.0_f32] {
                let bcx = sign * r * 0.14;
                let bcy = peak_y - r * 0.01;
                let brx = r * 0.12;
                let bry = r * 0.042;
                let (rs, rc) = (sign * 0.18).sin_cos();
                let mut wing_f = PathBuilder::fill();
                let mut wing_s = PathBuilder::stroke(px((r * 0.024).max(0.5)));
                for i in 0..=12 {
                    let th = 2.0 * PI * (i as f32 / 12.0);
                    let ex = brx * th.cos();
                    let ey = bry * th.sin();
                    let pt = rot_local(bcx + ex * rc - ey * rs, bcy + ex * rs + ey * rc);
                    if i == 0 {
                        wing_f.move_to(pt);
                        wing_s.move_to(pt);
                    } else {
                        wing_f.line_to(pt);
                        wing_s.line_to(pt);
                    }
                }
                wing_f.close();
                if let Ok(path) = wing_f.build() {
                    window.paint_path(path, rgba(0xB8845EFF));
                }
                if let Ok(path) = wing_s.build() {
                    window.paint_path(path, rgba(0x5A3A25FF));
                }
            }
            let knot_c = rot_local(0.0, peak_y - r * 0.01);
            window.paint_quad(PaintQuad {
                bounds: Bounds {
                    origin: point(knot_c.x - px(r * 0.055), knot_c.y - px(r * 0.042)),
                    size: gpui::size(px(r * 0.11), px(r * 0.084)),
                },
                corner_radii: Corners::all(px(r * 0.042)),
                background: rgba(0x9E6B47FF).into(),
                border_widths: Edges::all(px((r * 0.022).max(0.45))),
                border_color: rgba(0x5A3A25FF).into(),
                border_style: gpui::BorderStyle::Solid,
            });

            // 6. 正面弧形立体鸭舌帽檐 (下沿深色内衬暗影 + 上表层暖棕帽舌)
            let visor_w = cap_w * 0.97;
            let mut visor_under = PathBuilder::fill();
            visor_under.move_to(rot_local(-visor_w, side_y + r * 0.01));
            visor_under.curve_to(rot_local(visor_w, side_y + r * 0.01), rot_local(0.0, side_y - r * 0.24));
            visor_under.curve_to(rot_local(-visor_w, side_y + r * 0.01), rot_local(0.0, side_y - r * 0.08));
            visor_under.close();
            if let Ok(path) = visor_under.build() {
                window.paint_path(path, rgba(0x3B2416FF));
            }

            let mut visor_top = PathBuilder::fill();
            visor_top.move_to(rot_local(-visor_w, side_y + r * 0.01));
            visor_top.curve_to(rot_local(visor_w, side_y + r * 0.01), rot_local(0.0, side_y - r * 0.34));
            visor_top.curve_to(rot_local(-visor_w, side_y + r * 0.01), rot_local(0.0, side_y - r * 0.15));
            visor_top.close();
            if let Ok(path) = visor_top.build() {
                window.paint_path(path, rgba(0xB37D58FF));
            }

            let mut visor_edge = PathBuilder::stroke(px((r * 0.026).max(0.55)));
            visor_edge.move_to(rot_local(-visor_w, side_y + r * 0.01));
            visor_edge.curve_to(rot_local(visor_w, side_y + r * 0.01), rot_local(0.0, side_y - r * 0.34));
            visor_edge.curve_to(rot_local(-visor_w, side_y + r * 0.01), rot_local(0.0, side_y - r * 0.15));
            if let Ok(path) = visor_edge.build() {
                window.paint_path(path, rgba(0x5A3A25FF));
            }

            // 7. 侦探金色发光放大镜 (置于左眼，带左侧帽扣连接挂绳，手柄向左外侧微倾延伸)
            let mag_x = -r * 0.38;
            let mag_y = -r * 0.18;
            let mag_r = r * 0.32;
            let mut cord_b = PathBuilder::stroke(px((r * 0.028).max(0.6)));
            cord_b.move_to(rot_local(-cap_w + r * 0.02, side_y + r * 0.04));
            cord_b.line_to(rot_local(mag_x - mag_r * 1.12, mag_y + mag_r * 0.16));
            if let Ok(path) = cord_b.build() {
                window.paint_path(path, rgba(0x2E2520FF));
            }
            let mut handle_b = PathBuilder::stroke(px((r * 0.075).max(1.2)));
            handle_b.move_to(rot_local(mag_x - mag_r * 1.10, mag_y + mag_r * 0.16));
            handle_b.line_to(rot_local(mag_x - mag_r * 2.05, mag_y + mag_r * 0.30));
            if let Ok(path) = handle_b.build() {
                window.paint_path(path, rgba(0x8B5A2BFF));
            }
            let mut ferrule_b = PathBuilder::stroke(px((r * 0.06).max(1.0)));
            ferrule_b.move_to(rot_local(mag_x - mag_r * 0.96, mag_y + mag_r * 0.14));
            ferrule_b.line_to(rot_local(mag_x - mag_r * 1.18, mag_y + mag_r * 0.17));
            if let Ok(path) = ferrule_b.build() {
                window.paint_path(path, rgba(0xD4AF37FF));
            }
            let mag_c = rot_local(mag_x, mag_y);
            window.paint_quad(PaintQuad {
                bounds: Bounds {
                    origin: point(mag_c.x - px(mag_r), mag_c.y - px(mag_r)),
                    size: gpui::size(px(mag_r * 2.0), px(mag_r * 2.0)),
                },
                corner_radii: Corners::all(px(mag_r)),
                background: rgba(0xFFEB8238).into(),
                border_widths: Edges::all(px((r * 0.05).max(0.85))),
                border_color: rgba(0xD4AF37FF).into(),
                border_style: gpui::BorderStyle::Solid,
            });
        }
        NomiHeadwear::Santa => {
            let brim_y = -head_r + r * 0.22;
            let tip_x = r * 0.62;
            let tip_y = -head_r - r * 0.04;

            // 1. 垂坠红绒圣诞帽身（严格控制在 44pt 安全高度以内）
            let mut hat = PathBuilder::fill();
            hat.move_to(rot_local(-r * 0.56, brim_y + r * 0.05));
            hat.cubic_bezier_to(
                rot_local(tip_x, tip_y),
                rot_local(-r * 0.42, -head_r - r * 0.28),
                rot_local(r * 0.10, -head_r - r * 0.34),
            );
            hat.cubic_bezier_to(
                rot_local(r * 0.56, brim_y + r * 0.05),
                rot_local(r * 0.54, -head_r - r * 0.12),
                rot_local(r * 0.46, -head_r + r * 0.02),
            );
            hat.curve_to(rot_local(-r * 0.56, brim_y + r * 0.05), rot_local(0.0, brim_y - r * 0.12));
            hat.close();
            if let Ok(path) = hat.build() {
                window.paint_path(path, rgba(0xD62828FF));
            }

            // 右侧垂坠折叠阴影层次
            let mut fold = PathBuilder::fill();
            fold.move_to(rot_local(r * 0.14, -head_r - r * 0.16));
            fold.curve_to(rot_local(tip_x, tip_y), rot_local(r * 0.44, -head_r - r * 0.16));
            fold.cubic_bezier_to(
                rot_local(r * 0.34, brim_y - r * 0.02),
                rot_local(r * 0.52, -head_r - r * 0.08),
                rot_local(r * 0.44, -head_r + r * 0.03),
            );
            fold.curve_to(rot_local(r * 0.14, -head_r - r * 0.16), rot_local(r * 0.30, -head_r - r * 0.10));
            fold.close();
            if let Ok(path) = fold.build() {
                window.paint_path(path, rgba(0x9B1B24FF));
            }

            // 2. 蓬松雪白毛绒帽檐
            let brim_puffs = [
                (-r * 0.54, brim_y + r * 0.09, r * 0.115),
                (-r * 0.38, brim_y + r * 0.04, r * 0.125),
                (-r * 0.18, brim_y + r * 0.01, r * 0.135),
                (0.0, brim_y, r * 0.14),
                (r * 0.18, brim_y + r * 0.01, r * 0.135),
                (r * 0.38, brim_y + r * 0.04, r * 0.125),
                (r * 0.54, brim_y + r * 0.09, r * 0.115),
            ];
            for (px_off, py_off, pr) in brim_puffs {
                let sc = rot_local(px_off, py_off + r * 0.02);
                window.paint_quad(PaintQuad {
                    bounds: Bounds {
                        origin: point(sc.x - px(pr), sc.y - px(pr)),
                        size: gpui::size(px(pr * 2.0), px(pr * 2.0)),
                    },
                    corner_radii: Corners::all(px(pr)),
                    background: rgba(0xCBD5E1FF).into(),
                    border_widths: Edges::all(px(0.0)),
                    border_color: rgba(0x00000000).into(),
                    border_style: gpui::BorderStyle::Solid,
                });
            }
            for (px_off, py_off, pr) in brim_puffs {
                let fc = rot_local(px_off, py_off);
                window.paint_quad(PaintQuad {
                    bounds: Bounds {
                        origin: point(fc.x - px(pr), fc.y - px(pr)),
                        size: gpui::size(px(pr * 2.0), px(pr * 2.0)),
                    },
                    corner_radii: Corners::all(px(pr)),
                    background: rgba(0xFFFFFFFF).into(),
                    border_widths: Edges::all(px(0.0)),
                    border_color: rgba(0x00000000).into(),
                    border_style: gpui::BorderStyle::Solid,
                });
            }

            // 3. 右端白色大绒球
            let pom_c = rot_local(tip_x + r * 0.04, tip_y + r * 0.03);
            let pom_r = px(r * 0.125);
            window.paint_quad(PaintQuad {
                bounds: Bounds {
                    origin: point(pom_c.x - pom_r, pom_c.y - pom_r),
                    size: gpui::size(pom_r * 2.0, pom_r * 2.0),
                },
                corner_radii: Corners::all(pom_r),
                background: rgba(0xFFFFFFFF).into(),
                border_widths: Edges::all(px(0.6)),
                border_color: rgba(0xCBD5E1FF).into(),
                border_style: gpui::BorderStyle::Solid,
            });
        }
        NomiHeadwear::Beanie => {
            let cap_y = -r * 0.74;
            draw_arc_fill_rotated(pivot, yaw_dx * 0.4, cap_y, r * 0.58, r * 0.58, PI, 2.0 * PI, tilt, rgba(0xD92D38FF), window);
            let cuff_c = rot_local(0.0, cap_y + r * 0.04);
            window.paint_quad(PaintQuad {
                bounds: Bounds {
                    origin: point(cuff_c.x - px(r * 0.62), cuff_c.y - px(r * 0.09)),
                    size: gpui::size(px(r * 1.24), px(r * 0.18)),
                },
                corner_radii: Corners::all(px(r * 0.09)),
                background: rgba(0xF44336FF).into(),
                border_widths: Edges::all(px(0.0)),
                border_color: rgba(0x00000000).into(),
                border_style: gpui::BorderStyle::Solid,
            });
            let pom_c = rot_local(0.0, cap_y - r * 0.56);
            window.paint_quad(PaintQuad {
                bounds: Bounds {
                    origin: point(pom_c.x - px(r * 0.13), pom_c.y - px(r * 0.13)),
                    size: gpui::size(px(r * 0.26), px(r * 0.26)),
                },
                corner_radii: Corners::all(px(r * 0.13)),
                background: rgba(0xFFFFFFFF).into(),
                border_widths: Edges::all(px(0.0)),
                border_color: rgba(0x00000000).into(),
                border_style: gpui::BorderStyle::Solid,
            });
        }
        NomiHeadwear::Neko => {
            draw_arc_stroke_rotated(pivot, 0.0, 0.0, head_r + r * 0.015, head_r + r * 0.015, -0.80 * PI + tilt, -0.20 * PI + tilt, 0.0, px(r * 0.075), rgba(0x232630FF), window);
            for side in [-1.0_f32, 1.0_f32] {
                let angle = -0.5 * PI + side * 0.46 + tilt;
                let nx = angle.cos();
                let ny = angle.sin();
                let tx = -ny;
                let ty = nx;
                let base_c = point(cx + px(nx * head_r * 0.98), cy + px(ny * head_r * 0.98));
                let left_p = point(base_c.x - px(tx * r * 0.21), base_c.y - px(ty * r * 0.21));
                let right_p = point(base_c.x + px(tx * r * 0.21), base_c.y + px(ty * r * 0.21));
                let tip_p = point(cx + px(nx * (head_r + r * 0.28)), cy + px(ny * (head_r + r * 0.28)));

                let mut outer = PathBuilder::fill();
                outer.move_to(left_p);
                outer.curve_to(tip_p, point((left_p.x + tip_p.x) * 0.5 - px(tx * r * 0.06), (left_p.y + tip_p.y) * 0.5 - px(ty * r * 0.06)));
                outer.curve_to(right_p, point((right_p.x + tip_p.x) * 0.5 + px(tx * r * 0.06), (right_p.y + tip_p.y) * 0.5 + px(ty * r * 0.06)));
                outer.close();
                if let Ok(p) = outer.build() {
                    window.paint_path(p, rgba(0x2A2D38FF));
                }

                let in_l = point(base_c.x - px(tx * r * 0.11), base_c.y - px(ty * r * 0.11));
                let in_r = point(base_c.x + px(tx * r * 0.11), base_c.y + px(ty * r * 0.11));
                let in_tip = point(cx + px(nx * (head_r + r * 0.19)), cy + px(ny * (head_r + r * 0.19)));
                let mut inner = PathBuilder::fill();
                inner.move_to(in_l);
                inner.line_to(in_tip);
                inner.line_to(in_r);
                inner.close();
                if let Ok(p) = inner.build() {
                    window.paint_path(p, rgba(0xFF7EB3FF));
                }
            }
        }
        NomiHeadwear::Beret => {
            let beret_tilt = -0.16_f32 + tilt;
            let rot_b = |lx: f32, ly: f32| -> Point<Pixels> {
                let x = lx + yaw_dx * 0.4;
                point(cx + px(x * beret_tilt.cos() - ly * beret_tilt.sin()), cy + px(x * beret_tilt.sin() + ly * beret_tilt.cos()))
            };
            let mut band = PathBuilder::fill();
            band.move_to(rot_b(-r * 0.54, -head_r + r * 0.12));
            band.curve_to(rot_b(r * 0.54, -head_r + r * 0.12), rot_b(0.0, -head_r - r * 0.01));
            band.curve_to(rot_b(-r * 0.54, -head_r + r * 0.12), rot_b(0.0, -head_r + r * 0.06));
            band.close();
            if let Ok(p) = band.build() {
                window.paint_path(p, rgba(0x581421FF));
            }
            let mut crown = PathBuilder::fill();
            crown.move_to(rot_b(-r * 0.52, -head_r + r * 0.08));
            crown.cubic_bezier_to(rot_b(-r * 0.04, -head_r - r * 0.22), rot_b(-r * 0.78, -head_r - r * 0.02), rot_b(-r * 0.56, -head_r - r * 0.23));
            crown.cubic_bezier_to(rot_b(r * 0.52, -head_r + r * 0.08), rot_b(r * 0.50, -head_r - r * 0.22), rot_b(r * 0.74, -head_r - r * 0.02));
            crown.curve_to(rot_b(-r * 0.52, -head_r + r * 0.08), rot_b(0.0, -head_r - r * 0.01));
            crown.close();
            if let Ok(p) = crown.build() {
                window.paint_path(p, rgba(0xC22B42FF));
            }
            let mut stem = PathBuilder::stroke(px(r * 0.032));
            stem.move_to(rot_b(-r * 0.04, -head_r - r * 0.21));
            stem.line_to(rot_b(r * 0.02, -head_r - r * 0.27));
            if let Ok(p) = stem.build() {
                window.paint_path(p, rgba(0x6B1220FF));
            }
            draw_sparkle(rot_b(r * 0.30, -head_r + r * 0.01), r * 0.065, rgba(0xFFD54FFF), window);
        }
        other => {
            headwear::draw_extended_headwear(other, hat_canvas, time_sec, glow_color, pitch, window);
        }
    }
}

/// 任务完成欢呼撒花特效：双波次庆典爆发光环 + 24 颗大尺寸多形态礼花（彩纸片 / 彩珠 / 星芒 / S 形彩带）
fn draw_cheer_confetti(
    cx: Pixels,
    cy: Pixels,
    r: f32,
    bounds: Bounds<Pixels>,
    time_sec: f32,
    window: &mut Window,
) {
    // 1. 双波次金色/霓虹爆发光环（围绕 NOMI 外圈向外冲击扩散）
    for (wave_phase, rgb_hex) in [(0.0_f32, 0xFFD166_u32), (0.50_f32, 0x4EF0FF_u32)] {
        let wp = (time_sec * 1.65 + wave_phase).fract();
        let alpha = ((1.0 - wp) * 0.85).clamp(0.0, 1.0);
        if alpha > 0.03 {
            let halo_r = px(r * (0.92 + 0.72 * wp));
            let ring_w = px(((1.0 - wp) * 2.1 + 0.6).max(0.6));
            let a_byte = (alpha * 255.0) as u32;
            window.paint_quad(PaintQuad {
                bounds: Bounds {
                    origin: point(cx - halo_r, cy - halo_r),
                    size: gpui::size(halo_r * 2.0, halo_r * 2.0),
                },
                corner_radii: Corners::all(halo_r),
                background: rgba(0x00000000).into(),
                border_widths: Edges::all(ring_w),
                border_color: rgba((rgb_hex << 8) | a_byte).into(),
                border_style: gpui::BorderStyle::Solid,
            });
        }
    }

    // 2. 24 颗大尺寸多形态礼花粒子
    // (angle, phase, speed_off, rgb_hex, kind: 0=大彩纸片, 1=亮彩珠, 2=大四角星芒, 3=S形彩带丝带)
    let particles: [(f32, f32, f32, u32, u8); 24] = [
        (-2.85, 0.00, 0.00, 0xFFD166, 0), // 左侧金彩纸
        (-2.50, 0.14, 0.12, 0xFF5D8F, 3), // 左上霓虹粉彩带
        (-2.15, 0.28, 0.24, 0x4EF0FF, 2), // 左上电光青大星芒
        (-1.80, 0.42, 0.08, 0x68D391, 0), // 顶左嫩绿彩纸
        (-1.45, 0.06, 0.30, 0xFFFFFF, 2), // 顶中纯白大星芒
        (-1.10, 0.22, 0.18, 0xFFD166, 3), // 顶右金色长彩带
        (-0.78, 0.36, 0.26, 0xC084FC, 0), // 右上亮紫彩纸
        (-0.45, 0.50, 0.10, 0xFF5D8F, 2), // 右上霓虹粉星芒
        (-0.12, 0.10, 0.35, 0x4EF0FF, 3), // 右侧电光青彩带
        (0.20, 0.26, 0.20, 0xFFD166, 0),  // 右中金彩纸
        (0.55, 0.44, 0.14, 0x68D391, 2),  // 右下嫩绿星芒
        (0.95, 0.62, 0.28, 0xFF9F43, 1),  // 下右暖橙彩珠
        (2.35, 0.56, 0.22, 0xC084FC, 1),  // 左下亮紫彩珠
        (2.80, 0.70, 0.16, 0x4EF0FF, 0),  // 左中下电光青彩纸
        (-2.98, 0.78, 0.32, 0xFF9F43, 2), // 正左暖橙大星芒
        (-2.32, 0.84, 0.05, 0xFFD166, 1), // 左上亮金珠
        (-1.62, 0.64, 0.38, 0xFF5D8F, 3), // 正顶粉紫长彩带
        (-0.92, 0.74, 0.12, 0x68D391, 1), // 右上绿彩珠
        (-0.28, 0.88, 0.26, 0xFFFFFF, 2), // 右侧纯白星芒
        (0.05, 0.68, 0.34, 0xFF5D8F, 0),  // 右侧深粉彩纸
        (0.38, 0.82, 0.18, 0x4EF0FF, 1),  // 右下青彩珠
        (-0.62, 0.18, 0.40, 0xFFD166, 2), // 右斜上大金星芒
        (-2.02, 0.92, 0.24, 0xC084FC, 3), // 左上亮紫长彩带
        (2.58, 0.32, 0.36, 0xFFD166, 3),  // 左下金色飘带
    ];

    let min_x = bounds.origin.x - px(14.0);
    let max_x = bounds.origin.x + bounds.size.width + px(26.0);
    let min_y = bounds.origin.y - px(12.0);
    let max_y = bounds.origin.y + bounds.size.height + px(10.0);

    for (idx, (angle, phase, speed_off, rgb_hex, kind)) in particles.into_iter().enumerate() {
        let p = (time_sec * 1.45 + phase + speed_off * 0.25).fract();
        let alpha = (p * 8.0).min((1.0 - p) * 2.8).clamp(0.0, 1.0);
        if alpha <= 0.02 {
            continue;
        }
        let a_byte = (alpha * 255.0) as u32;
        let color = rgba((rgb_hex << 8) | a_byte);

        // 从屏幕黑底内侧 (0.38r) 猛然向外炸开至 1.85r 并摇曳下落
        let dist = r * (0.38 + 1.47 * p.powf(0.48));
        let flutter = (p * 11.0 + idx as f32 * 1.3).sin() * r * 0.14;
        let gravity = p * p * r * 0.58;

        let raw_x = cx + px(angle.cos() * dist + flutter);
        let raw_y = cy + px(angle.sin() * dist + gravity);
        let px_c = raw_x.clamp(min_x, max_x);
        let py_c = raw_y.clamp(min_y, max_y);

        match kind {
            0 => {
                // 大号旋转飘落彩纸片
                let rot = p * 7.5 + idx as f32 * 0.9;
                let hw = r * 0.22;
                let hh = r * 0.11;
                let (s, c) = rot.sin_cos();
                let corner = |dx: f32, dy: f32| {
                    point(px_c + px(dx * c - dy * s), py_c + px(dx * s + dy * c))
                };
                let mut quad = PathBuilder::fill();
                quad.move_to(corner(-hw, -hh));
                quad.line_to(corner(hw, -hh));
                quad.line_to(corner(hw, hh));
                quad.line_to(corner(-hw, hh));
                quad.close();
                if let Ok(path) = quad.build() {
                    window.paint_path(path, color);
                }
            }
            2 => {
                // 大号璀璨闪烁四角星芒
                let sr = r * (0.25 * (0.72 + 0.28 * (p * 14.0).sin()));
                draw_sparkle(point(px_c, py_c), sr, color, window);
            }
            3 => {
                // S 形飘舞庆典长彩带丝带（长度约 0.48r，线宽 1.6pt）
                let rot = angle + (p * 9.0 + idx as f32).sin() * 0.45;
                let half_len = r * 0.24;
                let amp = r * 0.08 * (p * 12.0 + idx as f32).cos();
                let (s, c) = rot.sin_cos();
                let pt_at = |dx: f32, dy: f32| {
                    point(px_c + px(dx * c - dy * s), py_c + px(dx * s + dy * c))
                };
                let mut ribbon = PathBuilder::stroke(px(1.6));
                ribbon.move_to(pt_at(-half_len, 0.0));
                ribbon.cubic_bezier_to(
                    pt_at(half_len, 0.0),
                    pt_at(-half_len * 0.35, -amp),
                    pt_at(half_len * 0.35, amp),
                );
                if let Ok(path) = ribbon.build() {
                    window.paint_path(path, color);
                }
            }
            _ => {
                // 高亮彩珠亮片
                let d = px(r * 0.22);
                window.paint_quad(PaintQuad {
                    bounds: Bounds {
                        origin: point(px_c - d / 2.0, py_c - d / 2.0),
                        size: gpui::size(d, d),
                    },
                    corner_radii: Corners::all(d / 2.0),
                    background: color.into(),
                    border_widths: Edges::all(px(0.0)),
                    border_color: rgba(0x00000000).into(),
                    border_style: gpui::BorderStyle::Solid,
                });
            }
        }
    }
}

/// 变脸过程中的 AMOLED 屏内特效：甩头流光残影 / 眨眼唤醒脉冲环
fn draw_face_change_screen_overlay(
    center: Point<Pixels>,
    r: f32,
    fc: NomiFaceChange,
    glow: Rgba,
    window: &mut Window,
) {
    let p = fc.progress.clamp(0.0, 1.0);
    let dir = fc.direction;
    let glow_rgb = glow_rgb_u32(glow);

    match fc.style {
        NomiFaceChangeStyle::HeadWhip if p > 0.22 && p < 0.78 => {
            let intensity = (((p - 0.22) / 0.56) * PI).sin().clamp(0.0, 1.0);
            let travel = ((p - 0.22) / 0.56 - 0.5) * -dir * r * 1.45;
            let base_alpha = intensity * 0.75;
            let streaks = [
                (-r * 0.34_f32, r * 0.68_f32, 1.15_f32, 0x38BDF8_u32),
                (-r * 0.08, r * 0.88, 1.45, glow_rgb),
                (r * 0.18, r * 0.74, 1.15, 0xFFFFFF),
                (r * 0.36, r * 0.52, 0.90, 0x38BDF8),
            ];
            for (sy, len, stroke_w, rgb_hex) in streaks {
                let a_byte = (base_alpha * 255.0) as u32;
                let mut line = PathBuilder::stroke(px(stroke_w));
                line.move_to(point(center.x + px(travel - len * 0.5), center.y + px(sy)));
                line.line_to(point(center.x + px(travel + len * 0.5), center.y + px(sy)));
                if let Ok(path) = line.build() {
                    window.paint_path(path, rgba((rgb_hex << 8) | a_byte));
                }
            }
        }
        NomiFaceChangeStyle::BlinkMorph if p >= 0.5 => {
            let t2 = (p - 0.5) / 0.5;
            let alpha = (1.0 - t2).max(0.0).powf(1.35) * 0.60;
            if alpha > 0.02 {
                let ring_r = px(r * (0.24 + t2 * 0.74));
                let a_byte = (alpha * 255.0) as u32;
                let border_w = px((1.4 * (1.0 - t2 * 0.5)).max(0.55));
                window.paint_quad(PaintQuad {
                    bounds: Bounds {
                        origin: point(center.x - ring_r, center.y - ring_r),
                        size: gpui::size(ring_r * 2.0, ring_r * 2.0),
                    },
                    corner_radii: Corners::all(ring_r),
                    background: rgba(0x00000000).into(),
                    border_widths: Edges::all(border_w),
                    border_color: rgba((glow_rgb << 8) | a_byte).into(),
                    border_style: gpui::BorderStyle::Solid,
                });
            }
        }
        NomiFaceChangeStyle::TransformSnap => {
            let smooth01 = |t: f32| {
                let c = t.clamp(0.0, 1.0);
                c * c * (3.0 - 2.0 * c)
            };
            let close_amt = if p < 0.5 {
                smooth01(p / 0.5)
            } else {
                1.0 - smooth01((p - 0.5) / 0.5)
            };
            if close_amt > 0.02 {
                let inner_gap = ((1.0 - close_amt) * r * 1.02).max(0.0);
                let outer_span = r * 1.06;
                let seam_a = ((0.35 + 0.60 * close_amt) * 255.0) as u32;
                for i in 0..6 {
                    let a0 = (i as f32 * PI) / 3.0 + (1.0 - close_amt) * 0.35;
                    let a1 = a0 + PI / 3.0 + 0.02;
                    let mut blade = PathBuilder::fill();
                    blade.move_to(point(center.x + px(a0.cos() * inner_gap), center.y + px(a0.sin() * inner_gap)));
                    blade.line_to(point(center.x + px(a0.cos() * outer_span), center.y + px(a0.sin() * outer_span)));
                    blade.line_to(point(center.x + px(a1.cos() * outer_span), center.y + px(a1.sin() * outer_span)));
                    blade.line_to(point(center.x + px(a1.cos() * inner_gap), center.y + px(a1.sin() * inner_gap)));
                    blade.close();
                    if let Ok(built) = blade.build() {
                        window.paint_path(built, rgba(0x1E293BE8));
                    }
                    let mut seam = PathBuilder::stroke(px((r * 0.024).max(0.5)));
                    seam.move_to(point(center.x + px(a0.cos() * inner_gap), center.y + px(a0.sin() * inner_gap)));
                    seam.line_to(point(center.x + px(a0.cos() * outer_span), center.y + px(a0.sin() * outer_span)));
                    if let Ok(built) = seam.build() {
                        window.paint_path(built, rgba((0x38BDF8 << 8) | seam_a));
                    }
                }
                if p > 0.38 && p < 0.62 {
                    let spark = (((p - 0.38) / 0.24) * PI).sin().clamp(0.0, 1.0);
                    let ring_r = px(r * (0.12 + 0.36 * (1.0 - spark)));
                    let a_byte = (spark * 255.0) as u32;
                    window.paint_quad(PaintQuad {
                        bounds: Bounds {
                            origin: point(center.x - ring_r, center.y - ring_r),
                            size: gpui::size(ring_r * 2.0, ring_r * 2.0),
                        },
                        corner_radii: Corners::all(ring_r),
                        background: rgba(0x00000000).into(),
                        border_widths: Edges::all(px((r * 0.036 * spark).max(0.65))),
                        border_color: rgba((0x7DD3FC << 8) | a_byte).into(),
                        border_style: gpui::BorderStyle::Solid,
                    });
                }
            }
        }
        _ => {}
    }
}

/// 国潮发光折扇挥掠遮面变脸特效（在 NOMI 头像前方横扫遮脸并洒出金色星芒）
fn draw_face_change_fan(
    center: Point<Pixels>,
    r: f32,
    p: f32,
    dir: f32,
    tilt: f32,
    window: &mut Window,
) {
    let p = p.clamp(0.0, 1.0);
    let fade_alpha = (p / 0.14).min((1.0 - p) / 0.14).clamp(0.0, 1.0);
    if fade_alpha <= 0.02 {
        return;
    }
    let smooth_p = p * p * (3.0 - 2.0 * p);
    let sweep_x = dir * (1.0 - 2.0 * smooth_p) * r * 1.48;
    let fan_pivot_x = sweep_x * 0.68;
    let fan_pivot_y = r * 0.78;
    let (ts, tc) = tilt.sin_cos();
    let pivot_pt = point(
        center.x + px(fan_pivot_x * tc - fan_pivot_y * ts),
        center.y + px(fan_pivot_x * ts + fan_pivot_y * tc),
    );

    let fan_rot = tilt - dir * (p - 0.5) * 0.92;
    let (fs, fc) = fan_rot.sin_cos();
    let fan_pt = |lx: f32, ly: f32| -> Point<Pixels> {
        point(
            pivot_pt.x + px(lx * fc - ly * fs),
            pivot_pt.y + px(lx * fs + ly * fc),
        )
    };

    let spread_half = (0.36 + 0.34 * (p * PI).sin()) * PI * 0.5;
    let a_start = -0.5 * PI - spread_half;
    let a_end = -0.5 * PI + spread_half;
    let fan_r = r * 1.32;
    let inner_r = fan_r * 0.28;
    let main_a = (fade_alpha * 255.0) as u32;

    // 1. 朱红鎏金扇面主体 + 鎏金边缘框线
    let mut sector = PathBuilder::fill();
    let mut border = PathBuilder::stroke(px((r * 0.045).max(0.8)));
    for i in 0..=18 {
        let theta = a_start + (a_end - a_start) * (i as f32 / 18.0);
        let pt = fan_pt(fan_r * theta.cos(), fan_r * theta.sin());
        if i == 0 {
            sector.move_to(pt);
            border.move_to(pt);
        } else {
            sector.line_to(pt);
            border.line_to(pt);
        }
    }
    for i in 0..=12 {
        let theta = a_end - (a_end - a_start) * (i as f32 / 12.0);
        let pt = fan_pt(inner_r * theta.cos(), inner_r * theta.sin());
        sector.line_to(pt);
        border.line_to(pt);
    }
    let start_outer = fan_pt(fan_r * a_start.cos(), fan_r * a_start.sin());
    sector.close();
    border.line_to(start_outer);
    if let Ok(path) = sector.build() {
        window.paint_path(path, rgba((0xD11D44 << 8) | main_a));
    }

    // 深红内扇轴暗影层
    let mid_r = fan_r * 0.52;
    let mut inner_shade = PathBuilder::fill();
    for i in 0..=12 {
        let theta = a_start + (a_end - a_start) * (i as f32 / 12.0);
        let pt = fan_pt(mid_r * theta.cos(), mid_r * theta.sin());
        if i == 0 {
            inner_shade.move_to(pt);
        } else {
            inner_shade.line_to(pt);
        }
    }
    for i in 0..=10 {
        let theta = a_end - (a_end - a_start) * (i as f32 / 10.0);
        inner_shade.line_to(fan_pt(inner_r * theta.cos(), inner_r * theta.sin()));
    }
    inner_shade.close();
    if let Ok(path) = inner_shade.build() {
        let shade_a = (fade_alpha * 195.0) as u32;
        window.paint_path(path, rgba((0x7F1D1D << 8) | shade_a));
    }

    // 2. 放射状鎏金扇骨与外围金边
    let rib_a = (fade_alpha * 195.0) as u32;
    for i in 0..=6 {
        let theta = a_start + (a_end - a_start) * (i as f32 / 6.0);
        let mut rib = PathBuilder::stroke(px((r * 0.025).max(0.55)));
        rib.move_to(pivot_pt);
        rib.line_to(fan_pt(fan_r * 0.97 * theta.cos(), fan_r * 0.97 * theta.sin()));
        if let Ok(path) = rib.build() {
            window.paint_path(path, rgba((0xFDE047 << 8) | rib_a));
        }
    }
    if let Ok(path) = border.build() {
        window.paint_path(path, rgba((0xFACC15 << 8) | main_a));
    }

    // 扇轴金钉
    let rivet_r = px((r * 0.055).max(0.85));
    window.paint_quad(PaintQuad {
        bounds: Bounds {
            origin: point(pivot_pt.x - rivet_r, pivot_pt.y - rivet_r),
            size: gpui::size(rivet_r * 2.0, rivet_r * 2.0),
        },
        corner_radii: Corners::all(rivet_r),
        background: rgba((0xD4AF37 << 8) | main_a).into(),
        border_widths: Edges::all(px(0.0)),
        border_color: rgba(0x00000000).into(),
        border_style: gpui::BorderStyle::Solid,
    });

    // 3. 扇缘尾迹闪烁金星芒
    let spark_a = (fade_alpha * 230.0) as u32;
    for (idx, offset) in [-0.72_f32, -0.50, -0.28].into_iter().enumerate() {
        let theta = -0.5 * PI + dir * (offset + (p - 0.5) * 0.4);
        let sx = fan_r * 0.96 * theta.cos();
        let sy = fan_r * 0.96 * theta.sin();
        let sr = r * (0.075 + 0.035 * (p * PI * 3.0 + idx as f32).sin());
        let rgb_hex = if idx == 1 { 0xFFFFFF_u32 } else { 0xFDE047_u32 };
        draw_sparkle(fan_pt(sx, sy), sr, rgba((rgb_hex << 8) | spark_a), window);
    }
}

fn draw_alert_badge_bounded(
    cx: Pixels,
    cy: Pixels,
    rx: f32,
    ry: f32,
    bounds: Bounds<Pixels>,
    time_sec: f32,
    window: &mut Window,
) {
    let badge_w = px(rx * 0.28);
    let badge_h = px(ry * 0.50);
    let float_y = px((time_sec * 6.0).sin() * 0.8);
    let min_x = bounds.origin.x + px(1.0);
    let max_x = bounds.origin.x + bounds.size.width - badge_w - px(0.5);
    let min_y = bounds.origin.y - px(2.5);
    let max_y = bounds.origin.y + bounds.size.height - badge_h;
    let bx = (cx + px(rx * 0.78)).clamp(min_x, max_x.max(min_x));
    let by = (cy - px(ry * 1.08) + float_y).clamp(min_y, max_y.max(min_y));
    let bar_h = badge_h * 0.60;
    let dot_d = badge_w * 0.90;

    window.paint_quad(PaintQuad {
        bounds: Bounds { origin: point(bx, by), size: gpui::size(badge_w, bar_h) },
        corner_radii: Corners::all(badge_w / 2.0),
        background: rgba(0xFF4D6DFF).into(),
        border_widths: Edges::all(px(0.0)),
        border_color: rgba(0x00000000).into(),
        border_style: gpui::BorderStyle::Solid,
    });
    window.paint_quad(PaintQuad {
        bounds: Bounds { origin: point(bx + (badge_w - dot_d) / 2.0, by + bar_h + px(0.9)), size: gpui::size(dot_d, dot_d) },
        corner_radii: Corners::all(dot_d / 2.0),
        background: rgba(0xFF4D6DFF).into(),
        border_widths: Edges::all(px(0.0)),
        border_color: rgba(0x00000000).into(),
        border_style: gpui::BorderStyle::Solid,
    });
}

#[allow(clippy::too_many_arguments)]
fn draw_work_pulse_dots(
    cx: Pixels,
    cy: Pixels,
    rx: f32,
    ry: f32,
    bounds: Bounds<Pixels>,
    time_sec: f32,
    glow_color: Rgba,
    window: &mut Window,
) {
    let dot_d = (rx * 0.14).clamp(1.9, 2.5);
    let min_x = bounds.origin.x + px(1.0);
    let max_x = bounds.origin.x + bounds.size.width - px(dot_d + 0.5);
    let min_y = bounds.origin.y - px(2.5);
    let max_y = bounds.origin.y + bounds.size.height - px(dot_d);

    let offsets = [
        (rx * 0.72, -ry * 0.90),
        (rx * 0.87, -ry * 1.00),
        (rx * 1.01, -ry * 1.08),
    ];
    for (idx, (ox, oy)) in offsets.into_iter().enumerate() {
        let wave = 0.5 + 0.5 * (time_sec * 6.5 - idx as f32 * 0.95).sin();
        let alpha = 0.28 + 0.70 * wave;
        let dx = (cx + px(ox)).clamp(min_x, max_x.max(min_x));
        let dy = (cy + px(oy - wave * 0.65)).clamp(min_y, max_y.max(min_y));
        window.paint_quad(PaintQuad {
            bounds: Bounds {
                origin: point(dx, dy),
                size: gpui::size(px(dot_d), px(dot_d)),
            },
            corner_radii: Corners::all(px(dot_d * 0.5)),
            background: scale_alpha(glow_color, alpha).into(),
            border_widths: Edges::all(px(0.0)),
            border_color: rgba(0x00000000).into(),
            border_style: gpui::BorderStyle::Solid,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheme2_stays_inside_44pt_session_row_height() {
        let canvas = 36.0_f32;
        let row_pad = 4.0_f32;
        let base_r = canvas * BASE_RADIUS_RATIO;
        let bezel_r = base_r * (1.0 + BEZEL_WIDTH_RATIO);
        let center_y = canvas * 0.5 + CENTER_Y_BIAS_PT;

        // 44pt 行高下，圆盘底部与配饰顶部均控制在上下 4pt 留白以内
        assert!(center_y + bezel_r <= canvas + row_pad);
        let max_hat_top = center_y - 1.40 * base_r;
        assert!(
            max_hat_top >= -row_pad,
            "hat top ({max_hat_top:.2}pt) exceeds 44pt row top padding (-{row_pad}pt)"
        );
    }

    #[test]
    fn working_motion_differs_from_rest_and_uses_yaw_for_look_around() {
        let g_look = NomiGenome {
            work_style: NomiWorkStyle::LookAround,
            ..NomiGenome::from_seed("work-a")
        };
        let rest = WorkMotion::compute(
            &g_look,
            NomiLookMode::Head3D,
            NomiWorkAnim::DiscoHop,
            Act::Rest,
            1.0,
        );
        assert_eq!(rest.yaw, 0.0);
        assert_eq!(rest.pitch, 0.0);
        assert_eq!(rest.tilt, 0.0);

        let working_2d = WorkMotion::compute(
            &g_look,
            NomiLookMode::Face2D,
            NomiWorkAnim::DiscoHop,
            Act::Work,
            0.7,
        );
        let max_2d_yaw = (1..=10)
            .map(|i| {
                WorkMotion::compute(
                    &g_look,
                    NomiLookMode::Face2D,
                    NomiWorkAnim::DiscoHop,
                    Act::Work,
                    i as f32 * 0.07,
                )
                .yaw
                .abs()
            })
            .fold(0.0_f32, f32::max);
        assert!(max_2d_yaw > 0.05);
        assert_eq!(working_2d.pitch, 0.0);

        let work_3d_hop = WorkMotion::compute(
            &g_look,
            NomiLookMode::Head3D,
            NomiWorkAnim::DiscoHop,
            Act::Work,
            0.7,
        );
        let max_3d_yaw = (1..=10)
            .map(|i| {
                WorkMotion::compute(
                    &g_look,
                    NomiLookMode::Head3D,
                    NomiWorkAnim::DiscoHop,
                    Act::Work,
                    i as f32 * 0.07,
                )
                .yaw
                .abs()
            })
            .fold(0.0_f32, f32::max);
        assert!(max_3d_yaw > 0.05);
        assert_eq!(work_3d_hop.scale_x, 1.0);
        assert_eq!(work_3d_hop.scale_y, 1.0);

        let work_fc_motion = WorkMotion::compute(
            &g_look,
            NomiLookMode::Head3D,
            NomiWorkAnim::FaceChange,
            Act::Work,
            0.7,
        );
        assert_eq!(work_fc_motion.yaw, 0.0);
        assert_eq!(work_fc_motion.bob_y, 0.0);

        let work_fc_found = (0..30).any(|i| {
            compute_work_face_change(&g_look, i as f32 * 0.15).1.is_some()
        });
        assert!(work_fc_found);
    }
}
