extern crate regex;
extern crate rustc_serialize;

use regex::Regex;
use regex::Captures;

use rustc_serialize::json::Json;
use std::iter::*;


pub fn normalize (expression: &str) -> Vec<String> {
    let mut matches: Vec<String> = vec![];
    let mut re = Regex::new(r"[\['](\??(.*?))[\]']").unwrap();
    let mut norm = re.replace_all(expression, |caps: &Captures| {
        let result = format!("[#{}]", matches.len());
        matches.push(caps.at(1).unwrap().to_string());
        result
    });

    re = Regex::new(r"'?\.'?|\['?").unwrap();
    norm = re.replace_all(norm.as_ref(), ";");

    re = Regex::new(r"(?:;)?(\^+)(?:;)?").unwrap();
    norm = re.replace_all(norm.as_ref(), |caps: &Captures| {
        let c: Vec<char> = caps.at(1).unwrap().chars().collect();
        let cs: Vec<String> = c.iter().map(|x| x.to_string()).collect();

        format!(";{};", cs.connect(";"))
    });

    re = Regex::new(r";;;|;;").unwrap();
    norm = re.replace_all(norm.as_ref(), ";..;");

    re = Regex::new(r";$|'?\]|'$").unwrap();
    norm = re.replace_all(norm.as_ref(), "");

    return norm.to_string().split(';').map(|expr| {
        let re2 = Regex::new("#([0-9]+)").unwrap();
        match re2.captures(expr) {
            Some(caps) => {
                let idx: usize = ::std::str::FromStr::from_str(caps.at(1).unwrap()).unwrap();
                matches[idx].to_string()
            },
            None => expr.to_string()
        }
    }).collect();
}

#[derive(Debug, Clone)]
pub struct JsonResult<'a> {
    path: Vec<&'a str>,
    object: Vec<Json>
}

#[derive(Debug, Clone)]
pub struct JsonPath<'a> {
    pattern: Vec<&'a str>,
    store: JsonResult<'a>
}

impl<'a> JsonPath<'a> {
    fn append (&mut self, stores: &Vec<Json>) -> () {
        for obj in stores.iter() {
            self.store.object.push(obj.clone());
        }
    }

    pub fn trace(&mut self, obj: &Json) {
        if self.pattern.len() == 0 {
            self.store = JsonResult { path: vec![], object: vec![obj.clone()] };
            return
        }

        match self.pattern[0].clone().as_ref() {
            "*" => self.all_item(obj),
            ".." => self.collect(obj),
            loc if obj.find(loc.as_ref()).is_some() => { // オブジェクトの時のキーからの値取得
                let a = obj.find(loc.as_ref());
                self.fetch(a.unwrap())
            },
            loc if obj.is_array() => { // 配列の時のキーからの値取得
                let number: Result<usize, ::std::num::ParseIntError> = ::std::str::FromStr::from_str(loc.as_ref());
                let a = obj.as_array().unwrap();

                match number {
                    Ok(idx) => self.fetch(&a[idx]),
                    Err(_)=> ()
                }
            }
            _ => ()
        }
    }

    fn traversal<F> (&mut self, patterns: &Vec<&str>, obj: &Json, mut f: F)
        where F: FnMut(&str, &Vec<&str>, &Json) {
        match *obj {
            Json::Array(ref arr) => {
                for i in 0..arr.len() {
                    f(i.to_string().as_ref(), patterns, obj)
                }
            },
            Json::Object(ref o) => {
                for (i, _) in o.iter() {
                    f(i, patterns, obj)
                }
            },
            _ => ()
        }
    }

