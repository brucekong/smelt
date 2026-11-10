use crate::session_list::{
    SashSpec, SectionKind, SectionLayoutSpec, compute_sidebar_sashes, select_flex_master_indices,
    sidebar_bottom_sections_start_index,
};
use crate::RuntimeKey;

#[test]
fn remote_machine_with_few_items_is_never_flex_master() {
    let specs = vec![
        // Local machine with projects
        SectionLayoutSpec {
            id: "runtime-Local".to_string(),
            kind: SectionKind::Runtime(RuntimeKey::Local),
            is_compact_empty: false,
            natural_h: 588.0,
            max_cap: 500.0,
        },
        // Remote machine with only 1 project (like Desktop)
        SectionLayoutSpec {
            id: "runtime-Remote(1)".to_string(),
            kind: SectionKind::Runtime(RuntimeKey::Remote("r1".to_string())),
            is_compact_empty: false,
            natural_h: 41.0,
            max_cap: 500.0,
        },
    ];

    // Both Local and Remote are active (expanded)
    let active = vec![0, 1];
    let masters = select_flex_master_indices(&active, &specs, |_| false);

    // Only Local machine is the flex master.
    // Remote machine adapts to its natural height (41px) and does not steal 50% flex space!
    assert_eq!(masters, vec![0]);
}

#[test]
fn remote_machine_with_few_items_when_local_is_folded() {
    let specs = vec![
        SectionLayoutSpec {
            id: "runtime-Local".to_string(),
            kind: SectionKind::Runtime(RuntimeKey::Local),
            is_compact_empty: false,
            natural_h: 588.0,
            max_cap: 500.0,
        },
        SectionLayoutSpec {
            id: "runtime-Remote(1)".to_string(),
            kind: SectionKind::Runtime(RuntimeKey::Remote("r1".to_string())),
            is_compact_empty: false,
            natural_h: 41.0,
            max_cap: 500.0,
        },
    ];

    // Local is folded, only Remote is active
    let active = vec![1];
    let masters = select_flex_master_indices(&active, &specs, |_| false);

    // Even when Local is folded, a remote machine with few items (natural_h <= max_cap)
    // is NOT a flex master. It stays compact (41px) instead of stretching across the screen!
    assert_eq!(masters, Vec::<usize>::new());
}

#[test]
fn remote_machine_with_many_items_when_local_is_folded() {
    let specs = vec![
        SectionLayoutSpec {
            id: "runtime-Local".to_string(),
            kind: SectionKind::Runtime(RuntimeKey::Local),
            is_compact_empty: false,
            natural_h: 588.0,
            max_cap: 500.0,
        },
        SectionLayoutSpec {
            id: "runtime-Remote(1)".to_string(),
            kind: SectionKind::Runtime(RuntimeKey::Remote("r1".to_string())),
            is_compact_empty: false,
            natural_h: 1200.0,
            max_cap: 500.0,
        },
    ];

    // Local is folded, Remote with 1200px of content is active
    let active = vec![1];
    let masters = select_flex_master_indices(&active, &specs, |_| false);

    // When Local is folded and Remote has content exceeding max_cap, Remote absorbs available space
    assert_eq!(masters, vec![1]);
}

#[test]
fn local_machine_remains_flex_master_to_absorb_space_for_remote_sticky_bottom() {
    let specs = vec![
        SectionLayoutSpec {
            id: "runtime-Local".to_string(),
            kind: SectionKind::Runtime(RuntimeKey::Local),
            is_compact_empty: false,
            natural_h: 588.0,
            max_cap: 500.0,
        },
        SectionLayoutSpec {
            id: "runtime-Remote(1)".to_string(),
            kind: SectionKind::Runtime(RuntimeKey::Remote("r1".to_string())),
            is_compact_empty: false,
            natural_h: 41.0,
            max_cap: 500.0,
        },
    ];

    let active = vec![0, 1];
    // 即使曾经拖动过 sash 使得 runtime-Local 具有记录高度，
    // 本机依然作为 flex master 吸收剩余空间，将下方远程机推至底部贴底吸附
    let masters = select_flex_master_indices(&active, &specs, |id| id == "runtime-Local");

    assert_eq!(masters, vec![0]);
}

#[test]
fn remote_custom_height_disables_remote_flex_master() {
    let specs = vec![
        SectionLayoutSpec {
            id: "runtime-Remote(1)".to_string(),
            kind: SectionKind::Runtime(RuntimeKey::Remote("r1".to_string())),
            is_compact_empty: false,
            natural_h: 1200.0,
            max_cap: 500.0,
        },
    ];

    let active = vec![0];
    let masters = select_flex_master_indices(&active, &specs, |id| id == "runtime-Remote(1)");

    assert_eq!(masters, Vec::<usize>::new());
}

