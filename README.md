# rsshogi-usi

[crates.io](https://crates.io/crates/rsshogi-usi) |
[docs.rs](https://docs.rs/rsshogi-usi) |
[Documentation](https://nyoki-mtl.github.io/rsshogi-usi/) |
[MIT License](LICENSE)

将棋エンジンで共有しやすい USI protocol surface を提供する Rust crate です。
USI command model、parser、formatter を engine 固有ロジックから切り離し、
GUI と engine 間の line I/O を安定して実装しやすくします。

## 主な機能

- **Command model**: `UsiCommand`, `GoParams`, `InfoCommand`, `PositionSpec` などの型付き surface
- **Parser**: GUI→Engine / Engine→GUI の USI line を parse
- **Formatter**: command model から canonical な USI line を出力
- **Mate line helpers**: `GoMate`, `GoParams`, `CheckmateResponse`, `UsiCommand` から `go mate` / `checkmate` を組み立てやすい
- **Diagnostics**: parse failure に token / byte 位置 metadata を付けて consumer 側の調査を助ける
- **Position replay helper**: `PositionSpec + moves` を engine-agnostic な callback で replay できる
- **Portability checks**: `validate_portable_command` で ShogiHome など単一 token 前提 GUI へ出す line を事前確認できる
- **互換性保持**: engine 固有 token は stringly な拡張 surface に残し、protocol crate に取り込みすぎない
- **Transcript coverage**: roundtrip と compatibility を transcript test で固定

## インストール

```bash
cargo add rsshogi-usi
```

```toml
[dependencies]
rsshogi-usi = "0.2.0"
```

## クイックスタート

```rust
use rsshogi_usi::{format_command, parse_line, UsiCommand};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let command = parse_line("position startpos moves 7g7f 3c3d")?;

    match command {
        UsiCommand::Position { spec, moves } => {
            let canonical = format_command(&UsiCommand::Position { spec, moves });
            println!("{canonical}");
        }
        _ => unreachable!(),
    }

    Ok(())
}
```

## 対応 surface

- GUI→Engine: `usi`, `isready`, `setoption`, `position`, `go`, `stop`, `ponderhit`, `quit`, `usinewgame`, `gameover`
- Engine→GUI: `id`, `option`, `usiok`, `readyok`, `info`, `bestmove`, `checkmate`
- 拡張 command: `eval`, `test_movegen`, `test_see`, `ponderhit ...` (ShogiHome early ponder 互換入力)

## ドキュメント

- **[ガイド / mdBook](https://nyoki-mtl.github.io/rsshogi-usi/)** - 利用ガイドと設計メモ
- **[Rust API リファレンス](https://docs.rs/rsshogi-usi)** - 型と関数の詳細
- **[CHANGELOG](CHANGELOG.md)** - release ごとの差分要約
- **[Minimal Engine Example](https://github.com/nyoki-mtl/rsshogi-usi/blob/main/crates/rsshogi-usi/examples/minimal_engine.rs)** - `parse_line -> match UsiCommand -> format_command` の最小例

## rshogi-usi からの移行

このcrateは`rshogi-usi`の後継です。dependency名とRust import名を次のように変更してください。

```diff
-rshogi-usi = "0.1.6"
+rsshogi-usi = "0.2.0"
```

```diff
-use rshogi_usi::{format_command, parse_line, UsiCommand};
+use rsshogi_usi::{format_command, parse_line, UsiCommand};
```

公開APIとprotocol behaviorは`rshogi-usi 0.1.6`を引き継いでいます。
`rsshogi`へのdependencyは持たず、engine-agnosticなprotocol crateとして独立しています。

## ライセンス

MIT License
