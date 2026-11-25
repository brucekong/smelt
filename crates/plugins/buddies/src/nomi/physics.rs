//! NOMI 物理弹性动力学系统与鼠标调戏交互。
//!
//! 所有形变与偏移量均按 36pt 会话头像画布严格限幅，确保在捏扁、回弹、拖拽与微呼吸过程中
//! 头像本体与头顶配饰始终保持在画布边界内，不产生越界裁剪。

use std::time::Instant;

use super::genome::{NomiExpression, NomiHeadwear, NomiTemper};

/// 拖拽最大位移（pt）：严格收束以防头顶配饰越界
const MAX_DRAG_OFFSET_PT: f32 = 1.2;
/// 释放回弹初振幅：与 `bounciness <= 1.10` 组合使最大拉伸 <= 1.09
const RELEASE_JIGGLE_AMP: f32 = 0.08;
/// 小豆芽摆动角上限（度）
const MAX_SPROUT_ANGLE_DEG: f32 = 22.0;
/// 双击变脸过渡时长（秒）
const FACE_CHANGE_DURATION_SEC: f32 = 0.48;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NomiFaceChangeStyle {
    HeadWhip,
    FanSwipe,
    BlinkMorph,
    TransformSnap,
}

#[derive(Clone, Copy, Debug)]
pub struct NomiFaceChange {
    pub from_expression: NomiExpression,
    pub from_headwear: NomiHeadwear,
    pub progress: f32,
    pub style: NomiFaceChangeStyle,
    pub direction: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct NomiTransform {
    pub scale_x: f32,
    pub scale_y: f32,
    pub offset_x: f32,
    pub offset_y: f32,
    pub gaze_x: f32,
    pub gaze_y: f32,
    pub is_dizzy: bool,
    pub is_squished: bool,
    pub sprout_angle: f32, // 头顶小豆芽摆动角（度）
    pub face_change: Option<NomiFaceChange>,
}

impl Default for NomiTransform {
    fn default() -> Self {
        Self {
            scale_x: 1.0,
            scale_y: 1.0,
            offset_x: 0.0,
            offset_y: 0.0,
            gaze_x: 0.0,
            gaze_y: 0.0,
            is_dizzy: false,
            is_squished: false,
            sprout_angle: 0.0,
            face_change: None,
        }
    }
}

pub struct NomiPhysics {
    pub is_pressed: bool,
    pub press_time: Option<Instant>,
    pub release_time: Option<Instant>,
    pub initial_squash_amp: f32,

    pub drag_start: (f32, f32),
    pub current_cursor: (f32, f32),

    pub gaze_target: (f32, f32),
    pub current_gaze: (f32, f32),

    pub combo_pokes: usize,
    pub last_poke: Option<Instant>,
    pub dizzy_until: Option<Instant>,

    pub sprout_vel: f32,
    pub sprout_angle: f32,

    pub last_step: Option<Instant>,

    pub face_change_start: Option<Instant>,
    pub face_change_from: Option<(NomiExpression, NomiHeadwear)>,
    pub face_change_style: NomiFaceChangeStyle,
    pub face_change_dir: f32,
    pub face_change_seq: usize,
}

impl Default for NomiPhysics {
    fn default() -> Self {
        Self {
            is_pressed: false,
            press_time: None,
            release_time: None,
            initial_squash_amp: 0.0,
            drag_start: (0.0, 0.0),
            current_cursor: (0.0, 0.0),
            gaze_target: (0.0, 0.0),
            current_gaze: (0.0, 0.0),
            combo_pokes: 0,
            last_poke: None,
            dizzy_until: None,
            sprout_vel: 0.0,
            sprout_angle: 0.0,
            last_step: None,
            face_change_start: None,
            face_change_from: None,
            face_change_style: NomiFaceChangeStyle::FanSwipe,
            face_change_dir: 1.0,
            face_change_seq: 0,
        }
    }
}

impl NomiPhysics {
    pub fn new() -> Self {
        Self::default()
    }

    /// 鼠标按下：单按捏扁，双击触发切换时清零连戳计数防止误入眩晕，连续快戳 5 下触发眩晕彩蛋
    pub fn on_mouse_down(&mut self, pos: (f32, f32), click_count: usize, now: Instant) {
        self.is_pressed = true;
        self.press_time = Some(now);
        self.drag_start = pos;
        self.current_cursor = pos;

        if click_count == 2 {
            self.combo_pokes = 0;
            self.dizzy_until = None;
            return;
        }

        // 连击连戳计数
        if let Some(last) = self.last_poke {
            if now.saturating_duration_since(last).as_secs_f32() < 0.6 {
                self.combo_pokes += 1;
            } else {
                self.combo_pokes = 1;
            }
        } else {
            self.combo_pokes = 1;
        }
        self.last_poke = Some(now);

        // 连续戳 5 次进入晕乎乎彩蛋状态 (维持 2 秒)
        if self.combo_pokes >= 5 {
            self.dizzy_until = Some(now + std::time::Duration::from_millis(2000));
            self.combo_pokes = 0;
        }
    }

