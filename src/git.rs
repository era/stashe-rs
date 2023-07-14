use anyhow::Context;
use anyhow::Result;
use git2::DiffOptions;
use git2::Oid;
use git2::Repository;
use std::fs::File;

pub struct Repo {
    repo: Repository,
}

#[derive(Debug)]
pub struct Stash {
    index: usize,
    title: String,
    commit_id: Oid,
}

impl Stash {
    fn new(index: usize, title: String, commit_id: Oid) -> Self {
        Self {
            index,
            title,
            commit_id,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum LineDiff {
    // +
    Addition(String),
    // -
    Deletion(String),
    // =
    ContextEndOfAFile(String),
    // >
    AddEndOfAFile(String),
    // <
    RemoveEndOfAFile(String),
    // F
    FileHeader(String),
    // H
    HunkHeader(String),
    // B
    LineBinary(String),
    // ' '
    SameAsPrevious(String),
}

#[derive(Debug)]
pub struct StashDiff {
    pub diffs: Vec<LineDiff>,
    stash: Stash,
}

impl LineDiff {
    fn new(context: char, content: String) -> LineDiff {
        match context {
            '+' => Self::Addition(content),
            '-' => Self::Deletion(content),
            '=' => Self::ContextEndOfAFile(content),
            '>' => Self::AddEndOfAFile(content),
            '<' => Self::RemoveEndOfAFile(content),
            'F' => Self::FileHeader(content),
            'H' => Self::HunkHeader(content),
            'B' => Self::LineBinary(content),
            _ => Self::SameAsPrevious(content),
        }
    }
}

impl StashDiff {
    fn new(diffs: Vec<LineDiff>, stash: Stash) -> Self {
        Self { diffs, stash }
    }

    pub fn title(&self) -> &str {
        &self.stash.title
    }

    pub fn index(&self) -> usize {
        self.stash.index
    }
}

impl Repo {
    pub fn new(path: &str) -> Result<Self> {
        let repo = Repository::init(path).context("could not create a repo object")?;
        Ok(Self { repo })
    }

    pub fn stashes(&mut self) -> Result<Vec<StashDiff>> {
        let stashes = self.stash_show()?;
        let mut result = Vec::new();
        for s in stashes {
            result.push(StashDiff::new(self.diff(&s)?, s));
        }

        Ok(result)
    }

    fn stash_show(&mut self) -> Result<Vec<Stash>> {
        let mut stashes = Vec::new();
        self.repo
            .stash_foreach(|index: usize, title: &str, id: &Oid| {
                stashes.push(Stash::new(index, title.to_string(), id.clone()));
                true
            })
            .context("could not iterate on the stashes")?;

        Ok(stashes)
    }

    fn diff(&mut self, stash: &Stash) -> Result<Vec<LineDiff>> {
        let stash_commit = self
            .repo
            .find_commit(stash.commit_id)
            .context("Failed to find stash commit")?;
        let stash_tree = self
            .repo
            .find_tree(stash_commit.tree_id())
            .context("Failed to find stash tree")?;

        let diff = self
            .repo
            .diff_tree_to_workdir_with_index(
                Some(&stash_tree),
                Some(DiffOptions::new().reverse(true)),
            )
            .context("Failed to get diff")?;

        let mut diffs = Vec::new();

        diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
            let l = LineDiff::new(
                line.origin(),
                std::str::from_utf8(line.content()).unwrap().into(),
            );
            diffs.push(l);
            true
        })
        .context("could not get print from diffs")?;

        Ok(diffs)
    }

    pub fn stash(&mut self, msg: &str) -> Result<()> {
        let signature = match self.repo.signature() {
            Ok(s) => s,
            _ => git2::Signature::now("stash-rs application", "stashapp")
                .context("could not create signature")?,
        };

        self.repo
            .stash_save(&signature, msg, None)
            .context("could not stash")?;
        Ok(())
    }

    pub fn stash_apply(&mut self, stash: &StashDiff) -> Result<()> {
        self.repo
            .stash_pop(stash.stash.index, None)
            .context("could not stash pop the index to apply")?;
        Ok(())
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use std::io::Write;
    use tempdir::TempDir;

    fn commit_all(repo: &mut Repository) {
        let mut index = repo.index().unwrap();
        index
            .add_all(&["."], git2::IndexAddOption::DEFAULT, None)
            .unwrap();
        let oid = index.write_tree().unwrap();
        let signature = git2::Signature::now("stash-rs application", "stashapp").unwrap();
        let tree = repo.find_tree(oid).unwrap();
        let msg: &str = "Inital Commit";
        repo.commit(Some("HEAD"), &signature, &signature, &msg, &tree, &[])
            .unwrap();
    }

    #[test]
    fn liststash() {
        let tmpdir = TempDir::new("myrepo").unwrap();
        let mut repo = Repository::init(tmpdir.path())
            .context("could not create a repo object")
            .unwrap();
        let mut file = File::create(tmpdir.path().join("a.txt")).unwrap();

        commit_all(&mut repo);
        file.write_all("Hello, world!".as_bytes()).unwrap();

        let mut repo = Repo::new(tmpdir.path().as_os_str().to_str().unwrap()).unwrap();
        repo.stash("this is a test").unwrap();

        let stashes = repo.stashes().unwrap();

        assert!(stashes.len() == 1);

        let stash = stashes.get(0).unwrap();
        assert_eq!("On master: this is a test", stash.title());

        assert_eq!(&LineDiff::FileHeader("diff --git b/a.txt a/a.txt\nindex e69de29..5dd01c1 100644\n--- b/a.txt\n+++ a/a.txt\n".into())
                , stash.diffs.get(0).unwrap());

        assert_eq!(
            &LineDiff::HunkHeader("@@ -0,0 +1 @@\n".into()),
            stash.diffs.get(1).unwrap()
        );

        assert_eq!(
            &LineDiff::Addition("Hello, world!".into()),
            stash.diffs.get(2).unwrap()
        );

        repo.stash_apply(stash).unwrap();
        repo.stash("this is a test 2").unwrap();
        let stashes = repo.stashes().unwrap();
        assert!(stashes.len() == 1);
        let stash = stashes.get(0).unwrap();
        assert_eq!("On master: this is a test 2", stash.title());
    }
}
