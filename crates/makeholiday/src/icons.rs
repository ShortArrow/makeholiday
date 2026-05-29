pub const PRESET_ICONS: &[(&str, &str)] = &[
    ("airplane", "出張・旅行"),
    ("birthday", "誕生日"),
    ("star", "お気に入り"),
    ("heart", "記念日"),
    ("gift", "プレゼント"),
    ("vacation", "休暇"),
    ("meeting", "会議"),
    ("deadline", "締め切り"),
    ("medical", "通院"),
    ("school", "学校行事"),
    ("sports", "スポーツ"),
    ("music", "音楽・ライブ"),
];

pub fn format_icons_list() -> String {
    PRESET_ICONS
        .iter()
        .map(|(name, desc)| format!("{name:<12} {desc}"))
        .collect::<Vec<_>>()
        .join("\n")
}
