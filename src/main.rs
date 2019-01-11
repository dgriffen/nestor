#![allow(proc_macro_derive_resolution_fallback)]
#![feature(await_macro, async_await, futures_api)]

#[macro_use]
extern crate diesel;

use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::rc::Rc;

use crate::config::{Args, Command};
use crate::database::rustbot_model::RFactoid;
use crate::handler::handle_message;

use tokio_async_await::compat::backward;
use irc::client::ext::ClientExt;
use irc::client::reactor::IrcReactor;
use structopt::StructOpt;

mod commands;
mod config;
mod database;
mod handler;

fn main() {
    let args = Args::from_args();
    let config = Rc::new(config::Config::load(args.config).unwrap());
    let db = database::Db::open(&config.bot_settings.database_url).unwrap();

    match args.command {
        Command::Export { file } => {
            let mut output = File::create(file).unwrap();
            let factoids = db.all_factoids().unwrap();
            for factoid in factoids {
                let factoid_json = serde_json::to_string(&RFactoid::from(factoid)).unwrap();
                writeln!(output, "{}", factoid_json).unwrap();
            }
        }
        Command::Import { file } => {
            let input = File::open(file).unwrap();
            let input = BufReader::new(input);

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
        Command::Launch {} => {
            let handler = Rc::new(handler::Handler::new(db));

            let mut reactor = IrcReactor::new().unwrap();
            let client = reactor
                .prepare_client_and_connect(&config.irc_config)
                .unwrap();
            client.identify().unwrap();
            reactor.register_client_with_handler(client, move |client, message| {
                let handler = handler.clone();
                let config = config.clone();
                let future = handle_message(client.clone(), message, config, handler);
                backward::Compat::new(future)
            });

            reactor.run().unwrap();
        }
    }
}
