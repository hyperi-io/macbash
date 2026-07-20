//  Project:   macbash
//  File:      src/testutil.rs
//  Purpose:   Helpers shared by the unit-test modules
//  Language:  Rust
//
//  License:   Apache-2.0
//  Copyright: (c) 2025-2026 HYPERI PTY LIMITED

use crate::rules::Rule;

/// Wrap a single-line rule example into a script the scanner will actually
/// apply that rule to.
///
/// Two gates make a bare example unusable: a rule whose pattern targets the
/// shebang line must BE line 1, and a rule with `shebang_match` never fires
/// unless line 1 satisfies it. Both the scanner and the fixer test modules
/// need this, and they used to carry their own copies -- which is how they
/// drifted into two spellings of the same three-way decision.
pub(crate) fn synthesise_script(rule: &Rule, example: &str) -> String {
    if rule.pattern.starts_with("^#") {
        // Shebang-pattern rule -- feed the example directly as the shebang
        // line. A synthetic shebang above it would self-match.
        return format!("{example}\n");
    }
    let header = if rule.shebang_match.contains("sh") && !rule.shebang_match.contains("bash") {
        "#!/bin/sh"
    } else {
        // Covers both "no shebang_match at all" and a bash-flavoured one.
        "#!/bin/bash"
    };
    format!("{header}\n{example}\n")
}
