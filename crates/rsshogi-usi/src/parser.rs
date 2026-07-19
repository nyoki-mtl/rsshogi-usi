use crate::error::{
    CanonicalTokenMismatch, ParseError, ParseErrorKind, ParseErrorSite, PortabilityError,
    PortabilityErrorKind,
};
use crate::format::format_command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PositionSpec {
    StartPos,
    Sfen { board: String, side_to_move: String, hands: String, ply: String },
}

const STARTPOS_SFEN_BOARD: &str = "lnsgkgsnl/1r5b1/ppppppppp/9/9/9/PPPPPPPPP/1B5R1/LNSGKGSNL";
const STARTPOS_SFEN_SIDE_TO_MOVE: &str = "b";
const STARTPOS_SFEN_HANDS: &str = "-";
const STARTPOS_SFEN_PLY: &str = "1";

impl PositionSpec {
    #[must_use]
    pub const fn as_sfen_parts(&self) -> (&str, &str, &str, &str) {
        match self {
            Self::StartPos => (
                STARTPOS_SFEN_BOARD,
                STARTPOS_SFEN_SIDE_TO_MOVE,
                STARTPOS_SFEN_HANDS,
                STARTPOS_SFEN_PLY,
            ),
            Self::Sfen { board, side_to_move, hands, ply } => {
                (board.as_str(), side_to_move.as_str(), hands.as_str(), ply.as_str())
            }
        }
    }

    #[must_use]
    pub fn to_sfen(&self) -> String {
        let (board, side_to_move, hands, ply) = self.as_sfen_parts();
        format!("{board} {side_to_move} {hands} {ply}")
    }

    pub fn replay<P, E, StartPosFn, SfenFn, ApplyMoveFn>(
        &self,
        moves: &[String],
        mut build_startpos: StartPosFn,
        mut build_sfen: SfenFn,
        mut apply_move: ApplyMoveFn,
    ) -> Result<P, PositionReplayError<E>>
    where
        StartPosFn: FnMut() -> Result<P, E>,
        SfenFn: FnMut(&str) -> Result<P, E>,
        ApplyMoveFn: FnMut(&mut P, &str) -> Result<(), E>,
    {
        let mut position = match self {
            Self::StartPos => {
                build_startpos().map_err(|source| PositionReplayError::BuildStartPos { source })?
            }
            Self::Sfen { .. } => {
                let sfen = self.to_sfen();
                build_sfen(&sfen)
                    .map_err(|source| PositionReplayError::BuildSfen { sfen, source })?
            }
        };

        for (index, mv) in moves.iter().enumerate() {
            apply_move(&mut position, mv).map_err(|source| PositionReplayError::ApplyMove {
                move_index: index + 1,
                move_text: mv.clone(),
                source,
            })?;
        }

        Ok(position)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PositionReplayError<E> {
    BuildStartPos { source: E },
    BuildSfen { sfen: String, source: E },
    ApplyMove { move_index: usize, move_text: String, source: E },
}

impl<E: std::fmt::Display> std::fmt::Display for PositionReplayError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BuildStartPos { source } => {
                write!(f, "failed to build startpos position: {source}")
            }
            Self::BuildSfen { sfen, source } => {
                write!(f, "failed to build SFEN position `{sfen}`: {source}")
            }
            Self::ApplyMove { move_index, move_text, source } => {
                write!(f, "failed to apply move #{move_index} `{move_text}`: {source}")
            }
        }
    }
}

impl<E> std::error::Error for PositionReplayError<E>
where
    E: std::error::Error + 'static,
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::BuildStartPos { source }
            | Self::BuildSfen { source, .. }
            | Self::ApplyMove { source, .. } => Some(source),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GoMate {
    Ply(u32),
    Infinite,
}

impl GoMate {
    #[must_use]
    pub const fn ply(ply: u32) -> Self {
        Self::Ply(ply)
    }

    #[must_use]
    pub const fn infinite() -> Self {
        Self::Infinite
    }

    #[must_use]
    pub const fn as_ply(&self) -> Option<u32> {
        match self {
            Self::Ply(ply) => Some(*ply),
            Self::Infinite => None,
        }
    }

    #[must_use]
    #[allow(clippy::cast_possible_wrap)]
    pub const fn as_i32_saturating(&self) -> i32 {
        match self {
            Self::Ply(ply) => {
                if *ply > i32::MAX as u32 {
                    i32::MAX
                } else {
                    *ply as i32
                }
            }
            Self::Infinite => i32::MAX,
        }
    }

