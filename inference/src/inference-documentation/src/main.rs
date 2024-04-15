#![warn(clippy::all, clippy::pedantic)]

use std::{env, process};

use inference_documentation::{build_inference_documentation, InferenceDocumentationConfig};
use inference_proc_macros::inference_spec;

fn main() {
    let config =
        InferenceDocumentationConfig::from_cmd_line_args(env::args()).unwrap_or_else(|err| {
            eprintln!("Problem parsing arguments: {err}");
            process::exit(1);
        });

    build_inference_documentation(&config);
}

#[inference_spec(main)]
mod spec {
    use inference_proc_macros::{inference, inference_fun};

    #[inference_fun(main::main)]
    fn s_main() {
        inference! {
            r#"main -> ()"#
        };
    }
}
