use crate::app;
use bevy::window::WindowResolution;
use clap::ArgAction;
use std::str::FromStr;

#[derive(clap::Parser)]
pub(crate) struct Cli {
    #[clap(subcommand)]
    command: CliCommand,
}

impl Cli {
    pub(crate) fn run(self) {
        self.command.run();
    }
}

#[derive(clap::Subcommand)]
enum CliCommand {
    Dump,
    Run {
        #[clap(short, long)]
        resolution: Option<Resolution>,
        #[clap(long, action = ArgAction::SetTrue)]
        inspector: bool,
    },
}

#[derive(Debug, Copy, Clone)]
struct Resolution {
    width: f32,
    height: f32,
}

impl FromStr for Resolution {
    type Err = ResolutionInvalidFormat;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (w, h) = s.split_once('x').ok_or(ResolutionInvalidFormat)?;
        let width = w.parse::<f32>().map_err(|_| ResolutionInvalidFormat)?;
        let height = h.parse::<f32>().map_err(|_| ResolutionInvalidFormat)?;
        Ok(Self { width, height })
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Invalid format for resolution; expected <width>x<height>")]
struct ResolutionInvalidFormat;

impl CliCommand {
    fn run(self) {
        match self {
            Self::Dump => {
                let mut app = app::get_app(Default::default());
                bevy_mod_debugdump::print_render_graph(&mut app);
            }
            Self::Run {
                resolution,
                inspector,
            } => {
                app::get_app(app::AppSettings {
                    resolution: resolution
                        .map(|r| WindowResolution::new(r.width, r.height))
                        .unwrap_or_default(),
                    with_inspector: inspector,
                })
                .run();
            }
        }
    }
}
