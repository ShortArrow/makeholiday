[ [English](../README.md) | **日本語** ]

# icscli

iCalendar (`.ics`) ファイルを作成・編集する小さなコマンドラインツールです。ターミナルから個人の祝日・予定カレンダーを管理する用途を想定しています。

> v0.1.x までは `makeholiday` という名前で出荷されていました。v0.2.0 で `ics*` エコシステム（`ics-core`, `icscli`, `icslint`, `lazyics`）に合わせて改名しています。詳細は [ADR-027](design/027-makeholiday-to-icscli-rename.md) を参照。

## 機能

- `init` — 新しい ICS カレンダーファイルを作成
- `add` — 終日イベントを追加（単日 / 複数日）
- `list` — イベント一覧表示。ソート・JSON 出力に対応
- `icons` — 同梱のプリセットアイコン名を表示
- `remove` — インデックス / 範囲 / サマリでイベント削除
- Microsoft 互換の Busy Status (`FREE` / `TENTATIVE` / `BUSY` / `OOF` / `WORKINGELSEWHERE`)
- イベントクラス分類 (`PUBLIC` / `PRIVATE` / `CONFIDENTIAL`)
- カテゴリと `X-ICSCLI-ICON` 独自 X-property

## インストール

```sh
cargo install --path .
```

インストールせずローカル実行する場合:

```sh
cargo run -- <subcommand> [options]
```

## 使い方

すべてのコマンドで `--file` / `-f` のグローバルオプションを受け付けます（デフォルト: `calendar.ics`）。

### カレンダーの初期化

```sh
icscli init
icscli --file holidays.ics init
```

### イベントの追加

```sh
# 単日イベント
icscli add --summary "元日" --start 2026-01-01

# 複数日範囲（inclusive）
icscli add --summary "年末年始" --start 2026-12-29 --end 2027-01-03

# Busy status / class / category / icon を指定
icscli add \
    --summary "出張" \
    --start 2026-05-10 --end 2026-05-12 \
    --busystatus oof \
    --class private \
    --category 仕事 --category 出張 \
    --icon airplane

# 対話モード（summary / start / end をプロンプト）
icscli add
```

受け付ける日付形式: `YYYY-MM-DD` と `YYYY/M/D`。

### イベント一覧

```sh
icscli list
icscli list --sort start
icscli list --sort start --sort summary --desc
icscli list --json
```

### プリセットアイコン名の表示

```sh
icscli icons
```

### イベントの削除

```sh
# 1 始まりインデックス / 範囲 / 混在指定
icscli remove 1
icscli remove 2,4
icscli remove 1,3-5,8

# サマリ指定
icscli remove --summary "元日"

# 対話モード（一覧表示後にプロンプト）
icscli remove
```

## ファイル形式

- iCalendar (RFC 5545) の `VCALENDAR` + `VEVENT`
- 終日イベント (`DTSTART;VALUE=DATE`, `DTEND;VALUE=DATE`)
- RFC 5545 上 `DTEND` は exclusive。CLI の `--end` は inclusive 入力として扱い、内部で調整します

## ロードマップ

プロダクトの方向性は [PRD.md](PRD.md) を参照してください（CRUD 強化、Outlook / Google / iCloud 拡張対応、RFC 準拠と独自拡張の境界整理、ICS ハンドリングライブラリ提供）。

## ドキュメント

- [SETUP](SETUP.jp.md) — インストールとプラットフォームセットアップ
- [USAGE](USAGE.jp.md) — 全コマンドリファレンス
- [PRD](PRD.jp.md) — プロダクト要件定義
- [ADR ポリシー](design/000-ADR-policy.md) — アーキテクチャ判断の記録方針
- [CONTRIBUTING](CONTRIBUTING.jp.md) — 開発ガイドライン
- [English README](../README.md)
- [CHANGELOG](../docs/CHANGELOG.jp.md)

## コントリビュート

[docs/CONTRIBUTING.md](CONTRIBUTING.md) を参照してください。

## ライセンス

以下のいずれか好きな方を選んで利用できます:

- Apache License, Version 2.0 ([LICENSE-APACHE](../LICENSE-APACHE))
- MIT license ([LICENSE-MIT](../LICENSE-MIT))

特に明示しない限り、本リポジトリへのコントリビューションは Apache-2.0 ライセンスの定義に従い、上記のデュアルライセンスで提供されたものとみなされます。
