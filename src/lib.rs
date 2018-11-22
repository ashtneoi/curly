use std::collections::HashMap;
use std::fmt;
use std::io;
use std::io::prelude::*;

pub enum RenderError {
    MissingBrace(Pos),
    MissingHash(Pos),
    MissingColon(Pos),
    UndefinedName(Pos, String),
    IoError(io::Error),
}

impl fmt::Debug for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RenderError::MissingBrace(pos) =>
                write!(f, "{:?}: Missing closing brace", pos),
            RenderError::MissingHash(pos) =>
                write!(f, "{:?}: Missing hash", pos),
            RenderError::MissingColon(pos) =>
                write!(f, "{:?}: Missing colon", pos),
            RenderError::UndefinedName(pos, name) =>
                write!(f, "{:?}: Name '{}' is undefined", pos, name),
            RenderError::IoError(err) =>
                write!(f, "IO error: {:?}", err),
        }
    }
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct Pos {
    raw: usize,
    col: usize,
    row: usize,
}

impl fmt::Display for Pos {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}({})", self.row, self.col, self.raw)
    }
}

impl fmt::Debug for Pos {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

#[derive(Debug)]
enum Token {
    Tag(Pos, Pos),
    Bang(Pos),
}

#[derive(Debug)]
enum Tag {
    Var(Pos, Pos),
    File(Pos, Pos, Pos),
    Def(Pos, Pos, Pos),
}

#[derive(Debug)]

#[derive(Debug)]
enum Replacement {
    Null,
    Var(String, Option<String>),
    File(String),
}

// {foo}x{:bar:}y{!
// tokens: {foo} {:bar:} !
// tags: {foo} {:bar:}
// labels: foo :bar:
pub fn render(
    mut tmpl: impl Read,
    out: &mut impl Write,
    ctx: &HashMap<String, String>,
) -> Result<(), RenderError> {
    let mut tmpl_all = String::new();
    tmpl.read_to_string(&mut tmpl_all).unwrap();
    let tmpl_all = tmpl_all;

    // Gather tokens.

    let mut tokens = Vec::new();
    let mut pos = Pos { raw: 0, col: 1, row: 1 };
    let mut chars = tmpl_all.chars().peekable();
    'outer: loop {
        let tag_from;
        loop {
            let (c, width) = match chars.next() {
                Some(c) => (c, c.len_utf8()),
                None => break 'outer,
            };
            match c {
                '{' => {
                    let escaped = chars.peek().map_or(false, |c| *c == '!');
                    if escaped {
                        pos.raw += width;
                        pos.col += 1;
                        tokens.push(Token::Bang(pos));
                    } else {
                        tag_from = pos;
                        pos.raw += width;
                        pos.col += 1;
                        break;
                    }
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

        let tag_to;
        loop {
            let (c, width) = match chars.next() {
                Some(c) => (c, c.len_utf8()),
                None => return Err(RenderError::MissingBrace(tag_from)),
            };
            match c {
                '}' => {
                    name_to = pos;
                    pos.raw += width;
                    pos.col += 1;
                    replace_to = pos;
                    break;
                },
                '{' => return Err(RenderError::MissingBrace(pos)),
                '\n' => return Err(RenderError::MissingBrace(tag_from)),
                _ => {
                    pos.raw += width;
                    pos.col += 1;
                },
            }
        }

        tokens.push(Token::Tag(tag_from, tag_to));
    }
    let tokens = tokens;

    // Gather replacements.

    let mut replace = Vec::new();
    let mut tokens = tokens.iter();
    let mut tag_stack = Vec::new();
    let mut ctx_stack = Vec::new();
    loop {
        match tokens.next() {
            Some(Tag(tag_from, tag_to)) => {
                let label_from = tag_from + '{'.len_utf8();
                let label_to = tag_from - '}'.len_utf8();
                let label = tmpl_all[tag_from..tag_to].to_string();
                if label.starts_with('#') {
                    if !label.ends_with('#') {
                        return Err(RenderError::MissingHash(tag_to));
                    }
                    let name_from = '#'.len_utf8();
                    let name_to = label.len() - '#'.len_utf8();
                    tag_stack.push(Tag::File(
                        label[name_from..name_to].to_string(),
                    ));
                } else if label.starts_with(':') {
                    if !label.ends_with(':') {
                        return Err(RenderError::MissingColon(tag_to));
                    }
                    let name_from = ':'.len_utf8();
                    let name_to = label.len() - ':'.len_utf8();
                    tag_stack.push(Tag::Def(
                        label[name_from..name_to].to_string(),
                    ));
                }
            },
        }
    }
    let replace = replace;

    // Perform replacements.

    let mut maybe_prev: Option<Pos> = None;
    for (replace_from, replace_to, name) in replace {
        if let Some(prev) = maybe_prev {
            out.write(tmpl_all[prev.raw..replace_from.raw].as_bytes())
                .map_err(|e| RenderError::IoError(e))?;
        } else {
            out.write(tmpl_all[0..replace_from.raw].as_bytes())
                .map_err(|e| RenderError::IoError(e))?;
        }

        if let Some((name_from, name_to)) = name {
            let name = &tmpl_all[name_from.raw..name_to.raw];
            if let Some(val) = ctx.get(name) {
                out.write(val.as_bytes())
                    .map_err(|e| RenderError::IoError(e))?;
            } else {
                return Err(
                    RenderError::UndefinedName(name_from, name.to_string())
                );
            }
        }

        maybe_prev = Some(replace_to);
    }
    if let Some(prev) = maybe_prev {
        out.write(tmpl_all[prev.raw..tmpl_all.len()].as_bytes())
            .map_err(|e| RenderError::IoError(e))?;
    } else {
        out.write(tmpl_all.as_bytes())
            .map_err(|e| RenderError::IoError(e))?;
    }

    return Ok(());
}

#[cfg(test)]
mod test {
    use crate::{Pos, render, RenderError};
    use std::collections::HashMap;
    use std::str;

    #[test]
    fn test_simple_render() {
        let r = "hello there.";
        let mut w = Vec::new();
        render(r.as_bytes(), &mut w, &HashMap::<String, String>::new())
            .unwrap();
        assert_eq!(str::from_utf8(&w).unwrap(), "hello there.");
    }

    #[test]
    fn test_escape() {
        let r = "hello {!there}.";
        let mut w = Vec::new();
        render(r.as_bytes(), &mut w, &HashMap::<String, String>::new())
            .unwrap();
        assert_eq!(str::from_utf8(&w).unwrap(), "hello {there}.");
    }

    #[test]
    fn test_render() {
        let r = "hello {place}.";
        let mut w = Vec::new();
        let mut h = HashMap::new();
        h.insert("place".to_string(), "there".to_string());
        render(r.as_bytes(), &mut w, &h).unwrap();
        assert_eq!(str::from_utf8(&w).unwrap(), "hello there.");

        let r = "hello {place.";
        let mut w = Vec::new();
        match render(r.as_bytes(), &mut w, &HashMap::new()).unwrap_err() {
            RenderError::MissingBrace(
                Pos { raw: 6, row: 1, col: 7 },
            ) => (),
            _ => panic!(),
        }

        let r = "hello {place\n}.";
        let mut w = Vec::new();
        match render(r.as_bytes(), &mut w, &HashMap::new()).unwrap_err() {
            RenderError::MissingBrace(
                Pos { raw: 6, row: 1, col: 7 },
            ) => (),
            _ => panic!(),
        }

        let r = "hello {{";
        let mut w = Vec::new();
        match render(r.as_bytes(), &mut w, &HashMap::new()).unwrap_err() {
            RenderError::MissingBrace(
                Pos { raw: 7, row: 1, col: 8 },
            ) => (),
            _ => panic!(),
        }

        let r = "hello {place{.";
        let mut w = Vec::new();
        match render(r.as_bytes(), &mut w, &HashMap::new()).unwrap_err() {
            RenderError::MissingBrace(
                Pos { raw: 12, row: 1, col: 13 },
            ) => (),
            _ => panic!(),
        }

        let r = "hello {place}.";
        let mut w = Vec::new();
        let mut h = HashMap::new();
        h.insert("face".to_string(), "there".to_string());
        match render(r.as_bytes(), &mut w, &h).unwrap_err() {
            RenderError::UndefinedName(
                Pos { raw: 7, row: 1, col: 8 },
                place,
            ) => assert_eq!(place, "place"),
            _ => panic!(),
        }

        let r = "{aa}bb{cc}\ndd{ee}";
        let mut w = Vec::new();
        let mut h = HashMap::new();
        h.insert("aa".to_string(), "AA".to_string());
        h.insert("cc".to_string(), "CC".to_string());
        h.insert("ee".to_string(), "EE".to_string());
        render(r.as_bytes(), &mut w, &h).unwrap();
        assert_eq!(str::from_utf8(&w).unwrap(), "AAbbCC\nddEE");
    }
}
