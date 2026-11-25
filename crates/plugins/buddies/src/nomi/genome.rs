//! NOMI 基因组、全局唯一性花名册与会话确定性随机生成器。
//!
//! 每一个会话根据其唯一的会话 ID 计算 64 位哈希种子，并结合全局花名册（`NomiRoster`）保证：
//! - **唯一性**：同一时间存在于列表中的会话优先分配互不重复的表情（53 款）及 `(表情, 配饰)` 组合；
//! - **松散组合**：表情（53 款）、配饰（无帽 + 43 款）与工作律动风格（4 款）彼此独立抽样，
//!   双击切换时重新打散三者组合，并避开当前会话的原组合与其他会话正在使用的表情。

use std::collections::{HashMap, HashSet};
use std::f32::consts::TAU;
use std::hash::Hash;
use std::path::PathBuf;

use gpui::{Rgba, rgba};
use serde::{Deserialize, Serialize};

/// 53 款 NOMI 官方表情库（26 款经典日常 + 27 款音乐季）
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NomiExpression {
    // === 快乐与礼赞 (9) ===
    OkGesture,
    Jubilation,
    Proud,
    Gratitude,
    Triumph,
    Admiration,
    Pleased,
    Amused,
    Excitement,

    // === 惬意与休眠 (6) ===
    Relaxed,
    Shy,
    Acceptance,
    Sleeping,
    Sleepy,
    Tired,

    // === 思考与探索 (11) ===
    Dreamy,
    Doubt,
    Concerned,
    Concentration,
    Interest,
    Desire,
    Attraction,
    Surprise,
    Astonishment,
    Complicity,
    Impressed,

    // === 音乐季 · 弦乐家族 (9) ===
    MusicGuitarAcoustic,
    MusicGuitarJoy,
    MusicGuitarDeep,
    MusicElectricGuitar,
    MusicRockStar,
    MusicViolinJoy,
    MusicViolinDeep,
    MusicViolinSerene,
    MusicCello,

    // === 音乐季 · 打击与键盘 (5) ===
    MusicSnareDrum,
    MusicKeyboard,
    MusicMaracas,
    MusicTambourine,
    MusicDjDual,

    // === 音乐季 · 管乐与主唱 (6) ===
    MusicSaxophoneJoy,
    MusicSaxophoneDeep,
    MusicVocalJoy,
    MusicVocalDeep,
    MusicVocalSerene,
    MusicConductor,

    // === 音乐季 · 电音与舞台 (7) ===
    MusicDjSingle,
    MusicLightsticks,
    MusicNioSpeaker,
    MusicRadio,
    MusicDancingNotes,
    MusicConfetti,
    MusicPeaceV,
}

