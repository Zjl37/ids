use std::fmt::format;
use std::ptr::NonNull;

use pest::iterators::Pair;
use pest::Parser;

pub mod io;

#[derive(pest_derive::Parser)]
#[grammar = "ids.pest"]
pub struct IDSParser;

pub enum IDSExpr {
    Null,
    UnaryExpr {
        op: char,
        arg1: Box<IDSNode>,
    },
    BinExpr {
        op: char,
        arg1: Box<IDSNode>,
        arg2: Box<IDSNode>,
        op_arg: Option<String>,
    },
    TerExpr {
        op: char,
        arg1: Box<IDSNode>,
        arg2: Box<IDSNode>,
        arg3: Box<IDSNode>,
    },
}

pub struct IDSNode {
    pub ideographic: Option<char>,
    pub stroke_seq: Option<String>,
    pub glyph_hint: String,
    pub subtree: IDSExpr,
    pub glyph_variant: String,
}

pub struct IDS {
    pub root: IDSNode,
}

impl IDSNode {
    pub fn to_string_simp(&self, level: usize) -> String {
        if level == 1 && self.ideographic.is_some() {
            return self.ideographic.unwrap().to_string();
        }
        let lv_next = if level > 1 { level - 1 } else { level };
        match &self.subtree {
            IDSExpr::UnaryExpr { op, arg1 } => format!("{}{}", op, arg1.to_string_simp(lv_next)),
            IDSExpr::BinExpr { op, arg1, arg2, .. } => format!(
                "{}{}{}",
                op,
                arg1.to_string_simp(lv_next),
                arg2.to_string_simp(lv_next)
            ),
            IDSExpr::TerExpr {
                op,
                arg1,
                arg2,
                arg3,
            } => format!(
                "{}{}{}{}",
                op,
                arg1.to_string_simp(lv_next),
                arg2.to_string_simp(lv_next),
                arg3.to_string_simp(lv_next)
            ),
            IDSExpr::Null => "".to_string(),
        }
    }
    fn fmt_tree_r(&self, f: &mut std::fmt::Formatter<'_>, level: usize) -> std::fmt::Result {
        write!(f, "{:width$}* ", "", width = level * 4)?;
        match &self.subtree {
            IDSExpr::UnaryExpr { op, .. } => {
                write!(f, "{}", op)?;
            }
            IDSExpr::BinExpr {
                op,
                op_arg: overlay_arg,
                ..
            } => {
                write!(f, "{}", op)?;
                if let Some(s) = overlay_arg {
                    write!(f, "[{}]", s)?;
                }
            }
            IDSExpr::TerExpr { op, .. } => write!(f, "{}", op)?,
            _ => (),
        }
        write!(f, "\t")?;
        if let Some(ch) = self.ideographic {
            write!(f, "{}", ch)?;
            if self.glyph_variant.len() != 0 {
                write!(f, "({})", self.glyph_variant)?;
            }
            write!(f, " ")?
        }
        if let Some(s) = &self.stroke_seq {
            write!(f, "{} ", s)?;
        }
        if self.glyph_hint.len() > 0 {
            write!(f, "Hint: {{{}}}", self.glyph_hint)?;
        }
        writeln!(f)?;

        match &self.subtree {
            IDSExpr::UnaryExpr { arg1, .. } => arg1.fmt_tree_r(f, level + 1),
            IDSExpr::BinExpr { arg1, arg2, .. } => {
                arg1.fmt_tree_r(f, level + 1)?;
                arg2.fmt_tree_r(f, level + 1)
            }
            IDSExpr::TerExpr {
                op: _,
                arg1,
                arg2,
                arg3,
            } => {
                arg1.fmt_tree_r(f, level + 1)?;
                arg2.fmt_tree_r(f, level + 1)?;
                arg3.fmt_tree_r(f, level + 1)
            }
            _ => Ok(()),
        }
    }
}

impl std::fmt::Display for IDS {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ch) = self.root.ideographic {
            write!(f, "'{}' ", ch)?;
        }
        writeln!(f)?;
        self.root.fmt_tree_r(f, 0)
    }
}

