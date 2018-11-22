use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::ops;
use std::str;

pub enum RenderError {
    MissingBrace(Pos),
    MissingHash(Pos),
    MissingColon(Pos),
    MissingClosingTag(Pos),
    UnexpectedClosingTag(Pos),
    UndefinedName(Pos, String),
    IoError(io::Error),
}

impl fmt::Debug for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RenderError::MissingBrace(pos) =>
                write!(f, "{}: Missing closing brace", pos),
            RenderError::MissingHash(pos) =>
                write!(f, "{}: Missing hash", pos),
            RenderError::MissingColon(pos) =>
                write!(f, "{}: Missing colon", pos),
            RenderError::MissingClosingTag(pos) =>
                write!(f, "{}: Missing corresponding closing tag", pos),
            RenderError::UnexpectedClosingTag(pos) =>
                write!(f, "{}: Unexpected closing tag", pos),
            RenderError::UndefinedName(pos, name) =>
                write!(f, "{}: Name '{}' is undefined", pos, name),
            RenderError::IoError(err) =>
                write!(f, "IO error: {}", err),
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

impl ops::Add<char> for Pos {
    type Output = Self;

    fn add(self, rhs: char) -> Self::Output {
        Pos {
            raw: self.raw + rhs.len_utf8(),
            row: self.row + if rhs == '\n' { 1 } else { 0 },
            col: if rhs == '\n' { 1 } else { self.col + 1 },
        }
    }
}

impl ops::Sub<char> for Pos {
    type Output = Self;

    fn sub(self, rhs: char) -> Self::Output {
        if rhs == '\n' {
            panic!("can't do that");
        }

        Pos {
            raw: self.raw - rhs.len_utf8(),
            row: self.row,
            col: self.col - 1,
        }
    }
}

#[derive(Debug)]
enum Token {
    Tag(Pos, Pos),
    Bang(Pos),
}

#[derive(Debug)]
enum ItemKind {
    File,
    Def,
}

#[derive(Debug)]
struct Item {
    from: Pos,
    to: Pos, // TODO: Don't like how we don't set this for # and : tags.
    kind: ItemKind,
    name: String,
    ctx: HashMap<String, String>,
    replace: Vec<(Pos, Pos, String)>,
}

fn replace(
    tmpl_all: &str,
    (from, to): (Pos, Pos),
    replace: Vec<(Pos, Pos, String)>,
) -> String {
    let mut s = String::new();

    let mut maybe_prev: Option<Pos> = None;
    for (replace_from, replace_to, replacement) in replace {
        if let Some(prev) = maybe_prev {
            s.push_str(&tmpl_all[prev.raw..replace_from.raw]);
        } else {
            s.push_str(&tmpl_all[from.raw..replace_from.raw]);
        }

        s.push_str(&replacement);

        maybe_prev = Some(replace_to);
    }
    if let Some(prev) = maybe_prev {
        s.push_str(&tmpl_all[prev.raw..to.raw]);
    } else {
        s.push_str(&tmpl_all[from.raw..to.raw]);
    }

    s
}

fn ctx_get(item_stack: &Vec<Item>, name: &str) -> Option<String> {
    for item in item_stack.iter().rev() {
        println!("{:?}", item.ctx);
        match item.ctx.get(name) {
            Some(val) => return Some(val.to_string()),
            None => (),
        }
    }

    None
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
                    pos.raw += width;
                    pos.col += 1;
                    tag_to = pos;
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
    let tmpl_to = pos;
    let tokens = tokens;

    // Gather replacements.

    let mut tokens = tokens.iter();
    let mut item_stack = vec![Item {
        from: Pos { raw: 0, row: 1, col: 1 },
        to: tmpl_to,
        kind: ItemKind::Def,
        name: "".to_string(),
        ctx: ctx.clone(),
        replace: Vec::new(),
    }];
    loop {
        match tokens.next() {
            Some(&Token::Tag(tag_from, tag_to)) => {
                let label_from = tag_from + '{';
                let label_to = tag_to - '}';
                let label = &tmpl_all[label_from.raw..label_to.raw];
                if label.starts_with('#') {
                    if !label.ends_with('#') {
                        return Err(RenderError::MissingHash(tag_to));
                    }
                    if label.len() == 1 { // {#}
                        if item_stack.len() == 1 {
                            return Err(
                                RenderError::UnexpectedClosingTag(tag_from)
                            );
                        }
                        let cur = item_stack.pop().unwrap();
                        match cur.kind {
                            ItemKind::File => (),
                            _ => return Err(
                                RenderError::UnexpectedClosingTag(tag_from)
                            ),
                        }
                        let mut w = Vec::new();
                        let f = File::open(cur.name).map_err(
                            |e| RenderError::IoError(e)
                        )?;
                        render(&f, &mut w, &cur.ctx)?;
                        item_stack.last_mut().unwrap().replace.push((
                            cur.from,
                            tag_to,
                            String::from_utf8(w).unwrap(),
                        ));
                        // Replacements don't matter.
                    } else { // {#name#}
                        let name_from = '#'.len_utf8();
                        let name_to = label.len() - '#'.len_utf8();
                        item_stack.push(Item {
                            from: tag_from,
                            to: tag_from, // doesn't matter
                            kind: ItemKind::File,
                            name: label[name_from..name_to].to_string(),
                            ctx: HashMap::new(),
                            replace: Vec::new(),
                        });
                    }
                } else if label.starts_with(':') {
                    if !label.ends_with(':') {
                        return Err(RenderError::MissingColon(tag_to));
                    }
                    if label.len() == 1 { // {:}
                        if item_stack.len() == 1 {
                            return Err(
                                RenderError::UnexpectedClosingTag(tag_from)
                            );
                        }
                        let cur = item_stack.pop().unwrap();
                        item_stack.last_mut().unwrap().ctx.insert(
                            label.to_string(),
                            replace(&tmpl_all, (cur.from, tag_to), cur.replace),
                        );
                    } else { // {:name:}
                        let name_from = ':'.len_utf8();
                        let name_to = label.len() - ':'.len_utf8();
                        item_stack.push(Item {
                            from: tag_from,
                            to: tag_from, // doesn't matter
                            kind: ItemKind::Def,
                            name: label[name_from..name_to].to_string(),
                            ctx: HashMap::new(),
                            replace: Vec::new(),
                        });
                    }
                } else {
                    let val = match ctx_get(&item_stack, label) {
                        Some(v) => v,
                        None => return Err(RenderError::UndefinedName(
                            label_from,
                            label.to_string(),
                        )),
                    };
                    item_stack.last_mut().unwrap().replace.push((
                        tag_from,
                        tag_to,
                        val,
                    ));
                }
            },
            Some(&Token::Bang(pos)) => {
                let to = Pos {
                    raw: pos.raw + '!'.len_utf8(),
                    row: pos.row,
                    col: pos.col + 1,
                };
                item_stack.last_mut().unwrap().replace.push((
                    pos,
                    to,
                    "".to_string(),
                ));
            },
            None => break, // FIXME: so obviously this is just a for loop
        }
    }
    assert!(item_stack.len() >= 1);
    if item_stack.len() > 1 {
        return Err(RenderError::MissingClosingTag(
            item_stack.last().unwrap().from
        ));
    }

    let cur = item_stack.pop().unwrap();
    out.write(replace(
        &tmpl_all,
        (cur.from, cur.to),
        cur.replace,
    ).as_bytes())
        .map_err(|e| RenderError::IoError(e))?;

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
            e => panic!("{:?}", e),
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
