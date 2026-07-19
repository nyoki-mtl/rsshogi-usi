# Architecture Notes

## Current Focus

- `rsshogi-usi` を先に安定させる
- 共有対象は command model / parser / formatter / transcript test
- session 共通化は後回し

## References

- `_refs/rshogi-nnue`: 現行の USI parser / session surface の主参照
- `_refs/rshogi-dfpn`: `go mate` 系の consumer になりうる補助参照
- `_refs/rshogi-az`: 直近では protocol API へ反映しない

`_refs/` 以下は参照専用であり、編集しない。

## Confirmed Gaps

- `rshogi-nnue` には `crates/usi/src/parser.rs` があり、`UsiCommand`, `GoParams`, `PositionSpec`, `ParseError` の原型がある
- `rshogi-dfpn` 側には独立した共通 USI surface が薄いため、protocol 設計は `nnue` の surface を主参照にしつつ public API を絞る必要がある
- `position` / `setoption` の構文自体は `nnue` / `dfpn` で大きくぶれておらず、shared AST の追加分岐は当面不要
- `rshogi-dfpn` では `go mate infinite` を扱うため、shared API でも数値限定ではなく `infinite` を表現できる必要がある
- `eval`, `test_movegen`, `test_see` は表現対象に含めるが、engine 固有 enum ではなく `Extension { name, args }` の形で保持し、protocol crate の public API に consumer 固有責務を持ち込まない
- `go` formatter は typed field を canonical な順序で出力し、unknown / consumer-local token は `extras` に残して末尾へ送る

## Standard Surface Decisions

- Engine→GUI 側も shared crate に含め、`id`, `option`, `usiok`, `readyok`, `bestmove`, `info`, `checkmate` を標準 surface として扱う
- 将棋所系の `gameover`, `bestmove resign`, `bestmove win` は engine 固有ではなく GUI 相互運用の差分なので、shared crate で表現する
- `info` は `depth`, `seldepth`, `time`, `nodes`, `multipv`, `score`, `currmove`, `hashfull`, `nps`, `pv`, `string` までを型付きで持ち、未整理・将来拡張分は `extras` に残す
- `option` は標準 type と主要 parameter を共通化しつつ、未知 type は `Other(String)`、未知 token は `extras` で保持して public API を閉じすぎない
- `checkmate` は `moves | notimplemented | timeout | nomate` を shared crate に持ち、`go mate` の consumer 差分を protocol crate の外へ漏らさない
- `go mate` / `checkmate` は parser / formatter だけでなく helper でも組み立てやすくし、consumer が line 文字列を直書きしなくて済むようにする
- transcript fixture は command 群ごとに分割し、互換入力と canonical 出力を `=>` で並べて仕様メモとしても読める形を優先する
- `parse_line` は permissive parser として `position ... moves` の空 move 列や duplicate `go` keyword の last-one-wins を許容する一方、`checkmate timeout extra` のような予約 status に続く余計な token は reject する
- `parse_line_strict` は permissive parse 後に canonical formatter と一致する入力だけを受け付ける transcript / contract 用の入口とする
- transcript helper は shared crate から利用できるようにし、downstream でも `=>` fixture 形式を再利用できるようにする
- `PositionSpec` は stringly な protocol surface を維持するが、consumer の startpos / SFEN 分岐を薄くするための callback-based replay helper は protocol crate に置いてよい
- parse diagnostics は `ParseErrorKind` を大きく変えず、token / byte 位置 metadata を additive に足して consumer の調査性を上げる

## Deferred

- duplicate `go` option の厳密な禁止 policy。現時点では parser の last-one-wins と formatter の canonical order で roundtrip を安定させる
- `go mate` の値省略を共通 surface として許容するかどうか。`dfpn` の互換挙動はあるが、shared crate ではまだ採用しない
- `info` の残り subcommand (`currline`, `refutation`, `cpuload` など) をどこまで shared crate で型付きにするか
- `rsshogi-usi-session-core` の publish
- `session-core` 雛形の追加。0006 の consumer review では、`nnue` は通常探索の `bestmove/info` 系、`dfpn` は `checkmate` 系応答が中心で、共通化できるのは薄い lifecycle / cancellation shell に留まったため、2 consumer で同じ契約が再確認できるまで保留する
- `az` 依存の抽象化
- GUI 差異の完全吸収

