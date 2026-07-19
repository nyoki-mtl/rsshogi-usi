# USI インターフェース

USI（Universal Shogi Interface）は、将棋エンジンとGUIの間の通信プロトコルです。
チェスのUCI（Universal Chess Interface）を将棋向けに拡張したもので、Tord Romstad氏によって設計されました。

## Adapter 実装ガイド

`rsshogi-usi` の役割は session 実装の共通化ではなく、USI line surface の parse / format を安定して共有することです。consumer 側では 1 行ずつ `parse_line` し、`UsiCommand` を `match` で adapter 固有ロジックへ振り分け、GUI に返す command だけ `format_command` します。

```rust
use rsshogi_usi::{format_command, parse_line, UsiCommand};

fn handle_line(line: &str) -> Result<Vec<String>, rsshogi_usi::ParseError> {
    let command = parse_line(line)?;

    let responses = match command {
        UsiCommand::Usi => vec![UsiCommand::UsiOk],
        UsiCommand::IsReady => vec![UsiCommand::ReadyOk],
        UsiCommand::Position { .. } => Vec::new(),
        UsiCommand::Go(_) => Vec::new(),
        UsiCommand::Quit => Vec::new(),
        _ => Vec::new(),
    };

    Ok(responses.iter().map(format_command).collect())
}
```

標準 command の向きだけを薄く振り分けたい場合は、`UsiCommand::direction()` / `is_gui_to_engine()` / `is_engine_to_gui()` も使えます。`Extension` は generic surface を維持するため direction を固定せず、`direction()` は `None` を返します。

このパターンでは、crate と adapter の境界を次のように分けます。

- crate が担当するもの: `UsiCommand`、`GoParams`、`InfoCommand` などの command model と parser / formatter、および transcript による canonical line contract
- adapter が担当するもの: stdin/stdout loop、option の副作用、局面への反映、探索 task、cancel / join、warning 文言、`bestmove` / `checkmate` の payload と emission policy

実際の最小 loop は `crates/rsshogi-usi/examples/minimal_engine.rs` を参照してください。この example は reusable helper ではなく、consumer 側が最初に置く adapter stub の見本です。

canonical な transcript 契約を固定したい場合は `parse_line_strict` を使います。これは permissive parser で一度解釈した後、`format_command` の canonical 出力と一致する入力だけを受け付けます。downstream の fixture test では `assert_valid_transcript` / `assert_invalid_transcript` を使うと、repo 本体と同じ `=>` 形式の transcript をそのまま回せます。

```rust
use rsshogi_usi::{
    assert_invalid_transcript, assert_valid_transcript, format_command, parse_line,
    parse_line_strict,
};

fn validate_fixture(content: &str) {
    assert_valid_transcript(content, parse_line, format_command).unwrap();
}

fn validate_canonical_fixture(content: &str) {
    assert_valid_transcript(content, parse_line_strict, format_command).unwrap();
}

fn validate_rejected_fixture(content: &str) {
    assert_invalid_transcript(content, parse_line_strict).unwrap();
}
```

## Adapter 契約 Matrix

この matrix は shared state machine の仕様ではなく、どこまでが protocol crate の責務で、どこからが consumer adapter の責務かを確認するための一覧です。

| Sequence | `rsshogi-usi` が提供するもの | adapter が決めるもの |
| --- | --- | --- |
| `usi -> id/option/usiok` | `usi`, `id`, `option`, `usiok` の parse / format | engine 名・author・option catalog・実際に返す line の組み立て |
| `setoption` | `name` / `value` surface の parse / format | 値検証、state 更新、重い初期化の予約、warning policy |
| `position` | `PositionSpec` と `moves` の parse / format | 盤面構築、合法性確認、差し手適用、内部 state 反映 |
| `idle -> go` | `GoParams` の parse / canonical format | position 前提条件、探索開始、時間管理、最初の `info` / 終端 line |
| `searching -> stop` | `stop` の parse / format | cancel / join、ready 復帰、`bestmove` or `checkmate` の方針 |
| `searching -> go` | 新しい `go` の parse / format | 既存探索の停止順序、exactly-once emission、再開 sequencing |
| `idle -> isready / usinewgame` | `isready`, `usinewgame`, `readyok` の parse / format | pending work の flush、cache reset、探索 worker の待機保証 |
| `go mate` / `checkmate` | `GoParams::mate` と `checkmate` response の parse / format | mate-only flow、non-mate `go` の扱い、`checkmate` payload と fallback |