impl NomiExpression {
    pub const ALL: &'static [Self] = &[
        Self::OkGesture,
        Self::Jubilation,
        Self::Proud,
        Self::Gratitude,
        Self::Triumph,
        Self::Admiration,
        Self::Pleased,
        Self::Amused,
        Self::Excitement,
        Self::Relaxed,
        Self::Shy,
        Self::Acceptance,
        Self::Sleeping,
        Self::Sleepy,
        Self::Tired,
        Self::Dreamy,
        Self::Doubt,
        Self::Concerned,
        Self::Concentration,
        Self::Interest,
        Self::Desire,
        Self::Attraction,
        Self::Surprise,
        Self::Astonishment,
        Self::Complicity,
        Self::Impressed,
        Self::MusicGuitarAcoustic,
        Self::MusicGuitarJoy,
        Self::MusicGuitarDeep,
        Self::MusicElectricGuitar,
        Self::MusicRockStar,
        Self::MusicViolinJoy,
        Self::MusicViolinDeep,
        Self::MusicViolinSerene,
        Self::MusicCello,
        Self::MusicSnareDrum,
        Self::MusicKeyboard,
        Self::MusicMaracas,
        Self::MusicTambourine,
        Self::MusicDjDual,
        Self::MusicSaxophoneJoy,
        Self::MusicSaxophoneDeep,
        Self::MusicVocalJoy,
        Self::MusicVocalDeep,
        Self::MusicVocalSerene,
        Self::MusicConductor,
        Self::MusicDjSingle,
        Self::MusicLightsticks,
        Self::MusicNioSpeaker,
        Self::MusicRadio,
        Self::MusicDancingNotes,
        Self::MusicConfetti,
        Self::MusicPeaceV,
    ];

    /// 只有香甜沉睡 (Zzz) 在运行态自动唤醒为清醒活力表情，其余表情保持原样
    pub fn wake_for_work(self) -> Self {
        match self {
            Self::Sleeping => Self::Excitement,
            other => other,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NomiHeadwear {
    None,
    Sprout,
    Detective,
    Santa,
    Beanie,
    Neko,
    Beret,
    Zhuangyuan,
    Bainiangzi,
    VeiledGoddess,
    Tiger,
    OfficerCap,
    MonkeyKing,
    Nezha,
    BambooHat,
    Caishen,
    BambooCopter,
    RubberDuck,
    CyberDevil,
    Wizard,
    Pirate,
    AngelHalo,
    Chef,
    Pilot,
    Doctor,
    StrawHat,
    Cowboy,
    Gege,
    Barrister,
    Kippah,
    BaseballCap,
    BucketHat,
    Graduation,
    Doctoral,
    SaudiPrince,
    TanBeret,
    Newsboy,
    Captain,
    Hiphop,
    Panama,
    MiniDrone,
    OrbitRing,
    Bowtie,
    WinterScarf,
    BumblebeeHelm,
    OptimusCrest,
    MegatronCannon,
    SeekerWings,
    MatrixPendant,
    CommsVisor,
    CybertronRing,
    ShockwaveOptic,
    JazzVisor,
    DriftSamurai,
    LaserbeakDrone,
    GrimlockHelm,
    ArceeHelm,
    RatchetHelm,
    HotrodHelm,
    DevastatorHelm,
    WheeljackHelm,
}

impl NomiHeadwear {
    /// 可选配饰列表（不含 `None`）
    pub const ACCESSORIES: &'static [Self] = &[
        Self::Sprout,
        Self::Detective,
        Self::Santa,
        Self::Beanie,
        Self::Neko,
        Self::Beret,
        Self::Zhuangyuan,
        Self::Bainiangzi,
        Self::VeiledGoddess,
        Self::Tiger,
        Self::OfficerCap,
        Self::MonkeyKing,
        Self::Nezha,
        Self::BambooHat,
        Self::Caishen,
        Self::BambooCopter,
        Self::RubberDuck,
        Self::CyberDevil,
        Self::Wizard,
        Self::Pirate,
        Self::AngelHalo,
        Self::Chef,
        Self::Pilot,
        Self::Doctor,
        Self::StrawHat,
        Self::Cowboy,
        Self::Gege,
        Self::Barrister,
        Self::Kippah,
        Self::BaseballCap,
        Self::BucketHat,
        Self::Graduation,
        Self::Doctoral,
        Self::SaudiPrince,
        Self::TanBeret,
        Self::Newsboy,
        Self::Captain,
        Self::Hiphop,
        Self::Panama,
        Self::MiniDrone,
        Self::OrbitRing,
        Self::Bowtie,
        Self::WinterScarf,
        Self::BumblebeeHelm,
        Self::OptimusCrest,
        Self::MegatronCannon,
        Self::SeekerWings,
        Self::MatrixPendant,
        Self::CommsVisor,
        Self::CybertronRing,
        Self::ShockwaveOptic,
        Self::JazzVisor,
        Self::DriftSamurai,
        Self::LaserbeakDrone,
        Self::GrimlockHelm,
        Self::ArceeHelm,
        Self::RatchetHelm,
        Self::HotrodHelm,
        Self::DevastatorHelm,
        Self::WheeljackHelm,
    ];

    /// 含极简无配饰的全量列表（用于超 53 会话时的组合去重兜底）
    pub const ALL: &'static [Self] = &[
        Self::None,
        Self::Sprout,
        Self::Detective,
        Self::Santa,
        Self::Beanie,
        Self::Neko,
        Self::Beret,
        Self::Zhuangyuan,
        Self::Bainiangzi,
        Self::VeiledGoddess,
        Self::Tiger,
        Self::OfficerCap,
        Self::MonkeyKing,
        Self::Nezha,
        Self::BambooHat,
        Self::Caishen,
        Self::BambooCopter,
        Self::RubberDuck,
        Self::CyberDevil,
        Self::Wizard,
        Self::Pirate,
        Self::AngelHalo,
        Self::Chef,
        Self::Pilot,
        Self::Doctor,
        Self::StrawHat,
        Self::Cowboy,
        Self::Gege,
        Self::Barrister,
        Self::Kippah,
        Self::BaseballCap,
        Self::BucketHat,
        Self::Graduation,
        Self::Doctoral,
        Self::SaudiPrince,
        Self::TanBeret,
        Self::Newsboy,
        Self::Captain,
        Self::Hiphop,
        Self::Panama,
        Self::MiniDrone,
        Self::OrbitRing,
        Self::Bowtie,
        Self::WinterScarf,
        Self::BumblebeeHelm,
        Self::OptimusCrest,
        Self::MegatronCannon,
        Self::SeekerWings,
        Self::MatrixPendant,
        Self::CommsVisor,
        Self::CybertronRing,
        Self::ShockwaveOptic,
        Self::JazzVisor,
        Self::DriftSamurai,
        Self::LaserbeakDrone,
        Self::GrimlockHelm,
        Self::ArceeHelm,
        Self::RatchetHelm,
        Self::HotrodHelm,
        Self::DevastatorHelm,
        Self::WheeljackHelm,
    ];
}

