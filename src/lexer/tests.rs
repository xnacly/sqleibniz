#[allow(unused_macros)]
macro_rules! test_group_pass_assert {
    ($group_name:ident,$($ident:ident:$input:literal=$expected:expr_2021),*) => {
    mod $group_name {
        use crate::{lexer, types::Type};

        $(
            #[test]
            fn $ident() {
                let input = $input.as_bytes().to_vec();
                let mut l = lexer::Lexer::new(&input, "lexer_tests_pass");
                let toks = l.run();
                assert_eq!(l.errors.len(), 0);
                assert_eq!(toks.into_iter().map(|tok| tok.ttype).collect::<Vec<Type>>(), $expected);
            }
        )*
        }
    };
}

#[allow(unused_macros)]
macro_rules! test_group_fail {
    ($group_name:ident,$($name:ident:$value:literal),*) => {
        mod $group_name {
        use crate::lexer;
        $(
            #[test]
            fn $name() {
                let source = $value.as_bytes().to_vec();
                let mut l = lexer::Lexer::new(&source, "lexer_tests_fail");
                let toks = l.run();
                assert_eq!(toks.len(), 0);
                assert_ne!(l.errors.len(), 0);
            }
         )*
        }
    };
}

#[cfg(test)]
mod should_pass {

    test_group_pass_assert! {
        booleans,
        r#true: "true"=vec![Type::Boolean(true)],
        true_upper: "TRUE"=vec![Type::Boolean(true)],
        r#false: "false"=vec![Type::Boolean(false)],
        false_upper: "FALSE"=vec![Type::Boolean(false)]
    }

    test_group_pass_assert! {
        string,
        string: "'text'"=vec![Type::String(String::from("text"))],
        empty_string: "''"=vec![Type::String(String::from(""))],
        string_with_ending: "'str';"=vec![Type::String(String::from("str")), Type::Semicolon]
    }

    test_group_pass_assert! {
        symbol,
        // d is needed, because the lexer interprets . as a float start if the next character is
        // not an identifier, if so, it detects Type::Dot
        dot: ".d"=vec![Type::Dot, Type::Ident(String::from("d"))],
        star: "*"=vec![Type::Asterisk],
        semicolon: ";"=vec![Type::Semicolon],
        comma: ","=vec![Type::Comma],
        percent: "%"=vec![Type::Percent],
        equal: "="=vec![Type::Equal],
        at: "@"=vec![Type::At],
        colon: ":"=vec![Type::Colon],
        dollar: "$"=vec![Type::Dollar],
        question: "?"=vec![Type::Question]
    }

    test_group_pass_assert! {
        number,
        // edge cases
        zero: "0"=vec![Type::Number(0.0),],
        zero_float: ".0"=vec![Type::Number(0.0),],
        zero_hex: "0x0"=vec![Type::Number(0.0),],
        zero_float_with_prefix_zero: "0.0"=vec![Type::Number(0.0),],

        float_all_paths: "1_000.12_000e+3_5"=vec![Type::Number(1.00012e+38),],
        float_all_paths2: ".1_000e-1_2"=vec![Type::Number(1e-13),],
        hex: "0xABCDEF"=vec![Type::Number(0xABCDEF as f64),],
        hex_large_x: "0XABCDEF"=vec![Type::Number(0xABCDEF as f64)]
    }

    test_group_pass_assert! {
        blob,
        // edge cases
        empty: "X''"=vec![Type::Blob(vec![])],
        empty_small: "x''"=vec![Type::Blob(vec![])],

        filled: "X'12345'"=vec![Type::Blob(vec![49, 50, 51, 52, 53])],
        filled_small: "x'1234567'"=vec![Type::Blob(vec![49, 50, 51, 52, 53, 54, 55])]
    }
}

#[cfg(test)]
mod should_fail {
    test_group_fail! {
        empty_input,
        empty: "",
        empty_with_escaped: "\\",
        empty_with_space: " \t\n\r"
    }

    test_group_fail! {
        string,
        unterminated_string_eof: "'",
        unterminated_string_with_space: "'\n\t\r\n "
    }

    test_group_fail! {
        comment,
        line_comment: "-- comment",
        line_comment_with_newline: "--comment\n",
        multiline_comment_single_line: "/**/",
        multiline_comment: "/*\n\n\n*/"
    }

    test_group_fail! {
        number,
        bad_hex: "0x",
        bad_hex2: "0X",
        // was a test before, but due to lexer changes this just is Type::Dot
        // bad_float: ".",
        // was a test before, but due to lexer changes this just is Type::Dot*4
        // bad_float_multiple_dots: "....",
        bad_float_with_e: ".e",
        bad_float_with_large_e: ".E",
        bad_float_multiple_e: ".eeee",
        bad_float_combination: "12.e+-15"
    }

    test_group_fail! {
        blob,
        // edge cases
        no_quotes: "X",
        no_quotes_small: "x",
        unterminated: "X'",
        unterminated_small: "x'",
        unterminated1: "X'12819281",
        unterminated_small1: "x'102812",
        bad_hex: "X'1281928FFFY'"
    }
}