## Sequence Transcript

consumer に近い command sequence は `crates/rsshogi-usi/tests/transcripts/valid/adapter-sequences.txt` に追加してあります。この fixture は session 実装の正しさを証明するものではなく、line-level transcript contract を保ったまま典型的な導入フローを読めるようにするためのものです。

含めている sequence は次の 3 系統です。

- `usi -> setoption -> isready -> readyok` の起動ハンドシェイク
- `usinewgame -> position -> go -> info -> bestmove` の通常探索の最小パターン
- `position -> go mate infinite -> checkmate notimplemented` や `go infinite -> stop -> bestmove` のような adapter 側判断が絡む分岐

## USI プロトコルの概要

### 設計思想

USI プロトコルは以下の原則に基づいて設計されています：

- **テキストベース通信**: 標準入出力を使ったシンプルなコマンド形式
- **非同期性**: エンジンは思考中でもコマンドを受信できる必要がある
- **拡張性**: エンジン固有のオプションを定義可能
- **SFEN形式**: 盤面と指し手の表現に SFEN を使用

### プロトコルの標準化状況

重要な点として、USI プロトコルには**公式な標準化団体が存在しません**。

実質的な標準は「将棋所」GUIの実装となっています。
そのため、エンジン開発では将棋所との互換性確認が重要です。

### 主なコマンド分類

USI プロトコルのコマンドは以下のカテゴリに分類されます：

1. **初期化コマンド**: `usi`, `setoption`, `isready`
2. **対局制御コマンド**: `position`, `go`, `stop`
3. **情報通知コマンド**: `info`, `bestmove`
4. **拡張コマンド**: `usinewgame`, `gameover`, `quit`

## プロトコルフロー

### 起動シーケンス

エンジン起動時の典型的なコマンド流れ：

```
GUI → Engine: usi
Engine → GUI: id name rshogi-nnue 1.0
Engine → GUI: id author YourName
Engine → GUI: option name Hash type spin default 256 min 1 max 65536
Engine → GUI: option name Threads type spin default 1 min 1 max 512
Engine → GUI: option name USI_Ponder type check default false
Engine → GUI: usiok

GUI → Engine: setoption name Hash value 1024
GUI → Engine: setoption name Threads value 4

GUI → Engine: isready
Engine → GUI: readyok
```

### コマンド詳細

#### usi コマンド

GUIがエンジンにUSIモードを開始するよう指示します。

```
GUI → Engine: usi
```

エンジンの応答：

```rust
// エンジンの実装例
fn handle_usi_command(&self) {
    println!("id name rshogi-nnue 1.0");
    println!("id author YourName");

    // オプションの定義
    println!("option name Hash type spin default 256 min 1 max 65536");
    println!("option name Threads type spin default 1 min 1 max 512");
    println!("option name USI_Ponder type check default false");
    println!("option name MaxMovesToDraw type spin default 256 min 0 max 1000");

    println!("usiok");
}
```

#### オプションの種類

USI プロトコルでは、以下のオプションタイプが定義されています：

| タイプ | 説明 | 例 |
|--------|------|-----|
| `check` | チェックボックス（true/false） | `USI_Ponder` |
| `spin` | 整数値（min/max指定） | `Hash`, `Threads` |
| `combo` | 選択肢から1つ選択 | `BookFile` |
| `button` | ボタン（値なし） | `ClearHash` |
| `string` | 任意の文字列 | `BookFile` のパス |
| `filename` | ファイルパス | 評価関数ファイル |

ShogiHome などの GUI 実装では option 名を単一 token として扱う前提が強いため、`Clear Hash` のように空白を含む名前は避けるのが安全です。`rsshogi-usi` では送信前に `validate_portable_command` を使うと、この種の移植性問題を先に検出できます。

実装例：