/// 工作状态 (`Act::Work`) 下的招牌动态风格
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NomiWorkStyle {
    LookAround,
    GrooveTilt,
    BounceNod,
    StageVibe,
}

impl NomiWorkStyle {
    pub const ALL: &'static [Self] = &[
        Self::LookAround,
        Self::GrooveTilt,
        Self::BounceNod,
        Self::StageVibe,
    ];
}

/// 鼠标互动注视模式（全局作用于左侧所有会话 NOMI 头像）
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum NomiLookMode {
    /// 3D 头部注视：整个球体头部、金属外圈与配饰随鼠标做 3D 立体转向注视
    #[default]
    Head3D,
    /// 2D 表情跟随：球体保持正面朝向，仅屏幕内的五官表情跟随鼠标轻巧移动
    Face2D,
}

impl NomiLookMode {
    pub const ALL: &'static [Self] = &[Self::Head3D, Self::Face2D];
}

/// 进行中会话的动态风格（全局作用于左侧所有会话 NOMI 头像，可自由搭配 2D/3D 注视模式）
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum NomiWorkAnim {
    /// 循环变脸：挥扇变脸 / 甩头瞬变 / 眨眼 Q 弹轮番上阵
    #[default]
    FaceChange,
    /// 节拍蹦跳：踩点 Q 弹蹦跳与左右顾盼
    DiscoHop,
}

impl NomiWorkAnim {
    pub const ALL: &'static [Self] = &[Self::FaceChange, Self::DiscoHop];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum NomiBezel {
    /// 原厂钛灰金属（CNC 磨砂钛灰外圈）
    #[default]
    Titanium,
    /// 高光镜面铝银
    Silver,
    /// 深空曜黑金属
    SpaceBlack,
    /// 晨曦玫瑰流金
    RoseGold,
    /// 汽车人炽红烤漆
    CybertronRed,
    /// 赛博坦火种鎏金
    CybertronGold,
    /// 霸天虎枪铁暗紫
    DecepticonPurple,
}

