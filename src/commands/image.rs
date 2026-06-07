use crate::cli::ImageAction;
use crate::context::Context;
use crate::error::Result;

pub fn handle_image(action: ImageAction, ctx: &Context) -> Result<()> {
    let json = ctx.json;
    let quiet = ctx.quiet;
    // Image generation (plotters + custom SVG Krebs cycle) is planned for a later phase.
    // For the core skeleton we accept the commands and document the intent.
    let (name, extra) = match action {
        ImageAction::KrebsCycle {
            date_range,
            output,
            theme,
        } => (
            "image krebs-cycle",
            format!("range={}, theme={}, out={:?}", date_range, theme, output),
        ),
        ImageAction::TrainingLoadHeatmap { period, output } => (
            "image training-load-heatmap",
            format!("period={}, out={:?}", period, output),
        ),
        ImageAction::AntioxidantShield { style, output } => (
            "image antioxidant-shield",
            format!("style={}, out={:?}", style, output),
        ),
        ImageAction::FullDashboard {
            date,
            layout,
            output,
        } => (
            "image full-dashboard",
            format!("date={}, layout={}, out={:?}", date, layout, output),
        ),
    };

    if json {
        println!(
            "{}",
            serde_json::json!({ "success": true, "command": name, "detail": extra, "note": "image generation not implemented in skeleton phase; will use plotters + custom SVG" })
        );
    } else if !quiet {
        println!("{} — {}", name, extra);
        println!("(image generation is a later milestone; see spec/01-spec.md)");
    }
    Ok(())
}