```rust
pub enum OptionType {
    Check { default: bool },
    Spin { default: i32, min: i32, max: i32 },
    Combo { default: String, options: Vec<String> },
    Button,
    String { default: String },
    Filename { default: String },
}

impl OptionType {
    pub fn to_usi_string(&self, name: &str) -> String {
        match self {
            OptionType::Check { default } => {
                format!("option name {} type check default {}", name, default)
            }
            OptionType::Spin { default, min, max } => {
                format!(
                    "option name {} type spin default {} min {} max {}",
                    name, default, min, max
                )
            }
            OptionType::Combo { default, options } => {
                let vars = options.join(" var ");
                format!(
                    "option name {} type combo default {} var {}",
                    name, default, vars
                )
            }
            OptionType::Button => {
                format!("option name {} type button", name)
            }
            OptionType::String { default } => {
                let default = if default.is_empty() { "<empty>" } else { default };
                format!("option name {} type string default {}", name, default)
            }
            OptionType::Filename { default } => {
                let default = if default.is_empty() { "<empty>" } else { default };
                format!("option name {} type filename default {}", name, default)
            }
        }
    }
}
```

#### setoption コマンド

GUIがエンジンのオプションを設定します：

```
GUI → Engine: setoption name Hash value 1024
GUI → Engine: setoption name Threads value 4
GUI → Engine: setoption name USI_Ponder value true
```

実装例：

```rust
fn handle_setoption(&mut self, name: &str, value: Option<&str>) {
    match name {
        "Hash" => {
            if let Some(val) = value {
                if let Ok(size) = val.parse::<usize>() {
                    self.transposition_table.resize(size);
                }
            }
        }
        "Threads" => {
            if let Some(val) = value {
                if let Ok(threads) = val.parse::<usize>() {
                    self.thread_pool.resize(threads);
                }
            }
        }
        "USI_Ponder" => {
            if let Some(val) = value {
                self.ponder_enabled = val == "true";
            }
        }
        "MaxMovesToDraw" => {
            if let Some(val) = value {
                if let Ok(moves) = val.parse::<usize>() {
                    self.max_moves_to_draw = moves;
                }
            }
        }
        _ => {
            eprintln!("Unknown option: {}", name);
        }
    }
}
```

#### isready / readyok コマンド

GUIがエンジンの準備完了を確認します：

```
GUI → Engine: isready
Engine → GUI: readyok
```

このコマンドは、`setoption` の適用が完了したことを確認するために使用されます。

実装例：

```rust
fn handle_isready(&mut self) {
    // 保留中の初期化処理を完了
    self.apply_pending_options();

    // 評価関数のロード（まだの場合）
    if !self.eval_loaded {
        self.load_evaluation_function();
        self.eval_loaded = true;
    }

    println!("readyok");
}
```

### 対局の流れ

#### usinewgame コマンド（オプション）

新しい対局の開始を通知します：

```
GUI → Engine: usinewgame
```

このコマンドは、エンジンが内部状態をリセットする機会を提供します：

```rust
fn handle_usinewgame(&mut self) {
    // 置換表をクリア
    self.transposition_table.clear();

    // 履歴情報をリセット
    self.history.clear();

    // その他の学習データをリセット
    self.killer_moves.clear();
}
```

#### position コマンド

現在の局面を設定します：

```
# 平手初期局面
GUI → Engine: position startpos

# 平手初期局面から指し手を適用
GUI → Engine: position startpos moves 7g7f 3c3d

# SFEN で指定
GUI → Engine: position sfen lnsgkgsnl/1r5b1/ppppppppp/9/9/9/PPPPPPPPP/1B5R1/LNSGKGSNL b - 1

# SFEN + 指し手
GUI → Engine: position sfen lnsgkgsnl/1r5b1/ppppppppp/9/9/9/PPPPPPPPP/1B5R1/LNSGKGSNL b - 1 moves 7g7f
```

実装例：

```rust
fn handle_position(&mut self, command: &str) {
    let parts: Vec<&str> = command.split_whitespace().collect();

    if parts.len() < 2 {
        return;  // エラー: 引数不足
    }

    // 初期局面を設定
    if parts[1] == "startpos" {
        self.position = Position::startpos();
    } else if parts[1] == "sfen" {
        // SFEN から局面を構築
        let sfen_end = parts.iter().position(|&s| s == "moves").unwrap_or(parts.len());
        let sfen = parts[2..sfen_end].join(" ");
        self.position = Position::from_sfen(&sfen);
    }

    // 指し手を適用
    if let Some(moves_idx) = parts.iter().position(|&s| s == "moves") {
        for move_str in &parts[moves_idx + 1..] {
            if let Ok(mv) = Move::from_usi(move_str) {
                let mut state = StateInfo::new();
                self.position.do_move(mv, &mut state);
            }
        }
    }
}
```