    /// 双击变脸特效：按当前表情/配饰与触发序号伪随机抽取花式（挥扇 / 机甲 / 甩头 / 眨眼），
    /// 不同会话首次双击不再固定同款，且同一会话连续双击不会重复同一种特效。
    pub fn start_face_change(
        &mut self,
        prev_expr: NomiExpression,
        prev_hat: NomiHeadwear,
        now: Instant,
    ) {
        self.is_pressed = false;
        self.press_time = None;
        self.release_time = None;
        self.initial_squash_amp = 0.0;
        self.dizzy_until = None;
        self.combo_pokes = 0;

        let styles = [
            NomiFaceChangeStyle::FanSwipe,
            NomiFaceChangeStyle::TransformSnap,
            NomiFaceChangeStyle::HeadWhip,
            NomiFaceChangeStyle::BlinkMorph,
        ];
        let expr_i = NomiExpression::ALL
            .iter()
            .position(|&e| e == prev_expr)
            .unwrap_or(0) as u64;
        let hat_i = NomiHeadwear::ALL
            .iter()
            .position(|&h| h == prev_hat)
            .unwrap_or(0) as u64;
        let mut h = expr_i
            .wrapping_mul(0x9E37_79B9_7F4A_7C15)
            ^ hat_i.wrapping_mul(0xBF58_476D_1CE4_E5B9)
            ^ (self.face_change_seq as u64).wrapping_mul(0x94D0_49BB_1331_11EB);
        h = (h ^ (h >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        h ^= h >> 27;

        let mut picked = styles[(h as usize) % styles.len()];
        if self.face_change_seq > 0 && picked == self.face_change_style {
            let cur_idx = styles.iter().position(|&s| s == picked).unwrap_or(0);
            let step = 1 + (((h >> 8) as usize) % (styles.len() - 1));
            picked = styles[(cur_idx + step) % styles.len()];
        }
        self.face_change_style = picked;
        self.face_change_dir = if ((h >> 16) & 1) == 0 { 1.0 } else { -1.0 };
        self.face_change_seq = self.face_change_seq.wrapping_add(1);
        self.face_change_from = Some((prev_expr, prev_hat));
        self.face_change_start = Some(now);
        self.sprout_vel += 18.0 * self.face_change_dir;
    }

    /// 鼠标移动：在直接拖动/微距捏脸时按 `range = 20pt` 高灵敏度更新注视目标
    pub fn on_mouse_move(&mut self, cursor_pos: (f32, f32), center_pos: (f32, f32), _now: Instant) {
        self.current_cursor = cursor_pos;
        self.update_gaze_for_row(cursor_pos, center_pos);

        if self.is_pressed {
            // 被按住拖拽时，小豆芽受惯性向相反方向倾斜
            let drag_x = (cursor_pos.0 - self.drag_start.0).clamp(-10.0, 10.0);
            self.sprout_vel += drag_x * -0.35;
        }
    }

    /// 仅当光标落在该 NOMI 所在会话行的纵向带（±21pt，严格不串扰上下相邻会话行）或处于按压拖拽时跟随鼠标；
    /// 移出本行后平稳回正。
    pub fn update_gaze_for_row(&mut self, cursor_pos: (f32, f32), center_pos: (f32, f32)) {
        let raw_dx = cursor_pos.0 - center_pos.0;
        let raw_dy = cursor_pos.1 - center_pos.1;
        let in_row = raw_dy.abs() <= 21.5 && (-36.0..=520.0).contains(&raw_dx);
        if !self.is_pressed && !in_row {
            self.gaze_target = (0.0, 0.0);
            return;
        }
        let range_x = 26.0;
        let range_y = 17.5;
        self.gaze_target = (
            (raw_dx / range_x).clamp(-1.0, 1.0),
            (raw_dy / range_y).clamp(-1.0, 1.0),
        );
    }

    /// 鼠标松开：释放捏脸，触发弹簧阻尼果冻回弹
    pub fn on_mouse_up(&mut self, now: Instant) {
        if self.is_pressed {
            self.is_pressed = false;
            self.release_time = Some(now);
            self.initial_squash_amp = RELEASE_JIGGLE_AMP;
            self.press_time = None;

            // 给头顶小豆芽一记适度初速度冲击
            self.sprout_vel += 18.0;
        }
    }

    /// 帧循环计算动力学状态与形变矩阵
    pub fn step(
        &mut self,
        now: Instant,
        time_sec: f32,
        temper: &NomiTemper,
        moving: bool,
    ) -> (NomiTransform, bool) {
        let mut needs_next_frame = false;

        let dt = self.last_step.map_or(1.0 / 60.0, |prev| {
            now.saturating_duration_since(prev).as_secs_f32().min(0.05)
        });
        self.last_step = Some(now);

        // 1. 眼神平滑逼近（柔和跟手、移出平稳回正，与 Web 端舒缓质感一致）
        let damp_speed = if self.gaze_target.0.abs() > 0.01 || self.gaze_target.1.abs() > 0.01 {
            9.0
        } else {
            6.5
        };
        let gaze_alpha = 1.0 - (-damp_speed * dt).exp();
        self.current_gaze.0 += (self.gaze_target.0 - self.current_gaze.0) * gaze_alpha;
        self.current_gaze.1 += (self.gaze_target.1 - self.current_gaze.1) * gaze_alpha;
        if (self.gaze_target.0 - self.current_gaze.0).abs() > 0.01
            || (self.gaze_target.1 - self.current_gaze.1).abs() > 0.01
        {
            needs_next_frame = true;
        }

        let is_dizzy = self.dizzy_until.is_some_and(|until| now < until);
        if is_dizzy {
            needs_next_frame = true;
        } else {
            self.dizzy_until = None;
        }

        // 2. 捏扁与果冻回弹动力学（带严格限幅防止越界）
        let (scale_x, scale_y, offset_x, offset_y, is_squished);

        if self.is_pressed {
            let press_dur = self
                .press_time
                .map_or(0.0, |t| now.saturating_duration_since(t).as_secs_f32());
            let squeeze = (press_dur * 8.0).min(1.0);
            scale_y = 1.0 - 0.12 * squeeze;
            scale_x = 1.0 + 0.09 * squeeze;

            let raw_dx = (self.current_cursor.0 - self.drag_start.0).clamp(-4.0, 4.0);
            let raw_dy = (self.current_cursor.1 - self.drag_start.1).clamp(-4.0, 4.0);
            offset_x = (raw_dx * 0.30).clamp(-MAX_DRAG_OFFSET_PT, MAX_DRAG_OFFSET_PT);
            offset_y = (raw_dy * 0.30).clamp(-MAX_DRAG_OFFSET_PT, MAX_DRAG_OFFSET_PT);
            is_squished = true;
            needs_next_frame = true;
        } else if let Some(rel) = self.release_time {
            let t = now.saturating_duration_since(rel).as_secs_f32();
            let duration = 0.85;

            if t < duration {
                let omega = temper.spring_stiffness.sqrt();
                let zeta = temper.spring_damping / (2.0 * omega);
                let envelope =
                    (-zeta * omega * t).exp() * self.initial_squash_amp * temper.bounciness;
                let oscillation = (omega * t).cos();

                let delta = (envelope * oscillation).clamp(-0.09, 0.09);
                scale_x = 1.0 - delta;
                scale_y = 1.0 + delta;
                offset_x = 0.0;
                offset_y = 0.0;
                is_squished = delta.abs() > 0.03;
                needs_next_frame = true;
            } else {
                scale_x = 1.0;
                scale_y = 1.0;
                offset_x = 0.0;
                offset_y = 0.0;
                is_squished = false;
                self.release_time = None;
            }
        } else {
            scale_x = 1.0;
            scale_y = 1.0;
            offset_x = 0.0;
            offset_y = 0.0;
            is_squished = false;
        }

        // 3. 小豆芽物理摆动 (单摆弹簧，基于真实帧时间 dt 积分) + 闲置柔和微风摇曳
        if self.sprout_vel.abs() > 0.05 || self.sprout_angle.abs() > 0.05 {
            let sprout_dt = dt.min(0.025);
            let k_spring = 90.0;
            let c_damper = 6.0;
            let spring_force = -k_spring * self.sprout_angle;
            let damper_force = -c_damper * self.sprout_vel;
            let acc = spring_force + damper_force;
            self.sprout_vel += acc * sprout_dt;
            self.sprout_angle = (self.sprout_angle + self.sprout_vel * sprout_dt)
                .clamp(-MAX_SPROUT_ANGLE_DEG, MAX_SPROUT_ANGLE_DEG);
            if self.sprout_vel.abs() <= 0.05 && self.sprout_angle.abs() <= 0.05 {
                self.sprout_vel = 0.0;
                self.sprout_angle = 0.0;
            } else {
                needs_next_frame = true;
            }
        }

        let idle_sway = if moving {
            (time_sec * 2.4 * temper.pace).sin() * 4.5
        } else {
            0.0
        };

        // 4. 双击丝滑变脸动画计时与进度推进
        let face_change = if let (Some(start), Some((from_expression, from_headwear))) =
            (self.face_change_start, self.face_change_from)
        {
            let elapsed = now.saturating_duration_since(start).as_secs_f32();
            if elapsed < FACE_CHANGE_DURATION_SEC {
                needs_next_frame = true;
                Some(NomiFaceChange {
                    from_expression,
                    from_headwear,
                    progress: (elapsed / FACE_CHANGE_DURATION_SEC).clamp(0.0, 1.0),
                    style: self.face_change_style,
                    direction: self.face_change_dir,
                })
            } else {
                self.face_change_start = None;
                self.face_change_from = None;
                None
            }
        } else {
            None
        };

        (
            NomiTransform {
                scale_x,
                scale_y,
                offset_x,
                offset_y,
                gaze_x: self.current_gaze.0,
                gaze_y: self.current_gaze.1,
                is_dizzy,
                is_squished,
                sprout_angle: (self.sprout_angle + idle_sway)
                    .clamp(-MAX_SPROUT_ANGLE_DEG, MAX_SPROUT_ANGLE_DEG),
                face_change,
            },
            needs_next_frame,
        )
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::nomi::genome::NomiGenome;

    #[test]
    fn press_squishes_and_release_jiggles_then_settles() {
        let mut physics = NomiPhysics::new();
        let temper = NomiGenome::from_seed("test").temper;
        let t0 = Instant::now();

        physics.on_mouse_down((18.0, 18.0), 1, t0);
        let (tr, animating) = physics.step(t0 + Duration::from_millis(200), 0.2, &temper, true);
        assert!(animating);
        assert!(tr.is_squished);
        assert!(tr.scale_y < 1.0);
        assert!(tr.scale_x > 1.0);
        assert!(tr.offset_x.abs() <= MAX_DRAG_OFFSET_PT);
        assert!(tr.offset_y.abs() <= MAX_DRAG_OFFSET_PT);

        physics.on_mouse_up(t0 + Duration::from_millis(200));
        let (tr_mid, animating_mid) =
            physics.step(t0 + Duration::from_millis(230), 0.23, &temper, true);
        assert!(animating_mid);
        assert!(tr_mid.scale_y <= 1.091 && tr_mid.scale_x <= 1.091);

        let mut settled = false;
        for step in 1..=200 {
            let now = t0 + Duration::from_millis(1200 + step * 16);
            let (tr_end, still) = physics.step(now, 1.2, &temper, false);
            if !still && !tr_end.is_squished {
                assert!((tr_end.scale_x - 1.0).abs() < 1e-4);
                assert!((tr_end.scale_y - 1.0).abs() < 1e-4);
                settled = true;
                break;
            }
        }
        assert!(settled);
    }

    #[test]
    fn five_quick_pokes_trigger_dizzy_easter_egg() {
        let mut physics = NomiPhysics::new();
        let temper = NomiGenome::from_seed("dizzy").temper;
        let t0 = Instant::now();

        for i in 0..5 {
            let t = t0 + Duration::from_millis(i * 150);
            physics.on_mouse_down((10.0, 10.0), 1, t);
            physics.on_mouse_up(t + Duration::from_millis(40));
        }

        let (tr, _) = physics.step(t0 + Duration::from_millis(700), 0.7, &temper, true);
        assert!(tr.is_dizzy);

        let (tr_after, _) = physics.step(t0 + Duration::from_millis(3000), 3.0, &temper, true);
        assert!(!tr_after.is_dizzy);
    }

    #[test]
    fn row_gaze_ignores_other_rows_and_tracks_current_row() {
        let mut physics = NomiPhysics::new();
        let center = (32.0, 100.0);

        // 悬停在同一行文字上 -> 仅该行跟随注视
        physics.update_gaze_for_row((120.0, 106.0), center);
        assert!((physics.gaze_target.0 - 1.0).abs() < 1e-4);
        assert!(physics.gaze_target.1 > 0.3);

        // 光标移到相邻会话行（y 偏移 44pt）-> 本行立刻回正，不串扰其他行
        physics.update_gaze_for_row((120.0, 144.0), center);
        assert_eq!(physics.gaze_target, (0.0, 0.0));
    }
}
