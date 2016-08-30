/**
 * Tyler Hardin
 * 8/29/2016
 *
 * A simple regex library. Only supports groups, alternatives, sequences,
 * repeats, and literal chars.
 */

use std::collections::{BTreeSet,BTreeMap};
use std::fmt;
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
            root : GrpNode::parse(&mut s.chars(), &mut 0, true)
        }
    }

    /**
     * Matches a str against a regex.
     *
     * * regex - the regular expression
     * * s     - a str to match
     */
    pub fn match_str(&self, s : &str) -> Option<MatchResult> {
        self.match_chars(&mut s.chars())
    }

    /**
     * Matches a char iterator against a regex.
     *
     * * regex - the regular expression
     * * itr   - an iterator to match
     */
     pub fn match_chars(&self, itr : &mut Chars) -> Option<MatchResult> {
        let mut mr = MatchResult::new();
        let res = self.root.match_chars(itr, &mut mr);

        // Did it match and did it match the whole string?
        if res.is_some() && itr.count() == 0 {
            Some(mr)
        } else {
            None
        }
     }
}

impl fmt::Debug for Regex {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Regex {}", self.root.debug())
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
    fn match_chars(&self, &mut Chars, &mut MatchResult) -> Option<String>;

    /**
     * Prints this node in normal regex syntax.
     */
    fn debug(&self) -> String;
}

/// Represents an alternation.
struct AltNode {
    /// A vec of alternative sequences.
    alts : Vec<SeqNode>
}

/// Represents a char literal.
struct CharNode {
    /// The char literal this node represents.
    c : char
}

/// Represents a character class.
struct CharClassNode {
    /// Elements matched by this class.
    elems : BTreeSet<char>,
    /// Whether the class is negated.
    negated : bool
}

/// Represents a group.
struct GrpNode {
    /// The number of this group.
    num : usize,
    /// The list of alternative sequences.
    alt : AltNode
}

/// Represents a *.
struct RptNode {
    /// The node to be repeated.
    node : Rc<Node>
}

/// Represents a sequence.
struct SeqNode {
    /// Nodes that together form a sequence.
    nodes : Vec<Rc<Node>>
}

