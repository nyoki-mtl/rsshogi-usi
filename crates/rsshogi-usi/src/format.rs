use crate::parser::{
    BestMove, BestMoveKind, CheckmateResponse, GameResult, GoMate, GoParams, InfoCommand,
    InfoScore, MateScore, PositionSpec, ScoreBound, ScoreValue, UsiCommand, UsiOption,
    UsiOptionKind,
};

#[must_use]
pub fn format_command(command: &UsiCommand) -> String {
    match command {
        UsiCommand::Usi => "usi".to_string(),
        UsiCommand::IsReady => "isready".to_string(),
        UsiCommand::UsiNewGame => "usinewgame".to_string(),
        UsiCommand::Stop => "stop".to_string(),
        UsiCommand::Quit => "quit".to_string(),
        UsiCommand::PonderHit => "ponderhit".to_string(),
        UsiCommand::SetOption { name, value } => format_setoption(name, value.as_deref()),
        UsiCommand::Position { spec, moves } => format_position(spec, moves),
        UsiCommand::Go(params) => format_go(params),
        UsiCommand::GameOver(result) => format_gameover(result),
        UsiCommand::Id { key, value } => format!("id {key} {value}"),
        UsiCommand::Option(option) => format_option(option),
        UsiCommand::UsiOk => "usiok".to_string(),
        UsiCommand::ReadyOk => "readyok".to_string(),
        UsiCommand::BestMove(bestmove) => format_bestmove(bestmove),
        UsiCommand::Info(info) => format_info(info),
        UsiCommand::Checkmate(response) => format_checkmate(response),
        UsiCommand::Extension { name, args } => format_simple_with_args(name, args),
    }
}

fn format_setoption(name: &str, value: Option<&str>) -> String {
    let mut out = format!("setoption name {name}");
    if let Some(value) = value {
        out.push_str(" value");
        if !value.is_empty() {
            out.push(' ');
            out.push_str(value);
        }
    }
    out
}

fn format_position(spec: &PositionSpec, moves: &[String]) -> String {
    let mut out = match spec {
        PositionSpec::StartPos => "position startpos".to_string(),
        PositionSpec::Sfen { board, side_to_move, hands, ply } => {
            format!("position sfen {board} {side_to_move} {hands} {ply}")
        }
    };

    if !moves.is_empty() {
        out.push_str(" moves ");
        out.push_str(&moves.join(" "));
    }

    out
}

fn format_go(params: &GoParams) -> String {
    let mut parts = vec!["go".to_string()];
    if params.ponder {
        parts.push("ponder".to_string());
    }
    push_opt_u64(&mut parts, "btime", params.btime);
    push_opt_u64(&mut parts, "wtime", params.wtime);
    push_opt_u64(&mut parts, "byoyomi", params.byoyomi);
    push_opt_u64(&mut parts, "binc", params.binc);
    push_opt_u64(&mut parts, "winc", params.winc);
    push_opt_u64(&mut parts, "movetime", params.movetime);
    push_opt_u64(&mut parts, "movestogo", params.movestogo);
    push_opt_u32(&mut parts, "depth", params.depth);
    push_opt_u64(&mut parts, "nodes", params.nodes);
    push_go_mate(&mut parts, params.mate.as_ref());
    if params.infinite {
        parts.push("infinite".to_string());
    }
    if !params.searchmoves.is_empty() {
        parts.push("searchmoves".to_string());
        parts.extend(params.searchmoves.iter().cloned());
    }
    parts.extend(params.extras.iter().cloned());
    parts.join(" ")
}

fn format_gameover(result: &GameResult) -> String {
    let result = match result {
        GameResult::Win => "win",
        GameResult::Lose => "lose",
        GameResult::Draw => "draw",
        GameResult::Other(other) => other,
    };

    format!("gameover {result}")
}

