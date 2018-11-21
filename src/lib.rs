use std::collections::HashMap;
use std::fmt;
use std::io::prelude::*;

pub enum RenderError {
    MissingBrace(Pos),
}

impl fmt::Debug for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RenderError::MissingBrace(pos) =>
                write!(f, "Missing closing brace (opening brace at {:?})", pos),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Pos {
    raw: usize,
    col: usize,
    row: usize,
}

impl fmt::Debug for Pos {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}({})", self.row, self.col, self.raw)
    }
}

pub fn render(
    mut tmpl: impl Read,
    out: &mut impl Write,
    ctx: &HashMap<String, String>,
) -> Result<(), RenderError> {
    let mut tmpl_all = String::new();
    tmpl.read_to_string(&mut tmpl_all).unwrap();
    let tmpl_all = tmpl_all;
    let mut braces = Vec::new();
    let mut pos = Pos { raw: 0, col: 1, row: 1 };
    let mut chars = tmpl_all.chars();
    'outer: loop {
        let open;
        loop {
            let (c, width) = match chars.next() {
                Some(c) => (c, c.len_utf8()),
                None => break 'outer,
            };
            match c {
                '{' => {
                    open = pos;
                    pos.raw += width;
                    pos.col += 1;
                    break;
                },
                '\n' => {
                    pos.raw += width;
                    pos.row += 1;
                    pos.col = 1;
                },
                _ => {
                    pos.raw += width;
                    pos.col += 1;
                },
            }
        }

        let close;
        loop {
            let (c, width) = match chars.next() {
                Some(c) => (c, c.len_utf8()),
                None => return Err(RenderError::MissingBrace(open)),
            };
            match c {
                '}' => {
                    close = pos;
                    pos.raw += width;
                    pos.col += 1;
                    break;
                },
                '\n' => return Err(RenderError::MissingBrace(open)),
                _ => {
                    pos.raw += width;
                    pos.col += 1;
                },
            }
        }

        braces.push((open, close));
    }
    println!("{:?}", braces);
    out.write(tmpl_all.as_bytes()).unwrap();
    return Ok(());
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
        render(r.as_bytes(), &mut w, &HashMap::<String, String>::new())
            .unwrap();
        assert_eq!(str::from_utf8(&w).unwrap(), "hello there");
    }

    #[test]
    fn test_render() {
        let r = "hello {place}";
        let mut w = Vec::new();
        let mut h = HashMap::new();
        h.insert("place".to_string(), "there".to_string());
        render(r.as_bytes(), &mut w, &h).unwrap();
        assert_eq!(str::from_utf8(&w).unwrap(), "hello there");
    }
}
