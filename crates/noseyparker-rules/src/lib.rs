mod rule;
mod rules;
mod ruleset;
mod util;

pub use rule::{Rule, RuleSyntax};
pub use rules::Rules;
pub use ruleset::RulesetSyntax;

// -------------------------------------------------------------------------------------------------
// test
// -------------------------------------------------------------------------------------------------
#[cfg(test)]
mod test {
    use pretty_assertions::assert_eq;
    use proptest::prelude::*;
    // use proptest::string::string_regex;

    proptest! {
        // Idea: load up psst rules, and for each one, generate strings conforming to its pattern, then
        // check some properties.
        //
        // See https://altsysrq.github.io/proptest-book/proptest/tutorial/transforming-strategies.html
        #[test]
        fn regex_gen_noop(s in r"((?:A3T[A-Z0-9]|AKIA|AGPA|AIDA|AROA|AIPA|ANPA|ANVA|ASIA)[A-Z0-9]{16})") {
            println!("{}", s);
        }
    }

    #[test]
    #[should_panic]
    fn failure() {
        assert_eq!(5, 42);
    }
}
