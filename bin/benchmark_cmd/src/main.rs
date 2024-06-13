#![deny(unused_crate_dependencies)]
use core::slice;
use std::{
    collections::{BTreeMap, HashMap},
    fs::File,
    io::{Read, Write},
    path::PathBuf,
    process::{Command, Stdio},
};

use anyhow::Result;
use clap::Parser;
use serde::Deserialize;

#[derive(Parser, Debug, Clone)]
#[command(name = "Benchmark Cmd")]
#[command(bin_name = "benchmark_cmd")]
enum CliRootParser {
    Expand { file: PathBuf },
    Exec { file: PathBuf },
}

#[derive(Debug, Deserialize)]
struct RootConfig {
    command: SubcommandConfig,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum CommandConfig {
    Command(String),
    Template(TemplateConfig),
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum SubcommandConfig {
    Vec(Vec<CommandConfig>),
    Single(CommandConfig),
}

impl SubcommandConfig {
    fn as_slice(&self) -> &[CommandConfig] {
        match self {
            SubcommandConfig::Vec(commands) => &commands,
            SubcommandConfig::Single(command) => slice::from_ref(command),
        }
    }

    /// Create all the possible command permutations from a command config
    pub fn instantiate(&self) -> Vec<String> {
        let mut result = vec![];
        for command_config in self.as_slice() {
            match command_config {
                CommandConfig::Command(cmd) => result.push(cmd.to_owned()),
                CommandConfig::Template(template) => {
                    let segments = Segment::build_from_template(template);

                    result.extend(permutate_segments(template, segments.as_slice()));
                }
            }
        }

        result
    }
}

#[derive(Debug, Deserialize)]
struct TemplateConfig {
    template: String,
    replace: HashMap<String, SubcommandConfig>,
}

fn main() -> Result<()> {
    let cli = CliRootParser::parse();
    match cli {
        CliRootParser::Expand { file } => {
            let config: RootConfig = serde_json::from_reader(File::open(file)?)?;
            config
                .command
                .instantiate()
                .into_iter()
                .for_each(|x| println!("{x}"));
        }
        CliRootParser::Exec { file } => {
            let config: RootConfig = serde_json::from_reader(File::open(file)?)?;
            let mut log = File::create("output.log")?;
            let instances = config.command.instantiate();
            let len = instances.len();
            for (i, program) in instances.into_iter().enumerate() {
                println!("({i}/{len}) '{program}'");
                let mut handle = Command::new("bash")
                    .args(["-c", program.as_str()])
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()?;
                let program_exit_status = handle.wait()?;
                log.write(format!("#### Starting '{}'\n", program).as_bytes())?;

                log.write("#### Stdout\n".as_bytes())?;
                let mut stdout = Vec::new();
                handle
                    .stdout
                    .take()
                    .expect("Piped stdout did not get captured")
                    .read_to_end(&mut stdout)?;
                log.write(&stdout)?;

                log.write("#### Stderr\n".as_bytes())?;
                let mut stderr = Vec::new();
                handle
                    .stderr
                    .take()
                    .expect("Piped stdout did not get captured")
                    .read_to_end(&mut stderr)?;
                log.write(&stderr)?;

                log.write(
                    format!(
                        "#### Stopping '{}' with '{}' \n",
                        program, program_exit_status
                    )
                    .as_bytes(),
                )?;
            }
            println!("({len}/{len}) Done.");
        }
    }

    Ok(())
}

/// Mark segements of text as either a literal or a key that needs to be replaced. Used for text formatting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Segment<'a> {
    Literal(&'a str),
    Replace(&'a str),
}

impl<'a> Segment<'a> {
    /// Split the template into a list of [Segment] that can then be formatted for each permutation.
    pub fn build_from_template(template: &TemplateConfig) -> Vec<Segment> {
        // Split literals into more literals and replace segements if found
        let mut parts = vec![Segment::Literal(template.template.as_str())];
        for (arg, _subcommand) in &template.replace {
            parts = parts
                .into_iter()
                .map(|part| match part {
                    Segment::Literal(literal) => {
                        let mut res: Vec<Segment> = literal
                            .split(arg)
                            .flat_map(|x| [Segment::Literal(x), Segment::Replace(&arg)])
                            .collect();
                        res.pop(); // The last replace makes this unbalance, so remove it
                        res
                    }
                    replace @ Segment::Replace(_) => vec![replace],
                })
                .flatten()
                .collect();
        }

        parts
            .into_iter()
            .filter(|x| match x {
                // Remove useless empty literals
                Segment::Literal("") => false,
                _ => true,
            })
            .collect()
    }
}

#[test]
fn test_segment_build_from_template() {
    let template = TemplateConfig {
        template: "wow".to_owned(),
        replace: {
            let mut e = HashMap::new();
            e.insert(
                "sub".to_owned(),
                SubcommandConfig::Single(CommandConfig::Command(String::from(""))),
            );
            e
        },
    };
    assert_eq!(
        Segment::build_from_template(&template),
        vec![Segment::Literal("wow")]
    );

    let template = TemplateConfig {
        template: "wow sub".to_owned(),
        replace: {
            let mut e = HashMap::new();
            e.insert(
                "sub".to_owned(),
                SubcommandConfig::Single(CommandConfig::Command(String::from("test"))),
            );
            e
        },
    };
    assert_eq!(
        Segment::build_from_template(&template),
        vec![Segment::Literal("wow "), Segment::Replace("sub")]
    );

    let template = TemplateConfig {
        template: "wow sub more".to_owned(),
        replace: {
            let mut e = HashMap::new();
            e.insert(
                "sub".to_owned(),
                SubcommandConfig::Single(CommandConfig::Command(String::from("test"))),
            );
            e
        },
    };
    assert_eq!(
        Segment::build_from_template(&template),
        vec![
            Segment::Literal("wow "),
            Segment::Replace("sub"),
            Segment::Literal(" more")
        ]
    );

    let template = TemplateConfig {
        template: "wow sub more".to_owned(),
        replace: {
            let mut e = HashMap::new();
            e.insert(
                "sub".to_owned(),
                SubcommandConfig::Single(CommandConfig::Command(String::from("test"))),
            );
            e.insert(
                "wow".to_owned(),
                SubcommandConfig::Single(CommandConfig::Command(String::from("t"))),
            );
            e.insert(
                " ".to_owned(),
                SubcommandConfig::Single(CommandConfig::Command(String::from("_"))),
            );
            e
        },
    };
    assert_eq!(
        Segment::build_from_template(&template),
        vec![
            Segment::Replace("wow"),
            Segment::Replace(" "),
            Segment::Replace("sub"),
            Segment::Replace(" "),
            Segment::Literal("more")
        ]
    );
}

/// Create all the permutations of a command
fn permutate_segments(template: &TemplateConfig, segments: &[Segment]) -> Vec<String> {
    // Collect all the possible replace keys and the possible substitutions
    let replacers = segments
        .iter()
        .filter_map(|x| match x {
            Segment::Literal(_) => None,
            Segment::Replace(x) => {
                let (replace_literal, subcommand) = template.replace.get_key_value(*x).unwrap();
                Some((replace_literal, subcommand.instantiate()))
            }
        })
        .collect::<BTreeMap<_, _>>();

    // Permutations a created by incrementing a counter an cycling each level of
    // value. This caluclates how often the level needs to be cycled.
    let permute_remainders = [1]
        .into_iter()
        .chain(replacers.iter().map(|x| x.1.len()).scan(1, |a, b| {
            *a *= b;
            Some(*a)
        }))
        .collect::<Vec<_>>();

    let permutation_count = permute_remainders.last().copied().unwrap_or_default();

    // Rearragging values for convenience and then sorting by key for repetability
    let replacers = replacers
        .into_iter()
        .zip(permute_remainders)
        .map(|x| (x.0 .0.as_str(), (x.0 .1, x.1)))
        .collect::<BTreeMap<_, _>>();

    let mut results = Vec::with_capacity(permutation_count);
    let segments = Segment::build_from_template(template);
    for i in 0..permutation_count {
        // Replace each segment with a specific permutation
        results.push(String::from_iter(segments.iter().map(
            |segment| match segment {
                Segment::Literal(lit) => *lit,
                Segment::Replace(replace_lit) => {
                    // Get the specific value that needs to be replaced in the specific permutation of the command
                    let (variants, len) = replacers.get(replace_lit).unwrap();
                    &variants[(i / len) % variants.len()]
                }
            },
        )))
    }

    results
}