fn format_option(option: &UsiOption) -> String {
    let mut parts =
        vec!["option".to_string(), "name".to_string(), option.name.clone(), "type".to_string()];
    parts.push(format_option_kind(&option.kind).to_string());

    if let Some(default) = option.default.as_deref() {
        parts.push("default".to_string());
        parts.push(format_option_default(&option.kind, default));
    }
    if let Some(min) = option.min {
        parts.push("min".to_string());
        parts.push(min.to_string());
    }
    if let Some(max) = option.max {
        parts.push("max".to_string());
        parts.push(max.to_string());
    }
    for value in &option.vars {
        parts.push("var".to_string());
        parts.push(value.clone());
    }
    parts.extend(option.extras.iter().cloned());
    parts.join(" ")
}

fn format_option_default(kind: &UsiOptionKind, default: &str) -> String {
    if matches!(kind, UsiOptionKind::String | UsiOptionKind::Filename) && default.is_empty() {
        "<empty>".to_string()
    } else {
        default.to_string()
    }
}

fn format_bestmove(bestmove: &BestMove) -> String {
    let mut out = format!("bestmove {}", format_bestmove_kind(&bestmove.bestmove));
    if let Some(ponder) = bestmove.ponder.as_deref() {
        out.push_str(" ponder ");
        out.push_str(ponder);
    }
    out
}

fn format_info(info: &InfoCommand) -> String {
    let mut parts = vec!["info".to_string()];
    push_opt_u32(&mut parts, "depth", info.depth);
    push_opt_u32(&mut parts, "seldepth", info.seldepth);
    push_opt_u64(&mut parts, "time", info.time);
    push_opt_u64(&mut parts, "nodes", info.nodes);
    push_opt_u64(&mut parts, "nps", info.nps);
    push_opt_u32(&mut parts, "hashfull", info.hashfull);
    push_opt_u32(&mut parts, "multipv", info.multipv);
    push_info_score(&mut parts, info.score.as_ref());
    if let Some(currmove) = info.currmove.as_deref() {
        parts.push("currmove".to_string());
        parts.push(currmove.to_string());
    }
    parts.extend(info.extras.iter().cloned());
    if !info.pv.is_empty() {
        parts.push("pv".to_string());
        parts.extend(info.pv.iter().cloned());
    } else if let Some(string) = info.string.as_deref() {
        parts.push("string".to_string());
        if !string.is_empty() {
            parts.push(string.to_string());
        }
    }
    parts.join(" ")
}

fn format_checkmate(response: &CheckmateResponse) -> String {
    match response {
        CheckmateResponse::Moves(moves) => format_simple_with_args("checkmate", moves),
        CheckmateResponse::NotImplemented => "checkmate notimplemented".to_string(),
        CheckmateResponse::Timeout => "checkmate timeout".to_string(),
        CheckmateResponse::NoMate => "checkmate nomate".to_string(),
    }
}

fn format_option_kind(kind: &UsiOptionKind) -> &str {
    match kind {
        UsiOptionKind::Check => "check",
        UsiOptionKind::Spin => "spin",
        UsiOptionKind::Combo => "combo",
        UsiOptionKind::Button => "button",
        UsiOptionKind::String => "string",
        UsiOptionKind::Filename => "filename",
        UsiOptionKind::Other(other) => other,
    }
}

fn format_bestmove_kind(kind: &BestMoveKind) -> &str {
    match kind {
        BestMoveKind::Move(mv) => mv,
        BestMoveKind::Resign => "resign",
        BestMoveKind::Win => "win",
    }
}

fn push_opt_u64(parts: &mut Vec<String>, name: &str, value: Option<u64>) {
    if let Some(value) = value {
        parts.push(name.to_string());
        parts.push(value.to_string());
    }
}

fn push_opt_u32(parts: &mut Vec<String>, name: &str, value: Option<u32>) {
    if let Some(value) = value {
        parts.push(name.to_string());
        parts.push(value.to_string());
    }
}

fn push_go_mate(parts: &mut Vec<String>, value: Option<&GoMate>) {
    if let Some(value) = value {
        parts.push("mate".to_string());
        match value {
            GoMate::Ply(ply) => parts.push(ply.to_string()),
            GoMate::Infinite => parts.push("infinite".to_string()),
        }
    }
}