#### go コマンド

エンジンに思考開始を指示します：

```
# 時間制限（黒番300秒、白番300秒、秒読み10秒）
GUI → Engine: go btime 300000 wtime 300000 byoyomi 10000

# 深さ制限
GUI → Engine: go depth 10

# ノード数制限
GUI → Engine: go nodes 1000000

# 無限思考（stopまで）
GUI → Engine: go infinite

# 詰将棋探索
GUI → Engine: go mate 11

# Ponder（相手の手番で予測思考）
GUI → Engine: go ponder
```

実装例：

```rust
fn handle_go(&mut self, command: &str) {
    let parts: Vec<&str> = command.split_whitespace().collect();

    let mut limits = SearchLimits::default();

    let mut i = 1;
    while i < parts.len() {
        match parts[i] {
            "btime" => {
                if i + 1 < parts.len() {
                    limits.btime = parts[i + 1].parse().ok();
                    i += 2;
                }
            }
            "wtime" => {
                if i + 1 < parts.len() {
                    limits.wtime = parts[i + 1].parse().ok();
                    i += 2;
                }
            }
            "byoyomi" => {
                if i + 1 < parts.len() {
                    limits.byoyomi = parts[i + 1].parse().ok();
                    i += 2;
                }
            }
            "binc" => {
                if i + 1 < parts.len() {
                    limits.binc = parts[i + 1].parse().ok();
                    i += 2;
                }
            }
            "winc" => {
                if i + 1 < parts.len() {
                    limits.winc = parts[i + 1].parse().ok();
                    i += 2;
                }
            }
            "depth" => {
                if i + 1 < parts.len() {
                    limits.depth = parts[i + 1].parse().ok();
                    i += 2;
                }
            }
            "nodes" => {
                if i + 1 < parts.len() {
                    limits.nodes = parts[i + 1].parse().ok();
                    i += 2;
                }
            }
            "mate" => {
                if i + 1 < parts.len() {
                    limits.mate = parts[i + 1].parse().ok();
                    i += 2;
                }
            }
            "infinite" => {
                limits.infinite = true;
                i += 1;
            }
            "ponder" => {
                limits.ponder = true;
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }

    // 探索を開始（別スレッドで）
    self.start_search(limits);
}
```

#### info コマンド

エンジンが探索中の情報をGUIに送信します：

```
Engine → GUI: info depth 5 seldepth 8 score cp 123 nodes 12345 nps 50000 time 247 pv 7g7f 3c3d
Engine → GUI: info string 置換表使用率: 45%
```

info コマンドのフィールド：

| フィールド | 説明 | 例 |
|-----------|------|-----|
| `depth` | 現在の探索深さ | `depth 10` |
| `seldepth` | 選択的探索の最大深さ | `seldepth 15` |
| `score cp` | 評価値（センチポーン） | `score cp 123` |
| `score mate` | 詰みまでの手数 | `score mate 5` |
| `nodes` | 探索ノード数 | `nodes 1234567` |
| `nps` | 1秒あたりのノード数 | `nps 500000` |
| `time` | 思考時間（ミリ秒） | `time 5000` |
| `pv` | 読み筋（Principal Variation） | `pv 7g7f 3c3d 2g2f` |
| `currmove` | 現在探索中の手 | `currmove 7g7f` |
| `hashfull` | 置換表使用率（‰） | `hashfull 450` |
| `string` | 任意の文字列 | `string デバッグ情報` |

実装例：

