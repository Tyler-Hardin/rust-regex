/**
 * Tyler Hardin
 * 2/10/2016
 * 
 * A simple regex library. Only supports groups, alternatives, sequences, 
 * repeats, and literal chars.
 */

use std::collections::BTreeMap;
use std::str::Chars;

/**
 * A collection mapping group number to matched string.
 */
pub type MatchResult = BTreeMap<usize,String>;

/**
 * A struct for representing and using regular expressions.
 */
pub struct Regex {
    root : GrpNode
}

impl Regex {
    /**
     * Creates a regex from a str that represents a regex. Panics if the
     * regex is not well-formed.
     *
     * @param c the charater iterator
     * @returns the regex
     */
    pub fn from_str(s : &str) -> Regex {
        Regex {
            root : Regex::parse(&mut s.chars(), &mut 0, true)
        }
    }

    /**
     * Matches a str against a regex.
     *
     * @param regex the regular expression
     * @param itr   an iterator for the string to match
     * @returns an Option for whether the string matched and a MatchResult
     */
    pub fn match_str(&self, s : &str) -> Option<MatchResult> {
        let mut itr = s.chars();
        let mut mr = MatchResult::new();
        let res = self.root.m(&mut itr, &mut mr);

        // Did it match and did it match the whole string?
        if res.is_some() && itr.count() == 0 {
            return Some(mr);
        }
        else {
            return None;
        }
    }

    /**
     * Helper function for Regex::from_chars. Does the actual parsing. The
     * type hierarchy goes:
     * group > alternation > sequence > (group or repeat or char).
     *
     * @param itr   pointer to current position in regex string
     * @param num   current group number (used to keep track of group numbers)
     * @returns a GrpNode of the group parsed.
     */
    fn parse(itr : &mut Chars, num : &mut usize, root : bool) -> GrpNode {
        let mut grp = GrpNode {
            num : *num,
            alt : AltNode {
                alts : vec!(SeqNode {
                    nodes : Vec::new()
                })
            }
        };

        let mut done = false;
        let mut alt_idx = 0;

        while !done {
            match itr.next() {
                Some(c) if c == '(' => {
                    // Parse this nested group.
                    *num += 1;
                    let new_grp = Regex::parse(itr, num, false);
                    grp.alt.alts[alt_idx].push_grp(new_grp);
                }
                Some(c) if c == '|' => {
                    // Create a new alternative sequence.
                    grp.alt.alts.push(SeqNode {
                        nodes : Vec::new()
                    });
                    alt_idx += 1;
                }
                Some(c) if c == ')' => {
                    // lparens should always be removed by the
                    // subgroup parse. So this must be an error.
                    if root {
                        panic!("Syntax error. Extra ')'.");
                    }
                    else {
                        done = true;
                    }
                }
                Some(c) if c == '*' => {
                    // Pop the previous node and nest it under a
                    // repeat node.
                    let n = grp.alt.alts[alt_idx].pop();
                    let rpt = Box::new(RptNode {
                        node : n
                    });
                    grp.alt.alts[alt_idx].push(rpt);
                }
                Some(c) => {
                    // Simple character. Just push it on the
                    // current senquence.
                    grp.alt.alts[alt_idx].push_char(c);
                }
                // We're done!
                None => { done = true; }
            }
        }
        grp
    }
}


// Interface for regex tree nodes.
trait Node {
    /**
     * Matches this node against (part of) a string
     *
     * @param itr   current position in the string
     * @mr  MatchResult in which to store group matches
     * @returns what this node matched
     */
    fn m(&self, &mut Chars, &mut MatchResult) -> Option<String>;

    /**
     * Prints this node in normal regex syntax.
     */
    fn debug(&self) -> String;
}

// Represents alternation.
struct AltNode {
    alts : Vec<SeqNode>
}

// Represents a character.
struct CharNode {
    c : char
}

// Represents a group.
struct GrpNode {
    num : usize,
    alt : AltNode
}

// Represents a *.
struct RptNode {
    node : Box<Node>
}

// Represents a sequence.
struct SeqNode {
    nodes : Vec<Box<Node>>
}

impl Node for AltNode {
    fn m(&self, itr : &mut Chars, mr : &mut MatchResult) -> Option<String> {
        // Try each alternative.
        for alt in &self.alts {
            // Store for backtracking.
            let mut clone = itr.clone();

            // Return the first successful match.
            let res = alt.m(&mut clone, mr);
            if res.is_some() {
                itr.clone_from(&clone);
                return res;
            }
        }

        return None;
    }

