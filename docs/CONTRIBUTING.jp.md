[ [English](CONTRIBUTING.md) | **日本語** ]

# makeholiday へのコントリビュート

コントリビューションを検討いただきありがとうございます。本書はリポジトリの開発フローと規約をまとめたものです。

## 行動規範 (Code of Conduct)

敬意と建設性をもって接してください。専用の `CODE_OF_CONDUCT.md` は将来追加する可能性があります。それまでは [Contributor Covenant](https://www.contributor-covenant.org/) の精神を準用します。

## 開発環境

- **ツールチェイン:** Rust, edition `2024`（[Cargo.toml](../Cargo.toml) 参照）
- **ビルド:** `cargo build`
- **テスト:** `cargo test`
- **ローカル実行:** `cargo run -- <subcommand> [options]`

OS 固有のセットアップは不要で、Windows / macOS / Linux でビルドできます。

## プロジェクト構成

[ADR-017](design/017-workspace-and-ics-core-crate.md) に基づく Cargo workspace 構成:

```
Cargo.toml                        # workspace マニフェスト（[workspace] と共通依存）
crates/
  ics-core/                       # 共有 ICS ライブラリ — 型モデル・パーサ・フォーマッタ
    Cargo.toml
    src/
      lib.rs                      # クレートルート + re-export
      event.rs                    # VEvent, BusyStatus, EventClass, format_event_line
      calendar.rs                 # VCALENDAR header/footer, format_vevent, format_calendar, insert_event
      parser.rs                   # parse_events, parse_indices
      query.rs                    # sort_events / SortKey / remove_event_by_* ヘルパ
  makeholiday/                    # CLI バイナリクレート
    Cargo.toml
    src/
      main.rs                     # Composition Root — リポジトリ配線とサブコマンド振り分け
      lib.rs                      # ライブラリ表面（下記モジュールを re-export）
      cli.rs                      # Clap 定義、日付パース
      error.rs                    # MhError（ADR-012）
      icons.rs                    # makeholiday namespace の PRESET_ICONS テーブル
      application/
        ports.rs                  # CalendarRepository trait（ADR-011）
        use_cases.rs              # init / add / list / remove のユースケース関数
      infrastructure/
        file_calendar_repository.rs  # tempfile + persist による atomic write 実装
    tests/
      cli.rs                      # assert_cmd による統合テスト
docs/
  README.jp.md
  PRD.md, PRD.jp.md
  CONTRIBUTING.md, CONTRIBUTING.jp.md
  design/                         # Architectural Decision Records (ADRs)
.github/workflows/                # CI / Release / Audit（[ADR-014](design/014-ci-cd-platform.md) 参照）
deny.toml                         # cargo-deny 設定
```

## ワークフロー

- **Trunk-based development.** `main` から短命ブランチを切り、小さな PR でマージします。長命の feature ブランチは避けます。
- **ブランチ命名:** `<type>/<short-slug>`。例: `feat/add-rrule`, `fix/parse-date`, `docs/contributing`。
- **1 PR 1 関心事.** リファクタリングと挙動変更を混ぜないこと。レビューが困難になります。

> **Solo フェーズ例外:** プロジェクトが solo フェーズの間は、feature ブランチ + PR ではなく `main` 直接コミットを許容します。`ics-core` を別リポジトリに切り出した時点、外部コントリビュータが PR を開いた時点、または `v1.0.0` タグ時点で自動的に解除されます。CI（テストマトリクス + clippy `-D warnings` + fmt + `cargo deny`）が gate として残り、「1 コミット 1 関心事」と Conventional Commits は引き続き適用します。[ADR-024](design/024-solo-phase-branching-carve-out.md) 参照。

## コミットメッセージ

`git log` に見られる既存スタイル（Conventional Commits 風）を踏襲してください:

- `feat: ...` — ユーザ向けの新機能
- `fix: ...` — バグ修正
- `chore: ...` — ツール、ビルド、gitignore、依存更新
- `refactor: ...` — 挙動を変えない内部再編
- `docs: ...` — ドキュメントのみ
- `test: ...` — テストのみ

件名は約 72 文字以内に。本文には *なぜ* を書きます。

## コーディング原則

- **TDD (Red → Green → Refactor).** 新挙動は失敗するテストから。既存テストがない場合は、変更前に現在の挙動を捉える最小の特性テストを追加します。
- **Tidy First / non-ad-hoc.** 変更範囲を最小化します。新規コード追加の前に関連コードの整理を優先します。
- **責務分離.** `cli`（入力パース）/ `commands`（オーケストレーション）/ `ics`（ドメインのシリアライズ）の境界を尊重し、依存は上位方針へ向けます。下位層が上位層へ逆参照しないこと。
- **意図を名前と構造で表現.** 関数内コメントは最小限に。意図が不明瞭なら、コメントよりも抽出やリネームを優先。インターフェースの契約は doc コメントに集約します。
- **状態中心の設計.** Given / When / Then で状態を捉えます。状態の意味が曖昧な場合は、アルゴリズムを書く前に合意を取ります。

## ドキュメント変更

- ユーザ向け変更は `README.md`（英語、プライマリ）と `docs/README.jp.md`（日本語訳）を同一 PR で更新します。
- プロダクト方針の変更は `docs/PRD.md`（および JP 版）を更新します。
- アーキテクチャ判断は `docs/design/` 配下に ADR として記録します。形式は [`000-ADR-policy.md`](design/000-ADR-policy.md) に従います。
- 既存ドキュメントを尊重し、決定の履歴を黙って書き換えないこと。

## 依存ポリシー

[ADR-013](design/013-dependency-policy.md) により、新規の runtime / build 依存追加には以下の簡易チェックリストを通すこと:

- **ライセンス互換性** — MIT / Apache-2.0 / BSD / MPL-2.0（あるいはこれを包含するもの）に限る。Copyleft（GPL, AGPL）は不可。
- **MSRV** — 現在の [rust-version](../Cargo.toml) でビルドできること。
- **メンテナンスシグナル** — 直近のコミット、open issue のトリアージ状況、被依存数。
- **代替検討** — 同種クレートとの比較理由を簡潔に。
- **表面の正当化** — 多目的フレームワークより、小さく目的が絞れたクレートを優先。

事前承認済み（チェックリスト不要）: `clap`, `chrono`, `uuid`, `serde`, `serde_json`, `tempfile`, `thiserror`, `assert_cmd`, `predicates`。

判断理由は PR description に数行で記載。レビューアが確認します。

## CLI フラグ命名

[ADR-020](design/020-cli-subcommand-policy.md) により、フラグは **共通意味は共通名** ルールに従う:

- 複数サブコマンドで同じ意味を持つフラグは、ロング名・ショート形を **必ず同一** にすること（例: `--summary` は常にイベントタイトル、`--file` / `-f` は常にカレンダーファイル）。
- 逆に、同じ名前を別の概念に再利用するのは禁止。
- 新規サブコマンドは動詞名を優先（`add`, `edit`, `search`）。名詞例外（`icons`）は組み込みデータ列挙用途のみ。
- 新規サブコマンドは clap の `long_about` か doc コメントで `--help` に使用例を 1 つ以上含めること。

新規フラグを追加する前に [ADR-020 のフラグ表](design/020-cli-subcommand-policy.md#global-vs-subcommand-local-flags) を必ず参照すること。

## テスト

- PR 提出前に `cargo test` を成功させること。
- 新機能はテストと一緒に提出します。統合カバレッジは `tests/cli.rs`、単体カバレッジは対象モジュール内（`#[cfg(test)] mod tests`）に置きます。
- バグ修正には、修正前に失敗する回帰テストを添えます。

## Issue / PR テンプレート

当面は本書の構成を直接踏襲してください。`.github/` 配下の専用テンプレートは将来導入する可能性があります。

## コントリビューションのライセンス

本プロジェクトのライセンスに合わせ、コントリビューションは **MIT OR Apache-2.0** のデュアルライセンスで提供されます。コントリビューションを提出することで、この条件での頒布に同意したものとみなされます。
