[ [English](USAGE.md) | **日本語** ]

# 使い方リファレンス

`icscli` の全コマンドを網羅したリファレンスです。インストールは [SETUP.md](SETUP.jp.md)、概要は [README](README.jp.md) を参照してください。

## 慣例

- 例はすべて `icscli` が `PATH` に通っている前提です。チェックアウトから直接動かす場合は `cargo run --` に置き換えてください（例: `cargo run -- list`）。
- ユーザー向けの正常出力は **stdout** に書きます。診断メッセージ（`Added: ...`, `Removed: ...`）とプロンプトは **stderr** に出します。
- 終了コードは成功時 `0`、ユーザー起因のエラー時 `1`。

## グローバルオプション

| オプション | デフォルト | 説明 |
|---|---|---|
| `--file <PATH>`, `-f <PATH>` | `calendar.ics` | サブコマンドの対象とする ICS ファイルパス。 |
| `--help`, `-h` | | ヘルプ表示。全サブコマンドで使用可。 |
| `--version`, `-V` | | バージョン表示。 |

## 日付入力形式

`--start` と `--end` が受け付ける形式:

- `YYYY-MM-DD`（例: `2026-01-01`）
- `YYYY/M/D`（例: `2026/1/1`。月日の 1 桁許容）

無効な日付は `invalid date '<input>' (expected YYYY-MM-DD or YYYY/M/D)` で拒否されます。

## サブコマンド

### `init`

ICS カレンダーファイルを新規作成。

```sh
icscli init
icscli --file holidays.ics init
```

- `VERSION:2.0` と `PRODID:-//icscli//EN` をもつ `VCALENDAR` を作成。
- 対象ファイルが既に存在すると失敗します。再初期化したい場合は先にファイルを削除してください。

### `add`

`VEVENT` をカレンダーに追加。

```sh
icscli add [--summary <TEXT>] [--start <DATE>] [--end <DATE>]
                [--busystatus <STATUS>] [--class <CLASS>]
                [--category <NAME> ...] [--icon <NAME>]
```

