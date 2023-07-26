#![feature(box_patterns)]
#[cfg(feature = "trace_log")]
extern crate env_logger;

extern crate base64;
extern crate r2papi;
extern crate r2pipe;
extern crate radeco_lib;
extern crate rustyline;

#[macro_use]
extern crate lazy_static;
extern crate clap;
extern crate syntect;

mod cli;
mod core;
mod highlighting;

use rustyline::completion::{Completer, FilenameCompleter};
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{CompletionType, Config, Context, EditMode, Editor, Helper};
use std::fs;
use std::process;

mod scheme {
    pub const HTTP: &'static str = "http://";
    pub const TCP: &'static str = "tcp://";

    pub fn is_http(url: &str) -> bool {
        url.starts_with(HTTP)
    }

    pub fn is_tcp(url: &str) -> bool {
        url.starts_with(TCP)
    }
}

// On unix platforms you can use ANSI escape sequences
#[cfg(unix)]
const PROMPT: &'static str = "\x1b[1;32m>>\x1b[0m ";
// File seperator for *nix based platforms
#[cfg(unix)]
const FILE_SP: char = '/';

// Windows consoles typically don't support ANSI escape sequences out
// of the box
#[cfg(windows)]
const PROMPT: &'static str = ">> ";
#[cfg(windows)]
// File seperator for Windows
const FILE_SP: char = '\\';

// Default max number of iterations of the engine.
pub const MAX_ITERATIONS: u32 = 100;

#[derive(Default)]
struct Completes {
    file_completer: FilenameCompleter,
}

impl Validator for Completes {}

impl Helper for Completes {}

impl Hinter for Completes {
    fn hint(&self, _line: &str, _pos: usize, _ctx: &Context<'_>) -> Option<String> {
        None
    }
}

impl Highlighter for Completes {}

impl Completer for Completes {
    type Candidate = String;
    fn complete(&self, line: &str, _pos: usize, ctx: &Context<'_>) -> rustyline::Result<(usize, Vec<String>)> {
        let cmds = vec![
            command::HELP,
            command::LOAD,
            command::CONNECT,
            command::FNLIST,
            command::ANALYZE,
            command::DOT,
            command::IR,
            command::DECOMPILE,
            command::FUNC_RENAME,
            command::QUIT,
        ];

        // Check if `line` contains a cmd that requires a function
        // as parameter.
        let complete_func: bool = cmds
            .iter()
            .filter(|s| line.len() >= s.len())
            .any(|s| &&line[..s.len()] == s && command::requires_func(&line[..s.len()]));

        // Complete commands.
        let mut ret: Vec<String> = cmds
            .into_iter()
            .filter(|s| line.len() < s.len())
            .filter(|s| s.starts_with(line))
            .map(|s| s.to_string())
            .collect();

        if complete_func {
            core::PROJ.with(|proj_opt| {
                if let Some(ref proj) = *proj_opt.borrow() {
                    let mut line_tokens = line.split(' ');
                    line_tokens.next(); // drop the command.

                    let to_compl = line_tokens.last().unwrap_or("");

                    let mut funcs: Vec<String> = core::fn_list(&proj)
                        .iter()
                        .filter(|f| f.len() > to_compl.len())
                        .filter(|f| &f[0..to_compl.len()] == to_compl)
                        .map(|f| {
                            format!(
                                "{}{}{}",
                                line,
                                if to_compl.len() == 0 { " " } else { "" },
                                &f[to_compl.len()..f.len()]
                            )
                        })
                        .collect();
                    ret.append(&mut funcs);
                }
            });
        }
        if line.starts_with("load") {
            match self.file_completer.complete(line, _pos, ctx) {
                Ok((n, ss)) => {
                    let mut completed_lines = ss
                        .into_iter()
                        .map(|s| {
                            if let Some(sep_loc) = line.rfind(FILE_SP) {
                                format!("{}{}", &line[..sep_loc + 1], s.display)
                            } else {
                                format!("{}{}", &line[..n], s.display)
                            }
                        })
                        .collect();
                    ret.append(&mut completed_lines);
                }
                Err(_) => {}
            }
        }

        Ok((0, ret))
    }
}

