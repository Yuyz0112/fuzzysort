#![feature(test)]
extern crate test;
extern crate unicase;
use unicase::UniCase;
use std::fmt;
use std::fs::File;
use std::io::prelude::*;

struct FuzzyResult {
    results: Vec<Info>,
    total: usize,
}
struct Info {
    score: usize,
    matches: Vec<usize>,
    highlighted: String,
    target: String,
}
impl fmt::Display for FuzzyResult {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[\r\n").ok();
        for (i, result) in self.results.iter().enumerate() {
            if i != 0 {
                write!(f, ",\r\n")?;
            }
            write!(f, "{}", result).ok();
        }
        write!(f, "\r\n]")
    }
}
impl fmt::Display for Info {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\t{{ score: {}, highlighted: '{}' }}", self.score, self.highlighted)
    }
}
impl Info {
    fn set_highlighted(&mut self, str: String) {
        self.highlighted = str;
    }
}

struct Fuzzysort {
    no_match_limit: usize,
    limit: Option<usize>,
    highlight_open: String,
    highlight_close: String,
}
impl Fuzzysort {
    fn go(&self, search: String, targets: &Vec<String>) -> FuzzyResult {
        if search.len() == 0 {
            FuzzyResult {
                results: Vec::new(),
                total: 0,
            }
        } else {
            let search_unicase = UniCase::new(&search);
            let mut results: Vec<Info> = Vec::new();
            for target in targets {
                let info_result = self.info(&search_unicase, target);
                match info_result {
                    Some(unwrap_result) => results.push(unwrap_result),
                    None => (),
                }
            }

            let total = results.len();
            results.sort_by(|a, b| a.score.cmp(&b.score));

            if self.limit != None && total > self.limit.unwrap() {
                results.truncate(self.limit.unwrap());
            }
            for result in &mut results {
                let s = self.highlight(result);
                result.set_highlighted(s);
            }

            FuzzyResult {
                results: results,
                total: total,
            }
        }
    }
    fn info(&self, search_unicase: &String, target: &String) -> Option<Info> {
        let mut search_chars = search_unicase.chars();

        let target_unicase = UniCase::new(&target);
        let mut target_chars = target_unicase.chars().enumerate();

        let mut no_match_count = 0;
        let mut matches_simple: Vec<usize> = Vec::new();

        let mut search_char = search_chars.next();
        while let Some(unwrap_search_char) = search_char {
            let target_char = target_chars.next();
            if target_char == None {
                break;
            }
            let unwrap_target_char = target_char.unwrap();
            let is_match = unwrap_search_char == unwrap_target_char.1;

            if is_match {
                matches_simple.push(unwrap_target_char.0);
                search_char = search_chars.next();
                no_match_count = 0;
            } else {
                no_match_count += 1;
                if no_match_count >= self.no_match_limit {
                    break;
                }
            }
        }
        if search_char == None {
            Some(self.info_strict(&search_unicase, target, matches_simple))
        } else {
            None
        }
    }
    fn info_strict(&self, search_unicase: &String, target: &String, matches_simple: Vec<usize>) -> Info {
        let mut search_chars = search_unicase.chars().enumerate();

        let target_unicase = UniCase::new(target);
        let mut target_chars = target_unicase.chars().enumerate();

        let mut no_match_count = 0;
        let mut matches_strict: Vec<usize> = Vec::new();
        let mut strict_success = false;

        let mut was_upper = false;
        let mut was_alphanum = false;
        let mut is_consec = false;

        let mut search_char = search_chars.next();
        if matches_simple[0] > 0 {
            let before_target_char = target_chars.nth(matches_simple[0] - 1).unwrap();
            was_upper = before_target_char.1.is_uppercase();
            was_alphanum = before_target_char.1.is_alphanumeric();
        }
        let mut target_char = target_chars.next();
        while let Some(unwrap_search_char) = search_char {
            if !is_consec {
                while let Some(unwrap_target_char) = target_char {
                    let is_upper = unwrap_target_char.1.is_uppercase();
                    let is_alphanum = unwrap_target_char.1.is_alphanumeric();
                    let is_beginning = is_upper && !was_upper || !was_alphanum || !is_alphanum;
                    was_upper = is_upper;
                    was_alphanum = is_alphanum;
                    if is_beginning {
                        break;
                    } else {
                        target_char = target_chars.next();
                    }
                }
            }

            if target_char == None {
                break;
            }
            let unwrap_target_char = target_char.unwrap();

            let is_match = unwrap_search_char.1 == unwrap_target_char.1;
            if is_match {
                matches_strict.push(unwrap_target_char.0);
                search_char = search_chars.next();
                if search_char == None {
                    break;
                }

                no_match_count = 0;
                is_consec = true;
                // skip ahead, but is nth faster than iterate?
                let next_matched_index = matches_simple[search_char.unwrap().0];
                if next_matched_index > unwrap_target_char.0 + 1 {
                    let relate_before_index = next_matched_index - unwrap_target_char.0 - 2;
                    let before_target_char = target_chars.nth(relate_before_index).unwrap();
                    is_consec = false;
                    was_upper = before_target_char.1.is_uppercase();
                    was_alphanum = before_target_char.1.is_alphanumeric();
                }
                target_char = target_chars.next();
            } else {
                no_match_count += 1;
                if no_match_count >= self.no_match_limit {
                    break;
                }
                is_consec = false;
                target_char = target_chars.next();
            }
        }
        let mut matches_best = matches_simple;
        if search_char == None {
            strict_success = true;
            matches_best = matches_strict;
        }
        let mut score = 0;
        let mut last_target_i = 0;
        for x in matches_best.iter_mut() {
            if last_target_i + 1 != *x {
                score += *x;
            }
            last_target_i = *x;
        }
        if !strict_success {
            score *= 1000
        }
        score += target.len() - search_unicase.len();
        Info {
            score: score,
            matches: matches_best,
            highlighted: String::new(),
            target: target.clone(),
        }
    }
    fn highlight (&self, result: &Info) -> String {
        let mut s = String::new();

        let mut target_chars = result.target.chars().enumerate();
        let mut target_char = target_chars.next();
        let mut matches = result.matches.iter();
        let mut match_index = matches.next();
        let mut opened = false;

        while let Some(unwrap_target_char) = target_char {
            if *match_index.unwrap() == unwrap_target_char.0 {
                match_index = matches.next();
                if !opened {
                    s.push_str(&self.highlight_open);
                    opened = true;
                }

                if match_index == None {
                    s.push(unwrap_target_char.1);
                    s.push_str(&self.highlight_close);

                    // short cut here
                    target_char = target_chars.next();
                    if target_char != None {
                        let slice_string = &result.target[target_char.unwrap().0..];
                        s.push_str(slice_string);
                    }
                    break;
                }
            } else if opened {
                s.push_str(&self.highlight_close);
                opened = false;
            }
            s.push(unwrap_target_char.1);
            target_char = target_chars.next();
        }
        s
    }
}

