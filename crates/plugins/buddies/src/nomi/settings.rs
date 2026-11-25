//! NOMI 伴侣全局外观与交互设置：挂在「设置 → 会话列表 → 行首头像」下方，
//! 仅当选中 `smelt.buddies/nomi` 时展示，统一影响 Smelt 左侧所有会话 NOMI 头像。

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{App, Context, Entity, SharedString, Window, div};
use rust_i18n::t;
use smelt_plugin_api::ui::{Segment, Segmented, Select, SettingsRow, SettingsSection};
use smelt_plugin_api::{self as api, PluginView, ViewHandle, ViewSpec};

use crate::cast::Cast;
use crate::peers::sessions::{SESSION_AVATAR_SETTINGS, SessionAvatarSettingsArgs};

use super::genome::{NomiBezel, NomiCyberOpticMode, NomiEyeGlow, NomiLookMode, NomiWorkAnim};

const NOMI_AVATAR_VIEW_ID: &str = "smelt.buddies/nomi";

pub(crate) fn spec() -> ViewSpec {
    ViewSpec {
        name: "nomi-settings".into(),
        title: smelt_plugin_api::text!("nomi_settings.section"),
        icon: crate::ICON.into(),
        draws_top_bar: false,
        instances: true,
        settings: None,
        fills: vec![SESSION_AVATAR_SETTINGS.into()],
        build: Rc::new(|_, cx| Rc::new(cx.new(NomiSettingsView::new)) as Rc<dyn ViewHandle>),
    }
}

struct NomiSettingsView {
    cast: Entity<Cast>,
    active: bool,
    _cast: gpui::Subscription,
}

impl NomiSettingsView {
    fn new(cx: &mut Context<Self>) -> Self {
        let cast = Cast::global(cx);
        let _cast = cx.observe(&cast, |_, _, cx| cx.notify());
        Self {
            cast,
            active: false,
            _cast,
        }
    }
}

impl PluginView for NomiSettingsView {
    fn open(&mut self, args: &api::serde_json::Value, _: &mut Window, cx: &mut Context<Self>) {
        let active = api::serde_json::from_value::<SessionAvatarSettingsArgs>(args.clone())
            .is_ok_and(|parsed| parsed.avatar_view == NOMI_AVATAR_VIEW_ID);
        if self.active != active {
            self.active = active;
            cx.notify();
        }
    }
}

fn bezel_label(bezel: NomiBezel) -> SharedString {
    match bezel {
        NomiBezel::Titanium => t!("nomi_settings.bezel_titanium").into(),
        NomiBezel::Silver => t!("nomi_settings.bezel_silver").into(),
        NomiBezel::SpaceBlack => t!("nomi_settings.bezel_space_black").into(),
        NomiBezel::RoseGold => t!("nomi_settings.bezel_rose_gold").into(),
        NomiBezel::CybertronRed => t!("nomi_settings.bezel_cybertron_red").into(),
        NomiBezel::CybertronGold => t!("nomi_settings.bezel_cybertron_gold").into(),
        NomiBezel::DecepticonPurple => t!("nomi_settings.bezel_decepticon_purple").into(),
    }
}

fn cyber_optic_label(mode: NomiCyberOpticMode) -> SharedString {
    match mode {
        NomiCyberOpticMode::Auto => t!("nomi_settings.cyber_optics_auto").into(),
        NomiCyberOpticMode::On => t!("nomi_settings.cyber_optics_on").into(),
        NomiCyberOpticMode::Off => t!("nomi_settings.cyber_optics_off").into(),
    }
}

fn glow_label(glow: NomiEyeGlow) -> SharedString {
    match glow {
        NomiEyeGlow::PureWhite => t!("nomi_settings.glow_white").into(),
        NomiEyeGlow::Amber => t!("nomi_settings.glow_amber").into(),
        NomiEyeGlow::Sakura => t!("nomi_settings.glow_sakura").into(),
    }
}

