[ [English](../CHANGELOG.md) | **日本語** ]

# 変更履歴 (Changelog)

本プロジェクトの注目すべき変更はすべて本ファイルに記録します。

形式は [Keep a Changelog](https://keepachangelog.com/ja/1.1.0/) に従い、`1.0.0` 以降は [Semantic Versioning](https://semver.org/lang/ja/spec/v2.0.0.html) に準拠します。1.0 以前は破壊的変更を含む場合があります（[ADR-004](design/004-trunk-based-and-semver.md) 参照）。

## [Unreleased]

### Added

- **`icscli split` サブコマンド** ([ADR-028](design/028-split-subcommand.md)) — 指定日付範囲に重なるイベントを新しい ICS ファイルへ切り出す。非破壊: 入力ファイルは変更されない (削除は `remove` で別途実施)。`--from` / `--to` はいずれも inclusive、片側省略可、`--out` は既存だとエラー。[ADR-017](design/017-workspace-and-ics-core-crate.md) maturity gate #4 (ICS file split) の最初のスライス。
- **`icscli split --uid <UID>`** (繰り返し可) — ADR-028 / gate #4 の 2 番目のスライス。集合メンバシップ述語で、マッチしない UID は静かにスキップ。`--from` / `--to` と組み合わせると use case 内のフィルタパイプラインで AND 合成 (交差)。lazyics の将来の「選択イベントをファイルに切り出し」モードは `use_cases::split` をそのまま呼ぶだけで実現可能。
- `ics_core::split_by_date_range(cal, from, to) -> VCalendar` — 日付範囲フィルタの純粋関数。境界の組み合わせに対して全域 (None/None は全件、from>to は空)。CLI 引数バリデーションは `icscli` use case 層に分離。
- `ics_core::split_by_uids(cal, uids) -> VCalendar` — UID 集合メンバシップの純粋フィルタ。空 `uids` は空結果 (数学的な集合の読み)、存在しない UID は静かにスキップ。
- `CalendarRepository::create_with(&VCalendar)` — 指定 VCalendar 内容で新規ストアを atomic 作成。`split` のような「新規ファイルに内容を書く」use case のため `create()` と対になる。

## [0.2.0] - 2026-06-04

v0.2.0「In-tree ICS エコシステム」マイルストーン。2026-06-04 の [ADR-017](design/017-workspace-and-ics-core-crate.md) 改訂により、`ics-core` は 4 つの maturity gate (時刻イベント、VTODO 編集、ICS 合成、ICS 切り出し) がすべて landing するまで workspace 内の path dep として留置。v0.2.0 は source release として ship、crates.io アップロードなし、`cargo install --git` で導入。

ship 時点の workspace テスト合計: 105 ics-core + 19+16 icscli + 81 icslint + 138+7 lazyics = **383 テスト緑**、clippy clean、fmt clean。

### 追加

- **`lazyics` — 対話的 TUI エディタ** ([ADR-025](design/025-lazyics-project-definition.md))。v0.2.0 で 6 フェーズ着地:
  - Phase 1: ratatui scaffolding — RAII ターミナルガード、keymap → Intent 間接化、TTY ガード付き Composition Root。
  - Phase 2: 実カレンダー読み込み (`icscli::infrastructure::FileCalendarRepository` 経由)、空カレンダーヒント。
  - Phase 3a: List view の multi-select Remove モード (`d`/`x` で突入、Space で mark、Enter/Shift+D で確定、Esc で cancel)。
  - Phase 3b: 7 フィールドの Add フォーム modal (Summary, Start, End, busystatus, class, categories, icon)。TextInput widget は char-indexed cursor で Unicode セーフ。
  - Phase 3c: `EventForm` + `FormMode { Add, Edit { event_index } }` 経由の Edit フォーム — 選択イベントから事前入力、`icscli::application::use_cases::edit` で送信。upstream の `EditPatch` に `Clone + PartialEq` derive 追加。
  - Phase 4a: multi-view (List / Timeline / Grid)、`Tab` 巡回 + `1` / `2` / `3` 直接ジャンプ、各 view ごとに time-granularity 切替。
  - Timeline / Grid の Granularity::Year (Grid は `cal -y` 風 12 月ミニグリッド)、`u` で week → month → year → week 巡回。
  - Add / Edit が全 view から到達可能。Grid は cursor 日を Start に事前入力、Timeline は選択イベントの UID を `OpenEditByUid` で発行。
  - In-app help overlay (`?` で開閉)、help text が canonical behavior spec ([memory: feedback-help-text-is-a-contract](#))。
  - List view の search-as-you-type filter (`/`)。Browse の `Esc` は no-op (q か Ctrl+C で quit)、overlay の `q` で overlay 閉。
  - Grid の月ジャンプ / 年ジャンプピッカー (`m` と `Y`)。年ピッカーは端でスクロールするので任意の年に到達可能。
  - Form ergonomics: `Ctrl+N` / `Ctrl+P` でフィールド移動、`h` / `l` でフォーカス中のピッカーを cycle。
  - Grid visual-range モード (`v`) で cursor↔anchor underline、`a` で Start と End が range 端に事前入力された Add フォームを開く (複数日イベントを 1 操作で作成)。
- **`icslint` — ICS リンタ** ([ADR-026](design/026-icslint-project-definition.md))。4 ファミリ計 20 ルール (RFC5545 / vendor / text / structure) と 3 つの出力形式 (`human` / `json` / `github`)。
- icscli (旧 makeholiday): `edit` サブコマンド、`--quiet` / `--interactive` flag、ADR-019 のパーサ correctness (行折り返し、BOM 処理、TEXT escape decode/encode)、各サブコマンドの `long_about` 例示 ([ADR-020](design/020-cli-subcommand-policy.md))。

### 変更

- **破壊的: CLI バイナリ `makeholiday` を `icscli` に改名**（[ADR-027](design/027-makeholiday-to-icscli-rename.md)）。workspace member は `crates/makeholiday/` から `crates/icscli/` へ移動。パッケージ名、`[[bin]]` 名、ライブラリ import path（`use icscli::*`）、エラー型（`MhError` → `IcsError`）も同時改名。crates.io Trusted Publisher binding を保つため、リポジトリは `github.com/ShortArrow/makeholiday` のまま据え置き。`makeholiday` は crates.io 未公開のため、Cargo migration shim は不要。
- **破壊的: ベンダー X-property `X-MAKEHOLIDAY-ICON` を `X-ICSCLI-ICON` に改名**。ADR-027 の方針通り、pre-1.0 のコヒーレンスを後方互換より優先。入力 .ics ファイル中の旧 `X-MAKEHOLIDAY-ICON` は `VEvent.unknown` 経由で raw round-trip されるが typed icon semantics は失われる。icon writer は常に `X-ICSCLI-ICON` を出力。
- **破壊的: 新規初期化カレンダーの PRODID を `-//makeholiday//EN` から `-//icscli//EN` に変更**。
- [ADR-017](design/017-workspace-and-ics-core-crate.md) §"Publishing strategy" 改訂 (2026-06-04): trigger #2 の「judged by the maintainer」が 4 つの maturity gate (時刻 VEvent typed 化、VTODO 編集、ICS 合成、ICS 切り出し) に具体化。`ics-core` の crates.io 公開と repo split は v0.2.0 だけでなく v0.3.0 マイルストーンからも除外。PRD §9 ロードマップを gate 単位で再構成。
- icscli `EditPatch` に `Clone + PartialEq` derive 追加 (lazyics の `ScreenAction::SubmitEdit` で持ち回せるように)。

### v0.2.0 では実施しない

- `ics-core` の crates.io 公開と repo split — 上記 4 maturity criterion 待ち。
- 時刻 VEvent typed 化 ([ADR-001](design/001-vendor-extension-typing.md) Rule 9 改訂) — v0.3.0。
- VTODO 編集機能の本格対応 ([ADR-021](design/021-vtodo-scope.md) 昇格) — v0.4.0。
- ICS ファイル合成 + 切り出し — v0.5.0。

## [0.1.0] - 2026-05-29

PRD §9 にいう Solid Local CLI としての `makeholiday` の最初のカット。ICS テキスト操作のみが対象、CalDAV / クラウドバックエンドは v0.2.0 / v0.3.0 にバージョンステージング。105 件の ics-core + 19 件の makeholiday 単体 + 16 件の CLI 統合テストが本リリースの全コミットを ゲート している。

### 変更
- ADR-019 Migration Step 2: typed TEXT フィールドの escape decode / encode 対応。新規 `crates/ics-core/src/parser/escape.rs` で RFC 5545 §3.3.11 に準拠した `decode_text` / `encode_text` / `split_text_list` / `join_text_list` を実装（`\\` / `\;` / `\,` / `\n` / `\N` のマッピング）。パーサは `SUMMARY` に `decode_text`、`CATEGORIES` 各項目に `split_text_list` を適用、フォーマッタは出力時に `encode_text` / `join_text_list` を適用。ADR-018 通り `RawProperty.value` は **escape 解釈しない** — escape ハンドリングは非対称で、型モデルが所有するフィールドのみが対象。`SUMMARY` 内のコンマ・セミコロン・改行・バックスラッシュ、`CATEGORIES` 項目内のコンマがラウンドトリップ可能になった。
- ADR-019 Migration Step 1: パーサのディスパッチを `LogicalLine` トークン経由に再編（`crates/ics-core/src/parser/line.rs`）。`parse_logical_line` で論理行 1 本ごとにプロパティ名（UPPERCASE）、パラメータ（キー UPPERCASE、値の囲み `"` を除去、順序保持）、生の value を抽出。下流ディスパッチは `strip_prefix("NAME:")` から `LogicalLine.name` の match へ。副作用として、追加パラメータ付きプロパティが正しくルーティングされる（例: `UID;X-FOO=bar:abc-123` で UID 取得）、`DTSTART;VALUE=DATE` の検出がパラメータ位置に依存しなくなる。`Error` に `parse_at_line` と `parse_at` コンストラクタを追加、`DTSTAMP` / `DTSTART` / `DTEND` のパースエラーと必須フィールド欠落エラーが 1-based 論理行番号と該当プロパティ名を含む形式で出力（例: `parse error at line 6 [DTSTAMP]: Invalid DTSTAMP: ...`）。`Error` の Display は手書きに切替、`thiserror` を ics-core の依存から削除。
- ics-core パーサを `crates/ics-core/src/parser/` モジュールディレクトリへ再編（ADR-019 Migration Step 0）。新規 `parser/unfold.rs` で UTF-8 BOM 剥がしと RFC 5545 §3.1 行折り返し展開（SPACE / HTAB で始まる継続行を直前の論理行に結合、折り返しマーカー空白は除去）を実装。`parse_calendar` がディスパッチ前に unfolder を通すようになり、Outlook / Google エクスポートが出す長い `SUMMARY` イベントや BOM 付き入力でパーサが壊れなくなる。既存のディスパッチロジックの挙動は不変、入力パイプラインに unfold パスが追加されただけ。出力側の行折り返し（ADR-018 §4）は後続ステップで導入予定。
- `CalendarRepository::load()` の返却型を `Result<String>` から `Result<VCalendar>` に、`save()` の引数を `&str` から `&VCalendar` に変更（ADR-011 §load が示唆していた ADR-017 過渡的状態を解消）。`FileCalendarRepository` が内部で `ics_core::parse_calendar` / `format_calendar` を呼ぶようになり、ユースケース側は `parse_calendar` / `format_calendar` のブリッジ呼び出しを削除して typed `VCalendar` を直接操作。テストはワイヤフォーマット文字列を `contains` で検査するのではなく typed フィールド（`cal.events[0].summary`、`event.microsoft.busystatus`、`cal.prodid` 等）で検証する形に更新。
- PRD §3 非ゴールから "サーバ / サービス同期" 行を削除。CalDAV / クラウドサービス同期は絶対 Non-Goal ではなく v0.2.0 でバージョンステージングされたスコープインに変更。§7 スコープ外の「マシン間でのカレンダー状態クラウド同期」も新規 §9 を参照するよう更新。§3 リストには ADR で裏付けされた絶対 Non-Goal（GUI/WebUI、ICS 以外形式、ベンダープロファイル変換）のみ残置。
- [ADR-017](design/017-workspace-and-ics-core-crate.md) に従い、リポジトリを Cargo workspace へ再編。`Cargo.toml` は workspace マニフェストに、`makeholiday` バイナリクレートは `crates/makeholiday/` 配下へ移動。挙動変更なし。
- 空の `crates/ics-core/` ワークスペースメンバを追加（ADR-017 Migration Step 2）。`makeholiday` から path 依存で接続。公開 API はまだ無し。型とパーサは Step 3 で移動。
- 型モデル（`VEvent`, `BusyStatus`, `EventClass`, `SortKey`）とパーサ・フォーマッタ・クエリヘルパを `crates/makeholiday/src/ics.rs` から `crates/ics-core/src/{event,calendar,parser,query}.rs` に移動（ADR-017 Migration Step 3）。makeholiday は `ics_core` 経由で型を利用するように変更。makeholiday namespace のプリセットアイコン（`PRESET_ICONS`, `format_icons_list`）は新規 `crates/makeholiday/src/icons.rs` に切り出し、`ics-core` には載せない。挙動変更なし。
- 型付き `ics_core::Error` を導入（ADR-017 §error type relationship）。`parse_events`, `parse_indices`, `insert_event`, `remove_event_by_summary`, `remove_events_by_indices` の返却型を `Result<T, String>` から `Result<T, ics_core::Error>` に変更。`Parse` バリアントは `message` に加えオプショナルな `line` と `property` を持ち、現状の flat parser では None のまま。ADR-019 の lexer ベースパーサが値を埋める。
- `makeholiday::error::MhError` を導入（ADR-012 / ADR-017 §error type relationship）。6 バリアント: `Io { path, source }`, `Parse(#[from] ics_core::Error)`, `InvalidInput(String)`, `Conflict(String)`, `NotFound(String)`, `AlreadyExists { path }`。`commands::*` は `Result<_, MhError>` を返却、`ics_core::Error` は `#[from]` 経由で `?` 伝播。テストは `matches!(err, MhError::InvalidInput(_))` の形でバリアントを検証するように更新。
- `crates/makeholiday/src/lib.rs` を新設（ADR-010 / ADR-017 準拠）しライブラリモジュールを宣言。`main.rs` は `use makeholiday::*` で取り込む薄い Composition Root に。ライブラリ表面を持つことで、将来のユースケース単体テストや ADR-022 の TUI からの再利用が可能に。
- `CalendarRepository` ポート（ADR-011）を `application::ports` に、ディスク実装 `FileCalendarRepository` を `infrastructure::file_calendar_repository` に新設。書き込みは `tempfile::NamedTempFile` + `persist` / `persist_noclobber` で原子化。プロセス中断で半端なファイルが残る可能性を排除。`tempfile = "3"` は `[dev-dependencies]` から `[dependencies]` へ移動。
- 旧 `commands.rs` のユースケースを `application::use_cases` に切り出し（ADR-009/017）。各ユースケースは `&Path` ではなく `&impl CalendarRepository` を受け取り、ファイル/パスの関心は Composition Root に集約。`commands.rs` 削除、9 件のテストは `use_cases.rs` に移動してリポジトリ抽象を検証。
- `cli.rs` を `presentation/cli.rs` に再配置（ADR-009 プレゼンテーション層）。`parse_date` を `cli.rs` から新規 `crate::input` モジュールへ抽出し、プレゼンテーション層（clap `value_parser`）とアプリケーション層（対話プロンプト）が層越境せずに共用できるよう変更。
- `ics_core::RawProperty` と `VEvent.unknown: Vec<RawProperty>` フィールドを導入（ADR-001 Migration Step 1）。パーサは登録済みベンダー prefix にマッチしない `X-*` プロパティを破棄せず取り込むようになり、フォーマッタは `source_index` 順にコンポーネント末尾へ出力（ADR-018 準拠）。`LANG=en` や `LANG="ja-JP"` のような params も保持、プロパティ名と param キーは UPPERCASE 正規化。既存の typed `X-*` 2 種（`X-MICROSOFT-CDO-BUSYSTATUS`, `X-MAKEHOLIDAY-ICON`）は引き続き型付き処理で `unknown` には重複しない（分離は Step 4/5）。
- `ics_core::profile::google` と `ics_core::profile::icloud` のスケルトンを追加（ADR-001 Migration Step 7）。両モジュールとも `PREFIXES`（`X-GOOGLE-` / `X-APPLE-`, `X-CALENDARSERVER-`）を登録し、`owns_property` を公開、空の `EventExtensions { unrecognized: Vec<RawProperty> }` を提供。パーサは prefix にマッチしたプロパティを対応するベンダーバケットにルーティング、フォーマッタはモジュール宣言順（`microsoft` → `google` → `icloud`）で出力後 `VEvent.unknown` を続ける。`VEvent` に `google: Option<google::EventExtensions>` と `icloud: Option<icloud::EventExtensions>` フィールド追加。これで ADR-001 Migration は **全 7 ステップ完了**: 3 ベンダーバケットがバイパスルーティングで安定し、具体的な要件に応じて typed フィールドへ昇格させる準備が整った。共通の `profile::matches_prefixes` ヘルパで prefix スキャンの重複を排除、`calendar::emit_unrecognized` で 4 バケット（microsoft, google, icloud, unknown）の sort + 出力パターンを集約。
- `ics_core::microsoft::EventExtensions` にベンダーごとの `unrecognized: Vec<RawProperty>` フォールバックを追加（ADR-001 Migration Step 6）。未 typed の `X-MICROSOFT-*` / `X-MICROSOFT-CDO-*` プロパティは `VEvent.unknown` ではなく `microsoft.unrecognized` にルーティングされるようになり、ADR-001 rule 4 の「安定 unknown バケット」不変条件が実体化。`microsoft::owns_property(name)` で prefix 判定を公開。パーサは VEVENT 内の全 `X-*` プロパティに単一の `source_index` を割り当てるので、フォーマッタのバケット内 sort が入力順序を保つ。フォーマッタは `microsoft.unrecognized`（`source_index` ソート）を Microsoft typed フィールド直後・非 Microsoft `unknown` より前に出力。
- `X-MAKEHOLIDAY-ICON` を `ics_core::VEvent` から外し、ADR-017 による ADR-001 rule 6 supersede に従って整理（ADR-001 Migration Step 5）。`VEvent.icon: Option<String>` フィールドを削除し、ics-core はパーサ・フォーマッタとも特別扱いをやめて、他の prefix-unmatched `X-*` プロパティと同様に `VEvent.unknown` 経由でラウンドトリップさせる。makeholiday 側に薄い reader/writer ヘルパ（`icons::read_icon`, `icons::write_icon`, `icons::ICON_PROPERTY`）を追加。`format_event_line` も `crate::display` に移し、内部で `read_icon` を呼ぶ形に再配置。CLI `--icon` フラグと表示出力は不変。
- `ics_core::profile::microsoft` モジュールを導入し、`X-MICROSOFT-CDO-BUSYSTATUS` をベンダーバンドルへ移動（ADR-001 Migration Step 4）。`VEvent.busystatus` を削除、`BusyStatus` は `MsBusyStatus` に改名して `ics_core::microsoft` に配置。`VEvent.microsoft: Option<microsoft::EventExtensions>` が `busystatus: Option<MsBusyStatus>` を保持。パーサは `X-MICROSOFT-CDO-BUSYSTATUS:` をバンドル側へルーティング、フォーマッタはバンドルに値がある場合のみ出力し、`TRANSP` のフォールバックもこの値から派生。Microsoft データもタイプ付き `transp` も無いイベントは TRANSP/BUSYSTATUS の両方を省略するようになった（以前は常にデフォルトから TRANSP:TRANSPARENT + BUSYSTATUS:FREE を出力していた）。CLI の `--busystatus` は引き続き `free` がデフォルトのため、`makeholiday add` の出力は不変。JSON 出力では `busystatus` が `microsoft` 配下にネスト。`microsoft::PREFIXES` は `X-MICROSOFT-CDO-` と `X-MICROSOFT-` を登録し、Step 6 の prefix ルーティングに備える。
- `ics_core::Transp` enum と `VEvent.transp: Option<Transp>` フィールドを導入（ADR-001 Migration Step 3）。パーサは `TRANSP:` を型付きフィールドへ直接読み込み、フォーマッタは `transp` が Some の場合はその値、None なら `busystatus.transp()` から派生してラウンドトリップ互換を維持。`busystatus: BusyStatus` は当面そのまま — Step 4 で `microsoft::EventExtensions` への移動（破壊的変更）を予定。
- `ics_core::VCalendar` と `ics_core::RawComponent` を導入（ADR-001 Migration Step 2）。`VCalendar` は `version`, `prodid`, optional `calscale` / `method`, `events: Vec<VEvent>`, `unrecognized_components: Vec<RawComponent>` を持つ。`VEvent` にも `unrecognized_components` フィールドを追加。パーサはドキュメント全体を `VCalendar` にパースし、`VTIMEZONE`（ネストした `STANDARD` / `DAYLIGHT` を含む）などのカレンダーレベルコンポーネントは `VCalendar.unrecognized_components` に、`VEVENT` 内の `VALARM` は `VEvent.unrecognized_components` に round-trip 保持。公開 API: `parse_calendar`, `format_calendar(&VCalendar)`。`parse_events` は薄い互換ラッパとして残置。クエリヘルパ（`remove_event_by_summary`, `remove_events_by_indices`）は `&VCalendar` を受け取り新しい `VCalendar` を返す形に。makeholiday のユースケースは依然 String ベースの `CalendarRepository` の周りで `parse_calendar` / `format_calendar` を呼ぶブリッジ（typed リポジトリ化は後続コミットで対応）。

### 追加
- ADR-020 のヘルプテキスト使用例: すべてのサブコマンド（`init`, `add`, `list`, `edit`, `icons`, `remove`）に `#[command(long_about = ...)]` で実行可能な使用例を最低 1 つ含める対応を実施、ADR-020 の "Required from day one" 規則を満たす。`makeholiday <subcommand> --help` でオプション一覧の前に複数行の説明と現実的なコマンド例（例: `makeholiday add --summary 元日 --start 2026-01-01`、`makeholiday edit 4 --category-clear --category 仕事 --category 出張`）が表示されるように。
- ADR-015 の最低限の出力制御を導入: グローバルフラグ `--quiet` / `-q`（add/edit/remove の "Added: ..." 等の status メッセージを stderr で抑制）、`--interactive`（stdin が TTY でなくてもプロンプトを強制）、`--no-interactive`（プロンプト禁止、必須引数欠落時はエラー）。どのオーバーライドも無ければ Composition Root が `std::io::stdin().is_terminal()` で自動判定 — パイプ / リダイレクトされた stdin はプロンプト待ちでハングする代わりに明確なエラーで早期失敗。`application::use_cases::RunContext { quiet, allow_prompts }` を新設して add / edit / remove に取り回し（list と icons は元から副作用なし）。Status メッセージは `RunContext::status` 経由で出力するので `--quiet` がユースケース横断で一貫。CLI 統合テストを 2 件追加: `--quiet` / `-q` の抑制動作と TTY 検出による拒否動作。
- `edit` サブコマンドで VEVENT CRUD 完結。`makeholiday edit <INDEX> [--summary X] [--start X] [--end X] [--busystatus X] [--class X] [--category X]... [--category-clear] [--icon X] [--icon-clear]` で 1-based インデックス指定によりイベントを部分更新。フラグ名は ADR-020 の「共通意味は共通名」ルールにより `add` と一致。`--start` のみ指定時はイベント期間を保ったまま日付移動、両方指定時は end >= start を検証。`--category-clear` で既存カテゴリを消去（`--category` 指定とは独立）、新カテゴリは上書き、何も指定しなければ据え置き。`--icon` / `--icon-clear` も同等（相互排他）。インデックス範囲外は `MhError::NotFound`。`EditPatch` 構造体を `application::use_cases` に新設して関数引数の肥大化を回避。
- PRD §9 ロードマップ章を新設し v0.1.x / v0.2.0 / v0.3.0 マイルストーンを明文化。v0.1.x は現行の「ICS テキスト操作」トラック（ロスレスラウンドトリップ、パーサ正しさ、CLI サブコマンドの揃え）。v0.2.0 は「ICS エコシステム」マイルストーン: `ics-core` を別リポジトリ + crates.io に切り出し、`lazyics`（lazygit インスパイアの TUI エディタ）と `icslint`（`ics-core` を消費するリンタ）を launch — 3 ツール同時 ship。v0.3.0 で CalDAV / クラウドバックエンド導入、timed `VEvent` 型化（ADR-001 Rule 9 改訂）、per-event リポジトリ抽象。CalDAV 応答も構文的に正当な VCALENDAR ブロブなので ics-core パーサと型モデルは 3 マイルストーン通して無改修で継承され、v0.3.0 の作業は I/O 境界に集中する。
- ドキュメント基盤一式: `README`, `PRD`, `CONTRIBUTING`, `SETUP`, `USAGE`（英語版と日本語版）。
- ADR 000〜023: ADR ポリシー、ベンダー拡張型付けモデル、言語/エディション、デュアルライセンス、Trunk-based + SemVer、Conventional Commits、テスト戦略、ドキュメント言語ポリシー、MSRV、モジュール階層、lib/main 分離、I/O 境界 + リポジトリパターン、エラーハンドリング、依存ポリシー、CI/CD プラットフォーム、診断出力、設定ポリシー、ワークスペース + `ics-core` クレート、ラウンドトリップ戦略、パーサ実装、CLI サブコマンドポリシー、VTODO スコープ、TUI フロントエンドポリシー、`convert` サブコマンド非提供決定。
- [ADR-024](design/024-solo-phase-branching-carve-out.md) — Solo フェーズの間 ADR-004 の feature ブランチ + PR セレモニーを一時停止する例外。`ics-core` のリポジトリ分離、外部コントリビュータの PR、`v1.0.0` タグのいずれかで自動解除。