impl Node for AltNode {
    fn match_chars(&self, itr : &mut Chars, mr : &mut MatchResult) -> Option<String> {
        // Try each alternative.
        for alt in &self.alts {
            // Store for backtracking.
            let mut clone = itr.clone();

            // Return the first successful match.
            let res = alt.match_chars(&mut clone, mr);
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
    fn match_chars(&self, itr : &mut Chars, _ : &mut MatchResult) -> Option<String> {
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

impl Node for CharClassNode {
    fn match_chars(&self, itr : &mut Chars, _ : &mut MatchResult) -> Option<String> {
        if let Some(c) = itr.next() {
            let in_elems = self.elems.contains(&c);
            if (self.negated && !in_elems) || (!self.negated && in_elems) {
                Some(c.to_string())
            } else {
                None
            }
        } else {
            None
        }
    }

    fn debug(&self) -> String {
        let mut s = String::new();
        s.push_str("[");

        if self.negated {
            s.push_str("^");
        }

        for c in &self.elems {
            s.push(*c);
        }

        s.push_str("]");
        s
    }
}

impl Node for GrpNode {
    fn match_chars(&self, itr : &mut Chars, mr : &mut MatchResult) -> Option<String> {
        let res = self.alt.match_chars(itr, mr);

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

        // Dont print parens around the entire regex, they are implicit.
        if self.num == 0 {
            s = self.alt.debug();
        } else {
            s.push_str("(");
            s = s + &self.alt.debug();
            s.push_str(")");
        }
        return s;
    }
}

impl Node for RptNode {
    fn match_chars(&self, itr : &mut Chars, mr : &mut MatchResult) -> Option<String> {
        let mut clone = itr.clone();
        let mut out = String::new();

        let mut res = self.node.match_chars(itr, mr);
        while res.is_some() {
            // Store file position for backtracking.
            clone.clone_from(itr);

            // Append the previous match to our total match.
            out = out + &res.expect("");

            // Try to match again.
            res = self.node.match_chars(itr, mr);
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
    fn match_chars(&self, itr : &mut Chars, mr : &mut MatchResult) -> Option<String> {
        let mut out = String::new();

        for n in &self.nodes {
            let res = n.match_chars(itr, mr);
            if res.is_some() {
                out = out + &res.expect("");
            } else {
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

impl CharClassNode {
    fn parse(mut itr : &mut Chars) -> Self {
        let mut elems = BTreeSet::new();
        let mut negated = false;

        let handle_escape = |itr : &mut Chars, elems : &mut BTreeSet<char>| {
            if let Some(next) = itr.next() {
                if let Some(mapping) = parse_escape_char(next) {
                    elems.insert(mapping);
                } else {
                    panic!("Syntax error. Invalid escape.");
                }
            } else {
                panic!("Syntax error. Expected char following escape.");
            }
        };

        match itr.next() {
            Some(c) if c == '^' => { negated = true; }
            Some(c) if c == ']' => { panic!("Syntax error. Empty char class."); }
            Some(c) if c == '\\' => { handle_escape(&mut itr, &mut elems); }
            Some(c) => { elems.insert(c); }
            None => { panic!("Syntax error. Unterminated char class."); }
        }

        let mut done = false;
        while let Some(c) = itr.next() {
            if c == ']' {
                done = true;
                break;
            } else if c == '\\' {
                handle_escape(itr, &mut elems);
            } else {
                elems.insert(c);
            }
        }

        if !done {
            panic!("Syntax error. Unterminated char class.");
        } else if elems.is_empty() {
            panic!("Syntax error. Empty char class.");
        }

        CharClassNode {
            elems : elems,
            negated : negated
        }
    }

    fn from_vec(elems : Vec<char>, negated : bool) -> CharClassNode {
        CharClassNode {
            elems : elems.iter().cloned().collect(),
            negated : negated
        }
    }
}

impl GrpNode {
    /**
     * Helper function for Regex constructors. Does the actual parsing. The
     * type hierarchy goes:
     * group > alternation > sequence > (group or repeat or char).
     *
     * Returns the root node, a group, of the string passed.
     *
     * * itr - pointer to current position in regex string
     * * num - current group number (used to keep track of group numbers)
     */
    fn parse(itr : &mut Chars, num : &mut usize, root : bool) -> Self {
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
                    grp.get_seq().push_grp(GrpNode::parse(itr, num, false));
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
                    } else {
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
                '[' => {
                    let n = Rc::new(CharClassNode::parse(itr));
                    grp.get_seq().push(n);
                }
                '\\' => {
                    if let Some(c) = itr.next() {
                        if let Some(node) = parse_escape(c) {
                            grp.get_seq().push(node);
                        } else {
                            panic!("Syntax error. Invalid escape.");
                        }
                    } else {
                        panic!("Syntax error. Expected char following escape.");
                    }
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

/**
 * Parses the char following an escape, but restricts matches to those which
 * map directly to a another char (rather than, e.g., full nodes like a char
 * class).
 */
fn parse_escape_char(c : char) -> Option<char> {
    match c {
        '\\'|'('|')'|'['|']'|'*'|'+'|'^' => Some(c),
        't' => Some('\t'),
        _   => None
    }
}

/**
 * Parses the char following an escape ('/'), allowing any result. (This is 
 * used outside of character classes.)
 */
fn parse_escape(c : char) -> Option<Rc<Node>> {
    match c {
        's' => Some(Rc::new(CharClassNode::from_vec(vec!(' ', '\t'), false))),
        'S' => Some(Rc::new(CharClassNode::from_vec(vec!(' ', '\t'), true))),
        c   => {
            if let Some(c) = parse_escape_char(c) {
                Some(Rc::new(CharNode { c : c}))
            } else { 
                None 
            }
        }
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
        } else {
            None
        }
    }
}

#[cfg(test)]
fn test_match(r : &str, testcase : &str) {
    let regex = Regex::from_str(r);
    assert!(regex.match_str(testcase).is_some());
}

#[cfg(test)]
fn test_result(r : &str, testcase : &str, mut mr : MatchResult) {
    let regex = Regex::from_str(r);
    mr.insert(0, testcase.to_string());
    println!("Regex: {:?}", regex);

    let res = regex.match_str(testcase);
    println!("Expected result: {:?}", &mr);
    println!("Actual result: {:?}", &res);

    assert!(res == Some(mr));
}

#[test]
fn test_escape_escape() {
    test_match("\\\\", "\\");
}

#[test]
fn test_tab() {
    test_match("\t", "\t");
    test_match("\\t", "\t");
}

#[test]
fn test_char_class_escape() {
    test_match("[\\t]", "\t");
    test_match("[\\^]", "^");
}

#[test]
fn test_star() {
    let mut mr = MatchResult::new();
    mr.insert(1, "aa".to_string());
    test_result("(a*)bc", "aabc", mr);
}

#[test]
fn test_plus() {
    let mut mr = MatchResult::new();
    mr.insert(1, "aaa".to_string());
    test_result("(a+)b", "aaab", mr);
}

#[test]
fn test_groups() {
    let mut mr = MatchResult::new();
    mr.insert(1, "ac".to_string());
    mr.insert(2, "c".to_string());
    mr.insert(3, "cdcdd".to_string());
    mr.insert(4, "d".to_string());
    test_result("(a(b|c))b((c|d)*)", "acbcdcdd", mr);
}

#[test]
fn test_alts() {
    let mut mr = MatchResult::new();
    mr.insert(3, "cdcdd".to_string());
    mr.insert(4, "d".to_string());
    test_result("(a(b|c)*)|((c|d)*)", "cdcdd", mr);
}

#[test]
fn test_char_class() {
    let mut mr = MatchResult::new();
    mr.insert(1, "a".to_string());
    mr.insert(2, "z".to_string());
    test_result("([abc])([xyz])", "az", mr);
}

#[test]
fn test_char_class_negated() {
    test_result("[^z]", "a", MatchResult::new());
}
