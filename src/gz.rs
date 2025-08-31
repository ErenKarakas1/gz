use core::panic;

mod git;
mod tui;

use crate::gz::git::{Git, run_git};
use crate::gz::tui::{CurrentScreen, run_tui};

pub fn sync(force: bool) {
    let git = Git::open();
    let (workdir, branch) = (git.workdir(), git.branch());
    let branch = branch.as_str();

    if force {
        print!("{}", run_git(workdir, ["fetch", "origin", branch]));
        let origin_branch = format!("origin/{}", branch);
        print!("{}", run_git(workdir, ["reset", "--hard", origin_branch.as_str()]));
    } else {
        print!("{}", run_git(workdir, ["pull", "--ff-only", "origin", branch]));
    }
}

pub fn stash() {
    let git = Git::open();
    print!("{}", run_git(git.workdir(), ["stash", "push", "--include-untracked"]));
}

pub fn uncommit(count: usize) {
    let git = Git::open();
    print!("{}", run_git(git.workdir(), ["reset", &format!("HEAD~{}", count)]));
}

pub fn branch(name: &str) {
    let git = Git::open();
    print!("{}", run_git(git.workdir(), ["switch", "--create", name]));
}

pub fn add() {
    if let Err(e) = run_tui(CurrentScreen::Add) {
        panic!("Failed to initialize Add screen: {}", e);
    }
}

pub fn done() {
    let git = Git::open();
    let (workdir, current) = (git.workdir(), git.branch());

    if current == "main" {
        eprintln!("{}", "You are already on 'main' branch.");
        return;
    }

    print!("{}", run_git(workdir, ["switch", "main"]));
    print!("{}", run_git(workdir, ["branch", "--delete", "--force", current.as_str()]));
    sync(false);
}
