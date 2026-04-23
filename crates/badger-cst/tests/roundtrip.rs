use badger_cst::parse;
use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn roundtrips_corpus_files() {
    for path in corpus_files() {
        let source = fs::read_to_string(&path).unwrap();
        let parse = parse(&source).unwrap_or_else(|error| {
            panic!(
                "failed to parse {} at byte {}: {}",
                path.display(),
                error.offset,
                error.message
            )
        });
        assert_eq!(
            parse.serialize(),
            source,
            "round-trip failed for {}",
            path.display()
        );
    }
}

fn corpus_files() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap();
    let mut files = Vec::new();
    gather_badger_files(&root.join("examples"), &mut files);
    gather_badger_files(&root.join("lib/std"), &mut files);
    files.sort();
    files
}

fn gather_badger_files(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            gather_badger_files(&path, files);
        } else if path.extension().is_some_and(|ext| ext == "badger") {
            files.push(path);
        }
    }
}
