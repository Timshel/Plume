use clap::{App, Arg, ArgMatches, SubCommand};

use plume_models::{migrations::IMPORTED_MIGRATIONS, Connection};
use std::path::Path;

pub fn command<'a, 'b>() -> App<'a, 'b> {
    SubCommand::with_name("migration")
        .about("Manage migrations")
        .subcommand(
            SubCommand::with_name("run").about("Run migrations"),
        )
}

pub fn run<'a>(args: &ArgMatches<'a>, conn: &Connection) {
    let conn = conn;
    match args.subcommand() {
        ("run") => run_(conn),
        ("", None) => command().print_help().unwrap(),
        _ => println!("Unknown subcommand"),
    }
}

fn run_(conn: &Connection) {
    let path = args.value_of("path").unwrap_or(".");
    plume_models::migrations::run_pending_migrations(conn, Path::new(path))
        .expect("Failed to run migrations")
}
