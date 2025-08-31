mod gz;

use clap::{Command, arg};

fn main() {
    let cmd = Command::new("gz")
        .bin_name("gz")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .disable_help_subcommand(true)
        .subcommand(
            Command::new("sync")
                .about("Sync current branch with origin/main")
                .arg(arg!(-f --force "Force reset instead of pull"))
        )
        .subcommand(Command::new("stash").about("Stash local changes including untracked files"))
        .subcommand(
            Command::new("uncommit")
                .about("Uncommit last N commits")
                .arg(arg!([count] "Number of commits to uncommit").default_value("1")),
        )
        .subcommand(
            Command::new("branch")
                .about("Create and switch to a new branch")
                .arg(arg!(<name> "Branch name"))
                .arg_required_else_help(true),
        )
        .subcommand(Command::new("add").about("Launch TUI to stage and unstage files"))
        .subcommand(Command::new("done").about("Switch back to main and delete current branch"));

    let matches = cmd.get_matches();
    match matches.subcommand() {
        Some(("sync", subm)) => gz::sync(subm.get_flag("force")),
        Some(("stash", _)) => gz::stash(),
        Some(("uncommit", subm)) => {
            let count: usize = subm.get_one::<String>("count").unwrap().parse().unwrap_or(1);
            gz::uncommit(count);
        }
        Some(("branch", subm)) => {
            let name = subm.get_one::<String>("name").expect("required");
            gz::branch(name);
        }
        Some(("add", _)) => gz::add(),
        Some(("done", _)) => gz::done(),
        _ => unreachable!(),
    };
}
