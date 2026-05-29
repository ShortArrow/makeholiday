[ [English](../CHANGELOG.md) | **日本語** ]

# 変更履歴 (Changelog)

本プロジェクトの注目すべき変更はすべて本ファイルに記録します。

形式は [Keep a Changelog](https://keepachangelog.com/ja/1.1.0/) に従い、`1.0.0` 以降は [Semantic Versioning](https://semver.org/lang/ja/spec/v2.0.0.html) に準拠します。1.0 以前は破壊的変更を含む場合があります（[ADR-004](design/004-trunk-based-and-semver.md) 参照）。

## [Unreleased]

### 変更
- [ADR-017](design/017-workspace-and-ics-core-crate.md) に従い、リポジトリを Cargo workspace へ再編。`Cargo.toml` は workspace マニフェストに、`makeholiday` バイナリクレートは `crates/makeholiday/` 配下へ移動。挙動変更なし。
- 空の `crates/ics-core/` ワークスペースメンバを追加（ADR-017 Migration Step 2）。`makeholiday` から path 依存で接続。公開 API はまだ無し。型とパーサは Step 3 で移動。
- 型モデル（`VEvent`, `BusyStatus`, `EventClass`, `SortKey`）とパーサ・フォーマッタ・クエリヘルパを `crates/makeholiday/src/ics.rs` から `crates/ics-core/src/{event,calendar,parser,query}.rs` に移動（ADR-017 Migration Step 3）。makeholiday は `ics_core` 経由で型を利用するように変更。makeholiday namespace のプリセットアイコン（`PRESET_ICONS`, `format_icons_list`）は新規 `crates/makeholiday/src/icons.rs` に切り出し、`ics-core` には載せない。挙動変更なし。
- 型付き `ics_core::Error` を導入（ADR-017 §error type relationship）。`parse_events`, `parse_indices`, `insert_event`, `remove_event_by_summary`, `remove_events_by_indices` の返却型を `Result<T, String>` から `Result<T, ics_core::Error>` に変更。`Parse` バリアントは `message` に加えオプショナルな `line` と `property` を持ち、現状の flat parser では None のまま。ADR-019 の lexer ベースパーサが値を埋める。
- `makeholiday::error::MhError` を導入（ADR-012 / ADR-017 §error type relationship）。6 バリアント: `Io { path, source }`, `Parse(#[from] ics_core::Error)`, `InvalidInput(String)`, `Conflict(String)`, `NotFound(String)`, `AlreadyExists { path }`。`commands::*` は `Result<_, MhError>` を返却、`ics_core::Error` は `#[from]` 経由で `?` 伝播。テストは `matches!(err, MhError::InvalidInput(_))` の形でバリアントを検証するように更新。
- `crates/makeholiday/src/lib.rs` を新設（ADR-010 / ADR-017 準拠）しライブラリモジュールを宣言。`main.rs` は `use makeholiday::*` で取り込む薄い Composition Root に。ライブラリ表面を持つことで、将来のユースケース単体テストや ADR-022 の TUI からの再利用が可能に。
- `CalendarRepository` ポート（ADR-011）を `application::ports` に、ディスク実装 `FileCalendarRepository` を `infrastructure::file_calendar_repository` に新設。書き込みは `tempfile::NamedTempFile` + `persist` / `persist_noclobber` で原子化。プロセス中断で半端なファイルが残る可能性を排除。`tempfile = "3"` は `[dev-dependencies]` から `[dependencies]` へ移動。
- 旧 `commands.rs` のユースケースを `application::use_cases` に切り出し（ADR-009/017）。各ユースケースは `&Path` ではなく `&impl CalendarRepository` を受け取り、ファイル/パスの関心は Composition Root に集約。`commands.rs` 削除、9 件のテストは `use_cases.rs` に移動してリポジトリ抽象を検証。

### 追加
- ドキュメント基盤一式: `README`, `PRD`, `CONTRIBUTING`, `SETUP`, `USAGE`（英語版と日本語版）。
- ADR 000〜023: ADR ポリシー、ベンダー拡張型付けモデル、言語/エディション、デュアルライセンス、Trunk-based + SemVer、Conventional Commits、テスト戦略、ドキュメント言語ポリシー、MSRV、モジュール階層、lib/main 分離、I/O 境界 + リポジトリパターン、エラーハンドリング、依存ポリシー、CI/CD プラットフォーム、診断出力、設定ポリシー、ワークスペース + `ics-core` クレート、ラウンドトリップ戦略、パーサ実装、CLI サブコマンドポリシー、VTODO スコープ、TUI フロントエンドポリシー、`convert` サブコマンド非提供決定。
- [ADR-024](design/024-solo-phase-branching-carve-out.md) — Solo フェーズの間 ADR-004 の feature ブランチ + PR セレモニーを一時停止する例外。`ics-core` のリポジトリ分離、外部コントリビュータの PR、`v1.0.0` タグのいずれかで自動解除。

## [0.1.0]

### 追加
- `init` サブコマンド — 新規 `VCALENDAR` ファイル作成。
- `add` サブコマンド — `--summary` / `--start` / `--end` で終日 `VEVENT` を追加。任意で `--busystatus`, `--class`, `--category`（繰り返し可）, `--icon` をサポート。必須引数省略時は対話プロンプト。
- `list` サブコマンド — `--sort`（繰り返し: `start` / `end` / `summary`）, `--desc`, `--json` でイベント列挙。
- `icons` サブコマンド — 同梱プリセットアイコン名を表示。
- `remove` サブコマンド — 1 始まりインデックス式（`N`, `N-M`, `N,M`, 混在）, `--summary` 一致, 対話選択で削除。
- デュアルライセンス: MIT OR Apache-2.0。