```rust
fn send_info(&self, info: &SearchInfo) {
    let mut output = String::from("info");

    if let Some(depth) = info.depth {
        output.push_str(&format!(" depth {}", depth));
    }

    if let Some(seldepth) = info.seldepth {
        output.push_str(&format!(" seldepth {}", seldepth));
    }

    if let Some(score) = info.score {
        if score.is_mate() {
            output.push_str(&format!(" score mate {}", score.mate_in()));
        } else {
            output.push_str(&format!(" score cp {}", score.centipawns()));
        }
    }

    if let Some(nodes) = info.nodes {
        output.push_str(&format!(" nodes {}", nodes));
    }

    if let Some(nps) = info.nps {
        output.push_str(&format!(" nps {}", nps));
    }

    if let Some(time) = info.time {
        output.push_str(&format!(" time {}", time));
    }

    if let Some(hashfull) = info.hashfull {
        output.push_str(&format!(" hashfull {}", hashfull));
    }

    if !info.pv.is_empty() {
        output.push_str(" pv");
        for mv in &info.pv {
            output.push_str(&format!(" {}", mv.to_usi()));
        }
    }

    println!("{}", output);
}
```

#### bestmove コマンド

エンジンが最善手を返します：

```
# 通常の指し手
Engine → GUI: bestmove 7g7f

# Ponder付き（次の予測手）
Engine → GUI: bestmove 7g7f ponder 3c3d

# 投了
Engine → GUI: bestmove resign

# 勝ち宣言
Engine → GUI: bestmove win
```

実装例：

```rust
fn send_bestmove(&self, best: Move, ponder: Option<Move>) {
    if best == Move::RESIGN {
        println!("bestmove resign");
    } else if best == Move::WIN {
        println!("bestmove win");
    } else {
        let mut output = format!("bestmove {}", best.to_usi());

        if let Some(ponder_move) = ponder {
            output.push_str(&format!(" ponder {}", ponder_move.to_usi()));
        }

        println!("{}", output);
    }
}
```

#### stop コマンド

思考を中断します：

```
GUI → Engine: stop
Engine → GUI: bestmove 7g7f
```

実装例：

```rust
fn handle_stop(&mut self) {
    // 探索スレッドに停止フラグを立てる
    self.search_stopped.store(true, Ordering::Relaxed);

    // 探索スレッドが終了するのを待つ
    // （実際には別スレッドで非同期に bestmove を返す）
}
```

### その他のコマンド

#### quit コマンド

エンジンを終了します：

```
GUI → Engine: quit
```

実装例：

```rust
fn handle_quit(&mut self) {
    // 探索を停止
    self.handle_stop();

    // リソースを解放
    self.cleanup();

    // プロセスを終了
    std::process::exit(0);
}
```

#### gameover コマンド

対局の終了を通知します（拡張コマンド）：

```
GUI → Engine: gameover win
GUI → Engine: gameover lose
GUI → Engine: gameover draw
```

このコマンドは、学習機能を持つエンジンが対局結果を記録するために使用できます：

```rust
fn handle_gameover(&mut self, result: &str) {
    match result {
        "win" => {
            // 勝ちの棋譜を学習データに追加
            self.learning.record_game(GameResult::Win, &self.game_record);
        }
        "lose" => {
            // 負けの棋譜を学習データに追加
            self.learning.record_game(GameResult::Lose, &self.game_record);
        }
        "draw" => {
            // 引き分けの棋譜を学習データに追加
            self.learning.record_game(GameResult::Draw, &self.game_record);
        }
        _ => {}
    }
}
```

## 実装上の注意点

### 非同期処理の必要性

USI プロトコルでは、**エンジンは思考中でもコマンドを受信できる必要があります**。

そのため、以下のような設計が一般的です：

```rust
pub struct UsiEngine {
    position: Position,
    search_thread: Option<JoinHandle<()>>,
    stop_flag: Arc<AtomicBool>,
}

impl UsiEngine {
    pub fn start_search(&mut self, limits: SearchLimits) {
        let pos = self.position.clone();
        let stop_flag = self.stop_flag.clone();

        // 探索を別スレッドで実行
        self.search_thread = Some(thread::spawn(move || {
            let best_move = search(&pos, limits, stop_flag);
            println!("bestmove {}", best_move.to_usi());
        }));
    }

    pub fn stop_search(&mut self) {
        self.stop_flag.store(true, Ordering::Relaxed);

        // 探索スレッドの終了を待つ
        if let Some(thread) = self.search_thread.take() {
            thread.join().unwrap();
        }

        self.stop_flag.store(false, Ordering::Relaxed);
    }
}
```

### 標準エラー出力の活用