    fn debug(&self) -> String {
        let mut s = String::new();

        for alt in &self.alts {
            s = s + &alt.debug();
            s.push('|');
        }
        s.pop();

        return s;
    }
}

impl Node for CharNode {
    fn m(&self, itr : &mut Chars, _ : &mut MatchResult) -> Option<String> {
        return match itr.next() {
            Some(c) if c == self.c => { Some(c.to_string()) }
            _ => { None }
        };
    }

    fn debug(&self) -> String {
        let mut s = String::new();
        s.push_str("Char{");
        s.push(self.c);
        s.push_str("}");
        return s;
    }
}

impl Node for GrpNode {
    fn m(&self, itr : &mut Chars, mr : &mut MatchResult) -> Option<String> {
        let res = self.alt.m(itr, mr);

        match res {
            Some(ref s) => {
                mr.insert(self.num, s.clone());
            }
            None => {}
        };

        return res;
    }

    fn debug(&self) -> String {
        let mut s = String::new();
        s.push_str("Grp{");

        // Dont print parens around the entire regex, they are implicit.
        if self.num == 0 {
            s = self.alt.debug();
        }
        else {
            s.push_str("(");
            s = s + &self.alt.debug();
            s.push_str(")");
        }
        s.push_str("}");
        return s;
    }
}

impl Node for RptNode {
    fn m(&self, itr : &mut Chars, mr : &mut MatchResult) -> Option<String> {
        let mut clone = itr.clone();
        let mut out = String::new();

        let mut res = self.node.m(itr, mr);
        while res.is_some() {
            // Store file position for backtracking.
            clone.clone_from(itr);

            // Append the previous match to our total match.
            out = out + &res.expect("");

            // Try to match again.
            res = self.node.m(itr, mr);
        }

        // Backtrack to the point after the last successful match.
        itr.clone_from(&clone);

        // Zero or more, so we always match. If zero matches were made,
        // this returns Some(""), which is what we want.
        Some(out)
    }

    fn debug(&self) -> String {
        return self.node.debug() + "*";
    }
}

impl Node for SeqNode {
    fn m(&self, itr : &mut Chars, mr : &mut MatchResult) -> Option<String> {
        let mut out = String::new();

        for n in &self.nodes {
            let res = n.m(itr, mr);
            if res.is_some() {
                out = out + &res.expect("");
            }
            else {
                return None;
            }
        }
        return Some(out);
    }

    fn debug(&self) -> String {
        let mut s = String::new();

        for n in &self.nodes {
            s = s + &n.debug();
        }

        return s;
    }
}

impl SeqNode {
    fn push_char(&mut self, c : char) {
        self.nodes.push(Box::new(CharNode { c : c }));
    }

    fn push_grp(&mut self, grp : GrpNode) {
        self.nodes.push(Box::new(grp));
    }

    fn push(&mut self, node : Box<Node>) {
        self.nodes.push(node);
    }

    fn pop(&mut self) -> Option<Box<Node>> {
        self.nodes.pop()
    }
}

#[test]
fn test_repeat() {
    let mut mr = MatchResult::new();
    let regex = Regex::from_str("(a*)bc");

    mr.insert(0, "aabc".to_string());
    mr.insert(1, "aa".to_string());

    assert!(regex.match_str("aabc") == Some(mr));
}

#[test]
fn test_groups() {
    let mut mr = MatchResult::new();
    let regex = Regex::from_str("(a(b|c))b((c|d)*)");

    mr.insert(0, "acbcdcdd".to_string());
    mr.insert(1, "ac".to_string());
    mr.insert(2, "c".to_string());
    mr.insert(3, "cdcdd".to_string());
    mr.insert(4, "d".to_string());

    let res = regex.match_str("acbcdcdd");
    assert!(res == Some(mr));
}

#[test]
fn test_alts() {
    let mut mr = MatchResult::new();
    let regex = Regex::from_str("(a(b|c)*)|((c|d)*)");

    mr.insert(0, "cdcdd".to_string());
    mr.insert(3, "cdcdd".to_string());
    mr.insert(4, "d".to_string());

    let res = regex.match_str("cdcdd");

    println!("{:?}", res);
    assert!(res == Some(mr));
}