impl NomiBezel {
    pub const ALL: &'static [Self] = &[
        Self::Titanium,
        Self::Silver,
        Self::SpaceBlack,
        Self::RoseGold,
        Self::CybertronRed,
        Self::CybertronGold,
        Self::DecepticonPurple,
    ];

    pub fn ring_light(&self) -> Rgba {
        match self {
            Self::Titanium => rgba(0xA0A4ABFF),
            Self::Silver => rgba(0xFFFFFFFF),
            Self::SpaceBlack => rgba(0x444752FF),
            Self::RoseGold => rgba(0xFBE2DBFF),
            Self::CybertronRed => rgba(0xFCA5A5FF),
            Self::CybertronGold => rgba(0xFEF08AFF),
            Self::DecepticonPurple => rgba(0xD8B4FEFF),
        }
    }

    pub fn ring_mid(&self) -> Rgba {
        match self {
            Self::Titanium => rgba(0x5A5C63FF),
            Self::Silver => rgba(0xB8BFCBFF),
            Self::SpaceBlack => rgba(0x222329FF),
            Self::RoseGold => rgba(0xD69482FF),
            Self::CybertronRed => rgba(0xDC2626FF),
            Self::CybertronGold => rgba(0xCA8A04FF),
            Self::DecepticonPurple => rgba(0x6B21A8FF),
        }
    }

    pub fn ring_dark(&self) -> Rgba {
        match self {
            Self::Titanium => rgba(0x25262BFF),
            Self::Silver => rgba(0x636A78FF),
            Self::SpaceBlack => rgba(0x0D0E12FF),
            Self::RoseGold => rgba(0x69372BFF),
            Self::CybertronRed => rgba(0x450A0AFF),
            Self::CybertronGold => rgba(0x422006FF),
            Self::DecepticonPurple => rgba(0x1E1B4BFF),
        }
    }

    /// 倒角弧线高光色（深空黑收敛亮度以还原哑光质感）
    pub fn ring_highlight(&self) -> Rgba {
        match self {
            Self::SpaceBlack => rgba(0xFFFFFF22),
            Self::Silver => rgba(0xFFFFFF66),
            _ => rgba(0xFFFFFF4A),
        }
    }
}

/// 赛博坦机械光学眼切换策略（全局作用于左侧所有会话 NOMI）
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum NomiCyberOpticMode {
    /// 智能联动：佩戴机甲主题配饰时自动启用对应角色机械光学眼
    #[default]
    Auto,
    /// 全员开启：任意配饰或裸机均切换为赛博光学眼
    On,
    /// 原厂纯色：始终保留 NOMI 原厂极简线条眼
    Off,
}

impl NomiCyberOpticMode {
    pub const ALL: &'static [Self] = &[Self::Auto, Self::On, Self::Off];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum NomiEyeGlow {
    /// 原厂冷白 (#FFFFFF)
    #[default]
    PureWhite,
    /// 暖阳琉金 (#FFE27A)
    Amber,
    /// 浪漫落樱粉 (#FFA8C2)
    Sakura,
}

impl NomiEyeGlow {
    pub const ALL: &'static [Self] = &[Self::PureWhite, Self::Amber, Self::Sakura];

    pub fn color(&self) -> Rgba {
        match self {
            Self::PureWhite => rgba(0xFFFFFFFF),
            Self::Amber => rgba(0xFFE27AFF),
            Self::Sakura => rgba(0xFFA8C2FF),
        }
    }

    /// AMOLED 屏幕柔和眼神泛光晕色
    pub fn glow_halo(&self) -> Rgba {
        match self {
            Self::PureWhite => rgba(0xFFFFFF1C),
            Self::Amber => rgba(0xFFE27A26),
            Self::Sakura => rgba(0xFFA8C226),
        }
    }
}

/// 全局 NOMI 头像外观偏好（统一作用于 Smelt 左侧所有会话 NOMI）
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct NomiPrefs {
    pub look_mode: NomiLookMode,
    pub work_anim: NomiWorkAnim,
    pub bezel: NomiBezel,
    pub glow: NomiEyeGlow,
    pub cyber_optics: NomiCyberOpticMode,
}

/// NOMI 弹簧与动态性格参数
#[derive(Clone, Copy, Debug)]
pub struct NomiTemper {
    pub spring_stiffness: f32,
    pub spring_damping: f32,
    pub bounciness: f32,
    pub pace: f32,
    pub phase: f32,
}