const SEP: &'static str = "END";

fn main() {
    #[cfg(feature = "trace_log")]
    env_logger::init();
    let (arg, cmd_opt, is_append_mode, is_batch_mode, no_highlight, max_it) = cli::parse_args();
    let config = Config::builder()
        .auto_add_history(true)
        .history_ignore_space(true)
        .completion_type(CompletionType::List)
        .edit_mode(EditMode::Emacs)
        .build();
    let mut rl = Editor::with_config(config);
    rl.set_helper(Some(Completes::default()));
    core::PROJ.with(move |proj| {
        use r2pipe::R2Pipe;
        let proj_result = match arg {
            Some(ref s) if scheme::is_http(s) => {
                core::load_proj_http(&s[scheme::HTTP.len()..], max_it)
                    .map_err(|e| Some(e.to_string()))
            }
            Some(ref s) if scheme::is_tcp(s) => {
                core::load_proj_tcp(&s[scheme::TCP.len()..], max_it)
                    .map_err(|e| Some(e.to_string()))
            }
            Some(ref s) if is_file(s) => Ok(core::load_proj_by_path(s, max_it)),
            Some(s) => Err(Some(format!("Invalid argument {}", s))),
            None => match R2Pipe::open() {
                Ok(r2p) => Ok(core::load_project_by_r2pipe(r2p, max_it)),
                Err(_) => Err(None),
            },
        };
        match proj_result {
            Ok(p) => *proj.borrow_mut() = Some(p),
            Err(Some(ref err)) => println!("{}", err),
            Err(None) => {}
        }
    });

    if is_batch_mode {
        core::PROJ.with(|proj_opt| {
            if proj_opt.borrow().is_none() {
                eprintln!("Project was not loaded!");
                return;
            }
            // If a command is specified by the user run it,
            // otherwise decompile all functions.
            if let Some(command) = cmd_opt {
                cmd(command, no_highlight, max_it);
            } else {
                let mut proj_ = proj_opt.borrow_mut();
                let proj = proj_.as_mut().unwrap();

                core::analyze_all_functions(proj, max_it);
                let decompiled = core::decompile_all_functions(proj);
                if no_highlight {
                    println!("{}", decompiled);
                } else {
                    highlighting::print_highlighted(&decompiled);
                }
            }

            process::exit(0);
        });
    }

    loop {
        let readline = rl.readline(PROMPT);
        match readline {
            Ok(line) => {
                cmd(line, !no_highlight, max_it);
                if is_append_mode {
                    println!("{}", SEP);
                }
            }
            Err(ReadlineError::Interrupted) => {}
            Err(ReadlineError::Eof) => break,
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }
}

mod command {
    pub const HELP: &'static str = "help";
    pub const LOAD: &'static str = "load";
    pub const CONNECT: &'static str = "connect";
    pub const FNLIST: &'static str = "fn_list";
    pub const ANALYZE: &'static str = "analyze";
    pub const DOT: &'static str = "dot";
    pub const IR: &'static str = "ir";
    pub const DECOMPILE: &'static str = "decompile";
    pub const FUNC_RENAME: &'static str = "fn_rn";
    pub const QUIT: &'static str = "quit";

    pub fn help() {
        let width = 30;
        println!("{:width$}    Show this help", HELP, width = width);
        println!(
            "{:width$}    Load binary",
            format!("{} path", LOAD),
            width = width
        );
        println!(
            "{:width$}    Connect to radare2 server",
            format!("{} (http|tcp)://<url>", CONNECT),
            width = width
        );
        println!("{:width$}    Show function list", FNLIST, width = width);
        println!(
            "{:width$}    Analyze <func>",
            format!("{} <func>", ANALYZE),
            width = width
        );
        println!(
            "{:width$}    Emit IR of <func>",
            format!("{} <func>", IR),
            width = width
        );
        println!(
            "{:width$}    Emit graph of the IR in Graphviz dot",
            format!("{} <func>", DOT),
            width = width
        );
        println!(
            "{:width$}    Decompile <func>",
            format!("{} <func>", DECOMPILE),
            width = width
        );
        println!(
            "{:width$}    Rename <old_func_name> to <new_func_name>",
            format!("{} <old_name> <new_name>", FUNC_RENAME),
            width = width
        );
        println!("{:width$}    Quit interactive prompt", QUIT, width = width);
    }

    /// Returns true if `cmd` requires a function as parameter.
    pub fn requires_func(cmd: &str) -> bool {
        match cmd {
            ANALYZE | DOT | IR | DECOMPILE | FUNC_RENAME => true,
            _ => false,
        }
    }
}

fn cmd(line: String, highlight: bool, max_it: u32) {
    if line.is_empty() {
        return;
    }

    let mut terms = line.split_whitespace();
    let op1 = terms.next();
    let op2 = terms.next();
    let op3 = terms.next();

    core::PROJ.with(|proj_opt| {
        match (op1, op2, op3) {
            (Some(command::HELP), _, _) => {
                command::help();
                return;
            }
            (Some(command::LOAD), Some(path), _) => {
                if is_file(path) {
                    *proj_opt.borrow_mut() = Some(core::load_proj_by_path(path, max_it));
                    return;
                } else {
                    println!("{} is not found.", path);
                    return;
                }
            }
            (Some(command::CONNECT), Some(url), _) => {
                let p_opt = if scheme::is_http(&url) {
                    core::load_proj_http(&url[scheme::HTTP.len()..], max_it)
                } else if scheme::is_tcp(&url) {
                    core::load_proj_tcp(&url[scheme::TCP.len()..], max_it)
                } else {
                    Err("Invalid url")
                };
                match p_opt {
                    Ok(p) => *proj_opt.borrow_mut() = Some(p),
                    Err(msg) => println!("{}", msg),
                }
                return;
            }
            (Some(command::QUIT), _, _) => {
                process::exit(0);
            }
            _ => {}
        };
        if proj_opt.borrow().is_none() {
            println!("Load a project first");
            return;
        }
        let mut proj_ = proj_opt.borrow_mut();
        let proj = proj_.as_mut().unwrap();
        match (op1, op2, op3) {
            (Some(command::ANALYZE), Some("*"), _) => {
                core::analyze_all_functions(proj, max_it);
            }
            (Some(command::FNLIST), _, _) => {
                let funcs = core::fn_list(&proj);
                println!("{}", funcs.join("\n"));
            }
            // TODO Show list of dependency information of analyses
            // TODO Add command for individual analyses
            (Some(command::ANALYZE), Some(f), _) => {
                if let Some(rfn) = core::get_function_mut(f, proj) {
                    core::analyze(rfn, max_it);
                } else {
                    println!("{} is not found", f);
                }
            }
            (Some(command::DOT), Some(f), _) => {
                if let Some(rfn) = core::get_function(f, &proj) {
                    println!("{}", core::emit_dot(rfn.ssa()));
                } else {
                    println!("{} is not found", f);
                }
            }
            (Some(command::IR), Some(f), _) => {
                if let Some(rfn) = core::get_function(f, &proj) {
                    println!("{}", core::emit_ir(rfn));
                } else {
                    println!("{} is not found", f);
                }
            }
            (Some(command::DECOMPILE), Some("*"), _) => {
                let decompiled = core::decompile_all_functions(&proj);
                if highlight {
                    highlighting::print_highlighted(&decompiled);
                } else {
                    println!("{}", decompiled);
                }
            }
            (Some(command::DECOMPILE), Some(f), _) => match core::decompile(f, &proj) {
                Ok(res) => {
                    if highlight {
                        highlighting::print_highlighted(&res);
                    } else {
                        println!("{}", res);
                    }
                }
                Err(err) => println!("{}", err),
            },
            (Some(command::FUNC_RENAME), Some(old_f), Some(new_f)) => {
                core::fn_rename(old_f, new_f, proj);
            }
            _ => {
                println!(
                    "Invalid command {} {}",
                    op1.unwrap_or(""),
                    op2.unwrap_or("")
                );
            }
        }
    });
}

fn is_file(path: &str) -> bool {
    fs::metadata(path).map(|f| f.is_file()).unwrap_or(false)
}