#[test]
fn empty_machine_is_never_flex_master() {
    let specs = vec![
        SectionLayoutSpec {
            id: "runtime-Local".to_string(),
            kind: SectionKind::Runtime(RuntimeKey::Local),
            is_compact_empty: true,
            natural_h: 28.0,
            max_cap: 28.0,
        },
    ];

    let active = vec![0];
    let masters = select_flex_master_indices(&active, &specs, |_| false);

    assert_eq!(masters, Vec::<usize>::new());
}

#[test]
fn archive_and_conversations_are_never_flex_master() {
    let specs = vec![
        SectionLayoutSpec {
            id: "conversations".to_string(),
            kind: SectionKind::Conversations,
            is_compact_empty: false,
            natural_h: 200.0,
            max_cap: 300.0,
        },
        SectionLayoutSpec {
            id: "archive".to_string(),
            kind: SectionKind::Archive,
            is_compact_empty: false,
            natural_h: 400.0,
            max_cap: 300.0,
        },
    ];

    let active = vec![0, 1];
    let masters = select_flex_master_indices(&active, &specs, |_| false);

    assert_eq!(masters, Vec::<usize>::new());
}

#[test]
fn natural_height_provides_safe_headroom_to_prevent_scrollbar() {
    // 4 conversation rows: each 32px height + 1px gap + 4px pb_1 = 135px content
    let conv_count = 4;
    let conv_natural_h = (conv_count as f32 * 33.0 + 16.0).max(32.0);
    let min_content_h = (conv_count as f32 * 32.0) + ((conv_count - 1) as f32 * 1.0) + 4.0;
    // Must have at least 10px breathing room so subpixel rounding never triggers a scrollbar
    assert!(conv_natural_h > min_content_h + 10.0);
}

#[test]
fn bottom_sections_start_index_correctly_identifies_boundary() {
    // 1. 标准场景：对话、本机、远程、归档
    let specs1 = vec![
        SectionLayoutSpec {
            id: "conversations".to_string(),
            kind: SectionKind::Conversations,
            is_compact_empty: false,
            natural_h: 100.0,
            max_cap: 300.0,
        },
        SectionLayoutSpec {
            id: "runtime-Local".to_string(),
            kind: SectionKind::Runtime(RuntimeKey::Local),
            is_compact_empty: false,
            natural_h: 500.0,
            max_cap: 600.0,
        },
        SectionLayoutSpec {
            id: "runtime-Remote(1)".to_string(),
            kind: SectionKind::Runtime(RuntimeKey::Remote("r1".to_string())),
            is_compact_empty: false,
            natural_h: 49.0,
            max_cap: 600.0,
        },
        SectionLayoutSpec {
            id: "archive".to_string(),
            kind: SectionKind::Archive,
            is_compact_empty: false,
            natural_h: 45.0,
            max_cap: 300.0,
        },
    ];
    // 远程机器与归档从索引 2 开始贴底吸附
    assert_eq!(sidebar_bottom_sections_start_index(&specs1), 2);

    // 2. 仅本机与归档（无远程机器）
    let specs2 = vec![
        SectionLayoutSpec {
            id: "conversations".to_string(),
            kind: SectionKind::Conversations,
            is_compact_empty: false,
            natural_h: 100.0,
            max_cap: 300.0,
        },
        SectionLayoutSpec {
            id: "runtime-Local".to_string(),
            kind: SectionKind::Runtime(RuntimeKey::Local),
            is_compact_empty: false,
            natural_h: 500.0,
            max_cap: 600.0,
        },
        SectionLayoutSpec {
            id: "archive".to_string(),
            kind: SectionKind::Archive,
            is_compact_empty: false,
            natural_h: 45.0,
            max_cap: 300.0,
        },
    ];
    // 归档从索引 2 开始贴底吸附
    assert_eq!(sidebar_bottom_sections_start_index(&specs2), 2);

    // 3. 无本机（仅远程机器与归档）：首个远程机器留在顶部，其余远程机与归档贴底
    let specs3 = vec![
        SectionLayoutSpec {
            id: "runtime-Remote(1)".to_string(),
            kind: SectionKind::Runtime(RuntimeKey::Remote("r1".to_string())),
            is_compact_empty: false,
            natural_h: 400.0,
            max_cap: 600.0,
        },
        SectionLayoutSpec {
            id: "runtime-Remote(2)".to_string(),
            kind: SectionKind::Runtime(RuntimeKey::Remote("r2".to_string())),
            is_compact_empty: false,
            natural_h: 49.0,
            max_cap: 600.0,
        },
        SectionLayoutSpec {
            id: "archive".to_string(),
            kind: SectionKind::Archive,
            is_compact_empty: false,
            natural_h: 45.0,
            max_cap: 300.0,
        },
    ];
    // Remote(2) 与归档从索引 1 开始贴底吸附
    assert_eq!(sidebar_bottom_sections_start_index(&specs3), 1);
}