    fn collect (&mut self, obj: &Json) {

        let mut new_pattern = Vec::new();
        {
            let tail = &self.pattern[1..];
            for x in tail.iter() {
                new_pattern.push(x.clone());
            }
        }

        let mut t = JsonPath {
            pattern: new_pattern.clone(),
            store: JsonResult { path : Vec::new(), object: Vec::new() }
        };
        t.trace(obj);

        &self.append(&t.store.object);

        self.clone().traversal(&new_pattern, obj, |key, patterns, o| {
            let mut a = Vec::new();
            for p in patterns.iter() {
                a.push(p.clone());
            }

            if o.is_array() {
                let tmp = o.as_array().unwrap();
                let mi: usize = ::std::str::FromStr::from_str(key).unwrap();

                if tmp[mi].is_object() || tmp[mi].is_array() {
                    a.insert(0, "..");
                    let a1 = tmp[mi].clone();

                    let mut tr = JsonPath {
                        pattern: a,
                        store: JsonResult { path : Vec::new(), object: Vec::new() }
                    };
                    tr.trace(&a1);

                    &self.append(&tr.store.object);
                    return
                }
            } else if o.is_object() {
                let tmp = o.as_object().unwrap();

                if tmp[key].is_object() || tmp[key].is_array() {
                    a.insert(0, "..");
                    let a1 = tmp[key].clone();

                    let mut tr = JsonPath {
                        pattern: a,
                        store: JsonResult { path : Vec::new(), object: Vec::new() }
                    };

                    tr.trace(&a1);

                    &self.append(&tr.store.object);
                }
            }

            return
        })
    }

    fn all_item (&mut self, obj: &Json) {
        let mut new_pattern = Vec::new();
        {
            let tail = &self.pattern[1..];
            for x in tail.iter() {
                new_pattern.push(x.clone());
            }
        }

        self.clone().traversal(&new_pattern, obj, |key, patterns, obj| {
            let a = {
                let mut t =  Vec::new();
                for p in patterns.iter() {
                    t.push(p.clone());
                }
                t.insert(0, key);
                t
            };

            let mut t = JsonPath {
                pattern: a,
                store: JsonResult { path : Vec::new(), object: Vec::new() }
            };
            t.trace(obj);

            &self.append(&t.store.object);
            return
        });
    }

    fn fetch (&mut self, obj: &Json) {
        let mut new_pattern = Vec::new();
        {
            let tail = &self.pattern[1..];
            for x in tail.iter() {
                new_pattern.push(x.clone());
            }
        }

        let mut t = JsonPath {
            pattern: new_pattern,
            store: self.store.clone()
        };
        t.trace(obj);

        &self.append(&t.store.object);

        return
    }
}


#[cfg(test)]
mod test {
    extern crate rustc_serialize;

    use super::*;

    #[test]
    fn it_normalize () {
        assert_eq!(normalize("$..author"), ["$","..","author"]);
        assert_eq!(normalize("$..book[0]"), ["$","..","book","0"]);
        assert_eq!(normalize("$.store.book[*].author"), ["$", "store", "book", "*", "author"]);
    }

    #[test]
    fn it_trace () {
        let json = r#"
        {
          "id": 0,
          "name": "madoka",
          "vector": [1,2,3,4,5],
          "childlen": [
              {
                "id": 1,
                "name": "homura"
              },
              {
                "id": 2,
                "name": "sayaka"
              },
              {
                "id": 3,
                "name": "ryoko"
              },
              {
                "id": 4,
                "name": "mami"
              }
          ]
        }"#;

        let mut new_store = JsonPath {
            pattern: vec!["name"],
            store: JsonResult { path: vec![], object: vec![] }
        };
        new_store.trace(&rustc_serialize::json::Json::from_str(json).unwrap());
        assert_eq!(new_store.store.object[0].as_string().unwrap(), "madoka");

        let mut new_store = JsonPath {
            pattern: vec!["childlen", "*"],
            store: JsonResult { path: vec![], object: vec![] }
        };
        new_store.trace(&rustc_serialize::json::Json::from_str(json).unwrap());
        assert_eq!(new_store.store.object.len(), 4);

        new_store = JsonPath {
            pattern: vec!["..", "name"],
            store: JsonResult { path: vec![], object: vec![] }
        };
        new_store.trace(&rustc_serialize::json::Json::from_str(json).unwrap());
        assert_eq!(new_store.store.object.len(), 5)
    }
}
