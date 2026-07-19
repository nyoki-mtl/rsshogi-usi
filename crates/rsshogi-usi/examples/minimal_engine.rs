use std::collections::BTreeMap;
use std::error::Error;
use std::io::{self, BufRead, Write};

use rsshogi_usi::{
    format_command, parse_line, BestMove, CheckmateResponse, GoParams, PositionSpec, UsiCommand,
    UsiOption,
};

#[derive(Default)]
struct MinimalEngine {
    options: BTreeMap<String, Option<String>>,
    position: Option<(PositionSpec, Vec<String>)>,
    quit: bool,
}

impl MinimalEngine {
    fn handle_command(&mut self, command: UsiCommand) -> Vec<UsiCommand> {
        match command {
            UsiCommand::Usi => Self::handle_usi(),
            UsiCommand::SetOption { name, value } => {
                self.options.insert(name, value);
                Vec::new()
            }
            UsiCommand::IsReady => vec![UsiCommand::readyok()],
            UsiCommand::UsiNewGame => {
                self.position = None;
                Vec::new()
            }
            UsiCommand::Position { spec, moves } => {
                self.position = Some((spec, moves));
                Vec::new()
            }
            UsiCommand::Go(params) => self.handle_go(&params),
            UsiCommand::Stop => {
                // This stub does not own an async search task. Real adapters cancel/join here.
                vec![UsiCommand::bestmove(BestMove::resign())]
            }
            UsiCommand::Quit => {
                self.quit = true;
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    fn handle_usi() -> Vec<UsiCommand> {
        vec![
            UsiCommand::id_name("minimal-engine"),
            UsiCommand::id_author("rshogi contributors"),
            UsiCommand::Option(UsiOption::spin("Hash", 16, 1, 1_048_576)),
            UsiCommand::Option(UsiOption::check("USI_Ponder", false)),
            UsiCommand::usiok(),
        ]
    }

    fn handle_go(&self, params: &GoParams) -> Vec<UsiCommand> {
        if params.mate.is_some() {
            return vec![UsiCommand::Checkmate(CheckmateResponse::NotImplemented)];
        }

        let status = if self.position.is_some() {
            "stub search policy lives in adapter"
        } else {
            "position is not set; returning stub bestmove"
        };

        vec![UsiCommand::info_string(status), UsiCommand::bestmove(BestMove::resign())]
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut writer = io::BufWriter::new(stdout.lock());
    let mut engine = MinimalEngine::default();

    for line in stdin.lock().lines() {
        let line = line?;
        let responses = match parse_line(&line) {
            Ok(command) => engine.handle_command(command),
            Err(err) => {
                eprintln!("parse error: {err}");
                continue;
            }
        };

        for response in responses {
            writeln!(writer, "{}", format_command(&response))?;
        }
        writer.flush()?;

        if engine.quit {
            break;
        }
    }

    Ok(())
}