/// NOMI 单个会话专属基因组
#[derive(Clone, Copy, Debug)]
pub struct NomiGenome {
    pub expression: NomiExpression,
    pub headwear: NomiHeadwear,
    pub work_style: NomiWorkStyle,
    pub bezel: NomiBezel,
    pub glow: NomiEyeGlow,
    pub cyber_optics: NomiCyberOpticMode,
    pub temper: NomiTemper,
}

impl NomiGenome {
    /// 由会话 ID 确定性生成初始基因（测试辅助）
    #[cfg(test)]
    pub fn from_seed(session_id: &str) -> Self {
        Self::from_seed_with_nonce(session_id, 0)
    }

    /// 结合双击切换计数 `nonce` 独立抽样表情、配饰与工作律动（松散组合）
    pub fn from_seed_with_nonce(session_id: &str, nonce: u32) -> Self {
        let base = fnv_hash(session_id);
        let n = u64::from(nonce);

        let h_expr = splitmix64(base ^ n.wrapping_mul(0x9e37_79b9_7f4a_7c15));
        let h_hat = splitmix64(base.rotate_left(19) ^ n.wrapping_mul(0xbf58_476d_1ce4_e5b9));
        let h_style = splitmix64(base.rotate_left(37) ^ n.wrapping_mul(0x94d0_49bb_1331_11eb));

        let expr_idx = (h_expr % (NomiExpression::ALL.len() as u64)) as usize;

        // 基础配饰概率：~18% 保持极简纯净裸机 (None)，~82% 佩戴丰富多彩的主题配饰
        let head_roll = h_hat % 100;
        let headwear = if head_roll < 18 {
            NomiHeadwear::None
        } else {
            let acc_idx = ((h_hat >> 16) % (NomiHeadwear::ACCESSORIES.len() as u64)) as usize;
            NomiHeadwear::ACCESSORIES[acc_idx]
        };

        let style_idx = ((h_style >> 4) % (NomiWorkStyle::ALL.len() as u64)) as usize;
        let bezel = NomiBezel::default();
        let glow = NomiEyeGlow::default();

        let stiffness = 140.0 + (((h_style >> 24) & 0xFF) as f32 / 255.0) * 80.0;
        let damping = 12.0 + (((h_style >> 32) & 0xFF) as f32 / 255.0) * 6.0;
        let bounciness = 0.85 + (((h_style >> 40) & 0xFF) as f32 / 255.0) * 0.25;
        let pace = 0.95 + (((h_style >> 48) & 0xFF) as f32 / 255.0) * 0.45;
        let phase = (((h_style >> 56) & 0xFF) as f32 / 255.0) * TAU;

        Self {
            expression: NomiExpression::ALL[expr_idx],
            headwear,
            work_style: NomiWorkStyle::ALL[style_idx],
            bezel,
            glow,
            cyber_optics: NomiCyberOpticMode::Auto,
            temper: NomiTemper {
                spring_stiffness: stiffness,
                spring_damping: damping,
                bounciness,
                pace,
                phase,
            },
        }
    }
}

/// 全局 NOMI 花名册：维护所有在线/已知会话的唯一性与松散组合，并支持双击换装持久化与全局外观偏好
pub struct NomiRoster<K> {
    rerolls: HashMap<String, u32>,
    assigned: HashMap<K, NomiGenome>,
    persist_path: Option<PathBuf>,
    prefs: NomiPrefs,
    prefs_path: Option<PathBuf>,
}

impl<K> Default for NomiRoster<K>
where
    K: Clone + Eq + Hash + Ord,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K> NomiRoster<K>
