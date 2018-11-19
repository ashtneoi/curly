use std::io::prelude::*;
use std::str;

pub enum Result {
}

pub fn render(mut tmpl: impl Read, out: &mut impl Write) {
    let mut tmpl_all = String::new();
    tmpl.read_to_string(&mut tmpl_all).unwrap();
    let tmpl_all = tmpl_all;
    out.write(tmpl_all.as_bytes()).unwrap();
}

#[cfg(test)]
#[test]
fn test_simple_render() {
    let r = "hello there";
    let mut w = Vec::new();
    render(r.as_bytes(), &mut w);
    assert_eq!(str::from_utf8(&w).unwrap(), "hello there");
}