## Consumer Boundary Review (0006)

- `nnue` / `dfpn` ともに `position`, `setoption`, `go` の token surface は `rsshogi-usi` の command model へ自然に写せる
- 一方で session の中心責務は、`nnue` では state validation / time allocation / worker orchestration、`dfpn` では `go mate` 専用 flow / cancellation / `checkmate` emission に寄っており、shared crate に上げると engine 固有性が強く残る
- `0005` で固定した transcript contract は adapter 側の warning 文言や unsupported command policyとは独立に維持できるため、canonicalization の責務は protocol crate に残す
- 将来 `session-core` を置くとしても候補は `PositionSpec + moves` 適用、薄い lifecycle helper、cancelable job shell 程度であり、option catalog・時間管理・出力規約は入れない

## Session-Core Decision (0007)

- `0007` では `crates/rsshogi-usi-session-core` を追加しない。現時点で共通化候補が薄く、workspace を増やす利益より責務境界を曖昧にするリスクが大きいためである
- root workspace は当面 `crates/rsshogi-usi` のみを維持し、public API の重心を command model / parser / formatter / transcript test に置く
- parser permissiveness、formatter canonicalization、canonical line contract は引き続き protocol crate の責務とし、session 層が再定義しない
- 将来 `session-core` を検討する admission criteria は次の 4 点とする
- 1) 2 consumer 以上で同じ lifecycle / cancellation contract が確認できる
- 2) その契約が I/O 非依存で表現できる
- 3) `PositionSpec + moves` 適用ヘルパや cancelable job shell のように engine 固有性を持ち込まずに切り出せる
- 4) `publish = false` の雛形から始めても protocol crate の責務を侵食しない

## Consumer Adapter Validation (0008)

- `0008` では session 共通化の前に、evidence を「この repo だけで確認できるもの」と「実 consumer 試験移植でしか確認できないもの」に分けて扱う
- repo 内で確認できるのは、`rsshogi-usi` の command model が `position`, `go`, `stop`, `isready`, `usinewgame`, `bestmove`, `checkmate` を表現できることと、parser / formatter / transcript test で canonical line contract が固定されていることまでである
- consumer 実地で確認すべきなのは、`go -> go` の cancel sequencing、`stop` / `isready` / `usinewgame` の cancel・join・ready 復帰、`bestmove` / `checkmate` の exactly-once emission、warning 文言や fallback policy である
- `session-core` を再検討する条件は、2 consumer 以上で `idle -> go`, `searching -> stop`, `searching -> go`, `idle -> isready/usinewgame` の transition matrix が一致し、その共通部分を I/O 非依存 helper として書けることである
- exclusion list も同時に固定する。`bestmove` / `checkmate` payload、warning 文言、時間管理、ponder、mate-only flow は adapter 側へ残し、protocol crate の transcript contract を session 側へ移さない

## Adapter Guidance (0009)

- `0009` では `session-core` の代わりに、adapter 導入の見本を docs / example / transcript で補強する
- `crates/rsshogi-usi/examples/minimal_engine.rs` は reusable helper ではなく、consumer 側が最初に書く最小 loop の参考実装として置く
- `crates/rsshogi-usi/tests/transcripts/valid/adapter-sequences.txt` は stateful harness ではなく、consumer 近似 sequence を line-level transcript contract の上に重ねた参照 fixture とする
- adapter 契約 matrix は shared state machine を固定するための仕様ではなく、protocol crate と consumer adapter の責務分担を見える化するための資料として扱う
- この段階でも protocol crate の責務は変わらない。`parse_line`, `format_command`, canonical transcript は `crates/rsshogi-usi` に残し、position 適用、探索、cancel/join、終端出力 policy は adapter 側に残す