where
    K: Clone + Eq + Hash + Ord,
{
    /// 纯内存花名册（供单元测试使用）
    pub fn new() -> Self {
        Self {
            rerolls: HashMap::new(),
            assigned: HashMap::new(),
            persist_path: None,
            prefs: NomiPrefs::default(),
            prefs_path: None,
        }
    }

    /// 加载本地保存的双击偏好花名册（`~/.smelt/nomi-choices.json`）与全局外观偏好（`~/.smelt/nomi-prefs.json`）
    pub fn load_default() -> Self {
        let path = default_choices_path();
        let rerolls = path
            .as_ref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|text| serde_json::from_str::<HashMap<String, u32>>(&text).ok())
            .unwrap_or_default();
        let prefs_path = default_prefs_path();
        let prefs = prefs_path
            .as_ref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|text| serde_json::from_str::<NomiPrefs>(&text).ok())
            .unwrap_or_default();
        Self {
            rerolls,
            assigned: HashMap::new(),
            persist_path: path,
            prefs,
            prefs_path,
        }
    }

    pub fn prefs(&self) -> NomiPrefs {
        self.prefs
    }

    pub fn set_prefs(&mut self, prefs: NomiPrefs) -> bool {
        if self.prefs == prefs {
            return false;
        }
        self.prefs = prefs;
        self.save_prefs();
        true
    }

    pub fn get(&self, key: &K, id: &str) -> NomiGenome {
        let mut genome = self
            .assigned
            .get(key)
            .copied()
            .unwrap_or_else(|| NomiGenome::from_seed_with_nonce(id, self.rerolls.get(id).copied().unwrap_or(0)));
        genome.bezel = self.prefs.bezel;
        genome.glow = self.prefs.glow;
        genome.cyber_optics = self.prefs.cyber_optics;
        genome
    }

    /// 同步会话列表：清理已关闭会话，保留现有会话外观，并为新会话分配不撞款组合
    pub fn sync(&mut self, keys: impl IntoIterator<Item = K>, id_of: impl Fn(&K) -> &str) {
        let mut active: Vec<K> = keys.into_iter().collect();
        active.sort();
        let active_set: HashSet<&K> = active.iter().collect();
        self.assigned.retain(|k, _| active_set.contains(k));

        // 已存在会话占用的表情、配饰与 (表情, 配饰) 集合
        let mut used_exprs: HashSet<NomiExpression> =
            self.assigned.values().map(|g| g.expression).collect();
        let mut used_hats: HashSet<NomiHeadwear> =
            self.assigned.values().map(|g| g.headwear).collect();
        let mut used_pairs: HashSet<(NomiExpression, NomiHeadwear)> =
            self.assigned.values().map(|g| (g.expression, g.headwear)).collect();

        for key in active {
            if self.assigned.contains_key(&key) {
                continue;
            }
            let id = id_of(&key);
            let nonce = self.rerolls.get(id).copied().unwrap_or(0);
            let genome = pick_unique_loose(id, nonce, &used_exprs, &used_hats, &used_pairs, None);
            used_exprs.insert(genome.expression);
            used_hats.insert(genome.headwear);
            used_pairs.insert((genome.expression, genome.headwear));
            self.assigned.insert(key, genome);
        }
    }

    /// 双击切换当前会话 NOMI：松散重抽表情/配饰/动作，避开自己上一款与其余所有会话正在使用的款式
    pub fn reroll(&mut self, key: &K, id: &str) -> NomiGenome {
        let prev = self.assigned.get(key).copied();
        let next_nonce = {
            let entry = self.rerolls.entry(id.to_string()).or_insert(0);
            *entry = entry.wrapping_add(1);
            *entry
        };

        let mut used_exprs = HashSet::new();
        let mut used_hats = HashSet::new();
        let mut used_pairs = HashSet::new();
        for (other_k, g) in &self.assigned {
            if other_k != key {
                used_exprs.insert(g.expression);
                used_hats.insert(g.headwear);
                used_pairs.insert((g.expression, g.headwear));
            }
        }

        let genome = pick_unique_loose(id, next_nonce, &used_exprs, &used_hats, &used_pairs, prev);
        self.assigned.insert(key.clone(), genome);
        self.save_rerolls();
        genome
    }

    fn save_rerolls(&self) {
        let Some(path) = &self.persist_path else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(&self.rerolls) {
            let _ = std::fs::write(path, json);
        }
    }

    fn save_prefs(&self) {
        let Some(path) = &self.prefs_path else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(&self.prefs) {
            let _ = std::fs::write(path, json);
        }
    }
}

