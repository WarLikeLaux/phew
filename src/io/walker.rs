use std::path::{Path, PathBuf};

use rayon::prelude::*;

const FORMATTABLE_EXTENSIONS: [&str; 2] = ["php", "html"];

pub fn collect_files(paths: &[String]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for path in paths {
        let target = Path::new(path);
        match std::fs::metadata(target) {
            Ok(meta) if meta.is_dir() => {
                let mut found = walk_dir(target);
                found.par_sort();
                files.extend(found);
            }
            _ => files.push(target.to_path_buf()),
        }
    }
    files
}

fn walk_dir(dir: &Path) -> Vec<PathBuf> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => {
            eprintln!("Error reading {}: {e}", dir.display());
            return Vec::new();
        }
    };
    let children: Vec<PathBuf> = entries.filter_map(Result::ok).map(|entry| entry.path()).collect();
    children.par_iter().flat_map(|path| classify(path)).collect()
}

fn classify(path: &Path) -> Vec<PathBuf> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(_) => return Vec::new(),
    };
    if metadata.is_dir() {
        return walk_dir(path);
    }
    if is_formattable(path) {
        return vec![path.to_path_buf()];
    }
    Vec::new()
}

fn is_formattable(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| FORMATTABLE_EXTENSIONS.contains(&ext))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempTree {
        root: PathBuf,
    }

    impl TempTree {
        fn new(label: &str) -> Self {
            let root = std::env::temp_dir().join(format!("phew_walker_{}_{}", std::process::id(), label));
            std::fs::create_dir_all(&root).unwrap();
            Self { root }
        }

        fn touch(&self, rel: &str) {
            let path = self.root.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, "").unwrap();
        }

        fn arg(&self) -> String {
            self.root.to_string_lossy().into_owned()
        }
    }

    impl Drop for TempTree {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn collects_only_formattable_extensions_sorted() {
        let tree = TempTree::new("filter");
        tree.touch("b.php");
        tree.touch("a.html");
        tree.touch("notes.txt");
        tree.touch("Makefile");

        let files = collect_files(&[tree.arg()]);
        let names: Vec<String> = files
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();

        assert_eq!(names, vec!["a.html", "b.php"]);
    }

    #[test]
    fn recurses_into_nested_directories() {
        let tree = TempTree::new("nested");
        tree.touch("top.php");
        tree.touch("deep/inner/leaf.php");
        tree.touch("deep/skip.css");

        let files = collect_files(&[tree.arg()]);

        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|p| p.ends_with("top.php")));
        assert!(files.iter().any(|p| p.ends_with("deep/inner/leaf.php")));
    }

    #[test]
    fn explicit_file_argument_bypasses_extension_filter() {
        let tree = TempTree::new("explicit");
        tree.touch("raw");

        let arg = tree.root.join("raw").to_string_lossy().into_owned();
        let files = collect_files(&[arg]);

        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("raw"));
    }
}
