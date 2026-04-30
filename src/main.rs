use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use video_gen::build::resolve_output_path;
use video_gen::log::{emit_error, emit_event};
use video_gen::{
    build_video, BinaryOptions, BuildOptions, EncodeOptions, LogMode, PlanOptions, VideoGenError,
};

#[derive(Debug, Parser)]
#[command(
    name = "video-gen",
    about = "image+audio -> mp4 video composer",
    subcommand_required = true,
    arg_required_else_help = true
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[command(about = "compose images and audio into a single mp4")]
    Build(BuildArgs),
}

#[derive(Debug, Args)]
struct BuildArgs {
    #[arg(short = 'd', long = "input-dir", default_value = "input")]
    input_dir: PathBuf,

    #[arg(short = 'o', long = "output")]
    output: Option<PathBuf>,

    #[arg(long = "lead-in", default_value = "0")]
    lead_in: String,

    #[arg(long = "tail", default_value = "0")]
    tail: String,

    #[arg(long = "gap", default_value = "0")]
    gap: String,

    #[arg(long = "fps", default_value = "30")]
    fps: String,

    #[arg(long = "crf", default_value = "20")]
    crf: String,

    #[arg(long = "preset", default_value = "medium")]
    preset: String,

    #[arg(long = "audio-bitrate", default_value = "192k")]
    audio_bitrate: String,

    #[arg(long = "keep-temp")]
    keep_temp: bool,

    #[arg(long = "quiet")]
    quiet: bool,

    #[arg(long = "json")]
    json: bool,
}

fn absolute_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}

fn parse_seconds(label: &str, value: &str) -> Result<u64, VideoGenError> {
    let seconds = value.parse::<f64>().map_err(|_| {
        VideoGenError::user(format!(
            "--{label}: expected a non-negative number, got \"{value}\""
        ))
    })?;
    if !seconds.is_finite() || seconds < 0.0 {
        return Err(VideoGenError::user(format!(
            "--{label}: expected a non-negative number, got \"{value}\""
        )));
    }
    Ok((seconds * 1000.0).round() as u64)
}

fn parse_integer(label: &str, value: &str, min: u32, max: u32) -> Result<u32, VideoGenError> {
    let parsed = value.parse::<u32>().map_err(|_| {
        VideoGenError::user(format!(
            "--{label}: expected integer in [{min},{max}], got \"{value}\""
        ))
    })?;
    if parsed < min || parsed > max {
        return Err(VideoGenError::user(format!(
            "--{label}: expected integer in [{min},{max}], got \"{value}\""
        )));
    }
    Ok(parsed)
}

fn mode(args: &BuildArgs) -> LogMode {
    if args.json {
        LogMode::Json
    } else if args.quiet {
        LogMode::Quiet
    } else {
        LogMode::Text
    }
}

fn run_build(args: BuildArgs) -> Result<(), (LogMode, VideoGenError)> {
    let mode = mode(&args);
    let input_dir = absolute_path(&args.input_dir);
    let output_path = resolve_output_path(args.output.as_deref(), &input_dir);
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let plan = PlanOptions {
        lead_in_ms: parse_seconds("lead-in", &args.lead_in).map_err(|err| (mode, err))?,
        tail_ms: parse_seconds("tail", &args.tail).map_err(|err| (mode, err))?,
        gap_ms: parse_seconds("gap", &args.gap).map_err(|err| (mode, err))?,
    };
    let encode = EncodeOptions {
        fps: parse_integer("fps", &args.fps, 1, 240).map_err(|err| (mode, err))?,
        crf: parse_integer("crf", &args.crf, 0, 51)
            .map_err(|err| (mode, err))?
            .try_into()
            .expect("crf is in u8 range"),
        preset: args.preset,
        audio_bitrate: args.audio_bitrate,
    };

    let options = BuildOptions {
        input_dir,
        output_path,
        work_dir: cwd,
        keep_temp: args.keep_temp,
        plan,
        encode,
        binaries: BinaryOptions::default(),
    };

    build_video(options, |event| emit_event(mode, &event)).map_err(|err| (mode, err))?;
    Ok(())
}

fn run(cli: Cli) -> Result<(), (LogMode, VideoGenError)> {
    match cli.command {
        Commands::Build(args) => run_build(args),
    }
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err((mode, err)) => {
            emit_error(mode, err.message(), err.exit_code());
            ExitCode::from(err.exit_code() as u8)
        }
    }
}
