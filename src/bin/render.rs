use crate::render;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::path::Path;
use std::process::exit;
use std::str;

fn exit_with_usage() -> ! {
    eprintln!("Usage: curly TMPLPATH [NAME=VAL ...]");
    exit(2);
}

fn main() {
    let mut args = env::args();
    args.next().unwrap();
    let tmplpath = match args.next() {
        Some(t) => t,
        None => exit_with_usage(),
    };

    let mut ctx = HashMap::new();
    for arg in args {
        let parts: Vec<_> = arg.splitn(2, '=').collect();
        if parts.len() != 2 {
            eprintln!("error: missing '=' in argument");
            exit_with_usage();
        }
        ctx.insert(parts[0].to_string(), parts[1].to_string());
    }

    let f = File::open(&tmplpath).unwrap_or_else(|e| {
        eprintln!("error: can't read file '{}' ({})", tmplpath, e);
        exit(1);
    });

    let mut out = Vec::new();
    match render(&f, &mut out, &ctx, Path::new(".")) {
        Ok(_) => (),
        Err(e) => {
            eprintln!("error: {}", e);
            exit(1);
        },
    }

    print!("{}", str::from_utf8(&out).unwrap());
}
