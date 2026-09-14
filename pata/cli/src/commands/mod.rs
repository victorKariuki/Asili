pub mod jaribu;
pub mod jenga;
pub mod mwalimu;
pub mod nadhifu;
pub mod njozi;
pub mod ongeza;
pub mod repl;
pub mod tenda;
pub mod thibitisha;

#[derive(Debug)]
pub struct CliError {
    pub message: String,
    pub exit_code: i32,
}

impl CliError {
    pub fn new(message: impl Into<String>, exit_code: i32) -> Self {
        Self {
            message: message.into(),
            exit_code,
        }
    }
}

pub type CliResult = Result<(), CliError>;

#[cfg(test)]
pub static TEST_CWD_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn dispatch(args: &[String]) -> CliResult {
    let Some((command, rest)) = args.split_first() else {
        return Err(CliError::new(usage(), 2));
    };

    match command.as_str() {
        "njozi" => njozi::run(rest),
        "ongeza" => ongeza::run(rest),
        "jenga" => jenga::run(rest),
        "jaribu" => jaribu::run(rest),
        "mwalimu" => mwalimu::run(rest),
        "nadhifu" => nadhifu::run(rest),
        "repl" => repl::run(rest),
        "tenda" => tenda::run(rest),
        "thibitisha" => thibitisha::run(rest),
        "--msaada" | "msaada" => {
            println!("{}", usage());
            Ok(())
        }
        _ => Err(CliError::new(format!("amri isiyotambuliwa: {command}"), 2)),
    }
}

fn usage() -> &'static str {
    r#"matumizi: pata <amri> [chagua...] [hoja...]

Amri ni kitendo unachotaka kufanya. Chagua na hoja hutofautiana kwa kila amri.

Amri:
  jenga [faili.as] [chagua...]  Jenga mradi (kutoka pata.toml) au faili moja.
  tenda <path.asb|manifest>      Tenda kilele bila kujenga upya.
  jaribu [chagua...]            Endesha majaribio (#[jaribio]).
  mwalimu                      Anza seva ya LSP (Mwalimu).
  repl                         Fungua REPL.
  njozi [jina]                 Unda mradi mpya.
  ongeza <lib> [chagua...]     Ongeza tegemezi.
  nadhifu [chagua...]          Nadhifisha chanzo.
  thibitisha [chagua...]       Thibitisha mradi.

Onyesha msaada kwa amri: pata <amri> --msaada.
"#
}
