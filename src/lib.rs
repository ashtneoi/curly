use std::collections::HashMap;
use std::fmt;
use std::io::prelude::*;

pub enum RenderError {
    MissingBrace(Pos),
    UndefinedName(Pos, String),
}

impl fmt::Debug for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RenderError::MissingBrace(pos) =>
                write!(f, "{:?}: Missing closing brace", pos),
            RenderError::UndefinedName(pos, name) =>
                write!(f, "{:?}: Name '{}' is undefined", pos, name),
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
    let mut replace = Vec::new();
    let mut pos = Pos { raw: 0, col: 1, row: 1 };
    let mut chars = tmpl_all.chars().peekable();
    'outer: loop {
        let replace_from;
        let name_from;
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
                        let bang_pos = pos;
                        pos.raw += '!'.len_utf8();
                        pos.col += 1;
                        replace.push((bang_pos, pos, None));
                    } else {
                        replace_from = pos;
                        pos.raw += width;
                        pos.col += 1;
                        name_from = pos;
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

        let replace_to;
        let name_to;
        loop {
            let (c, width) = match chars.next() {
                Some(c) => (c, c.len_utf8()),
                None => return Err(RenderError::MissingBrace(replace_from)),
            };
            match c {
                '}' => {
                    name_to = pos;
                    pos.raw += width;
                    pos.col += 1;
                    replace_to = pos;
                    break;
                },
                '\n' => return Err(RenderError::MissingBrace(replace_from)),
                _ => {
                    pos.raw += width;
                    pos.col += 1;
                },
            }
        }

        replace.push((replace_from, replace_to, Some((name_from, name_to))));
    }
    println!("replace: {:?}", replace);

    let mut maybe_prev: Option<Pos> = None;
    for (replace_from, replace_to, name) in replace {
        if let Some(prev) = maybe_prev {
            out.write(tmpl_all[prev.raw..replace_from.raw].as_bytes());
        } else {
            out.write(tmpl_all[0..replace_from.raw].as_bytes());
        }

        if let Some((name_from, name_to)) = name {
            let name = &tmpl_all[name_from.raw..name_to.raw];
            if let Some(val) = ctx.get(name) {
                out.write(val.as_bytes());
            } else {
                return Err(
                    RenderError::UndefinedName(name_from, name.to_string())
                );
            }
        }

        maybe_prev = Some(replace_to);
    }
    if let Some(prev) = maybe_prev {
        out.write(tmpl_all[prev.raw..tmpl_all.len()].as_bytes());
    } else {
        out.write(tmpl_all.as_bytes());
    }

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
    fn test_escape() {
        let r = "hello {!there}";
        let mut w = Vec::new();
        render(r.as_bytes(), &mut w, &HashMap::<String, String>::new())
            .unwrap();
        assert_eq!(str::from_utf8(&w).unwrap(), "hello {there}");
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
