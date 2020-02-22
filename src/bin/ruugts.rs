use clap::{App, Arg};
use std::error::Error;
use std::process::Command;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let is_wrapper = std::env::var("RUUGTS_WRAPPER").is_ok();

    let mut raw_args: Vec<_> = std::env::args()
        .skip(if is_wrapper { 2 } else { 1 })
        .collect();

    let matches = app(is_wrapper).get_matches_safe()?;

    log::trace!("{:?}", matches);
    log::trace!("{:?}", raw_args);

    let file = matches.value_of("file").unwrap();

    let index = raw_args
        .iter()
        .enumerate()
        .find(|(_i, arg)| &arg[..] == file)
        .unwrap()
        .0;

    let path = amargo::transform(file)?;
    raw_args[index] = path.as_ref().to_str().unwrap().to_string();

    log::trace!("{:?}", raw_args);
    let exit = Command::new("rustc").args(&raw_args).spawn()?.wait()?;

    if exit.success() {
        Ok(())
    } else {
        std::process::exit(exit.code().unwrap_or(1));
    }
}

fn app(is_wrapper: bool) -> App<'static, 'static> {
    let mut app = App::new("rustc").bin_name("ruugts");

    let file_index = if is_wrapper {
        app = app.arg(Arg::with_name("cmd").index(1).required(true));
        2
    } else {
        1
    };

    app.arg(Arg::with_name("file").index(file_index).required(true))
        .arg(Arg::with_name("edition").long("edition").takes_value(true))
        .arg(
            Arg::with_name("crate-name")
                .long("crate-name")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("error-format")
                .long("error-format")
                .takes_value(true),
        )
        .arg(Arg::with_name("json").long("json").takes_value(true))
        .arg(Arg::with_name("emit").long("emit").takes_value(true))
        .arg(Arg::with_name("out-dir").long("out-dir").takes_value(true))
        .arg(
            Arg::with_name("extern")
                .long("extern")
                .takes_value(true)
                .multiple(true)
                .number_of_values(1),
        )
        .arg(
            Arg::with_name("cfg")
                .long("cfg")
                .takes_value(true)
                .multiple(true)
                .number_of_values(1),
        )
        .arg(
            Arg::with_name("codegen")
                .short("C")
                .long("codegen")
                .takes_value(true)
                .multiple(true)
                .number_of_values(1),
        )
        .arg(
            Arg::with_name("L")
                .short("L")
                .takes_value(true)
                .multiple(true)
                .number_of_values(1),
        )
        .arg(
            Arg::with_name("o")
                .short("o")
                .takes_value(true)
                .number_of_values(1),
        )
        .arg(Arg::with_name("O").short("O"))
        .arg(Arg::with_name("test").long("test"))
        .arg(Arg::with_name("color").long("color"))
}
