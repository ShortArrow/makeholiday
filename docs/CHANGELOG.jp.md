[ [English](../CHANGELOG.md) | **日本語** ]

# 変更履歴 (Changelog)

本プロジェクトの注目すべき変更はすべて本ファイルに記録します。

形式は [Keep a Changelog](https://keepachangelog.com/ja/1.1.0/) に従い、`1.0.0` 以降は [Semantic Versioning](https://semver.org/lang/ja/spec/v2.0.0.html) に準拠します。1.0 以前は破壊的変更を含む場合があります（[ADR-004](design/004-trunk-based-and-semver.md) 参照）。

## [Unreleased]

### 変更
- [ADR-017](design/017-workspace-and-ics-core-crate.md) に従い、リポジトリを Cargo workspace へ再編。`Cargo.toml` は workspace マニフェストに、`makeholiday` バイナリクレートは `crates/makeholiday/` 配下へ移動。挙動変更なし。`ics-core` クレートは後続ステップで追加。

### 追加
- ドキュメント基盤一式: `README`, `PRD`, `CONTRIBUTING`, `SETUP`, `USAGE`（英語版と日本語版）。
- ADR 000〜023: ADR ポリシー、ベンダー拡張型付けモデル、言語/エディション、デュアルライセンス、Trunk-based + SemVer、Conventional Commits、テスト戦略、ドキュメント言語ポリシー、MSRV、モジュール階層、lib/main 分離、I/O 境界 + リポジトリパターン、エラーハンドリング、依存ポリシー、CI/CD プラットフォーム、診断出力、設定ポリシー、ワークスペース + `ics-core` クレート、ラウンドトリップ戦略、パーサ実装、CLI サブコマンドポリシー、VTODO スコープ、TUI フロントエンドポリシー、`convert` サブコマンド非提供決定。

## [0.1.0]

### 追加
- `init` サブコマンド — 新規 `VCALENDAR` ファイル作成。
- `add` サブコマンド — `--summary` / `--start` / `--end` で終日 `VEVENT` を追加。任意で `--busystatus`, `--class`, `--category`（繰り返し可）, `--icon` をサポート。必須引数省略時は対話プロンプト。
- `list` サブコマンド — `--sort`（繰り返し: `start` / `end` / `summary`）, `--desc`, `--json` でイベント列挙。
- `icons` サブコマンド — 同梱プリセットアイコン名を表示。
- `remove` サブコマンド — 1 始まりインデックス式（`N`, `N-M`, `N,M`, 混在）, `--summary` 一致, 対話選択で削除。
- デュアルライセンス: MIT OR Apache-2.0。