/// 从 53 款表情 × 61 款配饰中挑选唯一且松散解耦的组合
fn pick_unique_loose(
    id: &str,
    nonce: u32,
    used_exprs: &HashSet<NomiExpression>,
    used_hats: &HashSet<NomiHeadwear>,
    used_pairs: &HashSet<(NomiExpression, NomiHeadwear)>,
    prev: Option<NomiGenome>,
) -> NomiGenome {
    let mut candidate = NomiGenome::from_seed_with_nonce(id, nonce);
    let expr_len = NomiExpression::ALL.len(); // 53
    let start_expr_idx = NomiExpression::ALL
        .iter()
        .position(|&e| e == candidate.expression)
        .unwrap_or(0);

    // 13 与 53 互质，步长 13 可遍历全部 53 款表情而不重复
    let expr_stride = 13;
    let mut picked_expr = None;
    for step in 0..expr_len {
        let expr = NomiExpression::ALL[(start_expr_idx + step * expr_stride) % expr_len];
        if !used_exprs.contains(&expr) && prev.is_none_or(|p| p.expression != expr) {
            picked_expr = Some(expr);
            break;
        }
    }

    if let Some(expr) = picked_expr {
        candidate.expression = expr;
    }

    // 配饰选择与防撞：优先保证在当前会话列表中独一无二，且双击换装时不与自己上一款重复
    let hat_len = NomiHeadwear::ALL.len(); // 61 (含 None)
    let start_hat_idx = NomiHeadwear::ALL
        .iter()
        .position(|&h| h == candidate.headwear)
        .unwrap_or(0);

    // 17 与 61 互质，步长 17 可遍历全部 61 款配饰而不重复
    let hat_stride = 17;
    let mut picked_hat = None;

    if !used_hats.contains(&candidate.headwear)
        && prev.is_none_or(|p| p.headwear != candidate.headwear)
    {
        picked_hat = Some(candidate.headwear);
    } else {
        // 如果冲突或在换装中撞车，用互质步长在 61 款全量配饰中寻访未占用的款式
        for step in 1..hat_len {
            let hat = NomiHeadwear::ALL[(start_hat_idx + step * hat_stride) % hat_len];
            if !used_hats.contains(&hat) && prev.is_none_or(|p| p.headwear != hat) {
                picked_hat = Some(hat);
                break;
            }
        }
    }

    if let Some(hat) = picked_hat {
        candidate.headwear = hat;
    } else {
        // 当活跃会话数超过 60 个（帽子被占满）时，退而求其次保证 (表情, 配饰) 组合不重复
        'fallback: for step in 0..expr_len {
            let expr = NomiExpression::ALL[(start_expr_idx + step * expr_stride) % expr_len];
            for h_step in 0..hat_len {
                let hat = NomiHeadwear::ALL[(start_hat_idx + h_step * hat_stride) % hat_len];
                if !used_pairs.contains(&(expr, hat))
                    && prev.is_none_or(|p| (p.expression, p.headwear) != (expr, hat))
                {
                    candidate.expression = expr;
                    candidate.headwear = hat;
                    break 'fallback;
                }
            }
        }
    }

    candidate
}

fn default_choices_path() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".smelt").join("nomi-choices.json"))
}

fn default_prefs_path() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".smelt").join("nomi-prefs.json"))
}

