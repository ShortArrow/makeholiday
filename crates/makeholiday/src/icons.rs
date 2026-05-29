use ics_core::{RawProperty, VEvent};

/// Wire-format property name for the makeholiday icon extension.
pub const ICON_PROPERTY: &str = "X-MAKEHOLIDAY-ICON";

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

/// Read the icon associated with `event`, if any.
///
/// Per ADR-017's supersede of ADR-001 rule 6, `X-MAKEHOLIDAY-ICON` is
/// a makeholiday-side concern: it lives in `event.unknown` (the generic
/// raw-property bucket) and this helper is the read side.
pub fn read_icon(event: &VEvent) -> Option<&str> {
    event
        .unknown
        .iter()
        .find(|p| p.name == ICON_PROPERTY)
        .map(|p| p.value.as_str())
}

/// Write `icon` into `event`. Replaces an existing `X-MAKEHOLIDAY-ICON`
/// entry if present; otherwise appends a new one with `source_index`
/// set to one past the current maximum so the formatter emits it after
/// existing unknown properties.
pub fn write_icon(event: &mut VEvent, icon: impl Into<String>) {
    let value = icon.into();
    if let Some(existing) = event.unknown.iter_mut().find(|p| p.name == ICON_PROPERTY) {
        existing.value = value;
        return;
    }
    let next_index = event
        .unknown
        .iter()
        .map(|p| p.source_index)
        .max()
        .unwrap_or(0)
        + 1;
    event.unknown.push(RawProperty {
        name: ICON_PROPERTY.to_string(),
        params: vec![],
        value,
        source_index: next_index,
    });
}