fn push_info_score(parts: &mut Vec<String>, score: Option<&InfoScore>) {
    let Some(score) = score else {
        return;
    };

    parts.push("score".to_string());
    match &score.value {
        ScoreValue::Cp(cp) => {
            parts.push("cp".to_string());
            parts.push(cp.to_string());
        }
        ScoreValue::Mate(mate) => {
            parts.push("mate".to_string());
            match mate {
                MateScore::Ply(ply) => parts.push(ply.to_string()),
                MateScore::UnknownWin => parts.push("+".to_string()),
                MateScore::UnknownLose => parts.push("-".to_string()),
            }
        }
    }

    match score.bound {
        Some(ScoreBound::Lower) => parts.push("lowerbound".to_string()),
        Some(ScoreBound::Upper) => parts.push("upperbound".to_string()),
        None => {}
    }
}

fn format_simple_with_args(head: &str, args: &[String]) -> String {
    if args.is_empty() {
        head.to_string()
    } else {
        format!("{head} {}", args.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::{
        BestMove, CheckmateResponse, GoMate, GoParams, InfoCommand, InfoScore, MateScore,
        UsiCommand, UsiOption, UsiOptionKind,
    };

    use super::format_command;

    #[test]
    fn formats_go_command() {
        let command = UsiCommand::Go(GoParams {
            ponder: true,
            btime: Some(1_000),
            wtime: Some(2_000),
            mate: Some(GoMate::Ply(3)),
            searchmoves: vec!["7g7f".to_string()],
            ..GoParams::default()
        });
        assert_eq!(
            format_command(&command),
            "go ponder btime 1000 wtime 2000 mate 3 searchmoves 7g7f"
        );
    }

    #[test]
    fn formats_go_mate_infinite() {
        let command =
            UsiCommand::Go(GoParams { mate: Some(GoMate::Infinite), ..GoParams::default() });
        assert_eq!(format_command(&command), "go mate infinite");
    }

    #[test]
    fn formats_bestmove_with_ponder() {
        let command = UsiCommand::bestmove(BestMove::move_to("8h2b+").with_ponder("3a2b"));
        assert_eq!(format_command(&command), "bestmove 8h2b+ ponder 3a2b");
    }

    #[test]
    fn formats_option_with_defaults() {
        let command = UsiCommand::Option(UsiOption {
            name: "Style".to_string(),
            kind: UsiOptionKind::Combo,
            default: Some("Normal".to_string()),
            min: None,
            max: None,
            vars: vec!["Solid".to_string(), "Normal".to_string(), "Risky".to_string()],
            extras: Vec::new(),
        });
        assert_eq!(
            format_command(&command),
            "option name Style type combo default Normal var Solid var Normal var Risky"
        );
    }

    #[test]
    fn formats_empty_filename_default_as_empty_sentinel() {
        let command = UsiCommand::Option(UsiOption::filename("EvalFile", ""));
        assert_eq!(format_command(&command), "option name EvalFile type filename default <empty>");
    }

    #[test]
    fn formats_info_score_and_pv() {
        let command = UsiCommand::info(
            InfoCommand::new()
                .with_depth(12)
                .with_nodes(1_024)
                .with_score(InfoScore::mate(MateScore::unknown_win()).with_lowerbound())
                .with_pv(["7g7f", "3c3d"]),
        );
        assert_eq!(
            format_command(&command),
            "info depth 12 nodes 1024 score mate + lowerbound pv 7g7f 3c3d"
        );
    }

    #[test]
    fn formats_info_without_pv_or_string() {
        let command = UsiCommand::info(
            InfoCommand::new()
                .with_hashfull(104)
                .with_multipv_usize(2)
                .with_score(InfoScore::cp(-34).with_upperbound()),
        );
        assert_eq!(format_command(&command), "info hashfull 104 multipv 2 score cp -34 upperbound");
    }

    #[test]
    fn formats_checkmate_timeout() {
        let command = UsiCommand::Checkmate(CheckmateResponse::Timeout);
        assert_eq!(format_command(&command), "checkmate timeout");
    }
}
