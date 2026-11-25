//! NOMI 会话行首头像视图：作为 [`SESSION_AVATAR`] 的可选头像实现，与现有 3D 小人完全并行，
//! 用户可在「设置 → 通用 → 会话列表 → 行首头像」中自由切换。
//!
//! 采用方案 2：对齐 3D 小人的视觉体量（移除画布裁切并利用 44pt 行高上下留白），
//! 统一为「纯圆形头像徽章模式 + 原厂钛灰金属外圈 + 原厂冷白 AMOLED 眼神光」。

use std::cell::Cell;
use std::rc::Rc;

use gpui::{
    AppContext as _, Bounds, Context, Entity, InteractiveElement as _, IntoElement, MouseButton,
    MouseDownEvent, MouseMoveEvent, ParentElement as _, Pixels, Render, Styled as _, Subscription,
    Window, div, px,
};
use smelt_plugin_api::ui::motion::{ambient_application_active, ambient_motion_enabled};
use smelt_plugin_api::{self as api, PluginView, ViewHandle, ViewSpec};

use crate::buddy::CANVAS;
use crate::cast::{Cast, ConversationKey};
use crate::frames::Pacer;
use crate::peers::sessions::{SESSION_AVATAR, SessionAvatarArgs};

use super::physics::NomiPhysics;
use super::renderer::draw_nomi;

pub(crate) fn spec() -> ViewSpec {
    ViewSpec {
        name: "nomi".into(),
        title: smelt_plugin_api::text!("nomi"),
        icon: crate::ICON.into(),
        draws_top_bar: false,
        instances: true,
        settings: None,
        fills: vec![SESSION_AVATAR.into()],
        build: Rc::new(|_, cx| Rc::new(cx.new(NomiAvatar::new)) as Rc<dyn ViewHandle>),
    }
}

struct NomiAvatar {
    cast: Entity<Cast>,
    key: Option<ConversationKey>,
    physics: NomiPhysics,
    pacer: Rc<Pacer>,
    bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    clip_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    hover_enabled: bool,
    _cast: Subscription,
}

impl NomiAvatar {
    fn new(cx: &mut Context<Self>) -> Self {
        let cast = Cast::global(cx);
        let _cast = cx.observe(&cast, |_, _, cx| cx.notify());
        Self {
            cast,
            key: None,
            physics: NomiPhysics::new(),
            pacer: Rc::default(),
            bounds: Rc::new(Cell::new(None)),
            clip_bounds: Rc::new(Cell::new(None)),
            hover_enabled: true,
            _cast,
        }
    }
}

impl PluginView for NomiAvatar {
    fn open(&mut self, args: &api::serde_json::Value, _: &mut Window, cx: &mut Context<Self>) {
        if self.key.is_some() {
            return;
        }
        match api::serde_json::from_value::<SessionAvatarArgs>(args.clone()) {
            Ok(args) => {
                self.key = Some(ConversationKey {
                    machine: args.machine,
                    id: args.conversation,
                });
                cx.notify();
            }
            Err(error) => eprintln!("[buddies] NOMI 头像的参数不对：{error}"),
        }
    }
}

