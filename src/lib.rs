use std::collections::HashMap;
use std::io::prelude::*;
use std::ops::Index;

pub enum Result {
}

pub fn render(
    mut tmpl: impl Read,
    out: &mut impl Write,
    ctx: &HashMap<String, String>,
) {
    let mut tmpl_all = String::new();
    tmpl.read_to_string(&mut tmpl_all).unwrap();
    let tmpl_all = tmpl_all;
    out.write(tmpl_all.as_bytes()).unwrap();
}

#[cfg(test)]
mod test {
    use crate::render;
    use std::collections::HashMap;
    use std::str;

    #[test]
    fn test_simple_render() {
        let r = "hello there";
        let mut w = Vec::new();
        render(r.as_bytes(), &mut w, &HashMap::<String, String>::new());
        assert_eq!(str::from_utf8(&w).unwrap(), "hello there");
    }

    #[test]
    fn test_render() {
        let r = "hello {place}";
        let mut w = Vec::new();
        let mut h = HashMap::new();
        h.insert("place".to_string(), "there".to_string());
        render(r.as_bytes(), &mut w, &h);
        assert_eq!(str::from_utf8(&w).unwrap(), "hello there");
    }
}