デバッグ情報は標準エラー出力（stderr）に出力します：

```rust
eprintln!("Debug: 置換表ヒット率 = {:.2}%", hit_rate * 100.0);
```

これにより、GUI とのプロトコル通信（stdout）を妨げずにデバッグできます。

### エラーハンドリング

不正なコマンドを受信した場合、以下のいずれかの対応を取ります：

1. **無視**: 何もしない（最も安全）
2. **stderr に警告**: `eprintln!("Warning: Unknown command")`
3. **info string で通知**: `println!("info string Error: Invalid command")`

重要なのは、**プロトコル違反でクラッシュしない**ことです。

### 文字エンコーディング

USI プロトコルの文字エンコーディングは**明確に定義されていません**。

実用上は、以下の対応が推奨されます：

- **UTF-8** を基本とする
- BOM（Byte Order Mark）は付けない
- 将棋所との互換性テストを行う

## プロトコル互換性のテスト

### 将棋所でのテスト

実装したエンジンは、必ず将棋所でテストします：

1. **エンジン登録**: 将棋所の「対局 > エンジン管理」でエンジンを登録
2. **初期化テスト**: エンジンが正しく起動し、オプションが表示されるか確認
3. **対局テスト**: 実際に対局させて、正常に動作するか確認
4. **長時間テスト**: 数百局の連続対局で安定性を確認

### テストケース

以下のシナリオをテストします：

- [ ] `usi` → `usiok` の応答
- [ ] オプション設定（`setoption`）の反映
- [ ] `isready` → `readyok` の応答
- [ ] `position startpos` の設定
- [ ] `position sfen ...` の設定
- [ ] `go btime ... wtime ...` での思考
- [ ] `go byoyomi ...` での思考
- [ ] `go infinite` → `stop` の動作
- [ ] `info` の定期的な送信
- [ ] `bestmove` の応答
- [ ] 長時間対局での安定性
- [ ] 不正なコマンドへの耐性

## bestmove 出力ポリシー
rshogi-nnueは`bestmove`で報告する指し手をα境界を更新した候補に限定する。
TTヒットのみで生成された指し手がαを改善しない場合は`bestmove`として返さず置換表への保存に留める。
βカットを発生させた指し手は平均化後のスコアとともに返し`ponder`候補とノード統計の整合性を担保する。
このポリシーにより`bestmove ... ponder ...`出力が安定しログ比較やYaneuraOuとの突合が容易になる。

## デバッグのヒント

### ログファイルの活用

すべての入出力をログファイルに記録します：

```rust
pub struct UsiLogger {
    file: File,
}

impl UsiLogger {
    pub fn log_input(&mut self, line: &str) {
        writeln!(self.file, ">>> {}", line).ok();
    }

    pub fn log_output(&mut self, line: &str) {
        writeln!(self.file, "<<< {}", line).ok();
    }
}
```

### プロトコルバリデータの使用

可能であれば、プロトコルバリデータを使用します。
ただし、USI 用の標準的なバリデータは存在しないため、自作するか、実績のあるエンジンと比較します。

### GUIのログ機能

将棋所などのGUIは、エンジンとの通信ログを保存できます。
これを確認することで、問題を特定できます。

## まとめ

USI プロトコルの実装で重要なポイント：

- **標準入出力**を使ったシンプルなテキストプロトコル
- **非同期処理**が必須（思考中もコマンド受信）
- **将棋所**との互換性が実質的な標準
- **エラー耐性**を持つ実装（不正コマンドでクラッシュしない）
- **デバッグログ**を活用した開発

正しく実装されたUSIエンジンは、様々なGUIで動作し、大会にも参加できます。

## 参考資料

- [USI プロトコル仕様（将棋所版）](https://shogidokoro2.stars.ne.jp/usi.html) - 実質的な標準仕様
- [USIプロトコルの現状調査（2024年）](https://qiita.com/sunfish-shogi/items/3efcd3a727c04ada020d) - 互換性の問題と実装状況
- [やねうら王 USI実装](https://github.com/yaneurao/YaneuraOu) - 実装の参考例
- [UCI プロトコル仕様](http://wbec-ridderkerk.nl/html/UCIProtocol.html) - USI の元となったチェス用プロトコル