impl Render for NomiSettingsView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.active {
            return div().into_any_element();
        }

        let prefs = self.cast.read(cx).nomi_prefs();

        let look_selected = NomiLookMode::ALL
            .iter()
            .position(|&mode| mode == prefs.look_mode);
        let look_control = Segmented::new("nomi-look-mode")
            .segments([
                Segment::new(t!("nomi_settings.look_3d")),
                Segment::new(t!("nomi_settings.look_2d")),
            ])
            .selected(look_selected)
            .on_select(|ix, _, cx: &mut App| {
                let Some(&look_mode) = NomiLookMode::ALL.get(ix) else {
                    return;
                };
                Cast::global(cx).update(cx, |cast, cx| {
                    let mut next = cast.nomi_prefs();
                    next.look_mode = look_mode;
                    cast.set_nomi_prefs(next, cx);
                });
            });

        let work_anim_selected = NomiWorkAnim::ALL
            .iter()
            .position(|&anim| anim == prefs.work_anim);
        let work_anim_control = Segmented::new("nomi-work-anim")
            .segments([
                Segment::new(t!("nomi_settings.work_anim_face_change")),
                Segment::new(t!("nomi_settings.work_anim_disco_hop")),
            ])
            .selected(work_anim_selected)
            .on_select(|ix, _, cx: &mut App| {
                let Some(&work_anim) = NomiWorkAnim::ALL.get(ix) else {
                    return;
                };
                Cast::global(cx).update(cx, |cast, cx| {
                    let mut next = cast.nomi_prefs();
                    next.work_anim = work_anim;
                    cast.set_nomi_prefs(next, cx);
                });
            });

        let bezel_options = NomiBezel::ALL
            .iter()
            .copied()
            .map(|bezel| (bezel, bezel_label(bezel)));
        let bezel_control = Select::new("nomi-bezel")
            .options(bezel_options)
            .selected(Some(prefs.bezel))
            .on_change(|&bezel: &NomiBezel, _, cx: &mut App| {
                Cast::global(cx).update(cx, |cast, cx| {
                    let mut next = cast.nomi_prefs();
                    next.bezel = bezel;
                    cast.set_nomi_prefs(next, cx);
                });
            });

        let glow_selected = NomiEyeGlow::ALL.iter().position(|&g| g == prefs.glow);
        let glow_control = Segmented::new("nomi-eye-glow")
            .segments(NomiEyeGlow::ALL.iter().copied().map(glow_label))
            .selected(glow_selected)
            .on_select(|ix, _, cx: &mut App| {
                let Some(&glow) = NomiEyeGlow::ALL.get(ix) else {
                    return;
                };
                Cast::global(cx).update(cx, |cast, cx| {
                    let mut next = cast.nomi_prefs();
                    next.glow = glow;
                    cast.set_nomi_prefs(next, cx);
                });
            });

        let cyber_selected = NomiCyberOpticMode::ALL
            .iter()
            .position(|&m| m == prefs.cyber_optics);
        let cyber_control = Segmented::new("nomi-cyber-optics")
            .segments(NomiCyberOpticMode::ALL.iter().copied().map(cyber_optic_label))
            .selected(cyber_selected)
            .on_select(|ix, _, cx: &mut App| {
                let Some(&mode) = NomiCyberOpticMode::ALL.get(ix) else {
                    return;
                };
                Cast::global(cx).update(cx, |cast, cx| {
                    let mut next = cast.nomi_prefs();
                    next.cyber_optics = mode;
                    cast.set_nomi_prefs(next, cx);
                });
            });

        SettingsSection::new()
            .title(t!("nomi_settings.section"))
            .child(
                SettingsRow::new(t!("nomi_settings.look_mode"))
                    .description(t!("nomi_settings.look_mode_desc"))
                    .child(look_control),
            )
            .child(
                SettingsRow::new(t!("nomi_settings.work_anim"))
                    .description(t!("nomi_settings.work_anim_desc"))
                    .child(work_anim_control),
            )
            .child(
                SettingsRow::new(t!("nomi_settings.bezel"))
                    .description(t!("nomi_settings.bezel_desc"))
                    .child(bezel_control),
            )
            .child(
                SettingsRow::new(t!("nomi_settings.glow"))
                    .description(t!("nomi_settings.glow_desc"))
                    .child(glow_control),
            )
            .child(
                SettingsRow::new(t!("nomi_settings.cyber_optics"))
                    .description(t!("nomi_settings.cyber_optics_desc"))
                    .child(cyber_control),
            )
            .into_any_element()
    }
}