fn fnv_hash(s: &str) -> u64 {
    s.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

#[inline]
fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expression_catalog_has_all_53_official_expressions() {
        assert_eq!(NomiExpression::ALL.len(), 53);
        assert_ne!(NomiExpression::Sleeping.wake_for_work(), NomiExpression::Sleeping);
        assert_eq!(NomiExpression::Relaxed.wake_for_work(), NomiExpression::Relaxed);
        assert_eq!(NomiExpression::Sleepy.wake_for_work(), NomiExpression::Sleepy);
    }

    #[test]
    fn genome_is_deterministic_and_uses_factory_titanium_white() {
        let a = NomiGenome::from_seed("session-abc-123");
        let b = NomiGenome::from_seed("session-abc-123");
        assert_eq!(a.expression, b.expression);
        assert_eq!(a.headwear, b.headwear);
        assert_eq!(a.work_style, b.work_style);
        assert_eq!(a.bezel, NomiBezel::Titanium);
        assert_eq!(a.glow, NomiEyeGlow::PureWhite);
        assert_eq!(a.glow.color(), rgba(0xFFFFFFFF));
    }

    #[test]
    fn roster_guarantees_unique_expressions_and_headwear_across_active_sessions() {
        let mut roster: NomiRoster<String> = NomiRoster::new();
        let keys: Vec<String> = (0..40).map(|i| format!("conv-{i}")).collect();
        roster.sync(keys.iter().cloned(), |k| k.as_str());

        let exprs: HashSet<NomiExpression> =
            keys.iter().map(|k| roster.get(k, k).expression).collect();
        assert_eq!(exprs.len(), 40);

        let hats: HashSet<NomiHeadwear> =
            keys.iter().map(|k| roster.get(k, k).headwear).collect();
        assert_eq!(hats.len(), 40);

        let before = roster.get(&keys[0], &keys[0]);
        let after = roster.reroll(&keys[0], &keys[0]);
        assert_ne!(before.expression, after.expression);
        assert_ne!(before.headwear, after.headwear);

        let exprs_after: HashSet<NomiExpression> =
            keys.iter().map(|k| roster.get(k, k).expression).collect();
        assert_eq!(exprs_after.len(), 40);
        let hats_after: HashSet<NomiHeadwear> =
            keys.iter().map(|k| roster.get(k, k).headwear).collect();
        assert_eq!(hats_after.len(), 40);
    }

    #[test]
    fn balanced_bare_nomi_ratio() {
        let total = 500;
        let none_count = (0..total)
            .filter(|i| {
                NomiGenome::from_seed(&format!("sess-head-{i}")).headwear == NomiHeadwear::None
            })
            .count();
        assert!(
            (50..=140).contains(&none_count),
            "expected ~18% bare NOMIs, got {none_count}/{total}"
        );
    }

    #[test]
    fn genome_temper_stays_in_bounded_ranges() {
        for i in 0..200 {
            let g = NomiGenome::from_seed(&format!("sess-{i}"));
            assert!((140.0..=220.0).contains(&g.temper.spring_stiffness));
            assert!((12.0..=18.0).contains(&g.temper.spring_damping));
            assert!((0.85..=1.10).contains(&g.temper.bounciness));
            assert!((0.95..=1.40).contains(&g.temper.pace));
            assert!((0.0..=TAU).contains(&g.temper.phase));
        }
    }

    #[test]
    fn prefs_override_bezel_and_glow_across_all_sessions() {
        let mut roster: NomiRoster<String> = NomiRoster::new();
        roster.sync(["c1".to_string(), "c2".to_string()], |k| k.as_str());

        assert_eq!(roster.prefs(), NomiPrefs::default());
        assert_eq!(roster.get(&"c1".into(), "c1").bezel, NomiBezel::Titanium);

        let updated = NomiPrefs {
            look_mode: NomiLookMode::Face2D,
            work_anim: NomiWorkAnim::DiscoHop,
            bezel: NomiBezel::RoseGold,
            glow: NomiEyeGlow::Amber,
            cyber_optics: NomiCyberOpticMode::On,
        };
        assert!(roster.set_prefs(updated));
        assert!(!roster.set_prefs(updated));

        let g1 = roster.get(&"c1".into(), "c1");
        let g2 = roster.get(&"c2".into(), "c2");
        assert_eq!(g1.bezel, NomiBezel::RoseGold);
        assert_eq!(g1.glow, NomiEyeGlow::Amber);
        assert_eq!(g2.bezel, NomiBezel::RoseGold);
        assert_eq!(g2.glow, NomiEyeGlow::Amber);

        let json = serde_json::to_string(&updated).unwrap();
        assert_eq!(serde_json::from_str::<NomiPrefs>(&json).unwrap(), updated);
    }
}
