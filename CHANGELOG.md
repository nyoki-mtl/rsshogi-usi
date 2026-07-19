# Changelog

このファイルでは `rsshogi-usi` の user-visible changes を release ごとに要約する。
`0.1.x` は旧package名`rshogi-usi`で公開されていた。以前のreleaseは旧repositoryの
Git tag / GitHub Releases / task docsを参照する。

## [0.2.0] - 2026-07-20

### Changed

- package名を`rshogi-usi`から`rsshogi-usi`へ変更した。
- Rust import名を`rshogi_usi`から`rsshogi_usi`へ変更した。
- repository、docs.rs、GitHub Pages、README、release workflowを新しい名称へ移行した。

### Notes

- public APIとprotocol behaviorは`rshogi-usi 0.1.6`を引き継いでいる。
- `rsshogi`へのdependencyは追加しておらず、protocol crateの独立性は変わらない。
- 旧`rshogi-usi`のyankは、新crate公開とconsumer移行後の別gateで行う。

## [0.1.6] - 2026-06-02

### Added

- `GoParams::effective_movestogo()` を追加し、consumer が `movestogo 0` を未指定扱いにしたい場合の明示的な helper を提供した。
- `GoMate::as_i32_saturating()` を追加し、`GoMate::Ply(u32)` / `GoMate::Infinite` を `i32` depth budget に安全に変換しやすくした。

### Notes

- parser / formatter の roundtrip behavior は変更していない。`go movestogo 0` は command model 上では引き続き `movestogo: Some(0)` として保持される。
- repository maintenance として、root metadata / README の file mode を non-executable に正規化した。

## [0.1.5] - 2026-04-19

### Added

- `ParseErrorSite` と site-aware diagnostics を追加し、token position と byte range を `ParseError` から取得できるようにした
- `PositionSpec::replay(...)` と `PositionReplayError` を追加し、`position` + `moves` を consumer 側 position builder へ安全に流し込みやすくした
- `GoMate`, `GoParams`, `CheckmateResponse`, `UsiCommand` に `go mate` / `checkmate` 構築 helper を追加した
- `UsiCommandDirection` と direction helper を追加し、標準 USI command の向きを判定しやすくした
- `validate_portable_command(...)` を追加し、ShogiHome など単一 token option name 前提 GUI 向けの portability check を明示化した
- portable transcript test を追加し、canonical roundtrip とは別に portability contract を固定した

### Changed

- parser が `ponderhit btime ...` のような ShogiHome early ponder 互換入力を `Extension("ponderhit", ...)` として受理するようになった
- `option` の `string` / `filename` default に対して `<empty>` sentinel と空文字を相互正規化するようにした
- strict transcript / compatibility transcript を更新し、mate command と portability 周りの canonical behavior を強化した

### Notes

- この release は additive patch release で、既存 command model / parser / formatter の public surface を壊さずに診断性と互換性を強化している