fn main() {
    let fuzzysort = Fuzzysort {
        no_match_limit: 100,
        limit: None,
        highlight_open: String::from("<b>"),
        highlight_close: String::from("</b>"),
    };

    let mut f = File::open("testdata").expect("file not found");
    let mut contents = String::new();
    f.read_to_string(&mut contents).expect("something went wrong when reading the testdata");
    let v: Vec<String> = contents.split("\n").map(|x| x.to_string()).collect();

    let result = fuzzysort.go(String::from("query"), &v);
    println!("results: {}\r\ntotal: {}", result ,result.total);
}

#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;

    #[test]
    fn it_works() {
        let fuzzysort = Fuzzysort {
            no_match_limit: 100,
            limit: None,
            highlight_open: String::from("<b>"),
            highlight_close: String::from("</b>"),
        };
        let result = fuzzysort.go(String::from("query"), &vec![
            String::from("something with yreuq key word quer y"),
            String::from("quer y"),
            String::from("string with key word q u e r y in the middle."),
            String::from("string not match")
        ]);
        assert_eq!(result.total, 3);
        assert_eq!(result.results[0].highlighted, "<b>quer</b> <b>y</b>");
        assert_eq!(result.results[1].highlighted, "something with yreuq key word <b>quer</b> <b>y</b>");
    }

    #[bench]
    fn empty(b: &mut test::Bencher) {
        b.iter(|| 1)
    }

    #[bench]
    #[ignore]
    fn lowercase(b: &mut test::Bencher) {
        let target = String::from("Search some long string to test the lowercase benchmark.");
        assert_eq!(UniCase::new("İ"), UniCase::new("i̇"));
        b.iter(|| UniCase::new(&target))
    }

    #[bench]
    fn bench_go(b: &mut Bencher) {
        let fuzzysort = Fuzzysort {
            no_match_limit: 100,
            limit: None,
            highlight_open: String::from("<b>"),
            highlight_close: String::from("</b>"),
        };
        let mut f = File::open("testdata").expect("file not found");
        let mut contents = String::new();
        f.read_to_string(&mut contents).expect("something went wrong when reading the testdata");
        let test_data: Vec<String> = contents.split("\n").map(|x| x.to_string()).collect();
        fn test(f: &Fuzzysort, v: &Vec<String>) {
            f.go(String::from("e"), &v);
            f.go(String::from("a"), &v);
            f.go(String::from("word"), &v);
            f.go(String::from("longword"), &v);
        }
        b.iter(|| fuzzysort.go(String::from("query"), &test_data))
    }

    #[bench]
    #[ignore]
    fn bench_go_single(b: &mut test::Bencher) {
        let fuzzysort = Fuzzysort {
            no_match_limit: 100,
            limit: None,
            highlight_open: String::from("<b>"),
            highlight_close: String::from("</b>"),
        };
        b.iter(|| fuzzysort.go(String::from("query"), &vec![String::from("something with yreuq key word quer y")]))
    }

    #[bench]
    #[ignore]
    fn bench_info_single(b: &mut test::Bencher) {
        let fuzzysort = Fuzzysort {
            no_match_limit: 100,
            limit: None,
            highlight_open: String::from("<b>"),
            highlight_close: String::from("</b>"),
        };
        let search_lower = String::from("query").to_lowercase();
        let target = String::from("something with yreuq key word quer y");
        b.iter(|| fuzzysort.info(&search_lower, &target))
    }

    #[bench]
    #[ignore]
    fn bench_info_strict_single(b: &mut test::Bencher) {
        let fuzzysort = Fuzzysort {
            no_match_limit: 100,
            limit: None,
            highlight_open: String::from("<b>"),
            highlight_close: String::from("</b>"),
        };
        let search_lower = String::from("query").to_lowercase();
        let target = String::from("something with yreuq key word quer y");
        b.iter(|| fuzzysort.info_strict(&search_lower, &target, vec![19, 31, 32, 33, 35]))
    }

    #[bench]
    #[ignore]
    fn bench_highlight_single(b: &mut test::Bencher) {
        let fuzzysort = Fuzzysort {
            no_match_limit: 100,
            limit: None,
            highlight_open: String::from("<b>"),
            highlight_close: String::from("</b>"),
        };
        let info = Info {
            score: 96,
            matches: vec![30, 31, 32, 33, 35],
            highlighted: String::new(),
            target: String::from("something with yreuq key word quer y"),
        };
        b.iter(|| fuzzysort.highlight(&info))
    }
}
