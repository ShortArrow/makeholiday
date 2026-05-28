[ [English](PRD.md) | **日本語** ]

# プロダクト要件定義書 — makeholiday

> ステータス: **Draft**。1〜4 章と 5.1 は確定。それ以降は継続調整。

## 1. 背景 (Background)

iCalendar (RFC 5545) はカレンダーデータ交換のデファクト形式ですが、周辺ツールのエコシステムは分断されています。既存の ICS ツールはおおむね 2 つの失敗パターンに陥っています:

- **厳格な RFC 専用ツール** — ベンダー固有拡張（Outlook の `X-MICROSOFT-CDO-*`、Google の `X-GOOGLE-*`、iCloud のバリアントなど）を落とすか拒否し、ラウンドトリップで暗黙のデータロスを起こす。
- **ベンダー専用ツール** — ある 1 社の方言は手厚く扱えるが、他社の方言を一級データとして表現できない。

結果として、`.ics` を組み立てたり編集したい一般ユーザ（個人の祝日、チームカレンダー、カレンダーサービス間の橋渡し）は、毎回その場限りのスクリプトを書くか、ロスありの変換を受け入れるしかありません。

`makeholiday` はこの隙間を、小さく意図的なツールで埋めます: ベンダー拡張を不透明文字列ではなく一級値として扱う型付きコアの上に乗った、日々の ICS オーサリング用 CLI です。

## 2. ゴール (Goals)

優先順位順:

1. **CLI UX（最優先）.** 日々のカレンダー編集が気持ちよくできる CLI を提供する — 発見しやすいサブコマンド、妥当なデフォルト、スクリプト用途（フル引数）と対話モードの両立。設計の綺麗さと UX が衝突したら UX を優先する。
2. **ラウンドトリップのロスレス性.** ICS を読んで再出力したとき、順序、意味的に必要な空白、そして *すべて* のプロパティ（未知・ベンダー固有を含む）を保つ。`makeholiday` を通過したファイルは元のツールから見ても同じものに見える。型レベルの取り決めは [ADR-001](design/001-vendor-extension-typing.md)、順序セマンティクスは将来のラウンドトリップ戦略 ADR を参照。
3. **ベンダー拡張の型付き処理.** Outlook / Google / iCloud の拡張を生の `X-*` 文字列ではなく、型安全な別個の値としてモデル化する。RFC 5545 と各ベンダープロファイルの境界はコード上で明示し、ドキュメント化する。モデルは [ADR-001](design/001-vendor-extension-typing.md) を参照。
4. **ライブラリとしての再利用性.** ICS ハンドリングの中核は独立 crate として利用可能にし、他ツールが CLI まわりの依存なしで取り込めるようにする。

## 3. 非ゴール (Non-Goals)

- **サーバ / サービス同期.** CalDAV サーバ、Google Calendar API、iCloud Calendar、Outlook Online との直接連携は扱わない。`makeholiday` はローカルの `.ics` ファイルを対象とする。
- **GUI / WebUI.** デスクトップアプリも Web UI も提供しない。（TUI は別バイナリとして計画中 — [ADR-022](design/022-tui-front-end-policy.md) 参照）
- **ICS 以外のカレンダー形式.** Microsoft `.msg`、旧 vCalendar 1.0、独自バイナリカレンダー形式はスコープ外。
- **ベンダープロファイル変換.** ICS をあるベンダーの方言（Outlook / Google / iCloud）から別の方言へ変換することはスコープ外。ラウンドトリップでは入力時のプロファイルを変更せず保持する。[ADR-023](design/023-no-convert-subcommand.md) 参照。

## 4. ターゲットユーザー (Target Users)

両方の層を扱うが、CLI 層が優先順位を決める。

- **主要 — CLI 慣れした個人** — 個人の祝日・休暇・予定カレンダーをターミナルから管理したいユーザ。スクリプタブルさ、プレーンテキスト保存、最小限の手数を重視する。
- **副次 — カレンダーインテグレータ** — ICS を生成 / 取り込むツールを作っている人。ライブラリ表面と型付きベンダー拡張モデルを必要とする。

両者の要求が衝突した場合、CLI 層の要求を優先する。

## 5. 機能要件 (Functional Requirements)

### 5.1 提供済み (v0.1.0)

実装済みで `tests/cli.rs` の統合テストと `src/` 内の単体テストにより検証済み:

- **`init`** — `VCALENDAR` ファイルを新規作成（`PRODID:-//makeholiday//EN`, `VERSION:2.0`）。
- **`add`** — `VEVENT` を追加（終日、単日 / 複数日）。サポート項目:
  - `--summary`, `--start`, `--end`（入力は inclusive、内部で RFC の exclusive `DTEND` に変換）
  - 日付入力形式: `YYYY-MM-DD` と `YYYY/M/D`
  - `--busystatus`（`free` / `tentative` / `busy` / `oof` / `working`）→ `TRANSP` + `X-MICROSOFT-CDO-BUSYSTATUS` を出力
  - `--class`（`public` / `private` / `confidential`）
  - `--category`（繰り返し指定可）
  - `--icon`（独自拡張 `X-MAKEHOLIDAY-ICON`）
  - `--summary` / `--start` 省略時は対話モード
