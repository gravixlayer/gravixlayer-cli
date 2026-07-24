// src/cmd/package.rs — Package an agent project into a tar.gz archive.

use crate::cli::PackageArgs;
use crate::ctx::AppContext;
use crate::output;
use crate::scaffold::archive;

pub async fn handle(ctx: &AppContext, args: PackageArgs) -> anyhow::Result<()> {
    let source = args.source.canonicalize()?;
    if !source.is_dir() {
        anyhow::bail!("source is not a directory: {}", source.display());
    }

    let extra_excludes = ctx
        .project
        .as_ref()
        .map(|p| p.exclude.clone())
        .unwrap_or_default();

    if args.dry_run {
        let size = archive::estimate_archive_size(&source, &extra_excludes)?;
        output::info(
            ctx.output,
            format!(
                "Estimated uncompressed archive size: {:.1} KB ({size} bytes)",
                size as f64 / 1024.0
            ),
        );
        return Ok(());
    }

    let spinner = output::Spinner::new("Packaging source archive…");
    let bytes = archive::create_source_archive(&source, &extra_excludes)?;
    drop(spinner);

    let out_path = args.output.unwrap_or_else(|| {
        source
            .file_name()
            .map(|n| std::path::PathBuf::from(format!("{}.tar.gz", n.to_string_lossy())))
            .unwrap_or_else(|| std::path::PathBuf::from("archive.tar.gz"))
    });

    std::fs::write(&out_path, &bytes)?;
    output::success(
        ctx.output,
        format!(
            "Archive written to {} ({:.1} KB)",
            out_path.display(),
            bytes.len() as f64 / 1024.0
        ),
    );
    Ok(())
}
