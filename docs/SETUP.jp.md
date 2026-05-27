[ [English](SETUP.md) | **日本語** ]

# セットアップ

`makeholiday` をローカルマシンで動かすまでの手順です。日常的なコマンドは [USAGE.md](USAGE.jp.md)、開発フローは [CONTRIBUTING.md](CONTRIBUTING.jp.md) を参照してください。

## 前提

- **Rust ツールチェイン** (edition 2024 対応)。[rustup](https://rustup.rs/) からインストール:
  ```sh
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
  Windows では同サイトの `rustup-init.exe` を使用。
- **C リンカ.** Linux (gcc/clang) と macOS (Xcode Command Line Tools) には既に入っています。Windows は [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) の「C++ によるデスクトップ開発」ワークロードを導入してください。

ツールチェイン確認:

```sh
rustc --version
cargo --version
```

## インストール

### ローカルチェックアウトから

```sh
git clone https://github.com/ShortArrow/makeholiday.git
cd makeholiday
cargo install --path .
```

これで `makeholiday` バイナリが `~/.cargo/bin/`（Windows では `%USERPROFILE%\.cargo\bin\`）に配置されます。当該ディレクトリが `PATH` に含まれていることを確認してください。

### crates.io から

未公開。状況は [PRD §5.2](PRD.jp.md#52-計画中-planned) を参照。

### インストールせずビルドのみ

```sh
cargo build --release
./target/release/makeholiday --help
```

## 動作確認

```sh
makeholiday --help
makeholiday icons
```

ヘルプとプリセットアイコン一覧がエラーなく表示されれば OK です。

## プラットフォーム別メモ

- **Windows.** PowerShell や任意のモダンターミナルを使用してください。生成される `.ics` ファイルの改行は RFC 5545 が要求する CRLF で、ホスト OS とは独立です。
- **macOS.** 前提以外の追加手順はありません。
- **Linux.** 追加手順はありません。ディストロパッケージは提供しておらず、`cargo install` 経由で導入してください。

## アップデート

ローカルチェックアウトから:

```sh
git pull
cargo install --path . --force
```

`cargo install` は既存バイナリを既定で上書きしないため `--force` が必要です。

## アンインストール

```sh
cargo uninstall makeholiday
```

## トラブルシューティング

- **`error: linker 'cc' not found`** — 各プラットフォームの C ビルドツールを導入してください（前提セクション参照）。
- インストール後に **`makeholiday: command not found`** — `~/.cargo/bin` が `PATH` に入っているか確認。
- **ツールチェインが古い** — `rustup update stable` で更新。