- **`list`** — イベント列挙。`--sort`（繰り返し: `start` / `end` / `summary`）, `--desc`, `--json`。
- **`icons`** — 同梱のプリセットアイコン名を表示。
- **`remove`** — 1 始まりインデックス（`N`, `N-M`, `N,M`, 混在）、`--summary` 一致、対話選択で削除。

### 5.2 計画中 (Planned)

おおむね優先度順。受け入れ基準は着手時に展開する。

- **`edit` サブコマンド** — インデックス指定で既存イベントをその場編集。CRUD を完成させるのに必須。
- **`search` / `filter` サブコマンド** — 日付範囲・サマリ部分一致・カテゴリ・busy status でクエリ。
- **`import` / `export` バリアント** — 他 ICS からの一括取込み。ベンダープロファイルは入力時のまま保持し、正規化や変換は行わない（[ADR-023](design/023-no-convert-subcommand.md) 参照）。
- **ベンダー拡張対応 — Outlook プロファイル.** `X-MICROSOFT-CDO-*` 群、リマインダ、カテゴリ色などを一級型化。型モデルは [ADR-001](design/001-vendor-extension-typing.md)。
- **ベンダー拡張対応 — Google プロファイル.** `X-GOOGLE-*` と Google 固有値ハンドリングを一級型化。型モデルは [ADR-001](design/001-vendor-extension-typing.md)。
- **ベンダー拡張対応 — iCloud プロファイル.** Apple 固有拡張（`X-APPLE-*`, `X-CALENDARSERVER-*`）を一級型化。型モデルは [ADR-001](design/001-vendor-extension-typing.md)。
- **RFC ↔ ベンダー拡張の境界ドキュメント.** どのプロパティが RFC 5545 で、どれが各ベンダープロファイルに属するかをリファレンス化。可能ならコードから自動生成。境界ルールは [ADR-001](design/001-vendor-extension-typing.md) に定義。
- **再利用可能 ICS ハンドリングライブラリ（`ics-core` crate）.** 共有コアは `crates/ics-core/` にワークスペースメンバとして配置。外部公開時期は [ADR-017](design/017-workspace-and-ics-core-crate.md) で確定。型シェイプは [ADR-001](design/001-vendor-extension-typing.md)。
- **タスク管理プロパティ（`VTODO`）.** `ics-core` に型付き `VTodo` を載せ、makeholiday CLI は `list --include-todos` による読み取り専用表示のみ提供（編集サブコマンドなし）。[ADR-021](design/021-vtodo-scope.md) 参照。
- **TUI フロントエンド.** `ics-core` を消費する別ワークスペースメンバ（`crates/makeholiday-tui/`）として計画。着手時期は未定でメンテナの判断による。[ADR-022](design/022-tui-front-end-policy.md) 参照。

## 6. 非機能要件 (Non-Functional Requirements)

- **対応プラットフォーム.** Windows / macOS / Linux を一級サポート。CI で 3 系統すべてをカバーする。
- **パフォーマンス.** イベント数 1 万件規模までを通常マシンで 1 秒未満に。それ以上はストレッチゴール。
- **メモリ.** 可能ならファイル全読込みよりストリーミングパーサを優先。v0.x のブロッカーではない。
- **安定性.** 1.0 リリース以降は CLI 表面で SemVer を守る。1.0 以前の破壊的変更は changelog に明記。
- **エラー通知.** ICS パース時、エラーは入力行と問題のあるプロパティ名を特定して報告する。コマンドはサイレントに落とさず非ゼロ終了で失敗する。
- **国際化.** サマリ、カテゴリ等の自由文フィールドは非 ASCII (UTF-8) をエスケープなくロスなくラウンドトリップする。デフォルトの例文・ヘルプは英語、日本語訳は `docs/*.jp.md` に置く。

## 7. スコープ外 (Out of Scope)

非ゴールとは別物。これらはどの計画リリースにも明示的にコミットしないが、将来スコープインする可能性はある。

- マシン間でのカレンダー状態クラウド同期。
- カレンダー招待ワークフロー（iTIP の `REQUEST` / `REPLY` / `CANCEL` メソッドハンドリング）。
- 繰り返しイベントの離散インスタンスへの展開（RRULE の materialize）。ラウンドトリップでの RRULE *保持* はスコープ内、展開はスコープ外。
- タイムゾーンデータベース同梱。タイムゾーンが絡む場面ではシステムの tz データベースに依存する。

## 8. 未決事項 (Open Questions)

- **TUI フロントエンドの着手時期** — スコープと構造は [ADR-022](design/022-tui-front-end-policy.md) で確定済み。残る未決はメンテナ判断により *いつ* 着手するかだけ。
- **プリセットアイコン名 / 説明のライセンス** — `PRESET_ICONS` テーブルはプロジェクトライセンスで配布する。将来 SVG / 画像アセットを追加する際は再考。
