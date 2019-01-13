#![allow(proc_macro_derive_resolution_fallback)]
#![feature(await_macro, async_await, futures_api, proc_macro_hygiene)]

#[macro_use]
extern crate diesel;

use std::fs::File;
use std::io::Read;
use std::io::{BufRead, BufReader, Write};

use crate::config::{Args, Command, ImportType};
use crate::database::import_models::{RFactoid, WinError};
use crate::database::models::WinErrorVariant;

use irc_bot::Nestor;
use irc_bot_codegen::routes;
use structopt::StructOpt;

mod commands;
mod config;
mod database;

fn main() {
    let args = Args::from_args();

    let db = database::Db::open("rustybot.sqlite").unwrap();

    match args.command {
        Command::Export { file } => {
            let mut output = File::create(file).unwrap();
            let factoids = db.all_factoids().unwrap();
            for factoid in factoids {
                let factoid_json = serde_json::to_string(&RFactoid::from(factoid)).unwrap();
                writeln!(output, "{}", factoid_json).unwrap();
            }
        }
        Command::Import { file, import_type } => {
            let input = File::open(file).unwrap();
            let mut input = BufReader::new(input);

            match import_type {
                ImportType::Factoid => {
                    for line in input.lines() {
                        let line = line.unwrap();
                        let rfactoid: RFactoid = match serde_json::from_str(&line) {
                            Ok(rfactoid) => rfactoid,
                            Err(err) => {
                                println!("{}", err);
                                continue;
                            }
                        };
                        db.create_from_rfactoid(&rfactoid).unwrap();
                    }
                }
                ImportType::HResult => {
                    let mut buffer = String::new();
                    input.read_to_string(&mut buffer).unwrap();
                    let errors: Vec<WinError> = serde_json::from_str(&buffer).unwrap();

                    for error in errors {
                        let code = u32::from_str_radix(&error.code.trim()[2..], 16).unwrap();
                        db.create_error(
                            code,
                            WinErrorVariant::HResult,
                            &error.name.trim(),
                            &error.desc.trim(),
                        )
                        .unwrap()
                    }
                }
                ImportType::NtResult => {
                    let mut buffer = String::new();
                    input.read_to_string(&mut buffer).unwrap();
                    let errors: Vec<WinError> = serde_json::from_str(&buffer).unwrap();

                    for error in errors {
                        let code = u32::from_str_radix(&error.code.trim()[2..], 16).unwrap();
                        db.create_error(
                            code,
                            WinErrorVariant::NtStatus,
                            &error.name.trim(),
                            &error.desc.trim(),
                        )
                        .unwrap()
                    }
                }
                ImportType::Win32 => {
                    let mut buffer = String::new();
                    input.read_to_string(&mut buffer).unwrap();
                    let errors: Vec<WinError> = serde_json::from_str(&buffer).unwrap();

                    for error in errors {
                        let code = u32::from_str_radix(&error.code.trim()[2..], 16).unwrap();
                        db.create_error(
                            code,
                            WinErrorVariant::Win32,
                            &error.name.trim(),
                            &error.desc.trim(),
                        )
                        .unwrap()
                    }
                }
            }
        }

        Command::Launch {} => {
            Nestor::build()
                .manage(|| database::Db::open("rustybot.sqlite").unwrap())
                .route(routes![
                    commands::crate_info::crate_info,
                    commands::forget::forget,
                    commands::github::rfc,
                    commands::learn::learn,
                    commands::lock::lock,
                    commands::lock::unlock,
                    commands::qotd::qotd,
                    commands::rustc_error::rustc_error,
                    commands::windows_error::hresult,
                    commands::windows_error::nt_status,
                    commands::windows_error::win32,
                    commands::default::user_defined,
                ])
                .activate();
        }
    }
}
