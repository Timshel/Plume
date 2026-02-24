use clap::{App, ArgMatches, SubCommand};

use plume_models::Connection;

pub fn command<'a, 'b>() -> App<'a, 'b> {
    SubCommand::with_name("migration")
        .about("Manage migrations")
        .subcommand(
            SubCommand::with_name("run").about("Run migrations"),
        )
}

pub fn run<'a>(args: &ArgMatches<'a>, conn: &mut Connection) {
    let conn = conn;
    match args.subcommand() {
        ("run", _) => run_(conn),
        ("", _) => command().print_help().unwrap(),
        _ => println!("Unknown subcommand"),
    }
}

fn run_(conn: &mut Connection) {
    plume_models::migrations::run_pending_migrations(conn)
        .expect("Failed to run migrations");
}
