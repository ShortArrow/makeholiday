[ [English](PRD.md) | **日本語** ]

# プロダクト要件定義書 — makeholiday

> ステータス: **スケルトン**。章立てのみ。本文はフォローアップで埋める。

## 1. 背景 (Background)

<!-- TODO: 本ツールが存在する理由。既存 ICS ツールの不足点。 -->

## 2. ゴール (Goals)

<!-- TODO: 成功条件を計測可能な箇条書きで。 -->

## 3. 非ゴール (Non-Goals)

<!-- TODO: 明示的にスコープ外とするもの。 -->

## 4. ターゲットユーザー (Target Users)

<!-- TODO: ペルソナ。CLI 慣れした個人カレンダー管理者 / ICS 機能を組み込むインテグレータ等。 -->

## 5. 機能要件 (Functional Requirements)

### 5.1 提供済み (v0.1.0)

<!-- TODO: 提供済み機能を正式な要件として文書化する。 -->

- `init` — `VCALENDAR` ファイルを作成
- `add` — `VEVENT` を追加（終日、単日 / 複数日、busy status / class / categories / icon 付き）
- `list` — イベント列挙、多キーソート、JSON 出力
- `icons` — 同梱アイコン名表示
- `remove` — 1 始まりインデックス、範囲指定、サマリ指定で削除

### 5.2 計画中

<!-- TODO: 各項目を受け入れ基準付きの要件に展開する。 -->

- **ICS CRUD 強化** — 高度なクエリ、その場編集、一括 import/export
- **各社独自拡張対応** — Outlook / Google Calendar / iCloud の拡張をロスなく扱う
- **RFC 準拠と独自拡張の境界** — 境界を明文化し、ベンダープロファイルごとのラウンドトリップ保証
- **ICS ハンドリングライブラリ提供** — パース / フォーマットの中核を独立 crate として切り出す

## 6. 非機能要件 (Non-Functional Requirements)

<!-- TODO: パフォーマンス、対応 OS (Windows / macOS / Linux)、安定性、エラー通知、i18n。 -->

## 7. スコープ外 (Out of Scope)

<!-- TODO: 本プロダクトでは扱わないもの（例: サーバ同期、GUI）。 -->

## 8. 未決事項 (Open Questions)

<!-- TODO: 未確定の判断事項。 -->