#[test]
fn remote_machine_drag_sash_generated_when_local_is_folded() {
    // 场景：对话(0)和本机(1)折叠，远程机(2)和归档(3)展开，中间由 Spacer 隔开
    let expanded = vec![2, 3];
    let split_idx = 2;
    let sashes = compute_sidebar_sashes(&expanded, split_idx, true);

    // 1. 远程机顶部必须生成一个允许向上自由拉伸的单向 Sash（above: None, below: Some(2)）
    // 2. 远程机与归档之间生成常规的双向分配 Sash（above: Some(2), below: Some(3)）
    assert_eq!(
        sashes,
        vec![
            SashSpec {
                insert_before_idx: 2,
                above_idx: None,
                below_idx: Some(2),
            },
            SashSpec {
                insert_before_idx: 3,
                above_idx: Some(2),
                below_idx: Some(3),
            },
        ]
    );
}

#[test]
fn adjacent_expanded_sections_without_spacer_have_joint_sash() {
    // 场景：本机(1)展开，远程机(2)展开，无 Spacer
    let expanded = vec![1, 2];
    let split_idx = 2;
    let sashes = compute_sidebar_sashes(&expanded, split_idx, false);

    assert_eq!(
        sashes,
        vec![SashSpec {
            insert_before_idx: 2,
            above_idx: Some(1),
            below_idx: Some(2),
        }]
    );
}

#[test]
fn expanded_sections_across_spacer_get_independent_sashes() {
    // 场景：对话(0)展开，远程机(2)展开，中间被 Spacer 隔开
    let expanded = vec![0, 2];
    let split_idx = 2;
    let sashes = compute_sidebar_sashes(&expanded, split_idx, true);

    // 对话底部与 Spacer 之间有一个向下扩展 Sash，远程机顶部有一个向上扩展 Sash
    assert_eq!(
        sashes,
        vec![
            SashSpec {
                insert_before_idx: 2,
                above_idx: Some(0),
                below_idx: None,
            },
            SashSpec {
                insert_before_idx: 2,
                above_idx: None,
                below_idx: Some(2),
            },
        ]
    );
}

#[test]
fn only_top_section_expanded_has_sash_before_spacer() {
    // 场景：仅对话(0)展开，其余全部折叠
    let expanded = vec![0];
    let split_idx = 2;
    let sashes = compute_sidebar_sashes(&expanded, split_idx, true);

    assert_eq!(
        sashes,
        vec![SashSpec {
            insert_before_idx: 2,
            above_idx: Some(0),
            below_idx: None,
        }]
    );
}

#[test]
fn all_folded_generates_no_sashes() {
    let expanded = vec![];
    let split_idx = 2;
    let sashes = compute_sidebar_sashes(&expanded, split_idx, true);

    assert_eq!(sashes, Vec::<SashSpec>::new());
}

#[test]
fn multiple_remote_machines_expanded_at_bottom() {
    // 场景：Local 折叠 (has_spacer = true)，远程机 A(2) 和 远程机 B(4) 展开，远程机(3) 折叠
    let expanded = vec![2, 4];
    let split_idx = 2;
    let sashes = compute_sidebar_sashes(&expanded, split_idx, true);

    // 远程机 A 顶部有向上扩大的独立 Sash (None, Some(2))
    // 远程机 A 与 远程机 B 之间有相互调节的 Sash (Some(2), Some(4))
    assert_eq!(
        sashes,
        vec![
            SashSpec {
                insert_before_idx: 2,
                above_idx: None,
                below_idx: Some(2),
            },
            SashSpec {
                insert_before_idx: 4,
                above_idx: Some(2),
                below_idx: Some(4),
            },
        ]
    );
}

#[test]
fn multiple_remote_machines_and_archive_expanded_at_bottom() {
    // 场景：Local 折叠 (split_idx = 2)，nb-space-x(2)、staging-k8s(3)、archive(5) 展开
    let expanded = vec![2, 3, 5];
    let split_idx = 2;
    let sashes = compute_sidebar_sashes(&expanded, split_idx, true);

    assert_eq!(
        sashes,
        vec![
            SashSpec {
                insert_before_idx: 2,
                above_idx: None,
                below_idx: Some(2),
            },
            SashSpec {
                insert_before_idx: 3,
                above_idx: Some(2),
                below_idx: Some(3),
            },
            SashSpec {
                insert_before_idx: 5,
                above_idx: Some(3),
                below_idx: Some(5),
            },
        ]
    );
}