| フラグ | 型 | 補足 |
|---|---|---|
| `--summary <TEXT>` | 文字列 | イベントタイトル。必須（省略時はプロンプト）。 |
| `--start <DATE>` | 日付 | 開始日。必須（省略時はプロンプト）。 |
| `--end <DATE>` | 日付 | 終了日（inclusive）。単日イベントなら省略。 |
| `--busystatus <STATUS>` | `free` \| `tentative` \| `busy` \| `oof` \| `working` | デフォルト `free`。`TRANSP` と `X-MICROSOFT-CDO-BUSYSTATUS` を両方出力。 |
| `--class <CLASS>` | `public` \| `private` \| `confidential` | 任意。`CLASS:` を出力。 |
| `--category <NAME>` | 文字列、繰り返し可 | 複数値はカンマ区切りで 1 行の `CATEGORIES:` にまとめます。 |
| `--icon <NAME>` | 文字列 | `X-ICSCLI-ICON:<NAME>` を出力。プリセット名は [`icons`](#icons) 参照。任意の文字列も可。 |

#### 例

```sh
# 単日イベント（デフォルト）
icscli add --summary "元日" --start 2026-01-01

# 複数日範囲、OOF、private、カテゴリとアイコン付き
icscli add \
    --summary "出張" \
    --start 2026-05-10 --end 2026-05-12 \
    --busystatus oof --class private \
    --category 仕事 --category 出張 \
    --icon airplane

# 対話モード: Summary / Start date / End date を stderr 上で順にプロンプト
icscli add
```

#### 挙動

- CLI 上の `--end` は inclusive です。内部で `DTEND = --end + 1 日` に変換して、RFC 5545 の `VALUE=DATE` における exclusive 終端規約に合わせます。
- `--end < --start` のときは `--end must not be before --start` で失敗します。
- `--start == --end` は単日イベントとして扱います。
- `UID` には新規に UUIDv4 を割り当てます。`DTSTAMP` は現在 UTC。
- 新規 `VEVENT` は `END:VCALENDAR` の直前に挿入され、既存イベントは保持されます。

### `list`

カレンダー内のイベント一覧。

```sh
icscli list [--sort <FIELD> ...] [--desc] [--json]
```

| フラグ | 補足 |
|---|---|
| `--sort <FIELD>` | 繰り返し可。`start` \| `end` \| `summary`。複数指定で安定な多キーソート。 |
| `--desc` | 降順。 |
| `--json` | 人間可読行ではなく JSON 配列で出力。 |

#### 出力形式

人間可読（デフォルト）:

```
1: 2026-01-01 : 元日
2: 2026-12-29 to 2027-01-03 : 年末年始
```

`<index>: <start>[ to <end>] : <summary>`。複数日のみ `to <end>` が付きます。インデックスは 1 始まりで [`remove`](#remove) から利用できます。

JSON (`--json`):

```json
[
  {
    "uid": "…",
    "dtstamp": "2026-05-27T00:00:00Z",
    "dtstart": "2026-01-01",
    "dtend": "2026-01-02",
    "summary": "元日",
    "busystatus": "free"
  }
]
```

`dtend` は RFC 上の exclusive 値（inclusive 終端の翌日）です。任意フィールド（`class`, `categories`, `icon`）は存在する場合のみ出力されます。

### `icons`

同梱のプリセットアイコン名を表示。

```sh
icscli icons
```

各アイコン名とその日本語説明を出力します（例: `airplane    出張・旅行`）。これらは便宜上のプリセットで、`add --icon` は任意の文字列も受け付けます。

### `remove`

カレンダーからイベントを削除。

```sh
icscli remove [<INDEX_SPEC>] [--summary <TEXT>]
```

| 引数 / フラグ | 補足 |
|---|---|
| `<INDEX_SPEC>`（位置引数） | `list` 出力の 1 始まりインデックス。単一 (`4`), 列挙 (`2,4`), 範囲 (`6-10`), 混在 (`1,3-5,8`) を受け付けます。 |
| `--summary <TEXT>` | サマリが完全一致するイベントを全削除。 |

#### 例

```sh
# インデックス指定
icscli remove 1
icscli remove 2,4
icscli remove 1,3-5,8

# サマリ指定（一致する全イベント）
icscli remove --summary "元日"

# 対話モード: イベント一覧を表示して stderr 上で番号を尋ねる
icscli remove
```

#### エラー

- `<INDEX_SPEC>` と `--summary` の同時指定は即時失敗します。
- 範囲外インデックス (`< 1` または `> N`) は `Index out of range (1-N)` で失敗。
- `--summary` が 1 件もマッチしない場合は `No event found with summary: <text>` で失敗。

### `split`

指定した日付範囲に重なるイベントを **新規** ICS ファイルへ切り出します。非破壊 — 入力ファイル (`--file` / `-f`) は変更されません。詳細は [ADR-028](design/028-split-subcommand.md)。

```sh
icscli split --out <PATH> [--from <DATE>] [--to <DATE>]
```

| フラグ | 必須 | 補足 |
|---|---|---|
| `--out <PATH>` | はい | 出力先 ICS ファイル。既に存在する場合は失敗 (atomic create)。 |
| `--from <DATE>` | いずれか | 下限 (inclusive)。`YYYY-MM-DD` または `YYYY/M/D`。 |
| `--to <DATE>` | いずれか | 上限 (inclusive)。`YYYY-MM-DD` または `YYYY/M/D`。 |

`--from` / `--to` の少なくとも一方は必須。イベントは date span が `[from, to]` と **オーバーラップ** すれば一致 (境界をまたぐイベントも含む)。

#### 例

```sh
# 第 1 四半期を切り出し
icscli -f all.ics split --from 2026-01-01 --to 2026-03-31 --out q1.ics

# 2025 年末までをアーカイブ
icscli -f all.ics split --to 2025-12-31 --out archive-2025.ics

# 2027 年以降の予定
icscli -f all.ics split --from 2027-01-01 --out future.ics
```

#### エラー

- `--from` / `--to` を両方省略 → `split: at least one of --from or --to is required`。
- `--from` が `--to` より後ろ → `split: --from must not be after --to`。
- `--out` のパスが既存 → `file already exists: <path>`。

## ファイル形式

`icscli` は以下の慣例で RFC 5545 iCalendar を読み書きします:

- 改行: 出力は `CRLF` (`\r\n`)、入力はどちらも受け付け。
- 行折り返し: 長いプロパティ行の折り返しは現状未対応（RFC 5545 のラインフォールドの展開は将来対応 — [PRD §5.2](PRD.jp.md#52-計画中-planned)）。
- 全 `VEVENT` は終日: `DTSTART;VALUE=DATE`, `DTEND;VALUE=DATE`。
- 出力順: `UID`, `DTSTAMP`, `DTSTART`, `DTEND`, `SUMMARY`, `TRANSP`, `X-MICROSOFT-CDO-BUSYSTATUS`、続けて任意の `CLASS`, `CATEGORIES`, `X-ICSCLI-ICON`。

## 終了コード

| コード | 意味 |
|---|---|
| `0` | 成功。 |
| `1` | ユーザー起因のエラー全般: 引数不正、ファイル I/O 失敗、パースエラー、該当イベントなし、インデックス範囲外。 |

## 関連

- [README](README.jp.md) — 概要
- [SETUP](SETUP.jp.md) — インストールとプラットフォームセットアップ
- [PRD](PRD.jp.md) — 計画中コマンドと中長期方針
- [CONTRIBUTING](CONTRIBUTING.jp.md) — 開発フロー
