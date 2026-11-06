// SPDX-License-Identifier: MIT OR Apache-2.0
//! Tests of the token order.

use super::*;

// spec: rust-api / Compare tokens
#[test]
fn a_higher_generation_wins_over_a_higher_number() {
    let newer = Token {
        generation: 2,
        number: 1,
    };
    let older = Token {
        generation: 1,
        number: 9,
    };

    assert!(newer > older);
    assert!(older < newer);
}

#[test]
fn the_number_decides_inside_one_generation() {
    let first = Token {
        generation: 3,
        number: 4,
    };
    let second = Token {
        generation: 3,
        number: 5,
    };

    assert!(second > first);
    assert_eq!(first.cmp(&first), std::cmp::Ordering::Equal);
}
