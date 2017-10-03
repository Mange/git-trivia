use std::path::{Path, PathBuf};

use git2::{Repository, Tree, TreeEntry, ObjectType, Oid, Blob};

pub struct TreeWalker<'repo> {
    repo: &'repo Repository,
    tree_stack: Vec<Tree<'repo>>,
    cursor_stack: Vec<usize>,
    path_stack: PathBuf,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Entry {
    id: Oid,
    path: PathBuf,
    kind: EntryKind,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum EntryKind {
    File,
    Directory,
}

impl Entry {
    fn new<'repo>(path: &Path, tree_entry: TreeEntry<'repo>) -> Entry {
        let kind = match tree_entry.kind() {
            Some(ObjectType::Tree) => EntryKind::Directory,
            Some(ObjectType::Blob) => EntryKind::File,
            _ => unreachable!("Tree entries should always be either Trees or Blobs"),
        };

        let path = path.join(tree_entry.name().unwrap());

        Entry {
            id: tree_entry.id(),
            path: path,
            kind: kind,
        }
    }

    pub fn kind(&self) -> EntryKind {
        self.kind
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn is_dir(&self) -> bool {
        self.kind == EntryKind::Directory
    }

    pub fn is_file(&self) -> bool {
        self.kind == EntryKind::File
    }

    pub fn blob<'repo>(&self, repo: &'repo Repository) -> Option<Blob<'repo>> {
        match self.kind {
            EntryKind::File => repo.find_blob(self.id).ok(),
            EntryKind::Directory => None,
        }
    }

    fn tree<'repo>(&self, repo: &'repo Repository) -> Option<Tree<'repo>> {
        match self.kind {
            EntryKind::Directory => repo.find_tree(self.id).ok(),
            EntryKind::File => None,
        }
    }
}

impl<'repo> TreeWalker<'repo> {
    pub fn new(repo: &'repo Repository, tree: Tree<'repo>) -> TreeWalker<'repo> {
        TreeWalker {
            repo: repo,
            cursor_stack: vec![0],
            tree_stack: vec![tree],
            path_stack: PathBuf::new(),
        }
    }
}

impl<'repo> Iterator for TreeWalker<'repo> {
    type Item = Entry;

    fn next(&mut self) -> Option<Entry> {
        if self.cursor_stack.is_empty() {
            return None;
        }

        // Popping the values and then pushing them back before returning means we get around a lot
        // of ownership issues; getting a reference means we do a borrow of the Vec so we are not
        // able to mutate it further down in the method.
        let current_index = self.cursor_stack.pop().unwrap();
        let current_tree = self.tree_stack.pop().unwrap();

        if current_index < current_tree.len() {
            let entry = Entry::new(&self.path_stack, current_tree.get(current_index).unwrap());

            // Restore current position, but advanced by 1
            self.tree_stack.push(current_tree);
            self.cursor_stack.push(current_index + 1);

            // Recurse into directory for next iteration, if directory
            if let Some(tree) = entry.tree(self.repo) {
                self.path_stack.push(entry.path.file_name().unwrap());
                self.tree_stack.push(tree);
                self.cursor_stack.push(0);
            }

            Some(entry)
        } else {
            self.path_stack.pop();
            self.next()
        }
    }
}

#[test]
fn it_iterates_all_files() {
    let context = super::Context::load().unwrap();
    let tree = context.head_commit().unwrap().tree().unwrap();

    let walker = TreeWalker::new(context.repo(), tree);

    for entry in walker {
        // If this fails with a panic the output will be shown, but not otherwise. This makes for a
        // good debugging message.
        println!("{:?}", entry);
    }
}