    #[must_use]
    pub const fn is_infinite(&self) -> bool {
        matches!(self, Self::Infinite)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GoParams {
    pub ponder: bool,
    pub infinite: bool,
    pub btime: Option<u64>,
    pub wtime: Option<u64>,
    pub byoyomi: Option<u64>,
    pub binc: Option<u64>,
    pub winc: Option<u64>,
    pub movetime: Option<u64>,
    pub movestogo: Option<u64>,
    pub depth: Option<u32>,
    pub nodes: Option<u64>,
    pub mate: Option<GoMate>,
    pub searchmoves: Vec<String>,
    pub extras: Vec<String>,
}

impl GoParams {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub const fn mate(mate: GoMate) -> Self {
        Self {
            ponder: false,
            infinite: false,
            btime: None,
            wtime: None,
            byoyomi: None,
            binc: None,
            winc: None,
            movetime: None,
            movestogo: None,
            depth: None,
            nodes: None,
            mate: Some(mate),
            searchmoves: Vec::new(),
            extras: Vec::new(),
        }
    }

    #[must_use]
    pub const fn with_mate(mut self, mate: GoMate) -> Self {
        self.mate = Some(mate);
        self
    }

    #[must_use]
    pub const fn with_mate_ply(self, ply: u32) -> Self {
        self.with_mate(GoMate::ply(ply))
    }

    #[must_use]
    pub const fn with_mate_infinite(self) -> Self {
        self.with_mate(GoMate::infinite())
    }

    #[must_use]
    pub const fn effective_movestogo(&self) -> Option<u64> {
        match self.movestogo {
            Some(0) | None => None,
            Some(movestogo) => Some(movestogo),
        }
    }

    #[must_use]
    pub fn with_searchmove(mut self, mv: impl Into<String>) -> Self {
        self.searchmoves.push(mv.into());
        self
    }

    #[must_use]
    pub fn with_searchmoves<S>(mut self, moves: impl IntoIterator<Item = S>) -> Self
    where
        S: Into<String>,
    {
        self.searchmoves.extend(moves.into_iter().map(Into::into));
        self
    }

    #[must_use]
    pub fn with_searchmoves_display<S>(mut self, moves: impl IntoIterator<Item = S>) -> Self
    where
        S: ToString,
    {
        self.searchmoves.extend(moves.into_iter().map(|mv| mv.to_string()));
        self
    }

    #[must_use]
    pub fn with_extra(mut self, extra: impl Into<String>) -> Self {
        self.extras.push(extra.into());
        self
    }

    #[must_use]
    pub fn with_extras<S>(mut self, extras: impl IntoIterator<Item = S>) -> Self
    where
        S: Into<String>,
    {
        self.extras.extend(extras.into_iter().map(Into::into));
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameResult {
    Win,
    Lose,
    Draw,
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BestMoveKind {
    Move(String),
    Resign,
    Win,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsiCommandDirection {
    GuiToEngine,
    EngineToGui,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BestMove {
    pub bestmove: BestMoveKind,
    pub ponder: Option<String>,
}

impl BestMove {
    #[must_use]
    pub const fn new(bestmove: BestMoveKind) -> Self {
        Self { bestmove, ponder: None }
    }

    #[must_use]
    pub fn move_to(bestmove: impl Into<String>) -> Self {
        Self::new(BestMoveKind::Move(bestmove.into()))
    }

    #[must_use]
    pub const fn resign() -> Self {
        Self::new(BestMoveKind::Resign)
    }

    #[must_use]
    pub const fn win() -> Self {
        Self::new(BestMoveKind::Win)
    }

    #[must_use]
    pub fn with_ponder(mut self, ponder: impl Into<String>) -> Self {
        self.ponder = Some(ponder.into());
        self
    }

    #[must_use]
    pub fn with_optional_ponder<T>(mut self, ponder: Option<T>) -> Self
    where
        T: Into<String>,
    {
        self.ponder = ponder.map(Into::into);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UsiOptionKind {
    Check,
    Spin,
    Combo,
    Button,
    String,
    Filename,
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsiOption {
    pub name: String,
    pub kind: UsiOptionKind,
    pub default: Option<String>,
    pub min: Option<i64>,
    pub max: Option<i64>,
    pub vars: Vec<String>,
    pub extras: Vec<String>,
}

impl UsiOption {
    #[must_use]
    pub fn new(name: impl Into<String>, kind: UsiOptionKind) -> Self {
        Self {
            name: name.into(),
            kind,
            default: None,
            min: None,
            max: None,
            vars: Vec::new(),
            extras: Vec::new(),
        }
    }

    #[must_use]
    pub fn check(name: impl Into<String>, default: bool) -> Self {
        Self::new(name, UsiOptionKind::Check).with_default(default.to_string())
    }

    #[must_use]
    pub fn spin(name: impl Into<String>, default: i64, min: i64, max: i64) -> Self {
        Self::new(name, UsiOptionKind::Spin)
            .with_default(default.to_string())
            .with_min(min)
            .with_max(max)
    }

    #[must_use]
    pub fn combo<S>(
        name: impl Into<String>,
        default: impl Into<String>,
        vars: impl IntoIterator<Item = S>,
    ) -> Self
    where
        S: Into<String>,
    {
        Self::new(name, UsiOptionKind::Combo).with_default(default).with_vars(vars)
    }

    #[must_use]
    pub fn button(name: impl Into<String>) -> Self {
        Self::new(name, UsiOptionKind::Button)
    }

    #[must_use]
    pub fn string(name: impl Into<String>, default: impl Into<String>) -> Self {
        Self::new(name, UsiOptionKind::String)
            .with_default(normalize_empty_option_default(&UsiOptionKind::String, default.into()))
    }

    #[must_use]
    pub fn filename(name: impl Into<String>, default: impl Into<String>) -> Self {
        Self::new(name, UsiOptionKind::Filename)
            .with_default(normalize_empty_option_default(&UsiOptionKind::Filename, default.into()))
    }

    #[must_use]
    pub fn with_default(mut self, default: impl Into<String>) -> Self {
        self.default = Some(default.into());
        self
    }

    #[must_use]
    pub const fn with_min(mut self, min: i64) -> Self {
        self.min = Some(min);
        self
    }

    #[must_use]
    pub const fn with_max(mut self, max: i64) -> Self {
        self.max = Some(max);
        self
    }

    #[must_use]
    pub fn with_var(mut self, value: impl Into<String>) -> Self {
        self.vars.push(value.into());
        self
    }

    #[must_use]
    pub fn with_vars<S>(mut self, vars: impl IntoIterator<Item = S>) -> Self
    where
        S: Into<String>,
    {
        self.vars.extend(vars.into_iter().map(Into::into));
        self
    }

    #[must_use]
    pub fn with_extra(mut self, value: impl Into<String>) -> Self {
        self.extras.push(value.into());
        self
    }

    #[must_use]
    pub fn with_extras<S>(mut self, extras: impl IntoIterator<Item = S>) -> Self
    where
        S: Into<String>,
    {
        self.extras.extend(extras.into_iter().map(Into::into));
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScoreBound {
    Lower,
    Upper,
}

impl ScoreBound {
    #[must_use]
    pub const fn from_flags(lowerbound: bool, upperbound: bool) -> Option<Self> {
        match (lowerbound, upperbound) {
            (true, false) => Some(Self::Lower),
            (false, true) => Some(Self::Upper),
            (false, false) | (true, true) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MateScore {
    Ply(i32),
    UnknownWin,
    UnknownLose,
}

impl MateScore {
    #[must_use]
    pub const fn ply(ply: i32) -> Self {
        Self::Ply(ply)
    }

    #[must_use]
    pub const fn unknown_win() -> Self {
        Self::UnknownWin
    }

    #[must_use]
    pub const fn unknown_lose() -> Self {
        Self::UnknownLose
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScoreValue {
    Cp(i32),
    Mate(MateScore),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InfoScore {
    pub value: ScoreValue,
    pub bound: Option<ScoreBound>,
}

impl InfoScore {
    #[must_use]
    pub const fn new(value: ScoreValue) -> Self {
        Self { value, bound: None }
    }

    #[must_use]
    pub const fn cp(cp: i32) -> Self {
        Self::new(ScoreValue::Cp(cp))
    }

    #[must_use]
    pub const fn mate(mate: MateScore) -> Self {
        Self::new(ScoreValue::Mate(mate))
    }

    #[must_use]
    pub const fn with_bound(mut self, bound: ScoreBound) -> Self {
        self.bound = Some(bound);
        self
    }

    #[must_use]
    pub const fn with_optional_bound(mut self, bound: Option<ScoreBound>) -> Self {
        self.bound = bound;
        self
    }

    #[must_use]
    pub const fn with_bound_flags(self, lowerbound: bool, upperbound: bool) -> Self {
        self.with_optional_bound(ScoreBound::from_flags(lowerbound, upperbound))
    }

    #[must_use]
    pub const fn with_lowerbound(self) -> Self {
        self.with_bound(ScoreBound::Lower)
    }

    #[must_use]
    pub const fn with_upperbound(self) -> Self {
        self.with_bound(ScoreBound::Upper)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InfoCommand {
    pub depth: Option<u32>,
    pub seldepth: Option<u32>,
    pub time: Option<u64>,
    pub nodes: Option<u64>,
    pub multipv: Option<u32>,
    pub score: Option<InfoScore>,
    pub currmove: Option<String>,
    pub hashfull: Option<u32>,
    pub nps: Option<u64>,
    pub pv: Vec<String>,
    pub string: Option<String>,
    pub extras: Vec<String>,
}

impl InfoCommand {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn string(value: impl Into<String>) -> Self {
        Self::new().with_string(value)
    }

    #[must_use]
    pub const fn with_depth(mut self, depth: u32) -> Self {
        self.depth = Some(depth);
        self
    }

    #[must_use]
    pub const fn with_seldepth(mut self, seldepth: u32) -> Self {
        self.seldepth = Some(seldepth);
        self
    }

    #[must_use]
    pub const fn with_time(mut self, time: u64) -> Self {
        self.time = Some(time);
        self
    }

    #[must_use]
    pub const fn with_nodes(mut self, nodes: u64) -> Self {
        self.nodes = Some(nodes);
        self
    }

    #[must_use]
    pub const fn with_nps(mut self, nps: u64) -> Self {
        self.nps = Some(nps);
        self
    }

    #[must_use]
    pub const fn with_hashfull(mut self, hashfull: u32) -> Self {
        self.hashfull = Some(hashfull);
        self
    }

    #[must_use]
    pub const fn with_multipv(mut self, multipv: u32) -> Self {
        self.multipv = Some(multipv);
        self
    }

    #[must_use]
    pub fn with_multipv_usize(mut self, multipv: usize) -> Self {
        self.multipv =
            Some(u32::try_from(multipv).expect("info multipv from usize should fit into u32"));
        self
    }

    #[must_use]
    pub const fn with_score(mut self, score: InfoScore) -> Self {
        self.score = Some(score);
        self
    }

    #[must_use]
    pub const fn with_score_cp(self, cp: i32) -> Self {
        self.with_score(InfoScore::cp(cp))
    }

    #[must_use]
    pub const fn with_score_mate(self, mate: MateScore) -> Self {
        self.with_score(InfoScore::mate(mate))
    }

    #[must_use]
    pub fn with_currmove(mut self, currmove: impl Into<String>) -> Self {
        self.currmove = Some(currmove.into());
        self
    }

    #[must_use]
    pub fn with_pv<S>(mut self, pv: impl IntoIterator<Item = S>) -> Self
    where
        S: Into<String>,
    {
        self.pv = pv.into_iter().map(Into::into).collect();
        self.string = None;
        self
    }

    #[must_use]
    pub fn with_pv_display<S>(mut self, pv: impl IntoIterator<Item = S>) -> Self
    where
        S: ToString,
    {
        self.pv = pv.into_iter().map(|mv| mv.to_string()).collect();
        self.string = None;
        self
    }

    #[must_use]
    pub fn with_string(mut self, value: impl Into<String>) -> Self {
        self.string = Some(value.into());
        self.pv.clear();
        self
    }

    #[must_use]
    pub fn with_extra(mut self, value: impl Into<String>) -> Self {
        self.extras.push(value.into());
        self
    }

    #[must_use]
    pub fn with_extras<S>(mut self, extras: impl IntoIterator<Item = S>) -> Self
    where
        S: Into<String>,
    {
        self.extras.extend(extras.into_iter().map(Into::into));
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckmateResponse {
    Moves(Vec<String>),
    NotImplemented,
    Timeout,
    NoMate,
}

impl CheckmateResponse {
    #[must_use]
    pub fn moves<S>(moves: impl IntoIterator<Item = S>) -> Self
    where
        S: Into<String>,
    {
        Self::Moves(moves.into_iter().map(Into::into).collect())
    }

    #[must_use]
    pub fn moves_display<S>(moves: impl IntoIterator<Item = S>) -> Self
    where
        S: ToString,
    {
        Self::Moves(moves.into_iter().map(|mv| mv.to_string()).collect())
    }

    #[must_use]
    pub const fn not_implemented() -> Self {
        Self::NotImplemented
    }

    #[must_use]
    pub const fn timeout() -> Self {
        Self::Timeout
    }

    #[must_use]
    pub const fn nomate() -> Self {
        Self::NoMate
    }

    #[must_use]
    pub const fn as_moves(&self) -> Option<&[String]> {
        match self {
            Self::Moves(moves) => Some(moves.as_slice()),
            Self::NotImplemented | Self::Timeout | Self::NoMate => None,
        }
    }

    #[must_use]
    pub const fn is_reserved_status(&self) -> bool {
        matches!(self, Self::NotImplemented | Self::Timeout | Self::NoMate)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UsiCommand {
    Usi,
    IsReady,
    UsiNewGame,
    Stop,
    Quit,
    PonderHit,
    SetOption { name: String, value: Option<String> },
    Position { spec: PositionSpec, moves: Vec<String> },
    Go(GoParams),
    GameOver(GameResult),
    Id { key: String, value: String },
    Option(UsiOption),
    UsiOk,
    ReadyOk,
    BestMove(BestMove),
    Info(InfoCommand),
    Checkmate(CheckmateResponse),
    Extension { name: String, args: Vec<String> },
}

impl UsiCommand {
    #[must_use]
    pub const fn direction(&self) -> Option<UsiCommandDirection> {
        match self {
            Self::Usi
            | Self::IsReady
            | Self::UsiNewGame
            | Self::Stop
            | Self::Quit
            | Self::PonderHit
            | Self::SetOption { .. }
            | Self::Position { .. }
            | Self::Go(_)
            | Self::GameOver(_) => Some(UsiCommandDirection::GuiToEngine),
            Self::Id { .. }
            | Self::Option(_)
            | Self::UsiOk
            | Self::ReadyOk
            | Self::BestMove(_)
            | Self::Info(_)
            | Self::Checkmate(_) => Some(UsiCommandDirection::EngineToGui),
            Self::Extension { .. } => None,
        }
    }

    #[must_use]
    pub const fn is_gui_to_engine(&self) -> bool {
        matches!(self.direction(), Some(UsiCommandDirection::GuiToEngine))
    }

    #[must_use]
    pub const fn is_engine_to_gui(&self) -> bool {
        matches!(self.direction(), Some(UsiCommandDirection::EngineToGui))
    }

    #[must_use]
    pub fn id(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self::Id { key: key.into(), value: value.into() }
    }

    #[must_use]
    pub fn id_name(value: impl Into<String>) -> Self {
        Self::id("name", value)
    }

    #[must_use]
    pub fn id_author(value: impl Into<String>) -> Self {
        Self::id("author", value)
    }

    #[must_use]
    pub const fn readyok() -> Self {
        Self::ReadyOk
    }

    #[must_use]
    pub const fn usiok() -> Self {
        Self::UsiOk
    }

    #[must_use]
    pub const fn bestmove(bestmove: BestMove) -> Self {
        Self::BestMove(bestmove)
    }

    #[must_use]
    pub const fn info(info: InfoCommand) -> Self {
        Self::Info(info)
    }

    #[must_use]
    pub fn info_string(value: impl Into<String>) -> Self {
        Self::info(InfoCommand::string(value))
    }

    #[must_use]
    pub const fn go(params: GoParams) -> Self {
        Self::Go(params)
    }

    #[must_use]
    pub const fn go_mate(mate: GoMate) -> Self {
        Self::go(GoParams::mate(mate))
    }

    #[must_use]
    pub const fn go_mate_ply(ply: u32) -> Self {
        Self::go_mate(GoMate::ply(ply))
    }

    #[must_use]
    pub const fn go_mate_infinite() -> Self {
        Self::go_mate(GoMate::infinite())
    }

    #[must_use]
    pub fn ponderhit_with_args<S>(args: impl IntoIterator<Item = S>) -> Self
    where
        S: Into<String>,
    {
        let args = args.into_iter().map(Into::into).collect::<Vec<_>>();
        if args.is_empty() {
            Self::PonderHit
        } else {
            Self::extension("ponderhit", args)
        }
    }

    #[must_use]
    pub const fn checkmate(response: CheckmateResponse) -> Self {
        Self::Checkmate(response)
    }

    #[must_use]
    pub fn checkmate_moves<S>(moves: impl IntoIterator<Item = S>) -> Self
    where
        S: Into<String>,
    {
        Self::checkmate(CheckmateResponse::moves(moves))
    }

    #[must_use]
    pub const fn checkmate_not_implemented() -> Self {
        Self::checkmate(CheckmateResponse::not_implemented())
    }

    #[must_use]
    pub const fn checkmate_timeout() -> Self {
        Self::checkmate(CheckmateResponse::timeout())
    }

    #[must_use]
    pub const fn checkmate_nomate() -> Self {
        Self::checkmate(CheckmateResponse::nomate())
    }

    #[must_use]
    pub fn extension<S>(name: impl Into<String>, args: impl IntoIterator<Item = S>) -> Self
    where
        S: Into<String>,
    {
        Self::Extension { name: name.into(), args: args.into_iter().map(Into::into).collect() }
    }

    #[must_use]
    pub fn is_extension(&self, expected: &str) -> bool {
        matches!(self, Self::Extension { name, .. } if name == expected)
    }

    #[must_use]
    pub const fn as_extension(&self) -> Option<(&str, &[String])> {
        match self {
            Self::Extension { name, args } => Some((name.as_str(), args.as_slice())),
            _ => None,
        }
    }
}

pub fn parse_line(line: &str) -> Result<UsiCommand, ParseError> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Err(ParseError::new(ParseErrorKind::EmptyInput).with_site(ParseErrorSite {
            token_position: 1,
            byte_start: 0,
            byte_end: 0,
            token: None,
        }));
    }

    let tokens = tokenize(trimmed);
    let mut reader = TokenReader::new(&tokens, trimmed.len());
    let Some(command) = reader.next() else {
        return Err(ParseError::new(ParseErrorKind::EmptyInput).with_site(reader.end_site()));
    };

    match command.text {
        "usi" => finish_without_args("usi", &mut reader, UsiCommand::Usi),
        "isready" => finish_without_args("isready", &mut reader, UsiCommand::IsReady),
        "usinewgame" => finish_without_args("usinewgame", &mut reader, UsiCommand::UsiNewGame),
        "stop" => finish_without_args("stop", &mut reader, UsiCommand::Stop),
        "quit" => finish_without_args("quit", &mut reader, UsiCommand::Quit),
        "ponderhit" => {
            if reader.peek().is_some() {
                Ok(parse_extension("ponderhit", &mut reader))
            } else {
                finish_without_args("ponderhit", &mut reader, UsiCommand::PonderHit)
            }
        }
        "usiok" => finish_without_args("usiok", &mut reader, UsiCommand::UsiOk),
        "readyok" => finish_without_args("readyok", &mut reader, UsiCommand::ReadyOk),
        "setoption" => parse_setoption(&mut reader),
        "position" => parse_position(&mut reader),
        "go" => parse_go(&mut reader),
        "gameover" => parse_gameover(&mut reader),
        "id" => parse_id(&mut reader),
        "option" => parse_option(&mut reader),
        "bestmove" => parse_bestmove(&mut reader),
        "info" => parse_info(&mut reader),
        "checkmate" => parse_checkmate(&mut reader),
        "eval" | "test_movegen" | "test_see" => Ok(parse_extension(command.text, &mut reader)),
        _ => Err(ParseError::new(ParseErrorKind::UnknownCommand {
            command: command.text.to_owned(),
        })
        .with_site(command.site())),
    }
}

pub fn parse_line_strict(line: &str) -> Result<UsiCommand, ParseError> {
    let parsed = parse_line(line)?;
    let canonical = format_command(&parsed);
    let trimmed = line.trim();
    if canonical == trimmed {
        Ok(parsed)
    } else {
        let tokens = tokenize(trimmed);
        let mismatch = first_canonical_token_mismatch(trimmed, &canonical);
        let err = ParseError::new(ParseErrorKind::NonCanonical { canonical });
        Err(match mismatch {
            Some(mismatch) => err.with_canonical_token_mismatch(mismatch.clone()).with_site(
                site_for_token_position(&tokens, trimmed.len(), mismatch.token_position),
            ),
            None => err,
        })
    }
}

pub fn validate_portable_command(command: &UsiCommand) -> Result<(), PortabilityError> {
    match command {
        UsiCommand::SetOption { name, .. } => validate_option_name_portability("setoption", name),
        UsiCommand::Option(option) => validate_option_name_portability("option", &option.name),
        _ => Ok(()),
    }
}

fn first_canonical_token_mismatch(input: &str, canonical: &str) -> Option<CanonicalTokenMismatch> {
    let mut input_tokens = input.split_whitespace();
    let mut canonical_tokens = canonical.split_whitespace();
    let mut token_position = 1;

    loop {
        match (input_tokens.next(), canonical_tokens.next()) {
            (None, None) => return None,
            (Some(found), Some(expected)) if found == expected => {
                token_position += 1;
            }
            (found, expected) => {
                return Some(CanonicalTokenMismatch {
                    token_position,
                    expected: expected.map(str::to_owned),
                    found: found.map(str::to_owned),
                })
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TokenSpan<'a> {
    text: &'a str,
    byte_start: usize,
    byte_end: usize,
    token_position: usize,
}

impl TokenSpan<'_> {
    fn site(self) -> ParseErrorSite {
        ParseErrorSite {
            token_position: self.token_position,
            byte_start: self.byte_start,
            byte_end: self.byte_end,
            token: Some(self.text.to_owned()),
        }
    }
}

#[derive(Debug, Clone)]
struct TokenReader<'a> {
    tokens: &'a [TokenSpan<'a>],
    input_len: usize,
    index: usize,
}

impl<'a> TokenReader<'a> {
    const fn new(tokens: &'a [TokenSpan<'a>], input_len: usize) -> Self {
        Self { tokens, input_len, index: 0 }
    }

    fn next(&mut self) -> Option<TokenSpan<'a>> {
        let token = self.tokens.get(self.index).copied();
        if token.is_some() {
            self.index += 1;
        }
        token
    }

    fn peek(&self) -> Option<TokenSpan<'a>> {
        self.tokens.get(self.index).copied()
    }

    const fn end_site(&self) -> ParseErrorSite {
        ParseErrorSite {
            token_position: self.index + 1,
            byte_start: self.input_len,
            byte_end: self.input_len,
            token: None,
        }
    }

    fn missing_argument(&self, context: &'static str) -> ParseError {
        ParseError::new(ParseErrorKind::MissingArgument { context }).with_site(self.end_site())
    }

    fn unexpected_token(context: &'static str, token: TokenSpan<'a>) -> ParseError {
        ParseError::new(ParseErrorKind::UnexpectedToken { context, token: token.text.to_owned() })
            .with_site(token.site())
    }

    fn invalid_value(field: &'static str, token: TokenSpan<'a>) -> ParseError {
        ParseError::new(ParseErrorKind::InvalidValue { field, value: token.text.to_owned() })
            .with_site(token.site())
    }

    fn collect_remaining_strings(&mut self) -> Vec<String> {
        let remaining =
            self.tokens[self.index..].iter().map(|token| token.text.to_owned()).collect();
        self.index = self.tokens.len();
        remaining
    }
}

fn tokenize(input: &str) -> Vec<TokenSpan<'_>> {
    let mut tokens = Vec::new();
    let mut token_start = None;
    let mut token_position = 1;

    for (index, ch) in input.char_indices() {
        if ch.is_whitespace() {
            if let Some(start) = token_start.take() {
                tokens.push(TokenSpan {
                    text: &input[start..index],
                    byte_start: start,
                    byte_end: index,
                    token_position,
                });
                token_position += 1;
            }
        } else if token_start.is_none() {
            token_start = Some(index);
        }
    }

    if let Some(start) = token_start {
        tokens.push(TokenSpan {
            text: &input[start..input.len()],
            byte_start: start,
            byte_end: input.len(),
            token_position,
        });
    }

    tokens
}

fn site_for_token_position(
    tokens: &[TokenSpan<'_>],
    input_len: usize,
    token_position: usize,
) -> ParseErrorSite {
    tokens
        .iter()
        .find(|token| token.token_position == token_position)
        .copied()
        .map(TokenSpan::site)
        .unwrap_or(ParseErrorSite {
            token_position,
            byte_start: input_len,
            byte_end: input_len,
            token: None,
        })
}

fn finish_without_args(
    context: &'static str,
    reader: &mut TokenReader<'_>,
    command: UsiCommand,
) -> Result<UsiCommand, ParseError> {
    if let Some(token) = reader.next() {
        return Err(TokenReader::unexpected_token(context, token));
    }

    Ok(command)
}

fn parse_setoption(reader: &mut TokenReader<'_>) -> Result<UsiCommand, ParseError> {
    match reader.next() {
        Some(token) if token.text == "name" => {}
        Some(token) => return Err(TokenReader::unexpected_token("setoption", token)),
        None => return Err(reader.missing_argument("setoption name <NAME>")),
    }

    let mut name_parts = Vec::new();
    while let Some(token) = reader.next() {
        if token.text == "value" {
            let value = reader.collect_remaining_strings().join(" ");
            return Ok(UsiCommand::SetOption { name: name_parts.join(" "), value: Some(value) });
        }
        name_parts.push(token.text);
    }

    if name_parts.is_empty() {
        return Err(reader.missing_argument("setoption name <NAME>"));
    }

    Ok(UsiCommand::SetOption { name: name_parts.join(" "), value: None })
}

fn parse_position(reader: &mut TokenReader<'_>) -> Result<UsiCommand, ParseError> {
    let kind =
        next_required(reader, "position startpos | position sfen <board> <side> <hands> <ply>")?;

    let spec = match kind.text {
        "startpos" => PositionSpec::StartPos,
        "sfen" => PositionSpec::Sfen {
            board: next_required(reader, "position sfen board")?.text.to_owned(),
            side_to_move: next_required(reader, "position sfen side")?.text.to_owned(),
            hands: next_required(reader, "position sfen hands")?.text.to_owned(),
            ply: next_required(reader, "position sfen ply")?.text.to_owned(),
        },
        _ => return Err(TokenReader::unexpected_token("position", kind)),
    };

    let Some(marker) = reader.next() else {
        return Ok(UsiCommand::Position { spec, moves: Vec::new() });
    };

    if marker.text != "moves" {
        return Err(TokenReader::unexpected_token("position", marker));
    }

    let moves = reader.collect_remaining_strings();
    Ok(UsiCommand::Position { spec, moves })
}

fn parse_go(reader: &mut TokenReader<'_>) -> Result<UsiCommand, ParseError> {
    let mut params = GoParams::default();

    while let Some(token) = reader.next() {
        match token.text {
            "ponder" => params.ponder = true,
            "infinite" => params.infinite = true,
            "btime" => params.btime = Some(parse_u64(reader, "btime")?),
            "wtime" => params.wtime = Some(parse_u64(reader, "wtime")?),
            "byoyomi" => params.byoyomi = Some(parse_u64(reader, "byoyomi")?),
            "binc" => params.binc = Some(parse_u64(reader, "binc")?),
            "winc" => params.winc = Some(parse_u64(reader, "winc")?),
            "movetime" => params.movetime = Some(parse_u64(reader, "movetime")?),
            "movestogo" => params.movestogo = Some(parse_u64(reader, "movestogo")?),
            "depth" => params.depth = Some(parse_u32(reader, "depth")?),
            "nodes" => params.nodes = Some(parse_u64(reader, "nodes")?),
            "mate" => params.mate = Some(parse_go_mate(reader)?),
            "searchmoves" => {
                while let Some(next) = reader.peek() {
                    if is_go_keyword(next.text) {
                        break;
                    }
                    params.searchmoves.push(next.text.to_owned());
                    let _ = reader.next();
                }
            }
            other => params.extras.push(other.to_owned()),
        }
    }

    Ok(UsiCommand::Go(params))
}

fn parse_gameover(reader: &mut TokenReader<'_>) -> Result<UsiCommand, ParseError> {
    let result = match reader.next() {
        Some(token) if token.text == "win" => GameResult::Win,
        Some(token) if token.text == "lose" => GameResult::Lose,
        Some(token) if token.text == "draw" => GameResult::Draw,
        Some(token) => GameResult::Other(token.text.to_owned()),
        None => return Err(reader.missing_argument("gameover <win|lose|draw>")),
    };

    if let Some(token) = reader.next() {
        return Err(TokenReader::unexpected_token("gameover", token));
    }

    Ok(UsiCommand::GameOver(result))
}

fn parse_id(reader: &mut TokenReader<'_>) -> Result<UsiCommand, ParseError> {
    let key = next_required(reader, "id <key> <value>")?.text.to_owned();
    let value = reader.collect_remaining_strings().join(" ");
    if value.is_empty() {
        return Err(reader.missing_argument("id <key> <value>"));
    }

    Ok(UsiCommand::Id { key, value })
}

fn parse_option(reader: &mut TokenReader<'_>) -> Result<UsiCommand, ParseError> {
    expect_keyword(reader, "name", "option")?;
    let name = collect_until_keyword(reader, &["type"]).join(" ");
    if name.is_empty() {
        return Err(reader.missing_argument("option name <NAME> type <TYPE>"));
    }
    expect_keyword(reader, "type", "option")?;

    let kind_token = next_required(reader, "option type")?;
    let kind = parse_option_kind(kind_token.text);

    let mut option = UsiOption {
        name,
        kind,
        default: None,
        min: None,
        max: None,
        vars: Vec::new(),
        extras: Vec::new(),
    };

    while let Some(token) = reader.next() {
        match token.text {
            "default" => {
                option.default = Some(normalize_empty_option_default(
                    &option.kind,
                    collect_required_value(reader, &OPTION_PARAM_KEYWORDS, "option default")?,
                ));
            }
            "min" => option.min = Some(parse_i64(reader, "option min")?),
            "max" => option.max = Some(parse_i64(reader, "option max")?),
            "var" => {
                option.vars.push(collect_required_value(
                    reader,
                    &OPTION_PARAM_KEYWORDS,
                    "option var",
                )?);
            }
            other => option.extras.push(other.to_owned()),
        }
    }

    Ok(UsiCommand::Option(option))
}

fn parse_bestmove(reader: &mut TokenReader<'_>) -> Result<UsiCommand, ParseError> {
    let bestmove = match next_required(reader, "bestmove <move>")?.text {
        "resign" => BestMoveKind::Resign,
        "win" => BestMoveKind::Win,
        mv => BestMoveKind::Move(mv.to_owned()),
    };

    let ponder = match reader.next() {
        Some(token) if token.text == "ponder" => {
            Some(next_required(reader, "bestmove ponder <move>")?.text.to_owned())
        }
        Some(token) => return Err(TokenReader::unexpected_token("bestmove", token)),
        None => None,
    };

    if let Some(token) = reader.next() {
        return Err(TokenReader::unexpected_token("bestmove", token));
    }

    Ok(UsiCommand::BestMove(BestMove { bestmove, ponder }))
}

fn parse_info(reader: &mut TokenReader<'_>) -> Result<UsiCommand, ParseError> {
    let mut info = InfoCommand::default();

    while let Some(token) = reader.next() {
        match token.text {
            "depth" => info.depth = Some(parse_u32(reader, "info depth")?),
            "seldepth" => info.seldepth = Some(parse_u32(reader, "info seldepth")?),
            "time" => info.time = Some(parse_u64(reader, "info time")?),
            "nodes" => info.nodes = Some(parse_u64(reader, "info nodes")?),
            "multipv" => info.multipv = Some(parse_u32(reader, "info multipv")?),
            "score" => info.score = Some(parse_info_score(reader)?),
            "currmove" => {
                info.currmove = Some(next_required(reader, "info currmove")?.text.to_owned());
            }
            "hashfull" => info.hashfull = Some(parse_u32(reader, "info hashfull")?),
            "nps" => info.nps = Some(parse_u64(reader, "info nps")?),
            "pv" => {
                info.pv = reader.collect_remaining_strings();
                break;
            }
            "string" => {
                info.string = Some(reader.collect_remaining_strings().join(" "));
                break;
            }
            other => info.extras.push(other.to_owned()),
        }
    }

    Ok(UsiCommand::Info(info))
}

fn parse_info_score(reader: &mut TokenReader<'_>) -> Result<InfoScore, ParseError> {
    let kind = next_required(reader, "info score")?;
    let value = match kind.text {
        "cp" => ScoreValue::Cp(parse_i32(reader, "info score cp")?),
        "mate" => ScoreValue::Mate(parse_mate_score(reader)?),
        _ => return Err(TokenReader::unexpected_token("info score", kind)),
    };

    let mut bound = None;
    while let Some(token) = reader.peek() {
        match token.text {
            "lowerbound" => {
                bound = Some(ScoreBound::Lower);
                let _ = reader.next();
            }
            "upperbound" => {
                bound = Some(ScoreBound::Upper);
                let _ = reader.next();
            }
            _ => break,
        }
    }

    Ok(InfoScore { value, bound })
}

fn parse_mate_score(reader: &mut TokenReader<'_>) -> Result<MateScore, ParseError> {
    let value = next_required(reader, "info score mate")?;
    match value.text {
        "+" => Ok(MateScore::UnknownWin),
        "-" => Ok(MateScore::UnknownLose),
        _ => value
            .text
            .parse::<i32>()
            .map(MateScore::Ply)
            .map_err(|_| TokenReader::invalid_value("info score mate", value)),
    }
}

fn parse_checkmate(reader: &mut TokenReader<'_>) -> Result<UsiCommand, ParseError> {
    let first = next_required(reader, "checkmate <moves|notimplemented|timeout|nomate>")?;
    let remaining = reader.collect_remaining_strings();
    let response = if remaining.is_empty() {
        match first.text {
            "notimplemented" => CheckmateResponse::NotImplemented,
            "timeout" => CheckmateResponse::Timeout,
            "nomate" => CheckmateResponse::NoMate,
            mv => CheckmateResponse::Moves(vec![mv.to_owned()]),
        }
    } else {
        if matches!(first.text, "notimplemented" | "timeout" | "nomate") {
            return Err(ParseError::new(ParseErrorKind::UnexpectedToken {
                context: "checkmate",
                token: remaining[0].clone(),
            })
            .with_site(site_for_token_position(
                reader.tokens,
                reader.input_len,
                first.token_position + 1,
            )));
        }
        let mut moves = Vec::with_capacity(remaining.len() + 1);
        moves.push(first.text.to_owned());
        moves.extend(remaining);
        CheckmateResponse::Moves(moves)
    };

    Ok(UsiCommand::Checkmate(response))
}

fn parse_extension(name: &str, reader: &mut TokenReader<'_>) -> UsiCommand {
    UsiCommand::extension(name, reader.collect_remaining_strings())
}

fn next_required<'a>(
    reader: &mut TokenReader<'a>,
    context: &'static str,
) -> Result<TokenSpan<'a>, ParseError> {
    reader.next().ok_or_else(|| reader.missing_argument(context))
}

fn expect_keyword(
    reader: &mut TokenReader<'_>,
    expected: &'static str,
    context: &'static str,
) -> Result<(), ParseError> {
    match reader.next() {
        Some(token) if token.text == expected => Ok(()),
        Some(token) => Err(TokenReader::unexpected_token(context, token)),
        None => Err(reader.missing_argument(context)),
    }
}

fn collect_until_keyword(reader: &mut TokenReader<'_>, keywords: &[&str]) -> Vec<String> {
    let mut values = Vec::new();
    while let Some(token) = reader.peek() {
        if keywords.contains(&token.text) {
            break;
        }
        values.push(token.text.to_owned());
        let _ = reader.next();
    }
    values
}

fn collect_required_value(
    reader: &mut TokenReader<'_>,
    keywords: &[&str],
    context: &'static str,
) -> Result<String, ParseError> {
    let value = collect_until_keyword(reader, keywords).join(" ");
    if value.is_empty() {
        return Err(reader.missing_argument(context));
    }
    Ok(value)
}

fn parse_u64(reader: &mut TokenReader<'_>, field: &'static str) -> Result<u64, ParseError> {
    let value = next_required(reader, field)?;
    value.text.parse::<u64>().map_err(|_| TokenReader::invalid_value(field, value))
}

fn parse_u32(reader: &mut TokenReader<'_>, field: &'static str) -> Result<u32, ParseError> {
    let value = next_required(reader, field)?;
    value.text.parse::<u32>().map_err(|_| TokenReader::invalid_value(field, value))
}

fn parse_i32(reader: &mut TokenReader<'_>, field: &'static str) -> Result<i32, ParseError> {
    let value = next_required(reader, field)?;
    value.text.parse::<i32>().map_err(|_| TokenReader::invalid_value(field, value))
}

fn parse_i64(reader: &mut TokenReader<'_>, field: &'static str) -> Result<i64, ParseError> {
    let value = next_required(reader, field)?;
    value.text.parse::<i64>().map_err(|_| TokenReader::invalid_value(field, value))
}

fn parse_go_mate(reader: &mut TokenReader<'_>) -> Result<GoMate, ParseError> {
    let value = next_required(reader, "mate")?;
    if value.text == "infinite" {
        Ok(GoMate::Infinite)
    } else {
        value
            .text
            .parse::<u32>()
            .map(GoMate::Ply)
            .map_err(|_| TokenReader::invalid_value("mate", value))
    }
}

fn parse_option_kind(token: &str) -> UsiOptionKind {
    match token {
        "check" => UsiOptionKind::Check,
        "spin" => UsiOptionKind::Spin,
        "combo" => UsiOptionKind::Combo,
        "button" => UsiOptionKind::Button,
        "string" => UsiOptionKind::String,
        "filename" => UsiOptionKind::Filename,
        other => UsiOptionKind::Other(other.to_owned()),
    }
}

fn normalize_empty_option_default(kind: &UsiOptionKind, value: String) -> String {
    if matches!(kind, UsiOptionKind::String | UsiOptionKind::Filename) && value == "<empty>" {
        String::new()
    } else {
        value
    }
}

fn validate_option_name_portability(
    context: &'static str,
    name: &str,
) -> Result<(), PortabilityError> {
    if name.chars().any(char::is_whitespace) {
        Err(PortabilityError::new(PortabilityErrorKind::WhitespaceInOptionName {
            context,
            name: name.to_string(),
        }))
    } else {
        Ok(())
    }
}

fn is_go_keyword(token: &str) -> bool {
    matches!(
        token,
        "ponder"
            | "infinite"
            | "btime"
            | "wtime"
            | "byoyomi"
            | "binc"
            | "winc"
            | "movetime"
            | "movestogo"
            | "depth"
            | "nodes"
            | "mate"
            | "searchmoves"
    )
}

const OPTION_PARAM_KEYWORDS: [&str; 4] = ["default", "min", "max", "var"];

#[cfg(test)]
mod tests {
    use std::fmt;

    use super::{
        parse_line, parse_line_strict, validate_portable_command, BestMove, BestMoveKind,
        CheckmateResponse, GoMate, GoParams, InfoCommand, InfoScore, MateScore,
        PositionReplayError, PositionSpec, ScoreBound, ScoreValue, UsiCommand, UsiCommandDirection,
        UsiOption, UsiOptionKind,
    };
    use crate::{ParseErrorKind, ParseErrorSite, PortabilityError, PortabilityErrorKind};

    #[test]
    fn parses_go_mate_and_searchmoves() {
        let parsed = parse_line("go mate 5 searchmoves 7g7f 2g2f").expect("parse should succeed");
        assert_eq!(
            parsed,
            UsiCommand::Go(GoParams {
                mate: Some(GoMate::Ply(5)),
                searchmoves: vec!["7g7f".to_string(), "2g2f".to_string()],
                ..GoParams::default()
            })
        );
    }

    #[test]
    fn parses_go_mate_infinite() {
        let parsed = parse_line("go mate infinite").expect("parse should succeed");
        assert_eq!(
            parsed,
            UsiCommand::Go(GoParams { mate: Some(GoMate::Infinite), ..GoParams::default() })
        );
    }

    #[test]
    fn go_mate_helpers_cover_common_cases() {
        struct DummyMove(&'static str);

        impl fmt::Display for DummyMove {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.0)
            }
        }

        assert_eq!(GoMate::ply(7), GoMate::Ply(7));
        assert_eq!(GoMate::infinite(), GoMate::Infinite);
        assert_eq!(GoMate::ply(7).as_ply(), Some(7));
        assert_eq!(GoMate::infinite().as_ply(), None);
        assert_eq!(GoMate::ply(7).as_i32_saturating(), 7);
        assert_eq!(GoMate::ply(i32::MAX as u32).as_i32_saturating(), i32::MAX);
        assert_eq!(GoMate::ply(i32::MAX as u32 + 1).as_i32_saturating(), i32::MAX);
        assert_eq!(GoMate::infinite().as_i32_saturating(), i32::MAX);
        assert!(GoMate::infinite().is_infinite());
        assert!(!GoMate::ply(7).is_infinite());

        assert_eq!(GoParams::default().effective_movestogo(), None);
        assert_eq!(
            GoParams { movestogo: Some(0), ..GoParams::default() }.effective_movestogo(),
            None
        );
        assert_eq!(
            GoParams { movestogo: Some(1), ..GoParams::default() }.effective_movestogo(),
            Some(1)
        );
        assert_eq!(
            GoParams { movestogo: Some(u64::MAX), ..GoParams::default() }.effective_movestogo(),
            Some(u64::MAX)
        );

        assert_eq!(
            GoParams::mate(GoMate::ply(5)).with_searchmove("7g7f").with_extra("ponder"),
            GoParams {
                mate: Some(GoMate::Ply(5)),
                searchmoves: vec!["7g7f".to_string()],
                extras: vec!["ponder".to_string()],
                ..GoParams::default()
            }
        );

        let moves = [DummyMove("7g7f"), DummyMove("2g2f")];
        assert_eq!(
            GoParams::new()
                .with_mate_infinite()
                .with_searchmoves_display(moves.iter())
                .with_extras(["foo", "bar"]),
            GoParams {
                mate: Some(GoMate::Infinite),
                searchmoves: vec!["7g7f".to_string(), "2g2f".to_string()],
                extras: vec!["foo".to_string(), "bar".to_string()],
                ..GoParams::default()
            }
        );
        assert_eq!(GoParams::new().with_mate_ply(3), GoParams::mate(GoMate::ply(3)));
    }

    #[test]
    fn parses_extension_command_generically() {
        let parsed = parse_line("test_movegen captures").expect("parse should succeed");
        assert_eq!(parsed, UsiCommand::extension("test_movegen", ["captures"]));
    }

    #[test]
    fn parses_timed_ponderhit_as_extension() {
        let parsed = parse_line("ponderhit btime 37082 wtime 23103 byoyomi 10000")
            .expect("parse should succeed");
        assert_eq!(
            parsed,
            UsiCommand::extension(
                "ponderhit",
                ["btime", "37082", "wtime", "23103", "byoyomi", "10000"],
            )
        );
    }

    #[test]
    fn bestmove_helpers_cover_common_cases() {
        assert_eq!(BestMove::resign(), BestMove::new(BestMoveKind::Resign));
        assert_eq!(BestMove::win(), BestMove::new(BestMoveKind::Win));
        assert_eq!(
            BestMove::move_to("8h2b+").with_ponder("3a2b"),
            BestMove {
                bestmove: BestMoveKind::Move("8h2b+".to_string()),
                ponder: Some("3a2b".to_string()),
            }
        );
        assert_eq!(
            BestMove::move_to("8h2b+").with_optional_ponder(Some("3a2b")),
            BestMove {
                bestmove: BestMoveKind::Move("8h2b+".to_string()),
                ponder: Some("3a2b".to_string()),
            }
        );
        assert_eq!(
            BestMove::move_to("8h2b+").with_optional_ponder::<String>(None),
            BestMove { bestmove: BestMoveKind::Move("8h2b+".to_string()), ponder: None }
        );
    }

    #[test]
    fn position_spec_sfen_helpers_cover_startpos_and_sfen() {
        let startpos = PositionSpec::StartPos;
        assert_eq!(
            startpos.as_sfen_parts(),
            ("lnsgkgsnl/1r5b1/ppppppppp/9/9/9/PPPPPPPPP/1B5R1/LNSGKGSNL", "b", "-", "1",)
        );
        assert_eq!(
            startpos.to_sfen(),
            "lnsgkgsnl/1r5b1/ppppppppp/9/9/9/PPPPPPPPP/1B5R1/LNSGKGSNL b - 1"
        );

        let spec = PositionSpec::Sfen {
            board: "9/9/9/9/9/9/9/9/9".to_string(),
            side_to_move: "w".to_string(),
            hands: "Rb".to_string(),
            ply: "42".to_string(),
        };
        assert_eq!(spec.as_sfen_parts(), ("9/9/9/9/9/9/9/9/9", "w", "Rb", "42"));
        assert_eq!(spec.to_sfen(), "9/9/9/9/9/9/9/9/9 w Rb 42");
    }

    #[test]
    fn position_spec_replay_helper_replays_startpos_and_moves() {
        let spec = PositionSpec::StartPos;
        let replayed = spec
            .replay(
                &["7g7f".to_string(), "3c3d".to_string()],
                || Ok::<_, &'static str>(vec!["startpos".to_string()]),
                |sfen| Ok(vec![format!("sfen {sfen}")]),
                |position, mv| {
                    position.push(mv.to_string());
                    Ok(())
                },
            )
            .expect("replay should succeed");
        assert_eq!(replayed, vec!["startpos".to_string(), "7g7f".to_string(), "3c3d".to_string()]);
    }

    #[test]
    fn position_spec_replay_helper_reports_failing_move_index() {
        let spec = PositionSpec::StartPos;
        let err = spec
            .replay(
                &["7g7f".to_string(), "badmove".to_string()],
                || Ok::<_, &'static str>(Vec::<String>::new()),
                |_sfen| Ok(Vec::<String>::new()),
                |_position, mv| {
                    if mv == "badmove" {
                        Err("illegal move")
                    } else {
                        Ok(())
                    }
                },
            )
            .expect_err("second move should fail");
        assert_eq!(
            err,
            PositionReplayError::ApplyMove {
                move_index: 2,
                move_text: "badmove".to_string(),
                source: "illegal move",
            }
        );
        assert_eq!(err.to_string(), "failed to apply move #2 `badmove`: illegal move");
    }

    #[test]
    fn parses_position_sfen() {
        let parsed = parse_line(
            "position sfen lnsgkgsnl/1r5b1/p1pppp1pp/6p2/9/2P6/PP1PPPPPP/1B5R1/LNSGKGSNL b - 1 moves 7g7f",
        )
        .expect("parse should succeed");
        assert_eq!(
            parsed,
            UsiCommand::Position {
                spec: PositionSpec::Sfen {
                    board: "lnsgkgsnl/1r5b1/p1pppp1pp/6p2/9/2P6/PP1PPPPPP/1B5R1/LNSGKGSNL"
                        .to_string(),
                    side_to_move: "b".to_string(),
                    hands: "-".to_string(),
                    ply: "1".to_string(),
                },
                moves: vec!["7g7f".to_string()],
            }
        );
    }

    #[test]
    fn parses_bestmove_with_ponder() {
        let parsed =
            parse_line("bestmove 8h2b+ ponder 3a2b").expect("bestmove should parse correctly");
        assert_eq!(
            parsed,
            UsiCommand::BestMove(BestMove {
                bestmove: BestMoveKind::Move("8h2b+".to_string()),
                ponder: Some("3a2b".to_string()),
            })
        );
    }

    #[test]
    fn parses_option_with_var_and_default() {
        let parsed = parse_line(
            "option name Style type combo default Normal var Solid var Normal var Risky",
        )
        .expect("option should parse");
        assert_eq!(
            parsed,
            UsiCommand::Option(UsiOption {
                name: "Style".to_string(),
                kind: UsiOptionKind::Combo,
                default: Some("Normal".to_string()),
                min: None,
                max: None,
                vars: vec!["Solid".to_string(), "Normal".to_string(), "Risky".to_string()],
                extras: Vec::new(),
            })
        );
    }

    #[test]
    fn option_helpers_cover_common_cases() {
        assert_eq!(
            UsiOption::spin("Hash", 16, 1, 1_048_576),
            UsiOption {
                name: "Hash".to_string(),
                kind: UsiOptionKind::Spin,
                default: Some("16".to_string()),
                min: Some(1),
                max: Some(1_048_576),
                vars: Vec::new(),
                extras: Vec::new(),
            }
        );
        assert_eq!(
            UsiOption::combo("Style", "Normal", ["Solid", "Normal", "Risky"]),
            UsiOption {
                name: "Style".to_string(),
                kind: UsiOptionKind::Combo,
                default: Some("Normal".to_string()),
                min: None,
                max: None,
                vars: vec!["Solid".to_string(), "Normal".to_string(), "Risky".to_string()],
                extras: Vec::new(),
            }
        );
        assert_eq!(
            UsiOption::button("ClearHash").with_extra("note"),
            UsiOption {
                name: "ClearHash".to_string(),
                kind: UsiOptionKind::Button,
                default: None,
                min: None,
                max: None,
                vars: Vec::new(),
                extras: vec!["note".to_string()],
            }
        );
        assert_eq!(
            UsiOption::filename("EvalFile", ""),
            UsiOption {
                name: "EvalFile".to_string(),
                kind: UsiOptionKind::Filename,
                default: Some(String::new()),
                min: None,
                max: None,
                vars: Vec::new(),
                extras: Vec::new(),
            }
        );
    }

    #[test]
    fn normalizes_empty_string_defaults_for_option_commands() {
        let parsed = parse_line("option name EvalFile type filename default <empty>")
            .expect("filename option should parse");
        assert_eq!(
            parsed,
            UsiCommand::Option(UsiOption {
                name: "EvalFile".to_string(),
                kind: UsiOptionKind::Filename,
                default: Some(String::new()),
                min: None,
                max: None,
                vars: Vec::new(),
                extras: Vec::new(),
            })
        );
    }

    #[test]
    fn parses_info_score_and_pv() {
        let parsed = parse_line("info depth 12 score cp 34 lowerbound pv 7g7f 3c3d")
            .expect("info should parse");
        assert_eq!(
            parsed,
            UsiCommand::Info(InfoCommand {
                depth: Some(12),
                score: Some(InfoScore {
                    value: ScoreValue::Cp(34),
                    bound: Some(ScoreBound::Lower),
                }),
                pv: vec!["7g7f".to_string(), "3c3d".to_string()],
                ..InfoCommand::default()
            })
        );
    }

    #[test]
    fn parses_info_score_mate_symbol() {
        let parsed = parse_line("info score mate + nodes 123").expect("mate score should parse");
        assert_eq!(
            parsed,
            UsiCommand::Info(InfoCommand {
                nodes: Some(123),
                score: Some(InfoScore {
                    value: ScoreValue::Mate(MateScore::UnknownWin),
                    bound: None,
                }),
                ..InfoCommand::default()
            })
        );
    }

    #[test]
    fn info_string_helper_sets_only_string_field() {
        assert_eq!(
            InfoCommand::string("stub search policy lives in adapter"),
            InfoCommand {
                string: Some("stub search policy lives in adapter".to_string()),
                ..InfoCommand::default()
            }
        );
    }

    #[test]
    fn info_builder_helpers_cover_common_cases() {
        assert_eq!(
            InfoCommand::new()
                .with_depth(18)
                .with_seldepth(24)
                .with_nodes(1_024)
                .with_nps(50_000)
                .with_hashfull(321)
                .with_multipv(2)
                .with_score_cp(37)
                .with_pv(["7g7f", "3c3d"]),
            InfoCommand {
                depth: Some(18),
                seldepth: Some(24),
                nodes: Some(1_024),
                nps: Some(50_000),
                hashfull: Some(321),
                multipv: Some(2),
                score: Some(InfoScore::cp(37)),
                pv: vec!["7g7f".to_string(), "3c3d".to_string()],
                ..InfoCommand::default()
            }
        );
    }

    #[test]
    fn info_builder_accepts_display_pv_and_usize_multipv() {
        struct DummyMove(&'static str);

        impl fmt::Display for DummyMove {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.0)
            }
        }

        let pv = [DummyMove("7g7f"), DummyMove("3c3d")];
        assert_eq!(
            InfoCommand::new().with_multipv_usize(2).with_pv_display(pv.iter()),
            InfoCommand {
                multipv: Some(2),
                pv: vec!["7g7f".to_string(), "3c3d".to_string()],
                ..InfoCommand::default()
            }
        );
    }

    #[test]
    fn info_builder_last_call_wins_between_string_and_pv() {
        assert_eq!(
            InfoCommand::new().with_string("hello").with_pv(["7g7f"]),
            InfoCommand { pv: vec!["7g7f".to_string()], ..InfoCommand::default() }
        );
        assert_eq!(
            InfoCommand::new().with_pv(["7g7f"]).with_string("hello"),
            InfoCommand { string: Some("hello".to_string()), ..InfoCommand::default() }
        );
    }

    #[test]
    fn score_helpers_cover_common_cases() {
        assert_eq!(
            InfoScore::cp(120).with_lowerbound(),
            InfoScore { value: ScoreValue::Cp(120), bound: Some(ScoreBound::Lower) }
        );
        assert_eq!(
            InfoScore::mate(MateScore::unknown_win()).with_upperbound(),
            InfoScore {
                value: ScoreValue::Mate(MateScore::UnknownWin),
                bound: Some(ScoreBound::Upper),
            }
        );
        assert_eq!(MateScore::ply(5), MateScore::Ply(5));
        assert_eq!(MateScore::unknown_lose(), MateScore::UnknownLose);
        assert_eq!(ScoreBound::from_flags(true, false), Some(ScoreBound::Lower));
        assert_eq!(ScoreBound::from_flags(false, true), Some(ScoreBound::Upper));
        assert_eq!(ScoreBound::from_flags(false, false), None);
        assert_eq!(ScoreBound::from_flags(true, true), None);
        assert_eq!(
            InfoScore::cp(18).with_optional_bound(None),
            InfoScore { value: ScoreValue::Cp(18), bound: None }
        );
        assert_eq!(
            InfoScore::cp(18).with_bound_flags(true, false),
            InfoScore::cp(18).with_lowerbound()
        );
        assert_eq!(InfoScore::cp(18).with_bound_flags(true, true), InfoScore::cp(18));
    }

    #[test]
    fn parses_checkmate_variants() {
        let parsed = parse_line("checkmate nomate").expect("checkmate should parse");
        assert_eq!(parsed, UsiCommand::Checkmate(CheckmateResponse::NoMate));
    }

    #[test]
    fn checkmate_helpers_cover_common_cases() {
        struct DummyMove(&'static str);

        impl fmt::Display for DummyMove {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.0)
            }
        }

        let moves = CheckmateResponse::moves(["G*8f", "9f9g"]);
        assert_eq!(moves, CheckmateResponse::Moves(vec!["G*8f".to_string(), "9f9g".to_string()]));
        assert_eq!(
            CheckmateResponse::moves_display([DummyMove("8f8g"), DummyMove("9g9h")]),
            CheckmateResponse::Moves(vec!["8f8g".to_string(), "9g9h".to_string()])
        );
        assert_eq!(CheckmateResponse::timeout(), CheckmateResponse::Timeout);
        assert_eq!(CheckmateResponse::nomate(), CheckmateResponse::NoMate);
        assert_eq!(CheckmateResponse::not_implemented(), CheckmateResponse::NotImplemented);
        assert_eq!(moves.as_moves(), Some(["G*8f".to_string(), "9f9g".to_string()].as_slice()));
        assert_eq!(CheckmateResponse::timeout().as_moves(), None);
        assert!(CheckmateResponse::timeout().is_reserved_status());
        assert!(!moves.is_reserved_status());
    }

    #[test]
    fn rejects_checkmate_status_with_extra_tokens() {
        let parsed = parse_line("checkmate timeout 7g7f");
        assert!(parsed.is_err(), "reserved checkmate status should not accept extra tokens");
    }

    #[test]
    fn extension_helpers_expose_generic_surface() {
        let command = UsiCommand::extension("eval", ["nnue", "detail"]);
        assert!(command.is_extension("eval"));
        assert_eq!(
            command.as_extension(),
            Some(("eval", ["nnue".to_string(), "detail".to_string()].as_slice()))
        );
        assert_eq!(command.direction(), None);
        assert!(!command.is_gui_to_engine());
        assert!(!command.is_engine_to_gui());
        assert!(!UsiCommand::Usi.is_extension("eval"));
        assert_eq!(UsiCommand::Usi.as_extension(), None);
    }

    #[test]
    fn command_direction_helpers_classify_standard_commands() {
        assert_eq!(UsiCommand::Usi.direction(), Some(UsiCommandDirection::GuiToEngine));
        assert!(UsiCommand::Usi.is_gui_to_engine());
        assert!(!UsiCommand::Usi.is_engine_to_gui());

        assert_eq!(
            UsiCommand::SetOption { name: "Hash".to_string(), value: Some("256".to_string()) }
                .direction(),
            Some(UsiCommandDirection::GuiToEngine)
        );
        assert_eq!(
            UsiCommand::GameOver(super::GameResult::Win).direction(),
            Some(UsiCommandDirection::GuiToEngine)
        );

        assert_eq!(UsiCommand::readyok().direction(), Some(UsiCommandDirection::EngineToGui));
        assert!(UsiCommand::readyok().is_engine_to_gui());
        assert!(!UsiCommand::readyok().is_gui_to_engine());

        assert_eq!(
            UsiCommand::info_string("searching").direction(),
            Some(UsiCommandDirection::EngineToGui)
        );
        assert_eq!(
            UsiCommand::Checkmate(CheckmateResponse::Timeout).direction(),
            Some(UsiCommandDirection::EngineToGui)
        );
    }

    #[test]
    fn outbound_command_helpers_cover_common_cases() {
        assert_eq!(
            UsiCommand::id_name("minimal-engine"),
            UsiCommand::Id { key: "name".to_string(), value: "minimal-engine".to_string() }
        );
        assert_eq!(
            UsiCommand::id_author("rshogi contributors"),
            UsiCommand::Id { key: "author".to_string(), value: "rshogi contributors".to_string() }
        );
        assert_eq!(UsiCommand::readyok(), UsiCommand::ReadyOk);
        assert_eq!(UsiCommand::usiok(), UsiCommand::UsiOk);
        assert_eq!(
            UsiCommand::bestmove(BestMove::resign()),
            UsiCommand::BestMove(BestMove::resign())
        );
        assert_eq!(
            UsiCommand::info_string("searching"),
            UsiCommand::Info(InfoCommand::string("searching"))
        );
        assert_eq!(
            UsiCommand::info(InfoCommand::new().with_score_mate(MateScore::ply(3))),
            UsiCommand::Info(InfoCommand {
                score: Some(InfoScore::mate(MateScore::Ply(3))),
                ..InfoCommand::default()
            })
        );
        assert_eq!(
            UsiCommand::go(GoParams::new().with_mate_ply(5)),
            UsiCommand::Go(GoParams::mate(GoMate::ply(5)))
        );
        assert_eq!(
            UsiCommand::go_mate_infinite(),
            UsiCommand::Go(GoParams::mate(GoMate::Infinite))
        );
        assert_eq!(
            UsiCommand::checkmate_moves(["G*8f", "9f9g"]),
            UsiCommand::Checkmate(CheckmateResponse::Moves(vec![
                "G*8f".to_string(),
                "9f9g".to_string()
            ]))
        );
        assert_eq!(
            UsiCommand::checkmate_timeout(),
            UsiCommand::Checkmate(CheckmateResponse::Timeout)
        );
        assert_eq!(
            UsiCommand::checkmate_nomate(),
            UsiCommand::Checkmate(CheckmateResponse::NoMate)
        );
        assert_eq!(
            UsiCommand::checkmate_not_implemented(),
            UsiCommand::Checkmate(CheckmateResponse::NotImplemented)
        );
    }

    #[test]
    fn strict_parser_accepts_canonical_lines() {
        let parsed = parse_line_strict("go depth 10 searchmoves 7g7f 2g2f")
            .expect("strict parser should accept canonical command");
        assert_eq!(
            parsed,
            UsiCommand::Go(GoParams {
                depth: Some(10),
                searchmoves: vec!["7g7f".to_string(), "2g2f".to_string()],
                ..GoParams::default()
            })
        );
    }

    #[test]
    fn strict_parser_rejects_noncanonical_lines_with_expected_form() {
        let err = parse_line_strict("go searchmoves 7g7f 2g2f depth 10")
            .expect_err("strict parser should reject non-canonical command");
        assert_eq!(
            err.kind(),
            &ParseErrorKind::NonCanonical {
                canonical: "go depth 10 searchmoves 7g7f 2g2f".to_string(),
            }
        );
        assert_eq!(
            err.canonical_token_mismatch().map(|mismatch| (
                mismatch.token_position,
                mismatch.expected.as_deref(),
                mismatch.found.as_deref()
            )),
            Some((2, Some("depth"), Some("searchmoves")))
        );
        assert_eq!(
            err.site(),
            Some(&ParseErrorSite {
                token_position: 2,
                byte_start: 3,
                byte_end: 14,
                token: Some("searchmoves".to_string()),
            })
        );
        assert_eq!(
            err.to_string(),
            "non-canonical command; expected `go depth 10 searchmoves 7g7f 2g2f`; first token mismatch at position 2: expected `depth`, found `searchmoves`; at token 2 (`searchmoves`) bytes 3..14"
        );
    }

    #[test]
    fn ponderhit_helper_uses_extension_surface_for_nonstandard_args() {
        assert_eq!(
            UsiCommand::ponderhit_with_args(std::iter::empty::<&str>()),
            UsiCommand::PonderHit
        );
        assert_eq!(
            UsiCommand::ponderhit_with_args(["btime", "1000"]),
            UsiCommand::extension("ponderhit", ["btime", "1000"])
        );
    }

    #[test]
    fn portability_validation_rejects_whitespace_in_option_names() {
        let err = validate_portable_command(&UsiCommand::Option(UsiOption::button("Clear Hash")))
            .expect_err("whitespace should be rejected");
        assert_eq!(
            err,
            PortabilityError::new(PortabilityErrorKind::WhitespaceInOptionName {
                context: "option",
                name: "Clear Hash".to_string(),
            })
        );

        validate_portable_command(&UsiCommand::SetOption {
            name: "EvalFile".to_string(),
            value: Some("path with spaces.nnue".to_string()),
        })
        .expect("setoption values may contain spaces when the name is portable");
    }

    #[test]
    fn invalid_value_error_reports_token_site() {
        let err = parse_line("go depth nope").expect_err("invalid numeric value should fail");
        assert_eq!(
            err.kind(),
            &ParseErrorKind::InvalidValue { field: "depth", value: "nope".to_string() }
        );
        assert_eq!(
            err.site(),
            Some(&ParseErrorSite {
                token_position: 3,
                byte_start: 9,
                byte_end: 13,
                token: Some("nope".to_string()),
            })
        );
        assert_eq!(err.to_string(), "invalid depth: `nope`; at token 3 (`nope`) bytes 9..13");
    }
}
