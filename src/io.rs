use once_cell::sync::Lazy;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead};

thread_local!(
    static IDS_FILE_LIST: RefCell<Vec<String>> =
        RefCell::new(vec!["./ids/ids_lv0.txt".to_string()]);
);

pub fn set_ids_file_list(v: Vec<String>) {
    IDS_FILE_LIST.with(|fl| {
        *fl.borrow_mut() = v;
    });
}

pub fn add_ids_file(f: String) {
    IDS_FILE_LIST.with(|fl| {
        fl.borrow_mut().push(f);
    });
}

fn ids_split_var(s: &str) -> Box<dyn Iterator<Item = (&str, &str)> + '_> {
    if s.ends_with(')') {
        if let Some(p) = s.rfind('(') {
            if s.get(p - 1..p) != Some("#") {
                return Box::new(
                    s[p + 1..s.len() - 1]
                        .split(',')
                        .map(move |var| (var, &s[0..p])),
                );
            }
        } else {
            eprintln!("ids 语法可能有误：{}", s);
        }
    }
    return Box::new(vec![("", s)].into_iter());
}

pub fn load() -> Result<HashMap<char, HashMap<String, String>>, io::Error> {
    let mut ids_map = HashMap::new();

    let mut process_1 = |f: io::BufReader<_>| {
        for ln in f.lines() {
            if let Ok(ln) = ln {
                if ln.starts_with("*") {
                    continue;
                }

                let v: Vec<&str> = ln.split('\t').collect();
                if v.len() < 2 {
                    continue;
                }
                let mut it = v[0].chars();
                let ch = it.next();
                if ch.is_none() {
                    continue;
                }
                let ch = ch.unwrap();
                if it.next().is_some() {
                    eprintln!("ids 文件有误：'{}'不是一个字", v[0]);
                    continue;
                }

                let hm = ids_map.entry(ch).or_insert(HashMap::new());
                for seq in v[1].split(';').map(ids_split_var) {
                    for (var, ids) in seq {
                        hm.insert(var.to_string(), ids.to_string());
                    }
                }
            }
        }
    };

    IDS_FILE_LIST.with(|fl| -> Result<(), io::Error> {
        for filename in fl.borrow().iter() {
            let file = fs::File::open(filename)?;
            process_1(io::BufReader::new(file));
        }
        Ok(())
    })?;

    eprintln!("成功加载全部 ids 数据，一共有 {} 条记录", ids_map.len());

    Ok(ids_map)
}

static IDS_MAP: Lazy<HashMap<char, HashMap<String, String>>> = Lazy::new(|| load().unwrap());

pub fn query(ch: char, variant: &Vec<&str>) -> Option<(&'static str, &'static str)> {
    if let Some(vars_avail) = IDS_MAP.get(&ch) {
        let tuple2str = |p: (&'static String, &'static String)| (p.0.as_str(), p.1.as_str());
        let get_1 = || vars_avail.iter().next().map(tuple2str);

        if vars_avail.len() <= 1 {
            return get_1();
        }

        for &var in variant {
            if let Some((v, s)) = vars_avail.get_key_value(var) {
                return Some((v.as_str(), s.as_str()));
            }
        }

        let ret = vars_avail.get_key_value("").map(tuple2str).or_else(get_1);
        if let Some((v, _)) = ret {
            eprintln!(
                "找不到'{}'字的({})字形，将使用({})字形。",
                ch,
                variant.join(","),
                v
            );
        }
        return ret;
    }
    None
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    #[test]
    fn all_tree() -> Result<(), Box<dyn std::error::Error>> {
        use crate::ids;

        for (i, (ch, hm)) in IDS_MAP.iter().enumerate() {
            eprintln!("===== {:6} / {:6} ===== ", i + 1, IDS_MAP.len());
            let _ = io::stdout().flush();
            for (var, s) in hm {
                println!("==> {}", s);
                println!("{}", ids::create_tree(*ch, vec![var.as_str()])?);
                println!();
            }
            println!()
        }

        Ok(())
    }
}
