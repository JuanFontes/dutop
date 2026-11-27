#![allow(dead_code)]
use std::{fs, io::Write, path::Path};

pub fn file(root: &Path, name: &str, size: usize) {
    let path = root.join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut output = fs::File::create(path).unwrap();
    output.write_all(&vec![0x61; size]).unwrap();
    output.sync_all().unwrap();
}