impl Render for NomiAvatar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 与原版 3D 小人 `avatar.rs` 一致：保留 `CANVAS × CANVAS` 占位而不加 overflow_hidden，
        // 让头顶配饰自然利用 44pt 行高的上下留白，保持与小人相同的饱满视觉体量。
        let canvas = div().relative().size(px(CANVAS)).flex_shrink_0();
        let Some(key) = &self.key else {
            return canvas;
        };
        let moving = ambient_motion_enabled(
            ambient_application_active(cx),
            window.is_window_active(),
            true,
        ) && !cx.reduce_motion();

        let cast = self.cast.read(cx);
        let genome = cast.nomi_of(key);
        let prefs = cast.nomi_prefs();
        let look_mode = prefs.look_mode;
        let work_anim = prefs.work_anim;
        let clock = cast.clock();
        let now = cx.background_executor().now();
        let tick = clock.tick(now);
        let sample_at = clock.at(tick);
        let time_sec = if moving {
            clock.elapsed(sample_at)
        } else {
            0.0
        };
        let (act, act_animating) = cast.stage().act_of(key, sample_at, moving);
        self.hover_enabled = !matches!(act, crate::buddy::Act::Work);
        let mouse = window.mouse_position();
        let in_clip = self
            .clip_bounds
            .get()
            .is_none_or(|clip| clip.contains(&mouse));
        if moving
            && let Some(bounds) = self.bounds.get()
            && (self.physics.is_pressed || (self.hover_enabled && in_clip))
        {
            let center = bounds.center();
            self.physics.update_gaze_for_row(
                (mouse.x.as_f32(), mouse.y.as_f32()),
                (center.x.as_f32(), center.y.as_f32()),
            );
        } else if !self.physics.is_pressed {
            self.physics.gaze_target = (0.0, 0.0);
        }
        let (transform, physics_animating) =
            self.physics.step(now, time_sec, &genome.temper, moving);
        let animating = moving || act_animating || physics_animating;

        let pacer = self.pacer.clone();
        let view = cx.entity().downgrade();
        let bounds_cell = self.bounds.clone();
        let clip_bounds_cell = self.clip_bounds.clone();
        let paint_layer = gpui::canvas(
            move |bounds, window, _| {
                bounds_cell.set(Some(bounds));
                clip_bounds_cell.set(Some(window.content_mask().bounds));
                pacer.drew(tick, animating, bounds, clock, view, window);
            },
            move |bounds, _, window, _| {
                draw_nomi(
                    &genome,
                    look_mode,
                    work_anim,
                    &transform,
                    act,
                    time_sec,
                    bounds,
                    window,
                );
            },
        )
        .absolute()
        .size_full();

        canvas
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, ev: &MouseDownEvent, _, cx| {
                    // 拦截左键按下，避免把捏/戳/双击 NOMI 误判成拖拽整条会话行或双击重命名。
                    cx.stop_propagation();
                    let now = cx.background_executor().now();
                    let pos = (ev.position.x.as_f32(), ev.position.y.as_f32());
                    let clicks = ev.click_count;
                    this.physics.on_mouse_down(pos, clicks, now);
                    if clicks == 2
                        && let Some(key) = this.key.clone()
                    {
                        let prev = this.cast.read(cx).nomi_of(&key);
                        this.cast.update(cx, |cast, cx| cast.reroll_nomi(&key, cx));
                        this.physics
                            .start_face_change(prev.expression, prev.headwear, now);
                    }
                    cx.notify();
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    cx.stop_propagation();
                    this.physics.on_mouse_up(cx.background_executor().now());
                    cx.notify();
                }),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.physics.on_mouse_up(cx.background_executor().now());
                    cx.notify();
                }),
            )
            .on_mouse_move(cx.listener(|this, ev: &MouseMoveEvent, _, cx| {
                let Some(bounds) = this.bounds.get() else {
                    return;
                };
                let now = cx.background_executor().now();
                if this.physics.is_pressed && ev.pressed_button != Some(MouseButton::Left) {
                    this.physics.on_mouse_up(now);
                    cx.notify();
                    return;
                }
                let center = bounds.center();
                let dx = (ev.position.x - center.x).as_f32();
                let dy = (ev.position.y - center.y).as_f32();
                let in_clip = this
                    .clip_bounds
                    .get()
                    .is_none_or(|clip| clip.contains(&ev.position));
                // 运行中会话不响应悬停注视，仅非运行态在侧栏可见会话行内响应注视；离开后平滑回正。
                if this.physics.is_pressed
                    || (this.hover_enabled
                        && in_clip
                        && (-36.0..=520.0).contains(&dx)
                        && dy.abs() <= 21.5)
                {
                    let prev_target = this.physics.gaze_target;
                    this.physics.on_mouse_move(
                        (ev.position.x.as_f32(), ev.position.y.as_f32()),
                        (center.x.as_f32(), center.y.as_f32()),
                        now,
                    );
                    let moved = (this.physics.gaze_target.0 - prev_target.0).abs() > 0.01
                        || (this.physics.gaze_target.1 - prev_target.1).abs() > 0.01;
                    if this.physics.is_pressed || moved {
                        cx.notify();
                    }
                } else if this.physics.gaze_target.0.abs() > 0.01
                    || this.physics.gaze_target.1.abs() > 0.01
                {
                    this.physics.gaze_target = (0.0, 0.0);
                    cx.notify();
                }
            }))
            .child(paint_layer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nomi_spec_fills_session_avatar_without_replacing_buddy() {
        let avatar_spec = crate::avatar::spec();
        let nomi_spec = spec();
        assert_eq!(avatar_spec.name.as_ref(), "avatar");
        assert_eq!(nomi_spec.name.as_ref(), "nomi");
        assert!(nomi_spec.instances);
        assert!(nomi_spec.settings.is_none());
        assert_eq!(
            nomi_spec.fills,
            vec![gpui::SharedString::from(SESSION_AVATAR)]
        );

        let settings = crate::nomi::settings_spec();
        assert_eq!(settings.name.as_ref(), "nomi-settings");
        assert!(settings.instances);
        assert_eq!(
            settings.fills,
            vec![gpui::SharedString::from(
                crate::peers::sessions::SESSION_AVATAR_SETTINGS
            )]
        );
    }
}
