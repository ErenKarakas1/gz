use std::{collections::HashMap, ffi::OsStr, fmt::Debug, path::{Path, PathBuf}, process::{Command, Stdio}};

pub struct Git {
    workdir: PathBuf,
}

pub struct Entry {
    pub path: PathBuf,
    pub staged: bool,
}

impl Git {
    pub fn open() -> Self {
        let cwd = Path::new(".");
        let repo_root = PathBuf::from(run_git(cwd, ["rev-parse", "--show-toplevel"]).trim());
        return Self { workdir: repo_root };
    }

    pub fn workdir(&self) -> &Path {
        self.workdir.as_path()
    }

    pub fn branch(&self) -> String {
        let name = run_git(&self.workdir, ["rev-parse", "--abbrev-ref", "HEAD"]).trim().to_string();
        if name == "HEAD" {
            // detached, show short oid
            let short = run_git(&self.workdir, ["rev-parse", "--short", "HEAD"]).trim().to_string();
            return format!("DETACHED@{short}");
        }
        return name;
    }

    pub fn status(&self) -> Vec<Entry> {
        let out = run_git(&self.workdir, ["status", "--porcelain=v2", "--untracked-files=all"]);

        let mut staged = Vec::new();
        let mut unstaged = Vec::new();

        for line in out.lines() {
            if line.is_empty() { continue; }

            //  - '1 <XY> ... <path> (modified)
            //  - '2 <XY> ... <path> <orig-path>' (rename/copied)
            //  - '? <path>' (untracked)
            //  - 'u <XY> ...' (unmerged) —> treat as both staged and unstaged
            let tag = line.as_bytes()[0] as char;

            match tag {
                '1' | '2' | 'u' => {
                    // split by space, first token is record type, second is XY
                    let mut parts = line.splitn(9, ' '); // cap high enough
                    let _rec_type = parts.next();
                    let xy = parts.next().unwrap_or("");
                    // … skip to last field = path (or new-path for '2')
                    let path = line.rsplit_once(' ').map(|(_, p)| p).unwrap_or_default();

                    let (x, y) = (
                        xy.chars().next().unwrap_or('.'),
                        xy.chars().nth(1).unwrap_or('.'),
                    );

                    let p = PathBuf::from(path);

                    // staged if X != '.'
                    if x != '.' {
                        staged.push(Entry { path: p.clone(), staged: true });
                    }
                    // unstaged if Y != '.'
                    if y != '.' {
                        // If also staged, we still show it in staged first in UI.
                        // To keep the single top-down list unique, only push to unstaged
                        // when not already staged.
                        if x == '.' {
                            unstaged.push(Entry { path: p, staged: false });
                        }
                    }
                }
                '?' => {
                    // untracked
                    let path = line[2..].trim();
                    unstaged.push(Entry { path: PathBuf::from(path), staged: false });
                }
                '!' => {
                    // ignored — skip
                }
                _ => {
                    dbg!("Unrecognized status line: {}", line);
                }
            }
        }

        staged.sort_by(|a, b| a.path.cmp(&b.path));
        unstaged.sort_by(|a, b| a.path.cmp(&b.path));

        staged.extend(unstaged);
        return staged;
    }

    pub fn stage_paths(&self, paths: &[PathBuf]) {
        if paths.is_empty() { return; }
        let mut args = vec!["add", "--"];
        for p in paths {
            args.push(p.to_str().unwrap_or_default());
        }
        run_git(&self.workdir, args);
    }

    pub fn unstage_paths(&self, paths: &[PathBuf]) {
        if paths.is_empty() { return; }
        let mut args = vec!["restore", "--staged", "--"];
        for p in paths {
            args.push(p.to_str().unwrap_or_default());
        }
        run_git(&self.workdir, args);
    }

    pub fn staged_line_counts(&self) -> HashMap<String, (usize, usize)> {
        let out = run_git(&self.workdir, ["diff", "--cached", "--numstat"]);
        let mut map = HashMap::new();

        for line in out.lines() {
            // Format: "<adds>\t<dels>\t<path>"
            // Binary shows '-' '-': treat as (0,0)
            let mut it = line.splitn(3, '\t');
            let a = it.next().unwrap_or("");
            let d = it.next().unwrap_or("");
            let p = it.next().unwrap_or("");
            if p.is_empty() { continue; }

            let adds = a.parse::<usize>().unwrap_or(0);
            let dels = d.parse::<usize>().unwrap_or(0);
            map.insert(p.to_string(), (adds, dels));
        }

        return map;
    }

    pub fn unstaged_line_counts(&self) -> HashMap<String, (usize, usize)> {
        let out = run_git(&self.workdir, ["diff", "--numstat"]);
        let mut map = HashMap::new();

        for line in out.lines() {
            let mut it = line.splitn(3, '\t');
            let a = it.next().unwrap_or("");
            let d = it.next().unwrap_or("");
            let p = it.next().unwrap_or("");
            if p.is_empty() { continue; }

            let adds = a.parse::<usize>().unwrap_or(0);
            let dels = d.parse::<usize>().unwrap_or(0);
            map.insert(p.to_string(), (adds, dels));
        }

        return map;
    }
}

pub fn run_git<I, S>(cwd: &Path, args: I) -> String
where
    I: IntoIterator<Item = S> + Debug,
    S: AsRef<OsStr>,
{
    let mut cmd = Command::new("git");
    cmd.current_dir(cwd).args(args);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let out = cmd.output().expect("Failed to execute git");
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        panic!("Git command failed:\n {}", stderr);
    }

    return String::from_utf8_lossy(&out.stdout).into_owned();
}