fn parse_node_fr(pair: Pair<Rule>) -> IDSNode {
    match pair.as_rule() {
        Rule::ideographicA => IDSNode {
            ideographic: Some(pair.as_str().chars().next().unwrap()),
            stroke_seq: None,
            glyph_hint: "".to_string(),
            subtree: IDSExpr::Null,
            glyph_variant: match pair.into_inner().skip(1).next() {
                Some(p) => p.as_str().to_string(),
                None => "".to_string(),
            },
        },
        Rule::strokeExpr => {
            let mut inner_pairs = pair.into_inner();
            let mut hint = "";
            if inner_pairs.peek().unwrap().as_rule() == Rule::hint {
                hint = inner_pairs.next().unwrap().as_str();
                hint = &hint[1..hint.len() - 1];
            }
            IDSNode {
                ideographic: None,
                stroke_seq: Some(inner_pairs.next().unwrap().as_str().to_string()),
                glyph_hint: hint.to_string(),
                subtree: IDSExpr::Null,
                glyph_variant: "".to_string(),
            }
        }
        Rule::unaryExpr => {
            let mut inner_pairs = pair.into_inner();

            IDSNode {
                ideographic: None,
                stroke_seq: None,
                glyph_hint: "".to_string(),
                subtree: IDSExpr::UnaryExpr {
                    op: inner_pairs.next().unwrap().as_str().chars().next().unwrap(),
                    arg1: Box::new(parse_node_fr(inner_pairs.next().unwrap())),
                },
                glyph_variant: "".to_string(),
            }
        }
        Rule::binExpr => {
            let mut inner_pairs = pair.into_inner();
            let mut hint = "";
            if inner_pairs.peek().unwrap().as_rule() == Rule::hint {
                hint = inner_pairs.next().unwrap().as_str();
                hint = &hint[1..hint.len() - 1];
            }
            let pair_op = inner_pairs.next().unwrap();
            let ch = pair_op.as_str().chars().next().unwrap();
            let op_arg = pair_op
                .into_inner()
                .next()
                .map(|p| p.as_str())
                .map(|s| s[1..s.len() - 1].to_string());

            IDSNode {
                ideographic: None,
                stroke_seq: None,
                glyph_hint: hint.to_string(),
                subtree: IDSExpr::BinExpr {
                    op: ch,
                    arg1: Box::new(parse_node_fr(inner_pairs.next().unwrap())),
                    arg2: Box::new(parse_node_fr(inner_pairs.next().unwrap())),
                    op_arg,
                },
                glyph_variant: "".to_string(),
            }
        }
        Rule::terExpr => {
            let mut inner_pairs = pair.into_inner();
            let mut hint = "";
            if inner_pairs.peek().unwrap().as_rule() == Rule::hint {
                hint = inner_pairs.next().unwrap().as_str();
                hint = &hint[1..hint.len() - 1];
            }

            IDSNode {
                ideographic: None,
                stroke_seq: None,
                glyph_hint: hint.to_string(),
                subtree: IDSExpr::TerExpr {
                    op: inner_pairs.next().unwrap().as_str().chars().next().unwrap(),
                    arg1: Box::new(parse_node_fr(inner_pairs.next().unwrap())),
                    arg2: Box::new(parse_node_fr(inner_pairs.next().unwrap())),
                    arg3: Box::new(parse_node_fr(inner_pairs.next().unwrap())),
                },
                glyph_variant: "".to_string(),
            }
        }
        Rule::expr => parse_node_fr(pair.into_inner().next().unwrap()),
        _ => unreachable!(),
    }
}

fn extend_var_list<'a>(l: &Vec<&'a str>, v: &'a str) -> Vec<&'a str> {
    let mut ret = vec![v];
    ret.extend(l.iter().filter(|&&s| s != v));
    return ret;
}

pub fn extend_node(i: &mut IDSNode, var: &Vec<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let check_extend = |j: &mut Box<IDSNode>| -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ch) = j.ideographic {
            if let IDSExpr::Null = j.subtree {
                let node = create_node(ch, extend_var_list(var, &j.glyph_variant))?;
                *j = Box::new(node);
            }
        }
        Ok(())
    };
    match &mut i.subtree {
        IDSExpr::BinExpr {
            ref mut arg1,
            ref mut arg2,
            ..
        } => {
            check_extend(arg1)?;
            check_extend(arg2)
        }
        IDSExpr::TerExpr {
            op: _,
            ref mut arg1,
            ref mut arg2,
            ref mut arg3,
        } => {
            check_extend(arg1)?;
            check_extend(arg2)?;
            check_extend(arg3)
        }
        _ => Ok(()),
    }
}

pub fn create_node<'a>(ch: char, var: Vec<&str>) -> Result<IDSNode, Box<dyn std::error::Error>> {
    if let Some((v, s)) = io::query(ch, &var) {
        let ids0 = IDSParser::parse(Rule::expr, s)?.next().unwrap();
        let mut root = parse_node_fr(ids0);

        root.ideographic = Some(ch);
        root.glyph_variant = v.to_string();
        extend_node(&mut root, &var)?;
        return Ok(root);
    }

    Err("在 create_node 中 查询 IDS 失败".into())
}

pub fn create_tree(ch: char, var: Vec<&str>) -> Result<IDS, Box<dyn std::error::Error>> {
    Ok(IDS {
        root: create_node(ch, var)?,
    })
}
