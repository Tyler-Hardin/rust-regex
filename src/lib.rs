/**
 * Tyler Hardin
 * 2/10/2016
 *
 * A simple regex library. Only supports groups, alternatives, sequences,
 * repeats, and literal chars.
 */

use std::collections::BTreeMap;
use std::rc::Rc;
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
     */
    pub fn from_str(s : &str) -> Regex {
        Regex {
            root : Regex::parse(&mut s.chars(), &mut 0, true)
        }
    }

    /**
     * Matches a str against a regex.
     *
     * * regex - the regular expression
     * * itr -  an iterator for the string to match
     */
    pub fn match_str(&self, s : &str) -> Option<MatchResult> {
        let mut itr = s.chars();
        let mut mr = MatchResult::new();
        let res = self.root.m(&mut itr, &mut mr);

        // Did it match and did it match the whole string?
        if res.is_some() && itr.count() == 0 {
            Some(mr)
        }
        else {
            None
        }
    }

    /**
     * Helper function for Regex::from_chars. Does the actual parsing. The
     * type hierarchy goes:
     * group > alternation > sequence > (group or repeat or char).
     *
     * Returns the root node, a group, of the string passed.
     *
     * * itr - pointer to current position in regex string
     * * num - current group number (used to keep track of group numbers)
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

        while let Some(c) = itr.next() {
            match c {
                '(' => {
                    // Parse this nested group.
                    *num += 1;
                    let new_grp = Regex::parse(itr, num, false);
                    grp.get_seq().push_grp(new_grp);
                }
                '|' => {
                    // Create a new alternative sequence.
                    grp.add_alt();
                }
                ')' => {
                    // lparens should always be removed by the
                    // subgroup parse. So this must be an error.
                    if root {
                        panic!("Syntax error. Extra ')'.");
                    }
                    else {
                        break;
                    }
                }
                '*' => {
                    // Pop the previous node and nest it under a
                    // repeat node.
                    let n = grp.get_seq()
                        .pop()
                        .expect("Syntax error. * requires a preceeding node.");
                    let rpt = Rc::new(RptNode {
                        node : n
                    });
                    grp.get_seq().push(rpt);
                }
                '+' => {
                    // Clone the previous node and add a RptNode after it.
                    let n = grp.get_seq()
                        .clone_back()
                        .expect("Syntax error. + requires a preceeding node.");
                    let rpt = Rc::new(RptNode {
                        node : n
                    });
                    grp.get_seq().push(rpt);
                }
                c => {
                    // Char literal. Just push it on the
                    // current senquence.
                    grp.get_seq().push_char(c);
                }
            }
        }
        grp
    }
}


/// Interface for regex tree nodes.
trait Node {
    /**
     * Matches this node against (part of) a string. The match must start at
     * the first char of itr.
     *
     * Returns the string that the node matched.
     *
     * * itr -  current position in the string
     * * mr  -  MatchResult in which to store group matches
     */
    fn m(&self, &mut Chars, &mut MatchResult) -> Option<String>;

    /**
     * Prints this node in normal regex syntax.
     */
    fn debug(&self) -> String;
}

/// Represents an alternation.
struct AltNode {
    alts : Vec<SeqNode>
}

/// Represents a char literal.
struct CharNode {
    c : char
}

/// Represents a group.
struct GrpNode {
    num : usize,
    alt : AltNode
}

/// Represents a *.
struct RptNode {
    node : Rc<Node>
}

/// Represents a sequence.
struct SeqNode {
    nodes : Vec<Rc<Node>>
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
        match itr.next() {
            Some(c) if c == self.c => { Some(c.to_string()) }
            _ => { None }
        }
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

impl GrpNode {
    fn add_alt(&mut self) {
        self.alt.alts.push(SeqNode {
            nodes : Vec::new()
        });
    }

    fn get_seq(&mut self) -> &mut SeqNode {
        let len = self.alt.alts.len();
        return self.alt.alts.get_mut(len - 1).expect("");
    }
}

impl SeqNode {
    fn push_char(&mut self, c : char) {
        self.nodes.push(Rc::new(CharNode { c : c }));
    }

    fn push_grp(&mut self, grp : GrpNode) {
        self.nodes.push(Rc::new(grp));
    }

    fn push(&mut self, node : Rc<Node>) {
        self.nodes.push(node);
    }

    fn pop(&mut self) -> Option<Rc<Node>> {
        self.nodes.pop()
    }

    fn clone_back(&self) -> Option<Rc<Node>> {
        let len = self.nodes.len();
        if let Some(node) = self.nodes.get(len - 1) {
            Some(node.clone())
        }
        else {
            None
        }
    }
}

#[test]
fn test_star() {
    let mut mr = MatchResult::new();
    let regex = Regex::from_str("(a*)bc");

    mr.insert(0, "aabc".to_string());
    mr.insert(1, "aa".to_string());

    assert!(regex.match_str("aabc") == Some(mr));
}

#[test]
fn test_plus() {
    let mut mr = MatchResult::new();
    let regex = Regex::from_str("(a+)b");

    mr.insert(0, "aaab".to_string());
    mr.insert(1, "aaa".to_string());

    assert!(regex.match_str("aaab") == Some(mr));

    // Check that it doesn't match nothing.
    assert!(regex.match_str("b") == None);
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
