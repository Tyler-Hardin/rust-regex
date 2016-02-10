This library implements an API for (a small subset of) regular expressions. It supports groups, alternatives, sequences, * repeats, and character literals. It only supports matching (a whole string) and not searching within a string.

A usage example:

    fn main() {
        let regex = Regex::from_str("(a|b)((c|d)*)");
        let match_result = regex.match_str("bcddc");
        
        println!("{:?}", match_result);
        // Some({0: "bcddc", 1: "b", 2: "cddc", 3: "c"})
    }
